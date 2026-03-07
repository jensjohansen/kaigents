// File: operator/internal/controllers/setup.go
// Purpose: Wires all Kaigents reconcilers into the controller manager.
// Product/business importance: ensures CRDs are reconciled by the operator.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package controllers

import (
	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
	ctrl "sigs.k8s.io/controller-runtime"
)

// SetupAllReconcilers registers all Kaigents reconcilers with the manager.
func SetupAllReconcilers(mgr ctrl.Manager) error {
	reconcilers := []interface {
		SetupWithManager(ctrl.Manager) error
	}{
		&AgentReconciler{Client: mgr.GetClient()},
		&TeamReconciler{Client: mgr.GetClient()},
		&RunReconciler{Client: mgr.GetClient()},
		&ToolReconciler{Client: mgr.GetClient()},
		&MCPServerReconciler{Client: mgr.GetClient()},
		&ModelEndpointReconciler{Client: mgr.GetClient()},
	}

	_ = corev1alpha1.GroupVersion

	for _, r := range reconcilers {
		if err := r.SetupWithManager(mgr); err != nil {
			return err
		}
	}
	return nil
}
