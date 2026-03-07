// File: operator/internal/controllers/tool_controller.go
// Purpose: Reconciles Tool resources and updates readiness status.
// Product/business importance: keeps tool registrations observable and stable.
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

// ToolReconciler reconciles Tool resources.
type ToolReconciler struct {
	Client client.Client
}

// Reconcile sets the Tool status to Ready and records observed generation.
func (r *ToolReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.Tool]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.Tool { return &corev1alpha1.Tool{} },
		GetSetStatus: func(obj *corev1alpha1.Tool) (ReadyStatusSetter, bool) {
			return &toolStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

// SetupWithManager registers the reconciler with the controller manager.
func (r *ToolReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.Tool{}).Complete(r)
}

type toolStatusSetter struct {
	obj *corev1alpha1.Tool
}

func (s *toolStatusSetter) GetObservedGeneration() int64 { return s.obj.Status.ObservedGeneration }
func (s *toolStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *toolStatusSetter) SetPhase(value string) { s.obj.Status.Phase = value }
func (s *toolStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
