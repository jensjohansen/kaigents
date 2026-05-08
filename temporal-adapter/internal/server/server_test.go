// File: kaigents/temporal-adapter/internal/server/server_test.go
// Purpose: Unit tests for the Adapter HTTP server routing, request validation, and response encoding.
// Product/business importance: Ensures that the HTTP API boundary correctly validates inputs and returns
// appropriate error codes before any Temporal workflow is started. Prevents bad requests from
// corrupting the WorkRequest execution history.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package server_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"go.temporal.io/sdk/client"
	"go.uber.org/zap"

	"github.com/jensjohansen/kaigents/temporal-adapter/internal/server"
	"github.com/jensjohansen/kaigents/temporal-adapter/internal/workflow"
)

// stubTemporalClient satisfies the client.Client interface for tests that do not reach Temporal.
// Methods are embedded from the interface and will panic if unexpectedly called — that is intentional
// so that tests fail clearly if Temporal is called when it should not be.
type stubTemporalClient struct {
	client.Client
}

func newTestAdapter() *server.Adapter {
	logger := zap.NewNop()
	return server.New(&stubTemporalClient{}, logger)
}

func TestStartWorkRequest_MethodNotAllowed(t *testing.T) {
	adapter := newTestAdapter()

	for _, method := range []string{http.MethodGet, http.MethodPut, http.MethodDelete, http.MethodPatch} {
		request := httptest.NewRequest(method, "/v1/workrequests", nil)
		recorder := httptest.NewRecorder()

		adapter.Handler().ServeHTTP(recorder, request)

		if recorder.Code != http.StatusMethodNotAllowed {
			t.Errorf("method %s: expected 405, got %d", method, recorder.Code)
		}
	}
}

func TestStartWorkRequest_InvalidJSON(t *testing.T) {
	adapter := newTestAdapter()

	request := httptest.NewRequest(http.MethodPost, "/v1/workrequests", bytes.NewBufferString("not-json"))
	recorder := httptest.NewRecorder()

	adapter.Handler().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", recorder.Code)
	}
}

func TestStartWorkRequest_MissingWorkRequestID(t *testing.T) {
	adapter := newTestAdapter()

	body, _ := json.Marshal(server.StartWorkRequestRequest{
		Steps: []workflow.WorkItemDef{{WorkItemID: "step-1", StepName: "research"}},
	})
	request := httptest.NewRequest(http.MethodPost, "/v1/workrequests", bytes.NewBuffer(body))
	recorder := httptest.NewRecorder()

	adapter.Handler().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusBadRequest {
		t.Errorf("expected 400 for missing workRequestId, got %d", recorder.Code)
	}
}

func TestGetWorkRequest_MethodNotAllowed(t *testing.T) {
	adapter := newTestAdapter()

	for _, method := range []string{http.MethodPost, http.MethodPut, http.MethodDelete} {
		request := httptest.NewRequest(method, "/v1/workrequests/wr-123", nil)
		recorder := httptest.NewRecorder()

		adapter.Handler().ServeHTTP(recorder, request)

		if recorder.Code != http.StatusMethodNotAllowed {
			t.Errorf("method %s: expected 405 for GET-only path, got %d", method, recorder.Code)
		}
	}
}

func TestSignalWorkRequest_MethodNotAllowed(t *testing.T) {
	adapter := newTestAdapter()

	for _, method := range []string{http.MethodGet, http.MethodPut, http.MethodDelete} {
		request := httptest.NewRequest(method, "/v1/workrequests/wr-123/signal", nil)
		recorder := httptest.NewRecorder()

		adapter.Handler().ServeHTTP(recorder, request)

		if recorder.Code != http.StatusMethodNotAllowed {
			t.Errorf("method %s: expected 405 for signal path, got %d", method, recorder.Code)
		}
	}
}

func TestSignalWorkRequest_InvalidJSON(t *testing.T) {
	adapter := newTestAdapter()

	request := httptest.NewRequest(http.MethodPost, "/v1/workrequests/wr-123/signal", bytes.NewBufferString("bad"))
	recorder := httptest.NewRecorder()

	adapter.Handler().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusBadRequest {
		t.Errorf("expected 400 for invalid signal body, got %d", recorder.Code)
	}
}

func TestHealthz(t *testing.T) {
	adapter := newTestAdapter()

	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	recorder := httptest.NewRecorder()

	adapter.Handler().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusOK {
		t.Errorf("expected 200 from /healthz, got %d", recorder.Code)
	}
	if recorder.Body.String() != "ok" {
		t.Errorf("expected 'ok' body from /healthz, got %q", recorder.Body.String())
	}
}
