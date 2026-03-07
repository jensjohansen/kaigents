// File: operator/internal/controllers/agent_controller.go
// Purpose: Reconciles Agent resources and updates readiness status.
// Product/business importance: provides deterministic status transitions for Agent CRDs.
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

// AgentReconciler reconciles Agent resources.
type AgentReconciler struct {
	Client client.Client
}

// Reconcile sets the Agent status to Ready and records observed generation.
func (r *AgentReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.Agent]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.Agent { return &corev1alpha1.Agent{} },
		GetSetStatus: func(obj *corev1alpha1.Agent) (ReadyStatusSetter, bool) {
			return &agentStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

// SetupWithManager registers the reconciler with the controller manager.
func (r *AgentReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.Agent{}).Complete(r)
}

type agentStatusSetter struct {
	obj *corev1alpha1.Agent
}

func (s *agentStatusSetter) GetObservedGeneration() int64 { return s.obj.Status.ObservedGeneration }
func (s *agentStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *agentStatusSetter) SetPhase(value string) { s.obj.Status.Phase = value }
func (s *agentStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
