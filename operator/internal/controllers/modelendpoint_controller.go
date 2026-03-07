// File: operator/internal/controllers/modelendpoint_controller.go
// Purpose: Reconciles ModelEndpoint resources and updates readiness status.
// Product/business importance: makes model endpoint discovery observable for operators.
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

// ModelEndpointReconciler reconciles ModelEndpoint resources.
type ModelEndpointReconciler struct {
	Client client.Client
}

// Reconcile sets the ModelEndpoint status to Ready and records observed generation.
func (r *ModelEndpointReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.ModelEndpoint]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.ModelEndpoint { return &corev1alpha1.ModelEndpoint{} },
		GetSetStatus: func(obj *corev1alpha1.ModelEndpoint) (ReadyStatusSetter, bool) {
			return &modelEndpointStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

// SetupWithManager registers the reconciler with the controller manager.
func (r *ModelEndpointReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.ModelEndpoint{}).Complete(r)
}

type modelEndpointStatusSetter struct {
	obj *corev1alpha1.ModelEndpoint
}

func (s *modelEndpointStatusSetter) GetObservedGeneration() int64 {
	return s.obj.Status.ObservedGeneration
}
func (s *modelEndpointStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *modelEndpointStatusSetter) SetPhase(value string) { s.obj.Status.Phase = value }
func (s *modelEndpointStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
