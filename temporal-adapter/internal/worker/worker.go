// File: kaigents/temporal-adapter/internal/worker/worker.go
// Purpose: Registers Kaigents workflow and activity types with a Temporal worker and starts it.
// Product/business importance: The worker is the process that polls Temporal and executes WorkRequest
// workflows and WorkItem activities. Without it, no agent work executes.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package worker

import (
	"go.temporal.io/sdk/client"
	temporalworker "go.temporal.io/sdk/worker"

	"github.com/jensjohansen/kaigents/temporal-adapter/internal/activity"
	"github.com/jensjohansen/kaigents/temporal-adapter/internal/workflow"
)

// Start registers Kaigents workflows and activities with a Temporal worker and starts polling.
// Returns the running worker so the caller can stop it cleanly on shutdown.
// The worker polls the kaigents-workrequest task queue in the configured Temporal namespace.
func Start(temporalClient client.Client) (temporalworker.Worker, error) {
	temporalWorker := temporalworker.New(temporalClient, activity.TaskQueue, temporalworker.Options{})
	temporalWorker.RegisterWorkflow(workflow.WorkRequestWorkflow)
	temporalWorker.RegisterActivity(activity.ExecuteWorkItem)
	return temporalWorker, temporalWorker.Start()
}
