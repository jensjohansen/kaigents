// File: operator/internal/controllers/team_controller.go
// Purpose: Reconciles Team resources and updates readiness status.
// Product/business importance: keeps Team CRD status deterministic for platform UX.
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

// TeamReconciler reconciles Team resources.
type TeamReconciler struct {
	Client client.Client
}

// Reconcile sets the Team status to Ready and records observed generation.
func (r *TeamReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.Team]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.Team { return &corev1alpha1.Team{} },
		GetSetStatus: func(obj *corev1alpha1.Team) (ReadyStatusSetter, bool) {
			return &teamStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

// SetupWithManager registers the reconciler with the controller manager.
func (r *TeamReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.Team{}).Complete(r)
}

type teamStatusSetter struct {
	obj *corev1alpha1.Team
}

func (s *teamStatusSetter) GetObservedGeneration() int64 { return s.obj.Status.ObservedGeneration }
func (s *teamStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *teamStatusSetter) SetPhase(value string) { s.obj.Status.Phase = value }
func (s *teamStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
