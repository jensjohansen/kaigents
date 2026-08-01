// File: operator/internal/controllers/task_controller.go
// Purpose: Reconciles Task resources and updates readiness status.
// Product/business importance: provides deterministic status transitions for Task CRDs.
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

type TaskReconciler struct {
	Client client.Client
}

func (r *TaskReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.Task]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.Task { return &corev1alpha1.Task{} },
		GetSetStatus: func(obj *corev1alpha1.Task) (ReadyStatusSetter, bool) {
			return &taskStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

func (r *TaskReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.Task{}).Complete(r)
}

type taskStatusSetter struct {
	obj *corev1alpha1.Task
}

func (s *taskStatusSetter) GetObservedGeneration() int64 {
	return s.obj.Status.ObservedGeneration
}
func (s *taskStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *taskStatusSetter) SetPhase(value string) {}
func (s *taskStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
