// File: operator/internal/controllers/common.go
// Purpose: Shared reconcile helpers for controller status updates.
// Product/business importance: keeps status handling consistent across CRDs.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package controllers

import (
	"context"
	"time"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"

	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
)

// ReadyStatusSetter updates observed generation, phase, and conditions.
type ReadyStatusSetter interface {
	GetObservedGeneration() int64
	SetObservedGeneration(value int64)
	SetPhase(value string)
	SetConditions(value []corev1alpha1.Condition)
}

// Object is the minimal interface needed for status reconciliation.
type Object interface {
	client.Object
	GetGeneration() int64
}

// GetSetStatusFunc returns a setter that can update status for the object.
type GetSetStatusFunc[T Object] func(obj T) (ReadyStatusSetter, bool)

// ReconcileReadyFunc updates status to Ready for a given object.
type ReconcileReadyFunc[T Object] struct {
	Client       client.Client
	NewEmpty     func() T
	GetSetStatus GetSetStatusFunc[T]
}

// Reconcile fetches the object and writes a Ready status update with conditions and events.
func (r ReconcileReadyFunc[T]) Reconcile(ctx context.Context, namespacedName types.NamespacedName) error {
	obj := r.NewEmpty()
	if err := r.Client.Get(ctx, namespacedName, obj); err != nil {
		return client.IgnoreNotFound(err)
	}

	setter, ok := r.GetSetStatus(obj)
	if !ok {
		return nil
	}

	setter.SetObservedGeneration(obj.GetGeneration())
	setter.SetPhase("Ready")

	// Set Ready condition
	now := time.Now().Format(time.RFC3339)
	conditions := []corev1alpha1.Condition{
		{
			Type:               corev1alpha1.ConditionReady,
			Status:             "True",
			LastTransitionTime: now,
			Reason:             "Reconciled",
			Message:            "Resource is ready",
		},
	}
	setter.SetConditions(conditions)

	// Emit Event (non-critical)
	event := &corev1.Event{
		ObjectMeta: metav1.ObjectMeta{
			GenerateName: obj.GetName() + "-",
			Namespace:    obj.GetNamespace(),
		},
		InvolvedObject: corev1.ObjectReference{
			Kind:      obj.GetObjectKind().GroupVersionKind().Kind,
			Namespace: obj.GetNamespace(),
			Name:      obj.GetName(),
			UID:       obj.GetUID(),
		},
		Reason:  "Reconciled",
		Message: "Resource is ready",
		Source:  corev1.EventSource{Component: "kaigents-operator"},
		Type:    "Normal",
	}
	_ = r.Client.Create(ctx, event) // ignore error; status update is critical

	return r.Client.Status().Update(ctx, obj)
}
