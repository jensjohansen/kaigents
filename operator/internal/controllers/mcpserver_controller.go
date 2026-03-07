// File: operator/internal/controllers/mcpserver_controller.go
// Purpose: Reconciles MCPServer resources and updates readiness status.
// Product/business importance: keeps MCP server references stable for tool-plane usage.
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

// MCPServerReconciler reconciles MCPServer resources.
type MCPServerReconciler struct {
	Client client.Client
}

// Reconcile sets the MCPServer status to Ready and records observed generation.
func (r *MCPServerReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ready := ReconcileReadyFunc[*corev1alpha1.MCPServer]{
		Client:   r.Client,
		NewEmpty: func() *corev1alpha1.MCPServer { return &corev1alpha1.MCPServer{} },
		GetSetStatus: func(obj *corev1alpha1.MCPServer) (ReadyStatusSetter, bool) {
			return &mcpServerStatusSetter{obj: obj}, true
		},
	}
	if err := ready.Reconcile(ctx, types.NamespacedName{Name: req.Name, Namespace: req.Namespace}); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

// SetupWithManager registers the reconciler with the controller manager.
func (r *MCPServerReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).For(&corev1alpha1.MCPServer{}).Complete(r)
}

type mcpServerStatusSetter struct {
	obj *corev1alpha1.MCPServer
}

func (s *mcpServerStatusSetter) GetObservedGeneration() int64 { return s.obj.Status.ObservedGeneration }
func (s *mcpServerStatusSetter) SetObservedGeneration(value int64) {
	s.obj.Status.ObservedGeneration = value
}
func (s *mcpServerStatusSetter) SetPhase(value string) { s.obj.Status.Phase = value }
func (s *mcpServerStatusSetter) SetConditions(value []corev1alpha1.Condition) {
	s.obj.Status.Conditions = value
}
