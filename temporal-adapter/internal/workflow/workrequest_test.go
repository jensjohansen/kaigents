// File: kaigents/temporal-adapter/internal/workflow/workrequest_test.go
// Purpose: Unit tests for WorkRequest domain types, JSON marshaling, and workflow constants.
// Product/business importance: Confirms that the Kaigents domain types round-trip through JSON
// correctly, which is critical since they cross the HTTP boundary between the Rust engine and
// the Go adapter without any Temporal types leaking.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package workflow_test

import (
	"encoding/json"
	"testing"
	"time"

	"github.com/jensjohansen/kaigents/temporal-adapter/internal/workflow"
)

func TestWorkItemDef_JSONRoundTrip(t *testing.T) {
	original := workflow.WorkItemDef{
		WorkItemID:   "wi-001",
		StepName:     "research",
		AgentName:    "researcher-agent",
		Prompt:       "summarize the topic",
		RequiresGate: true,
		Metadata:     map[string]string{"priority": "high"},
	}

	encoded, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}

	var decoded workflow.WorkItemDef
	if err := json.Unmarshal(encoded, &decoded); err != nil {
		t.Fatalf("unmarshal failed: %v", err)
	}

	if decoded.WorkItemID != original.WorkItemID {
		t.Errorf("WorkItemID mismatch: got %q, want %q", decoded.WorkItemID, original.WorkItemID)
	}
	if decoded.StepName != original.StepName {
		t.Errorf("StepName mismatch: got %q, want %q", decoded.StepName, original.StepName)
	}
	if decoded.RequiresGate != original.RequiresGate {
		t.Errorf("RequiresGate mismatch: got %v, want %v", decoded.RequiresGate, original.RequiresGate)
	}
	if decoded.Metadata["priority"] != "high" {
		t.Errorf("Metadata mismatch: got %v", decoded.Metadata)
	}
}

func TestWorkItemDef_JSONFieldNames(t *testing.T) {
	item := workflow.WorkItemDef{WorkItemID: "wi-1", StepName: "step-1"}
	encoded, _ := json.Marshal(item)
	raw := map[string]interface{}{}
	_ = json.Unmarshal(encoded, &raw)

	if _, ok := raw["workItemId"]; !ok {
		t.Error("expected camelCase 'workItemId' in JSON output")
	}
	if _, ok := raw["stepName"]; !ok {
		t.Error("expected camelCase 'stepName' in JSON output")
	}
}

func TestWorkRequestInput_JSONRoundTrip(t *testing.T) {
	original := workflow.WorkRequestInput{
		WorkRequestID: "wr-abc123",
		ProcessName:   "onboarding",
		Steps: []workflow.WorkItemDef{
			{WorkItemID: "s1", StepName: "gather"},
			{WorkItemID: "s2", StepName: "review", RequiresGate: true},
		},
	}

	encoded, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}

	var decoded workflow.WorkRequestInput
	if err := json.Unmarshal(encoded, &decoded); err != nil {
		t.Fatalf("unmarshal failed: %v", err)
	}

	if decoded.WorkRequestID != original.WorkRequestID {
		t.Errorf("WorkRequestID mismatch: got %q, want %q", decoded.WorkRequestID, original.WorkRequestID)
	}
	if len(decoded.Steps) != 2 {
		t.Errorf("expected 2 steps, got %d", len(decoded.Steps))
	}
	if decoded.Steps[1].RequiresGate != true {
		t.Error("expected second step to have RequiresGate=true")
	}
}

func TestWorkRequestState_JSONRoundTrip(t *testing.T) {
	now := time.Now().UTC().Truncate(time.Second)
	original := workflow.WorkRequestState{
		WorkRequestID: "wr-xyz",
		Phase:         "WaitingForApproval",
		CurrentStep:   "review",
		ReworkCount:   1,
		Message:       "awaiting human gate",
		StartedAt:     now,
		UpdatedAt:     now,
	}

	encoded, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}

	var decoded workflow.WorkRequestState
	if err := json.Unmarshal(encoded, &decoded); err != nil {
		t.Fatalf("unmarshal failed: %v", err)
	}

	if decoded.Phase != "WaitingForApproval" {
		t.Errorf("Phase mismatch: got %q", decoded.Phase)
	}
	if decoded.ReworkCount != 1 {
		t.Errorf("ReworkCount mismatch: got %d", decoded.ReworkCount)
	}
}

func TestApprovalSignal_JSONRoundTrip(t *testing.T) {
	original := workflow.ApprovalSignal{Approved: false, Comment: "needs revision"}

	encoded, _ := json.Marshal(original)
	var decoded workflow.ApprovalSignal
	_ = json.Unmarshal(encoded, &decoded)

	if decoded.Approved != false {
		t.Error("expected Approved=false")
	}
	if decoded.Comment != "needs revision" {
		t.Errorf("Comment mismatch: got %q", decoded.Comment)
	}
}

func TestWorkflowConstants(t *testing.T) {
	if workflow.MaxReworkAttempts <= 0 {
		t.Errorf("MaxReworkAttempts must be positive, got %d", workflow.MaxReworkAttempts)
	}
	if workflow.SignalApprove == "" {
		t.Error("SignalApprove must not be empty")
	}
	if workflow.SignalRework == "" {
		t.Error("SignalRework must not be empty")
	}
	if workflow.SignalApprove == workflow.SignalRework {
		t.Error("SignalApprove and SignalRework must be distinct")
	}
}
