// File: operator/internal/controllers/reconcile_test.go
// Purpose: Unit tests for controller status reconciliation behavior.
// Product/business importance: validates deterministic status updates for CRDs.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package controllers

import (
	"context"
	"testing"

	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
	"k8s.io/apimachinery/pkg/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"
)

func TestAgentReconcilerSetsObservedGenerationAndPhase(t *testing.T) {
	scheme := runtime.NewScheme()
	if err := clientgoscheme.AddToScheme(scheme); err != nil {
		t.Fatalf("add client-go scheme: %v", err)
	}
	if err := corev1alpha1.AddToScheme(scheme); err != nil {
		t.Fatalf("add kaigents scheme: %v", err)
	}

	agent := &corev1alpha1.Agent{}
	agent.Name = "agent-1"
	agent.Namespace = "default"
	agent.Generation = 7

	c := fake.NewClientBuilder().WithScheme(scheme).WithStatusSubresource(agent).WithObjects(agent).Build()

	r := &AgentReconciler{Client: c}
	_, err := r.Reconcile(context.Background(), ctrlRequest("default", "agent-1"))
	if err != nil {
		t.Fatalf("reconcile: %v", err)
	}

	updated := &corev1alpha1.Agent{}
	if err := c.Get(context.Background(), objKey("default", "agent-1"), updated); err != nil {
		t.Fatalf("get updated: %v", err)
	}

	if updated.Status.ObservedGeneration != 7 {
		t.Fatalf("expected observedGeneration=7, got %d", updated.Status.ObservedGeneration)
	}
	if updated.Status.Phase != "Ready" {
		t.Fatalf("expected phase=Ready, got %q", updated.Status.Phase)
	}
}

func objKey(namespace, name string) client.ObjectKey {
	return client.ObjectKey{Namespace: namespace, Name: name}
}

func ctrlRequest(namespace, name string) ctrl.Request {
	return ctrl.Request{NamespacedName: objKey(namespace, name)}
}
