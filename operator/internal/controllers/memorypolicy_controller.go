// File: operator/internal/controllers/memorypolicy_controller.go
// Purpose: Reconciles MemoryPolicy resources and updates readiness status.
// Product/business importance: provides deterministic status transitions for MemoryPolicy CRDs.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package controllers

import (
	"context"

	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

type MemoryPolicyReconciler struct {
	Client client.Client
}

func (r *MemoryPolicyReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.MemoryPolicy]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.MemoryPolicy { return &corev1alpha1.MemoryPolicy{} },
		GetSetStatus: func(obj *corev1alpha1.MemoryPolicy) (ReadyStatusSetter, bool) {
			return &memoryPolicyStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

func (r *MemoryPolicyReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.MemoryPolicy{}).Complete(r)
}

type memoryPolicyStatusSetter struct {
	obj *corev1alpha1.MemoryPolicy
}

func (s *memoryPolicyStatusSetter) GetObservedGeneration() int64 {
	return s.obj.Status.ObservedGeneration
}
func (s *memoryPolicyStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *memoryPolicyStatusSetter) SetPhase(value string) {
	s.obj.Status.Phase = value
}
func (s *memoryPolicyStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
