// File: dashboard/cmd/kaigents-dashboard/main.go
// Purpose: Entry point for the Kaigents dashboard service.
// Product/business importance: Boots the web UI used by operators to manage the platform.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package main

import (
	"context"
	"flag"
	"os"
	"os/signal"
	"syscall"

	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
	"github.com/jensjohansen/kaigents/dashboard/internal/server"
	"github.com/jensjohansen/kaigents/dashboard/internal/artifacts"
	"go.uber.org/zap"
	"k8s.io/apimachinery/pkg/runtime"
	utilruntime "k8s.io/apimachinery/pkg/util/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

func main() {
	addr := flag.String("addr", ":8080", "HTTP listen address")
	flag.Parse()

	logger, _ := zap.NewProduction()
	defer logger.Sync()

	scheme := runtime.NewScheme()
	utilruntime.Must(clientgoscheme.AddToScheme(scheme))
	utilruntime.Must(corev1alpha1.AddToScheme(scheme))

	config := ctrl.GetConfigOrDie()
	k8sClient, err := client.New(config, client.Options{Scheme: scheme})
	if err != nil {
		logger.Fatal("failed to create k8s client", zap.Error(err))
	}

	temporalAdapterURL := os.Getenv("KAIGENTS_TEMPORAL_ADAPTER_URL")
	if temporalAdapterURL == "" {
		temporalAdapterURL = "http://kaigents-temporal-adapter.kaigents.svc.cluster.local:8080"
	}

	s3Cfg := artifacts.Config{
		Endpoint:        os.Getenv("KAIGENTS_S3_ENDPOINT"),
		Region:          os.Getenv("KAIGENTS_S3_REGION"),
		Bucket:          os.Getenv("KAIGENTS_S3_BUCKET"),
		AccessKey:       os.Getenv("KAIGENTS_S3_ACCESS_KEY"),
		SecretKey:       os.Getenv("KAIGENTS_S3_SECRET_KEY"),
		ForcePathStyle: os.Getenv("KAIGENTS_S3_FORCE_PATH_STYLE") == "true",
	}
	if s3Cfg.Bucket == "" {
		s3Cfg.Bucket = "kaigents-artifacts"
	}
	if s3Cfg.Region == "" {
		s3Cfg.Region = "us-east-1"
	}

	artifactProxy, err := artifacts.NewProxy(s3Cfg, logger)
	if err != nil {
		logger.Fatal("failed to initialize artifact proxy", zap.Error(err))
	}

	dashboard, err := server.New(k8sClient, logger, temporalAdapterURL, artifactProxy)
	if err != nil {
		logger.Fatal("failed to initialize dashboard server", zap.Error(err))
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	if err := dashboard.Run(ctx, *addr); err != nil {
		logger.Fatal("dashboard server error", zap.Error(err))
	}
}
