// File: dashboard/internal/server/server.go
// Purpose: HTTP server for the Kaigents dashboard MVP.
// Product/business importance: Provides the primary human-facing interface for platform observability.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package server

import (
	"context"
	"embed"
	"html/template"
	"net/http"
	"strings"

	"github.com/prometheus/client_golang/prometheus/promhttp"
	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
	"go.uber.org/zap"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

//go:embed templates/*
var templatesFS embed.FS

type Server struct {
	client             client.Client
	logger             *zap.Logger
	tmpl               *template.Template
	temporalAdapterURL string
}

func New(client client.Client, logger *zap.Logger, temporalAdapterURL string) (*Server, error) {
	tmpl, err := template.ParseFS(templatesFS, "templates/*.html")
	if err != nil {
		return nil, err
	}
	return &Server{
		client:             client,
		logger:             logger,
		tmpl:               tmpl,
		temporalAdapterURL: temporalAdapterURL,
	}, nil
}

func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/", s.handleIndex)
	mux.HandleFunc("/agents", s.handleAgents)
	mux.HandleFunc("/processes", s.handleProcesses)
	mux.HandleFunc("/runs", s.handleRuns)
	mux.HandleFunc("/runs/", s.handleRunDetail)
	mux.Handle("/metrics", promhttp.Handler())
	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("ok"))
	})
	return mux
}

func (s *Server) handleAgents(w http.ResponseWriter, r *http.Request) {
	var list corev1alpha1.AgentList
	if err := s.client.List(r.Context(), &list); err != nil {
		s.logger.Error("failed to list agents", zap.Error(err))
	}
	s.render(w, "agents", map[string]interface{}{
		"Page":   "agents",
		"Agents": list.Items,
	})
}

func (s *Server) handleProcesses(w http.ResponseWriter, r *http.Request) {
	var list corev1alpha1.ProcessList
	if err := s.client.List(r.Context(), &list); err != nil {
		s.logger.Error("failed to list processes", zap.Error(err))
	}
	s.render(w, "processes", map[string]interface{}{
		"Page":      "processes",
		"Processes": list.Items,
	})
}

func (s *Server) handleRuns(w http.ResponseWriter, r *http.Request) {
	var list corev1alpha1.RunList
	if err := s.client.List(r.Context(), &list); err != nil {
		s.logger.Error("failed to list runs", zap.Error(err))
	}
	s.render(w, "runs", map[string]interface{}{
		"Page": "runs",
		"Runs": list.Items,
	})
}

func (s *Server) handleRunDetail(w http.ResponseWriter, r *http.Request) {
	name := strings.TrimPrefix(r.URL.Path, "/runs/")
	if name == "" {
		http.Redirect(w, r, "/runs", http.StatusFound)
		return
	}

	ctx := r.Context()
	var run corev1alpha1.Run
	if err := s.client.Get(ctx, client.ObjectKey{Name: name, Namespace: "default"}, &run); err != nil {
		s.logger.Error("failed to get run", zap.String("name", name), zap.Error(err))
		http.NotFound(w, r)
		return
	}

	s.render(w, "run_detail", map[string]interface{}{
		"Page": "runs",
		"Run":  run,
	})
}

func (s *Server) render(w http.ResponseWriter, name string, data interface{}) {
	if err := s.tmpl.ExecuteTemplate(w, "layout", data); err != nil {
		s.logger.Error("failed to render template", zap.String("template", name), zap.Error(err))
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
	}
}

func (s *Server) handleIndex(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		http.NotFound(w, r)
		return
	}

	ctx := r.Context()
	var agents corev1alpha1.AgentList
	var processes corev1alpha1.ProcessList
	var runs corev1alpha1.RunList

	if err := s.client.List(ctx, &agents); err != nil {
		s.logger.Error("failed to list agents", zap.Error(err))
	}
	if err := s.client.List(ctx, &processes); err != nil {
		s.logger.Error("failed to list processes", zap.Error(err))
	}
	if err := s.client.List(ctx, &runs); err != nil {
		s.logger.Error("failed to list runs", zap.Error(err))
	}

	activeCount := 0
	for _, run := range runs.Items {
		if run.Status.Phase == "Running" || run.Status.Phase == "Pending" {
			activeCount++
		}
	}

	data := map[string]interface{}{
		"Page":           "home",
		"AgentCount":     len(agents.Items),
		"ProcessCount":   len(processes.Items),
		"ActiveRunCount": activeCount,
		"RecentRuns":     runs.Items,
	}

	if err := s.tmpl.ExecuteTemplate(w, "layout", data); err != nil {
		s.logger.Error("failed to render template", zap.Error(err))
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
	}
}

func (s *Server) Run(ctx context.Context, addr string) error {
	srv := &http.Server{Addr: addr, Handler: s.Handler()}
	go func() {
		<-ctx.Done()
		srv.Shutdown(context.Background())
	}()
	s.logger.Info("dashboard listening", zap.String("addr", addr))
	return srv.ListenAndServe()
}
