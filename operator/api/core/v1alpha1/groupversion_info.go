// File: operator/api/core/v1alpha1/groupversion_info.go
// Purpose: Registers the Kaigents core API group/version with the scheme.
// Product/business importance: enables controllers to decode Kaigents CRDs.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
)

// Group is the Kubernetes API group for Kaigents core resources.
const Group = "core.kaigents.io"

// Version is the Kubernetes API version for Kaigents core resources.
const Version = "v1alpha1"

var (
	// GroupVersion identifies the group/version for scheme registration.
	GroupVersion = schema.GroupVersion{Group: Group, Version: Version}

	// SchemeBuilder registers known types for this API group.
	SchemeBuilder = runtime.NewSchemeBuilder(addKnownTypes)
	// AddToScheme registers types into a runtime.Scheme.
	AddToScheme = SchemeBuilder.AddToScheme
)

func addKnownTypes(scheme *runtime.Scheme) error {
	scheme.AddKnownTypes(
		GroupVersion,
		&Agent{},
		&AgentList{},
		&Team{},
		&TeamList{},
		&Run{},
		&RunList{},
		&Tool{},
		&ToolList{},
		&MCPServer{},
		&MCPServerList{},
		&ModelEndpoint{},
		&ModelEndpointList{},
		&Task{},
		&TaskList{},
		&Process{},
		&ProcessList{},
	)

	metav1.AddToGroupVersion(scheme, GroupVersion)
	return nil
}
