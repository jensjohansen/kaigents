// File: operator/api/core/v1alpha1/tool_types.go
// Purpose: Defines the Tool CRD schema for Kaigents.
// Product/business importance: Tools represent registered connector capabilities.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// ToolSpec defines the desired state of a Tool registration.
type ToolSpec struct {
	MCPServerRef string `json:"mcpServerRef,omitempty"`
	ToolName     string `json:"toolName,omitempty"`
	Description  string `json:"description,omitempty"`
}

// ToolStatus defines the observed state of a Tool.
type ToolStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Phase              string      `json:"phase,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

// Tool is the schema for Kaigents Tool resources.
type Tool struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   ToolSpec   `json:"spec,omitempty"`
	Status ToolStatus `json:"status,omitempty"`
}

// ToolList contains a list of Tool resources.
type ToolList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []Tool `json:"items"`
}

// DeepCopyObject copies the Tool for runtime.Object.
func (in *Tool) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(Tool)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	return out
}

// DeepCopyObject copies the ToolList for runtime.Object.
func (in *ToolList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(ToolList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]Tool, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*Tool)
		}
	}
	return out
}
