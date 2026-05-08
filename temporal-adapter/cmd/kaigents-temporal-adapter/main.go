// File: kaigents/temporal-adapter/cmd/kaigents-temporal-adapter/main.go
// Purpose: Entry point for the kaigents-temporal-adapter service.
// Product/business importance: This binary is what runs in the cluster to bridge Kaigents WorkRequests
// to Temporal workflows. It is the executable deployed for M3 durable execution.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package main

import (
	"context"
	"os"
	"os/signal"
	"syscall"

	"go.temporal.io/sdk/client"
	"go.uber.org/zap"

	"github.com/jensjohansen/kaigents/temporal-adapter/internal/server"
	"github.com/jensjohansen/kaigents/temporal-adapter/internal/worker"
)

func main() {
	logger, _ := zap.NewProduction()
	defer logger.Sync()

	temporalHost := envOrDefault("TEMPORAL_HOST", "temporal-frontend.kaigents.svc.cluster.local:7233")
	temporalNamespace := envOrDefault("TEMPORAL_NAMESPACE", "kaigents")
	listenAddr := envOrDefault("LISTEN_ADDR", ":8080")

	temporalClient, err := client.Dial(client.Options{
		HostPort:  temporalHost,
		Namespace: temporalNamespace,
		Logger:    zapTemporalLogger{logger},
	})
	if err != nil {
		logger.Fatal("failed to connect to Temporal", zap.String("host", temporalHost), zap.Error(err))
	}
	defer temporalClient.Close()

	temporalWorker, err := worker.Start(temporalClient)
	if err != nil {
		logger.Fatal("failed to start Temporal worker", zap.Error(err))
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	adapter := server.New(temporalClient, logger)

	go func() {
		if err := adapter.Run(ctx, listenAddr); err != nil {
			logger.Error("HTTP server error", zap.Error(err))
		}
	}()

	<-ctx.Done()
	logger.Info("shutting down")
	temporalWorker.Stop()
}

func envOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// zapTemporalLogger adapts zap to the Temporal SDK logger interface.
type zapTemporalLogger struct{ z *zap.Logger }

func (l zapTemporalLogger) Debug(msg string, keyvals ...interface{}) {
	l.z.Sugar().Debugw(msg, keyvals...)
}
func (l zapTemporalLogger) Info(msg string, keyvals ...interface{}) {
	l.z.Sugar().Infow(msg, keyvals...)
}
func (l zapTemporalLogger) Warn(msg string, keyvals ...interface{}) {
	l.z.Sugar().Warnw(msg, keyvals...)
}
func (l zapTemporalLogger) Error(msg string, keyvals ...interface{}) {
	l.z.Sugar().Errorw(msg, keyvals...)
}
