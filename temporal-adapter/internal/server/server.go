// File: kaigents/temporal-adapter/internal/server/server.go
// Purpose: HTTP API server that translates Kaigents WorkRequest operations into Temporal workflow calls.
// Product/business importance: The adapter server is the integration boundary that keeps all Temporal
// concepts out of the Rust engine and Kaigents CRDs. Every agent team execution flows through this server.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package server

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/prometheus/client_golang/prometheus/promhttp"
	"go.temporal.io/sdk/client"
	"go.uber.org/zap"

	"github.com/jensjohansen/kaigents/temporal-adapter/internal/activity"
	"github.com/jensjohansen/kaigents/temporal-adapter/internal/workflow"
)

// StartWorkRequestRequest is the HTTP body for starting a new WorkRequest.
// Pure Kaigents domain — no Temporal types.
type StartWorkRequestRequest struct {
	WorkRequestID string                 `json:"workRequestId"`
	ProcessName   string                 `json:"processName,omitempty"`
	Steps         []workflow.WorkItemDef `json:"steps"`
}

// SignalRequest carries a signal payload for a running WorkRequest.
type SignalRequest struct {
	SignalType string                  `json:"signalType"`
	Payload    workflow.ApprovalSignal `json:"payload"`
}

// WorkRequestResponse is returned for start and query operations.
type WorkRequestResponse struct {
	WorkRequestID string                    `json:"workRequestId"`
	Phase         string                    `json:"phase"`
	CurrentStep   string                    `json:"currentStep,omitempty"`
	ReworkCount   int                       `json:"reworkCount"`
	Results       []activity.WorkItemResult `json:"results,omitempty"`
	StartedAt     time.Time                 `json:"startedAt"`
	UpdatedAt     time.Time                 `json:"updatedAt"`
	Message       string                    `json:"message,omitempty"`
}

// Adapter is the HTTP server that translates Kaigents operations to Temporal without leaking Temporal types.
type Adapter struct {
	temporalClient client.Client
	logger         *zap.Logger
	mux            *http.ServeMux
}

// New creates an Adapter wrapping the given Temporal client and registers all HTTP routes.
// Use this as the single entry point for constructing the adapter — do not construct Adapter directly.
func New(temporalClient client.Client, logger *zap.Logger) *Adapter {
	adapter := &Adapter{temporalClient: temporalClient, logger: logger, mux: http.NewServeMux()}
	adapter.mux.HandleFunc("/v1/workrequests", adapter.handleWorkrequests)
	adapter.mux.HandleFunc("/v1/workrequests/", adapter.handleWorkrequest)
	adapter.mux.Handle("/metrics", promhttp.Handler())
	adapter.mux.HandleFunc("/healthz", func(responseWriter http.ResponseWriter, r *http.Request) {
		responseWriter.WriteHeader(http.StatusOK)
		fmt.Fprint(responseWriter, "ok")
	})
	return adapter
}

// Handler returns the HTTP handler for use with an http.Server.
// Prefer calling Run for lifecycle-managed serving.
func (adapter *Adapter) Handler() http.Handler { return adapter.mux }

func (adapter *Adapter) handleWorkrequests(responseWriter http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(responseWriter, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	var req StartWorkRequestRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(responseWriter, "invalid request body: "+err.Error(), http.StatusBadRequest)
		return
	}
	if req.WorkRequestID == "" {
		http.Error(responseWriter, "workRequestId is required", http.StatusBadRequest)
		return
	}

	input := workflow.WorkRequestInput{
		WorkRequestID: req.WorkRequestID,
		ProcessName:   req.ProcessName,
		Steps:         req.Steps,
	}

	opts := client.StartWorkflowOptions{
		ID:        workflowID(req.WorkRequestID),
		TaskQueue: activity.TaskQueue,
	}
	_, err := adapter.temporalClient.ExecuteWorkflow(r.Context(), opts, workflow.WorkRequestWorkflow, input)
	if err != nil {
		adapter.logger.Error("failed to start WorkRequest workflow", zap.String("workRequestId", req.WorkRequestID), zap.Error(err))
		http.Error(responseWriter, "failed to start WorkRequest: "+err.Error(), http.StatusInternalServerError)
		return
	}

	resp := WorkRequestResponse{
		WorkRequestID: req.WorkRequestID,
		Phase:         "Running",
		StartedAt:     time.Now().UTC(),
		UpdatedAt:     time.Now().UTC(),
	}
	adapter.writeJSON(responseWriter, http.StatusAccepted, resp)
}

func (adapter *Adapter) handleWorkrequest(responseWriter http.ResponseWriter, r *http.Request) {
	parts := strings.Split(strings.TrimPrefix(r.URL.Path, "/v1/workrequests/"), "/")
	if len(parts) == 0 || parts[0] == "" {
		http.Error(responseWriter, "workRequestId is required", http.StatusBadRequest)
		return
	}
	workRequestID := parts[0]
	wfID := workflowID(workRequestID)

	if len(parts) == 2 && parts[1] == "signal" {
		if r.Method != http.MethodPost {
			http.Error(responseWriter, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		adapter.handleSignal(responseWriter, r, wfID, workRequestID)
		return
	}

	if r.Method != http.MethodGet {
		http.Error(responseWriter, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	adapter.handleQuery(responseWriter, r, wfID, workRequestID)
}

func (adapter *Adapter) handleSignal(responseWriter http.ResponseWriter, r *http.Request, wfID, workRequestID string) {
	var req SignalRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(responseWriter, "invalid request body: "+err.Error(), http.StatusBadRequest)
		return
	}

	signalName := workflow.SignalApprove
	if req.SignalType == "rework" {
		signalName = workflow.SignalRework
	}

	if err := adapter.temporalClient.SignalWorkflow(r.Context(), wfID, "", signalName, req.Payload); err != nil {
		adapter.logger.Error("failed to signal WorkRequest", zap.String("workRequestId", workRequestID), zap.Error(err))
		http.Error(responseWriter, "failed to signal WorkRequest: "+err.Error(), http.StatusInternalServerError)
		return
	}

	responseWriter.WriteHeader(http.StatusAccepted)
}

func (adapter *Adapter) handleQuery(responseWriter http.ResponseWriter, r *http.Request, wfID, workRequestID string) {
	queryResponse, err := adapter.temporalClient.QueryWorkflow(r.Context(), wfID, "", workflow.QueryState)
	if err != nil {
		adapter.logger.Error("failed to query WorkRequest", zap.String("workRequestId", workRequestID), zap.Error(err))
		http.Error(responseWriter, "failed to query WorkRequest: "+err.Error(), http.StatusInternalServerError)
		return
	}

	var state workflow.WorkRequestState
	if err := queryResponse.Get(&state); err != nil {
		http.Error(responseWriter, "failed to decode WorkRequest state: "+err.Error(), http.StatusInternalServerError)
		return
	}

	resp := WorkRequestResponse{
		WorkRequestID: state.WorkRequestID,
		Phase:         state.Phase,
		CurrentStep:   state.CurrentStep,
		ReworkCount:   state.ReworkCount,
		Results:       state.Results,
		StartedAt:     state.StartedAt,
		UpdatedAt:     state.UpdatedAt,
		Message:       state.Message,
	}
	adapter.writeJSON(responseWriter, http.StatusOK, resp)
}

func workflowID(workRequestID string) string {
	return "kaigents-wr-" + workRequestID
}

func (adapter *Adapter) writeJSON(responseWriter http.ResponseWriter, code int, value any) {
	responseWriter.Header().Set("Content-Type", "application/json")
	responseWriter.WriteHeader(code)
	if err := json.NewEncoder(responseWriter).Encode(value); err != nil {
		adapter.logger.Error("failed to write JSON response", zap.Error(err))
	}
}

// Run starts the HTTP server and blocks until ctx is cancelled or the server exits with an error.
// Shutdown is graceful with a 10-second timeout. Use this for lifecycle-managed serving.
func (adapter *Adapter) Run(ctx context.Context, addr string) error {
	srv := &http.Server{Addr: addr, Handler: adapter.mux}
	go func() {
		<-ctx.Done()
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := srv.Shutdown(shutdownCtx); err != nil {
			adapter.logger.Error("graceful shutdown error", zap.Error(err))
		}
	}()
	adapter.logger.Info("kaigents-temporal-adapter listening", zap.String("addr", addr))
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return err
	}
	return nil
}
