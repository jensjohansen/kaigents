// File: kaigents/temporal-adapter/internal/workflow/workrequest.go
// Purpose: Defines the Temporal workflow that drives a Kaigents WorkRequest, including rework loops
// and human-in-the-loop approval gates. All Temporal SDK types are confined to this file.
// Product/business importance: WorkRequest is the central execution model for multi-step agent
// team processes. This workflow provides durable, observable, human-gateable execution.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package workflow

import (
	"fmt"
	"time"

	"go.temporal.io/sdk/temporal"
	"go.temporal.io/sdk/workflow"

	"github.com/jensjohansen/kaigents/temporal-adapter/internal/activity"
)

const (
	WorkRequestWorkflowType = "WorkRequestWorkflow"
	SignalApprove           = "kaigents.approve"
	SignalRework            = "kaigents.rework"
	QueryState              = "kaigents.state"
	MaxReworkAttempts       = 3
)

// WorkItemDef describes a step in the WorkRequest process graph.
// Kept as a simple, engine-agnostic struct — no Temporal types exposed.
type WorkItemDef struct {
	WorkItemID   string            `json:"workItemId"`
	StepName     string            `json:"stepName"`
	AgentName    string            `json:"agentName,omitempty"`
	Prompt       string            `json:"prompt,omitempty"`
	RequiresGate bool              `json:"requiresGate,omitempty"`
	Metadata     map[string]string `json:"metadata,omitempty"`
}

// WorkRequestInput is the workflow input — pure Kaigents domain, no Temporal types.
type WorkRequestInput struct {
	WorkRequestID string        `json:"workRequestId"`
	ProcessName   string        `json:"processName,omitempty"`
	Steps         []WorkItemDef `json:"steps"`
}

// WorkRequestState is the queryable Kaigents-domain state of a running WorkRequest.
type WorkRequestState struct {
	WorkRequestID string                    `json:"workRequestId"`
	Phase         string                    `json:"phase"`
	CurrentStep   string                    `json:"currentStep,omitempty"`
	ReworkCount   int                       `json:"reworkCount"`
	Results       []activity.WorkItemResult `json:"results,omitempty"`
	StartedAt     time.Time                 `json:"startedAt"`
	UpdatedAt     time.Time                 `json:"updatedAt"`
	Message       string                    `json:"message,omitempty"`
}

// ApprovalSignal carries an approval or rejection decision.
type ApprovalSignal struct {
	Approved bool   `json:"approved"`
	Comment  string `json:"comment,omitempty"`
}

// WorkRequestWorkflow is the Temporal workflow that drives a Kaigents WorkRequest.
// All Temporal SDK types are confined to this file; the state type is engine-agnostic.
func WorkRequestWorkflow(ctx workflow.Context, input WorkRequestInput) (WorkRequestState, error) {
	logger := workflow.GetLogger(ctx)
	state := WorkRequestState{
		WorkRequestID: input.WorkRequestID,
		Phase:         "Running",
		StartedAt:     workflow.Now(ctx),
		UpdatedAt:     workflow.Now(ctx),
	}

	workflow.SetQueryHandler(ctx, QueryState, func() (WorkRequestState, error) {
		return state, nil
	})

	activityOpts := workflow.ActivityOptions{
		StartToCloseTimeout: 30 * time.Minute,
		RetryPolicy: &temporal.RetryPolicy{
			MaximumAttempts: 3,
		},
	}
	actCtx := workflow.WithActivityOptions(ctx, activityOpts)

	approvalCh := workflow.GetSignalChannel(ctx, SignalApprove)
	reworkCh := workflow.GetSignalChannel(ctx, SignalRework)

	stepIdx := 0
	reworkCount := 0

	for stepIdx < len(input.Steps) {
		step := input.Steps[stepIdx]
		state.CurrentStep = step.StepName
		state.UpdatedAt = workflow.Now(ctx)
		logger.Info("Executing WorkItem", "step", step.StepName, "workItemId", step.WorkItemID)

		var result activity.WorkItemResult
		err := workflow.ExecuteActivity(actCtx, activity.ExecuteWorkItem, activity.WorkItemInput{
			WorkItemID: step.WorkItemID,
			StepName:   step.StepName,
			AgentName:  step.AgentName,
			Prompt:     step.Prompt,
			Metadata:   step.Metadata,
		}).Get(ctx, &result)

		if err != nil {
			state.Phase = "Failed"
			state.Message = fmt.Sprintf("step %q failed: %v", step.StepName, err)
			state.UpdatedAt = workflow.Now(ctx)
			return state, err
		}

		state.Results = append(state.Results, result)
		state.UpdatedAt = workflow.Now(ctx)

		if step.RequiresGate {
			state.Phase = "WaitingForApproval"
			state.Message = fmt.Sprintf("step %q completed; waiting for human approval", step.StepName)
			state.UpdatedAt = workflow.Now(ctx)
			logger.Info("Waiting for approval signal", "step", step.StepName)

			var decided bool
			for !decided {
				workflow.NewSelector(ctx).
					AddReceive(approvalCh, func(ch workflow.ReceiveChannel, more bool) {
						var sig ApprovalSignal
						ch.Receive(ctx, &sig)
						if sig.Approved {
							state.Phase = "Running"
							state.Message = sig.Comment
							stepIdx++
							decided = true
						} else {
							reworkCount++
							state.ReworkCount = reworkCount
							if reworkCount > MaxReworkAttempts {
								state.Phase = "Failed"
								state.Message = fmt.Sprintf("max rework attempts (%d) exceeded at step %q", MaxReworkAttempts, step.StepName)
								decided = true
							} else {
								state.Phase = "Rework"
								state.Message = fmt.Sprintf("rework requested at step %q (attempt %d/%d): %s", step.StepName, reworkCount, MaxReworkAttempts, sig.Comment)
								stepIdx = 0
							}
							decided = true
						}
						state.UpdatedAt = workflow.Now(ctx)
					}).
					AddReceive(reworkCh, func(ch workflow.ReceiveChannel, more bool) {
						var sig ApprovalSignal
						ch.Receive(ctx, &sig)
						reworkCount++
						state.ReworkCount = reworkCount
						if reworkCount > MaxReworkAttempts {
							state.Phase = "Failed"
							state.Message = fmt.Sprintf("max rework attempts (%d) exceeded at step %q", MaxReworkAttempts, step.StepName)
						} else {
							state.Phase = "Rework"
							state.Message = fmt.Sprintf("rework signal at step %q (attempt %d/%d): %s", step.StepName, reworkCount, MaxReworkAttempts, sig.Comment)
							stepIdx = 0
						}
						state.UpdatedAt = workflow.Now(ctx)
						decided = true
					}).
					Select(ctx)
			}

			if state.Phase == "Failed" {
				return state, nil
			}
		} else {
			stepIdx++
		}
	}

	state.Phase = "Succeeded"
	state.CurrentStep = ""
	state.Message = "all steps completed"
	state.UpdatedAt = workflow.Now(ctx)
	return state, nil
}
