// File: operator/api/core/v1alpha1/task_types.go
// Purpose: Defines the Task CRD schema for Kaigents.
// Product/business importance: Tasks are the reusable building blocks for agent processes.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// TaskSpec defines the desired state of a Task.
type TaskSpec struct {
	// AgentName is the name of the Agent that should execute this task.
	AgentName string `json:"agentName,omitempty"`
	// Prompt is the specific instruction for this task.
	Prompt string `json:"prompt,omitempty"`
	// RequiresGate indicates if this task requires human approval before proceeding.
	RequiresGate bool `json:"requiresGate,omitempty"`
	// Metadata contains arbitrary key-value pairs for the task.
	Metadata map[string]string `json:"metadata,omitempty"`
}

// TaskStatus defines the observed state of a Task.
type TaskStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

// Task is the schema for Kaigents Task resources.
type Task struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   TaskSpec   `json:"spec,omitempty"`
	Status TaskStatus `json:"status,omitempty"`
}

// TaskList contains a list of Task resources.
type TaskList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []Task `json:"items"`
}

// DeepCopyObject copies the Task for runtime.Object.
func (in *Task) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(Task)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	if in.Spec.Metadata != nil {
		out.Spec.Metadata = make(map[string]string)
		for k, v := range in.Spec.Metadata {
			out.Spec.Metadata[k] = v
		}
	}
	return out
}

// DeepCopyObject copies the TaskList for runtime.Object.
func (in *TaskList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(TaskList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]Task, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*Task)
		}
	}
	return out
}
