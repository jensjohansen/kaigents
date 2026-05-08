// File: kaigents/temporal-adapter/internal/activity/workitem.go
// Purpose: Defines the Temporal activity that executes a Kaigents WorkItem, plus its input/result types.
// Product/business importance: Each WorkItem execution is the atomic unit of agent work. Retries here
// correspond to Kaigents WorkAttempts, giving operators per-step observability.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package activity

import (
	"context"
	"fmt"
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

	result.Output = fmt.Sprintf("step=%s workItemId=%s attempt=%d completed", input.StepName, input.WorkItemID, attempt)
	result.Status = "Succeeded"
	result.FinishedAt = time.Now().UTC()

	logger.Info("WorkItem completed", "workItemId", input.WorkItemID, "status", result.Status)
	return result, nil
}
