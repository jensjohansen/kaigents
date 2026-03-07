// File: operator/api/core/v1alpha1/modelendpoint_types.go
// Purpose: Defines the ModelEndpoint CRD schema for Kaigents.
// Product/business importance: Model endpoints provide discovery for Lemonade/OpenAI-compatible serving.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// ModelEndpointCapabilities describes supported model operations.
type ModelEndpointCapabilities struct {
	Chat       bool `json:"chat,omitempty"`
	Embeddings bool `json:"embeddings,omitempty"`
	Rerank     bool `json:"rerank,omitempty"`
}

// ModelEndpointSpec defines the desired state of a model endpoint reference.
type ModelEndpointSpec struct {
	URL          string                    `json:"url,omitempty"`
	ServiceDNS   string                    `json:"serviceDns,omitempty"`
	Provider     string                    `json:"provider,omitempty"`
	Capabilities ModelEndpointCapabilities `json:"capabilities,omitempty"`
}

// ModelEndpointStatus defines the observed state of a model endpoint.
type ModelEndpointStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Phase              string      `json:"phase,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

// ModelEndpoint is the schema for Kaigents ModelEndpoint resources.
type ModelEndpoint struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   ModelEndpointSpec   `json:"spec,omitempty"`
	Status ModelEndpointStatus `json:"status,omitempty"`
}

// ModelEndpointList contains a list of ModelEndpoint resources.
type ModelEndpointList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []ModelEndpoint `json:"items"`
}

// DeepCopyObject copies the ModelEndpoint for runtime.Object.
func (in *ModelEndpoint) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(ModelEndpoint)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	return out
}

// DeepCopyObject copies the ModelEndpointList for runtime.Object.
func (in *ModelEndpointList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(ModelEndpointList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]ModelEndpoint, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*ModelEndpoint)
		}
	}
	return out
}
