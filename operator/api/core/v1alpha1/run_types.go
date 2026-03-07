// File: operator/api/core/v1alpha1/run_types.go
// Purpose: Defines the Run CRD schema for Kaigents.
// Product/business importance: Runs capture execution requests and the status/timeline artifact references.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// RunTargetRef identifies the target resource for a run.
type RunTargetRef struct {
	Kind string `json:"kind"`
	Name string `json:"name"`
}

// RunSpec defines the desired run request.
type RunSpec struct {
	Target RunTargetRef `json:"target"`
	Input  string       `json:"input,omitempty"`
}

// RunArtifactRef describes a produced artifact reference for a run.
type RunArtifactRef struct {
	Name        string `json:"name"`
	ContentType string `json:"contentType,omitempty"`
	URI         string `json:"uri"`
	SizeBytes   int64  `json:"sizeBytes,omitempty"`
	SHA256      string `json:"sha256,omitempty"`
}

// RunStatus defines the observed state of a run.
type RunStatus struct {
	ObservedGeneration int64            `json:"observedGeneration,omitempty"`
	Phase              string           `json:"phase,omitempty"`
	Message            string           `json:"message,omitempty"`
	Artifacts          []RunArtifactRef `json:"artifacts,omitempty"`
	Conditions         []Condition      `json:"conditions,omitempty"`
}

// Run is the schema for Kaigents Run resources.
type Run struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   RunSpec   `json:"spec,omitempty"`
	Status RunStatus `json:"status,omitempty"`
}

// RunList contains a list of Run resources.
type RunList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []Run `json:"items"`
}

// DeepCopyObject copies the Run for runtime.Object.
func (in *Run) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(Run)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	if in.Status.Artifacts != nil {
		out.Status.Artifacts = append([]RunArtifactRef(nil), in.Status.Artifacts...)
	}
	return out
}

// DeepCopyObject copies the RunList for runtime.Object.
func (in *RunList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(RunList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]Run, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*Run)
		}
	}
	return out
}
