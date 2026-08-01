// File: operator/internal/controllers/execution_contract.go
// Purpose: Defines the structured execution contract passed to the runner.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package controllers

// ExecutionContract defines the structured data passed from the operator to the runner.
type ExecutionContract struct {
	RunID             string `json:"runId"`
	RunName           string `json:"runName"`
	TargetKind        string `json:"targetKind"`
	TargetName        string `json:"targetName"`
	Input             string `json:"input"`
	ModelEndpointURL  string `json:"modelEndpointUrl,omitempty"`
	ModelName         string `json:"modelName,omitempty"`
	ModelEndpointName string `json:"modelEndpointName,omitempty"`
	SystemPrompt      string `json:"systemPrompt,omitempty"`
	MCPServerURL      string `json:"mcpServerUrl,omitempty"`
	MCPServerName     string `json:"mcpServerName,omitempty"`
	SearchToolName    string `json:"searchToolName,omitempty"`
	ReadToolName      string `json:"readToolName,omitempty"`
	ContextWindowSize uint32 `json:"contextWindowSize,omitempty"`
}
