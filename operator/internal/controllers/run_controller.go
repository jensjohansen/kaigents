// File: operator/internal/controllers/run_controller.go
// Purpose: Reconciles Run resources and updates readiness status.
// Product/business importance: marks runs as Ready while execution is not yet implemented.
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

// RunReconciler reconciles Run resources.
type RunReconciler struct {
	Client client.Client
}

// Reconcile sets the Run status to Ready and records observed generation.
func (r *RunReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.Run]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.Run { return &corev1alpha1.Run{} },
		GetSetStatus: func(obj *corev1alpha1.Run) (ReadyStatusSetter, bool) {
			return &runStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

// SetupWithManager registers the reconciler with the controller manager.
func (r *RunReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.Run{}).Complete(r)
}

type runStatusSetter struct {
	obj *corev1alpha1.Run
}

func (s *runStatusSetter) GetObservedGeneration() int64 { return s.obj.Status.ObservedGeneration }
func (s *runStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *runStatusSetter) SetPhase(value string) { s.obj.Status.Phase = value }
func (s *runStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
