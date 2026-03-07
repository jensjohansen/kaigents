// File: operator/api/core/v1alpha1/team_types.go
// Purpose: Defines the Team CRD schema for Kaigents.
// Product/business importance: Teams compose agents with per-team prompt overrides.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// TeamAgentBinding connects an Agent to a Team with optional overrides.
type TeamAgentBinding struct {
	Name                 string `json:"name"`
	AgentRef             string `json:"agentRef"`
	SystemPromptOverride string `json:"systemPromptOverride,omitempty"`
	ContextPrompt        string `json:"contextPrompt,omitempty"`
}

// TeamSpec defines the desired state of a Team.
type TeamSpec struct {
	Agents []TeamAgentBinding `json:"agents,omitempty"`
}

// TeamStatus defines the observed state of a Team.
type TeamStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Phase              string      `json:"phase,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

// Team is the schema for Kaigents Team resources.
type Team struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   TeamSpec   `json:"spec,omitempty"`
	Status TeamStatus `json:"status,omitempty"`
}

// TeamList contains a list of Team resources.
type TeamList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []Team `json:"items"`
}

// DeepCopyObject copies the Team for runtime.Object.
func (in *Team) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(Team)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	if in.Spec.Agents != nil {
		out.Spec.Agents = append([]TeamAgentBinding(nil), in.Spec.Agents...)
	}
	return out
}

// DeepCopyObject copies the TeamList for runtime.Object.
func (in *TeamList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(TeamList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]Team, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*Team)
		}
	}
	return out
}
