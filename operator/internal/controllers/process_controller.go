// File: operator/internal/controllers/process_controller.go
// Purpose: Reconciles Process resources and updates readiness status.
// Product/business importance: provides deterministic status transitions for Process CRDs.
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

type ProcessReconciler struct {
	Client client.Client
}

func (r *ProcessReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.Process]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.Process { return &corev1alpha1.Process{} },
		GetSetStatus: func(obj *corev1alpha1.Process) (ReadyStatusSetter, bool) {
			return &processStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

func (r *ProcessReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.Process{}).Complete(r)
}

type processStatusSetter struct {
	obj *corev1alpha1.Process
}

func (s *processStatusSetter) GetObservedGeneration() int64 {
	return s.obj.Status.ObservedGeneration
}
func (s *processStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *processStatusSetter) SetPhase(value string) {}
func (s *processStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
