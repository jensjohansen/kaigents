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
	"strings"
	"time"

	"go.temporal.io/sdk/activity"
	"go.temporal.io/sdk/log"
)

const TaskQueue = "kaigents-workrequest"

// WorkItemInput describes a unit of work to execute.
// Temporal internals must not leak into this type.
type WorkItemInput struct {
	WorkItemID       string            `json:"workItemId"`
	StepName         string            `json:"stepName"`
	AgentName        string            `json:"agentName,omitempty"`
	Prompt           string            `json:"prompt,omitempty"`
	SystemPrompt     string            `json:"systemPrompt,omitempty"`
	ModelEndpointURL string            `json:"modelEndpointUrl,omitempty"`
	ModelName        string            `json:"modelName,omitempty"`
	MCPServerURL     string            `json:"mcpServerUrl,omitempty"`
	SearchToolName   string            `json:"searchToolName,omitempty"`
	ReadToolName     string            `json:"readToolName,omitempty"`
	Metadata         map[string]string `json:"metadata,omitempty"`
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
		"agentName", input.AgentName,
		"attempt", attempt,
	)

	started := time.Now().UTC()

	result := WorkItemResult{
		WorkItemID: input.WorkItemID,
		StartedAt:  started,
		Metadata:   map[string]string{"attempt": fmt.Sprintf("%d", attempt)},
	}

	if input.Prompt == "" {
		result.Output = fmt.Sprintf("step=%s workItemId=%s attempt=%d completed (no prompt)", input.StepName, input.WorkItemID, attempt)
		result.Status = "Succeeded"
		result.FinishedAt = time.Now().UTC()
		return result, nil
	}

	enhancedPrompt := input.Prompt

	if input.MCPServerURL != "" && input.SearchToolName != "" {
		searchResults, err := performWebSearch(ctx, input.MCPServerURL, input.SearchToolName, input.ReadToolName, input.Prompt, logger)
		if err != nil {
			logger.Warn("web search failed, continuing without search results", "error", err)
		} else if searchResults != "" {
			enhancedPrompt = fmt.Sprintf("%s\n\n--- Web search results (already fetched; do not attempt to call search tools) ---\n%s\n\nBased on the above search results, synthesize your response. The web search has already been performed for you.", input.Prompt, searchResults)
			input.SystemPrompt = input.SystemPrompt + "\n\nIMPORTANT: Web search and page reading have already been performed for you. The results are included in the user message below. Do NOT attempt to call searxng_web_search or web_url_read tools. Synthesize your response directly from the provided search results."
		}
	}

	output, err := callModel(ctx, input, enhancedPrompt, logger)
	if err != nil {
		result.Status = "Failed"
		result.ErrorMsg = err.Error()
		result.FinishedAt = time.Now().UTC()
		return result, err
	}

	result.Output = output
	result.Status = "Succeeded"
	result.FinishedAt = time.Now().UTC()

	logger.Info("WorkItem completed", "workItemId", input.WorkItemID, "status", result.Status, "outputLen", len(output))
	return result, nil
}

func performWebSearch(ctx context.Context, mcpServerURL, searchToolName, readToolName, prompt string, logger log.Logger) (string, error) {
	mcpClient := NewMcpClient(mcpServerURL)

	query := extractSearchQuery(prompt)
	if query == "" {
		return "", nil
	}

	logger.Info("performing web search", "query", query, "mcpServer", mcpServerURL)

	searchOutput, err := mcpClient.CallTool(ctx, searchToolName, map[string]interface{}{
		"query":  query,
		"pageno": 1,
	})
	if err != nil {
		return "", fmt.Errorf("web search failed: %w", err)
	}

	urls := extractURLs(searchOutput)
	if len(urls) == 0 {
		return searchOutput, nil
	}

	if readToolName == "" {
		return searchOutput, nil
	}

	var sourceTexts []string
	for _, url := range urls {
		readOutput, err := mcpClient.CallTool(ctx, readToolName, map[string]interface{}{
			"url": url,
		})
		if err != nil {
			logger.Warn("failed to read URL", "url", url, "error", err)
			continue
		}
		sourceTexts = append(sourceTexts, fmt.Sprintf("URL: %s\n%s", url, truncate(readOutput, 4000)))
	}

	if len(sourceTexts) == 0 {
		return searchOutput, nil
	}

	return fmt.Sprintf("%s\n\n--- Page contents ---\n%s", searchOutput, strings.Join(sourceTexts, "\n\n")), nil
}

func extractSearchQuery(prompt string) string {
	for _, line := range strings.Split(prompt, "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" {
			return trimmed
		}
	}
	return prompt
}

func extractURLs(jsonText string) []string {
	var urls []string
	var parsed map[string]interface{}
	if err := json.Unmarshal([]byte(jsonText), &parsed); err == nil {
		if results, ok := parsed["results"].([]interface{}); ok {
			for _, item := range results {
				if m, ok := item.(map[string]interface{}); ok {
					if url, ok := m["url"].(string); ok {
						urls = append(urls, url)
						if len(urls) >= 3 {
							break
						}
					}
				}
			}
		}
	}
	return urls
}

func callModel(ctx context.Context, input WorkItemInput, prompt string, logger log.Logger) (string, error) {
	url := input.ModelEndpointURL
	if url == "" {
		url = os.Getenv("KAIGENTS_MODEL_ENDPOINT_URL")
	}
	if url == "" {
		return "", fmt.Errorf("no model endpoint URL configured (neither per-step nor env var)")
	}

	modelName := input.ModelName
	if modelName == "" {
		modelName = os.Getenv("KAIGENTS_MODEL_NAME")
	}
	if modelName == "" {
		modelName = "gpt-oss-20b"
	}
	apiKey := os.Getenv("KAIGENTS_MODEL_API_KEY")

	var messages []map[string]string
	if input.SystemPrompt != "" {
		messages = append(messages, map[string]string{"role": "system", "content": input.SystemPrompt})
	}
	messages = append(messages, map[string]string{"role": "user", "content": prompt})

	reqBody, _ := json.Marshal(map[string]interface{}{
		"model":       modelName,
		"messages":    messages,
		"max_tokens":  2048,
		"temperature": 0.4,
	})

	endpointURL := strings.TrimSuffix(url, "/")
	chatPath := "/v1/chat/completions"
	if strings.HasSuffix(endpointURL, "/v1") {
		chatPath = "/chat/completions"
	}

	req, err := http.NewRequestWithContext(ctx, "POST", endpointURL+chatPath, bytes.NewBuffer(reqBody))
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
			FinishReason string `json:"finish_reason"`
			Message      struct {
				Content          string `json:"content"`
				ReasoningContent string `json:"reasoning_content"`
			} `json:"message"`
		} `json:"choices"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&res); err != nil {
		return "", err
	}

	if len(res.Choices) == 0 {
		return "", fmt.Errorf("no choices returned from model")
	}

	content := res.Choices[0].Message.Content
	if content == "" && res.Choices[0].Message.ReasoningContent != "" {
		logger.Info("model returned empty content, using reasoning_content as fallback",
			"finish_reason", res.Choices[0].FinishReason,
			"reasoning_content_len", len(res.Choices[0].Message.ReasoningContent))
		content = res.Choices[0].Message.ReasoningContent
	}

	return content, nil
}
