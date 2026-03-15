// File: operator/api/core/v1alpha1/agent_types.go
// Purpose: Defines the Agent CRD schema for Kaigents.
// Product/business importance: Agents are the reusable execution units for teams and runs.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// AgentToolRef references a Tool resource by name.
type AgentToolRef struct {
	Name string `json:"name"`
}

// AgentSpec defines the desired state of an Agent.
type AgentSpec struct {
	Runtime          string         `json:"runtime,omitempty"`
	SystemPrompt     string         `json:"systemPrompt,omitempty"`
	Tools            []AgentToolRef `json:"tools,omitempty"`
	ModelEndpointRef string         `json:"modelEndpointRef,omitempty"`
	ModelName        string         `json:"modelName,omitempty"`
}

// ConditionType names for Kaigents resources.
type ConditionType string

const (
	// ConditionReady indicates the resource is ready for use.
	ConditionReady ConditionType = "Ready"
	// ConditionValidationFailed indicates the resource spec failed validation.
	ConditionValidationFailed ConditionType = "ValidationFailed"
	// ConditionReconcileError indicates an error during reconciliation.
	ConditionReconcileError ConditionType = "ReconcileError"
)

// Condition expresses the state of a resource at a point in time.
type Condition struct {
	// Type indicates the condition type.
	Type ConditionType `json:"type"`
	// Status indicates the condition status (True/False/Unknown).
	Status string `json:"status"`
	// LastTransitionTime is the last time this condition transitioned.
	LastTransitionTime string `json:"lastTransitionTime,omitempty"`
	// Reason is a machine-readable reason for the condition.
	Reason string `json:"reason,omitempty"`
	// Message is a human-readable message.
	Message string `json:"message,omitempty"`
}

// AgentStatus defines the observed state of an Agent.
type AgentStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Phase              string      `json:"phase,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

// Agent is the schema for Kaigents Agent resources.
type Agent struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   AgentSpec   `json:"spec,omitempty"`
	Status AgentStatus `json:"status,omitempty"`
}

// AgentList contains a list of Agent resources.
type AgentList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []Agent `json:"items"`
}

// DeepCopyObject copies the Agent for runtime.Object.
func (in *Agent) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(Agent)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	if in.Spec.Tools != nil {
		out.Spec.Tools = append([]AgentToolRef(nil), in.Spec.Tools...)
	}
	return out
}

// DeepCopyObject copies the AgentList for runtime.Object.
func (in *AgentList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(AgentList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]Agent, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*Agent)
		}
	}
	return out
}
