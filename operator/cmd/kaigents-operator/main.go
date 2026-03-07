// File: operator/cmd/kaigents-operator/main.go
// Purpose: Kaigents operator entrypoint and controller-runtime manager setup.
// Product/business importance: boots the control plane to reconcile Kaigents CRDs.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package main

import (
	"flag"
	"fmt"
	"os"

	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
	"github.com/jensjohansen/kaigents/operator/internal/controllers"
	"github.com/jensjohansen/kaigents/operator/internal/version"
	"k8s.io/apimachinery/pkg/runtime"
	utilruntime "k8s.io/apimachinery/pkg/util/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/healthz"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"
	metricsserver "sigs.k8s.io/controller-runtime/pkg/metrics/server"
)

func main() {
	args := os.Args[1:]
	if len(args) > 0 && (args[0] == "version" || args[0] == "--version" || args[0] == "-v") {
		fmt.Println(version.Version)
		return
	}

	var metricsAddr string
	var probeAddr string
	var enableLeaderElection bool

	flag.StringVar(&metricsAddr, "metrics-bind-address", ":8080", "The address the metric endpoint binds to.")
	flag.StringVar(&probeAddr, "health-probe-bind-address", ":8081", "The address the probe endpoint binds to.")
	flag.BoolVar(&enableLeaderElection, "leader-elect", false, "Enable leader election for controller manager.")

	opts := zap.Options{Development: true}
	opts.BindFlags(flag.CommandLine)
	flag.Parse()

	ctrl.SetLogger(zap.New(zap.UseFlagOptions(&opts)))

	scheme := runtime.NewScheme()
	utilruntime.Must(clientgoscheme.AddToScheme(scheme))
	utilruntime.Must(corev1alpha1.AddToScheme(scheme))

	mgr, err := ctrl.NewManager(ctrl.GetConfigOrDie(), ctrl.Options{
		Scheme:                 scheme,
		Metrics:                metricsserver.Options{BindAddress: metricsAddr},
		HealthProbeBindAddress: probeAddr,
		LeaderElection:         enableLeaderElection,
		LeaderElectionID:       "kaigents-operator.core.kaigents.io",
	})
	if err != nil {
		ctrl.Log.Error(err, "unable to start manager")
		os.Exit(1)
	}

	if err := controllers.SetupAllReconcilers(mgr); err != nil {
		ctrl.Log.Error(err, "unable to set up reconcilers")
		os.Exit(1)
	}

	if err := mgr.AddHealthzCheck("healthz", healthz.Ping); err != nil {
		ctrl.Log.Error(err, "unable to set up health check")
		os.Exit(1)
	}
	if err := mgr.AddReadyzCheck("readyz", healthz.Ping); err != nil {
		ctrl.Log.Error(err, "unable to set up ready check")
		os.Exit(1)
	}

	ctrl.Log.Info("starting manager", "version", version.Version)
	if err := mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		ctrl.Log.Error(err, "problem running manager")
		os.Exit(1)
	}
}
