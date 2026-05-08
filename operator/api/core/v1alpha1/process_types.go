// File: operator/api/core/v1alpha1/process_types.go
// Purpose: Defines the Process CRD schema for Kaigents.
// Product/business importance: Processes define the graph of tasks that form an agent team workflow.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// ProcessStep defines a single step in a Process.
type ProcessStep struct {
	// ID is a unique identifier for this step within the process.
	ID string `json:"id"`
	// Name is a human-readable name for this step.
	Name string `json:"name"`
	// TaskRef references a Task resource.
	TaskRef string `json:"taskRef"`
}

// ProcessSpec defines the desired state of a Process.
type ProcessSpec struct {
	// Steps is the sequence of tasks to execute.
	Steps []ProcessStep `json:"steps"`
	// MaxReworkAttempts is the maximum number of times a rework can be requested.
	// Defaults to 3 if not specified.
	MaxReworkAttempts int `json:"maxReworkAttempts,omitempty"`
}

// ProcessStatus defines the observed state of a Process.
type ProcessStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

// Process is the schema for Kaigents Process resources.
type Process struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   ProcessSpec   `json:"spec,omitempty"`
	Status ProcessStatus `json:"status,omitempty"`
}

// ProcessList contains a list of Process resources.
type ProcessList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []Process `json:"items"`
}

// DeepCopyObject copies the Process for runtime.Object.
func (in *Process) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(Process)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	if in.Spec.Steps != nil {
		out.Spec.Steps = make([]ProcessStep, len(in.Spec.Steps))
		copy(out.Spec.Steps, in.Spec.Steps)
	}
	return out
}

// DeepCopyObject copies the ProcessList for runtime.Object.
func (in *ProcessList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(ProcessList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]Process, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*Process)
		}
	}
	return out
}
