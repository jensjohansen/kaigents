// File: operator/api/core/v1alpha1/mcpserver_types.go
// Purpose: Defines the MCPServer CRD schema for Kaigents.
// Product/business importance: MCP servers represent tool-plane endpoints or kmcp references.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// MCPServerSpec defines the desired state of an MCPServer reference.
type MCPServerSpec struct {
	URL       string `json:"url,omitempty"`
	KMCPName  string `json:"kmcpName,omitempty"`
	Transport string `json:"transport,omitempty"`
}

// MCPServerStatus defines the observed state of an MCPServer.
type MCPServerStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Phase              string      `json:"phase,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

// MCPServer is the schema for Kaigents MCPServer resources.
type MCPServer struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   MCPServerSpec   `json:"spec,omitempty"`
	Status MCPServerStatus `json:"status,omitempty"`
}

// MCPServerList contains a list of MCPServer resources.
type MCPServerList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []MCPServer `json:"items"`
}

// DeepCopyObject copies the MCPServer for runtime.Object.
func (in *MCPServer) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(MCPServer)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	return out
}

// DeepCopyObject copies the MCPServerList for runtime.Object.
func (in *MCPServerList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(MCPServerList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]MCPServer, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*MCPServer)
		}
	}
	return out
}
