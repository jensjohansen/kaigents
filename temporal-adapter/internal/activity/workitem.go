// File: kaigents/temporal-adapter/internal/activity/workitem.go
// Purpose: Defines the Temporal activity that executes a Kaigents WorkItem, plus its input/result types.
// Product/business importance: Each WorkItem execution is the atomic unit of agent work. Retries here
// correspond to Kaigents WorkAttempts, giving operators per-step observability.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package activity

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"

	"go.temporal.io/sdk/activity"
)

const TaskQueue = "kaigents-workrequest"

// WorkItemInput describes a unit of work to execute.
// Temporal internals must not leak into this type.
type WorkItemInput struct {
	WorkItemID string            `json:"workItemId"`
	StepName   string            `json:"stepName"`
	AgentName  string            `json:"agentName,omitempty"`
	Prompt     string            `json:"prompt,omitempty"`
	Metadata   map[string]string `json:"metadata,omitempty"`
}

// WorkItemResult describes the outcome of a WorkItem execution.
type WorkItemResult struct {
	WorkItemID string            `json:"workItemId"`
	Status     string            `json:"status"`
	Output     string            `json:"output,omitempty"`
	ErrorMsg   string            `json:"errorMsg,omitempty"`
	StartedAt  time.Time         `json:"startedAt"`
	FinishedAt time.Time         `json:"finishedAt"`
	Metadata   map[string]string `json:"metadata,omitempty"`
}

// ExecuteWorkItem is the Temporal activity that represents a Kaigents WorkItem / WorkAttempt.
// Each attempt by Temporal's retry logic corresponds to a Kaigents WorkAttempt.
func ExecuteWorkItem(ctx context.Context, input WorkItemInput) (WorkItemResult, error) {
	logger := activity.GetLogger(ctx)
	info := activity.GetInfo(ctx)
	attempt := info.Attempt

	logger.Info("WorkItem started",
		"workItemId", input.WorkItemID,
		"stepName", input.StepName,
		"attempt", attempt,
	)

	started := time.Now().UTC()

	result := WorkItemResult{
		WorkItemID: input.WorkItemID,
		StartedAt:  started,
		Metadata:   map[string]string{"attempt": fmt.Sprintf("%d", attempt)},
	}

	if input.Prompt != "" {
		output, err := callModel(ctx, input.Prompt)
		if err != nil {
			result.Status = "Failed"
			result.ErrorMsg = err.Error()
			result.FinishedAt = time.Now().UTC()
			return result, nil
		}
		result.Output = output
	} else {
		result.Output = fmt.Sprintf("step=%s workItemId=%s attempt=%d completed (no prompt)", input.StepName, input.WorkItemID, attempt)
	}

	result.Status = "Succeeded"
	result.FinishedAt = time.Now().UTC()

	logger.Info("WorkItem completed", "workItemId", input.WorkItemID, "status", result.Status)
	return result, nil
}

func callModel(ctx context.Context, prompt string) (string, error) {
	url := os.Getenv("KAIGENTS_MODEL_ENDPOINT_URL")
	if url == "" {
		return "Model output placeholder (KAIGENTS_MODEL_ENDPOINT_URL not set)", nil
	}
	modelName := os.Getenv("KAIGENTS_MODEL_NAME")
	if modelName == "" {
		modelName = "gpt-oss-20b"
	}
	apiKey := os.Getenv("KAIGENTS_MODEL_API_KEY")

	reqBody, _ := json.Marshal(map[string]interface{}{
		"model": modelName,
		"messages": []map[string]string{
			{"role": "user", "content": prompt},
		},
		"max_tokens": 1024,
	})

	req, err := http.NewRequestWithContext(ctx, "POST", url+"/v1/chat/completions", bytes.NewBuffer(reqBody))
	if err != nil {
		return "", err
	}
	req.Header.Set("Content-Type", "application/json")
	if apiKey != "" {
		req.Header.Set("Authorization", "Bearer "+apiKey)
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return "", fmt.Errorf("model call failed (%d): %s", resp.StatusCode, string(body))
	}

	var res struct {
		Choices []struct {
			Message struct {
				Content string `json:"content"`
			} `json:"message"`
		} `json:"choices"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&res); err != nil {
		return "", err
	}

	if len(res.Choices) == 0 {
		return "", fmt.Errorf("no choices returned from model")
	}

	return res.Choices[0].Message.Content, nil
}

