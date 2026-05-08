// File: operator/internal/controllers/run_controller.go
// Purpose: Reconciles Run resources and updates readiness status.
// Product/business importance: marks runs as Ready while execution is not yet implemented.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package controllers

import (
	"context"
	"fmt"
	"os"
	"time"

	corev1alpha1 "github.com/jensjohansen/kaigents/operator/api/core/v1alpha1"
	batchv1 "k8s.io/api/batch/v1"
	corev1 "k8s.io/api/core/v1"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

func firstNonEmpty(values ...string) string {
	for _, v := range values {
		if v != "" {
			return v
		}
	}
	return ""
}

func appendIfValue(envs []corev1.EnvVar, name string, value string) []corev1.EnvVar {
	if value == "" {
		return envs
	}
	return append(envs, corev1.EnvVar{Name: name, Value: value})
}

func deriveMCPServerURL(server *corev1alpha1.MCPServer, namespace string) string {
	if server == nil {
		return ""
	}
	if server.Spec.URL != "" {
		return server.Spec.URL
	}
	if server.Spec.Transport == "http" && server.Spec.KMCPName != "" {
		return fmt.Sprintf("http://%s.%s.svc.cluster.local", server.Spec.KMCPName, namespace)
	}
	return ""
}

// RunReconciler reconciles Run resources.
type RunReconciler struct {
	Client client.Client
}

// Reconcile drives Run execution by creating a Kubernetes Job and reflecting its status.
func (r *RunReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	run := &corev1alpha1.Run{}
	if err := r.Client.Get(ctx, req.NamespacedName, run); err != nil {
		return ctrl.Result{}, client.IgnoreNotFound(err)
	}

	jobName := req.Name + "-exec"
	jobKey := types.NamespacedName{Name: jobName, Namespace: req.Namespace}
	job := &batchv1.Job{}
	jobExists := true
	if err := r.Client.Get(ctx, jobKey, job); err != nil {
		if apierrors.IsNotFound(err) {
			jobExists = false
		} else {
			_ = r.updateRunStatus(ctx, run, "Failed", "Failed to read execution Job: "+err.Error(), corev1alpha1.ConditionReconcileError, "True")
			return ctrl.Result{}, err
		}
	}

	if !jobExists {
		runnerImage := os.Getenv("KAIGENTS_RUNNER_IMAGE")
		if runnerImage == "" {
			runnerImage = "busybox:1.36"
		}

		resolvedModelEndpointURL := ""
		resolvedModelName := ""
		resolvedModelEndpointName := ""
		resolvedSystemPrompt := ""
		resolvedMCPServerURL := ""
		resolvedMCPServerName := ""
		resolvedSearchToolName := ""
		resolvedReadToolName := ""

		modelEndpointRef := ""
		runModelName := run.Spec.ModelName
		if run.Spec.ModelEndpointRef != "" {
			modelEndpointRef = run.Spec.ModelEndpointRef
		}

		if run.Spec.Target.Kind == "Agent" {
			agent := &corev1alpha1.Agent{}
			if err := r.Client.Get(ctx, types.NamespacedName{Name: run.Spec.Target.Name, Namespace: req.Namespace}, agent); err == nil {
				modelEndpointRef = firstNonEmpty(modelEndpointRef, agent.Spec.ModelEndpointRef)
				resolvedModelName = firstNonEmpty(runModelName, agent.Spec.ModelName)
				resolvedSystemPrompt = agent.Spec.SystemPrompt

				allowedSet := make(map[string]struct{}, len(agent.Spec.AllowedTools))
				for _, t := range agent.Spec.AllowedTools {
					allowedSet[t] = struct{}{}
				}
				hasAllowlist := len(allowedSet) > 0

				for _, toolRef := range agent.Spec.Tools {
					tool := &corev1alpha1.Tool{}
					if err := r.Client.Get(ctx, types.NamespacedName{Name: toolRef.Name, Namespace: req.Namespace}, tool); err != nil {
						continue
					}

					if hasAllowlist {
						if _, permitted := allowedSet[tool.Spec.ToolName]; !permitted {
							_ = r.updateRunStatus(ctx, run, "Failed",
								fmt.Sprintf("tool %q (toolName=%q) is not in the agent allowedTools list; execution blocked", toolRef.Name, tool.Spec.ToolName),
								corev1alpha1.ConditionReconcileError, "True")
							return ctrl.Result{}, nil
						}
					}

					if resolvedSearchToolName == "" && tool.Spec.ToolName == "searxng_web_search" {
						resolvedSearchToolName = tool.Spec.ToolName
					}
					if resolvedReadToolName == "" && tool.Spec.ToolName == "web_url_read" {
						resolvedReadToolName = tool.Spec.ToolName
					}

					if resolvedMCPServerName == "" {
						resolvedMCPServerName = firstNonEmpty(tool.Spec.MCPServerRef, resolvedMCPServerName)
					}

					if tool.Spec.MCPServerRef != "" && resolvedMCPServerURL == "" {
						mcpServer := &corev1alpha1.MCPServer{}
						if err := r.Client.Get(ctx, types.NamespacedName{Name: tool.Spec.MCPServerRef, Namespace: req.Namespace}, mcpServer); err == nil {
							resolvedMCPServerName = firstNonEmpty(resolvedMCPServerName, mcpServer.Name, mcpServer.Spec.KMCPName)
							resolvedMCPServerURL = firstNonEmpty(deriveMCPServerURL(mcpServer, req.Namespace), resolvedMCPServerURL)
						}
					}
				}
			} else {
				resolvedModelName = runModelName
			}
		} else {
			resolvedModelName = runModelName
		}

		if modelEndpointRef != "" {
			me := &corev1alpha1.ModelEndpoint{}
			if err := r.Client.Get(ctx, types.NamespacedName{Name: modelEndpointRef, Namespace: req.Namespace}, me); err == nil {
				resolvedModelEndpointURL = firstNonEmpty(me.Spec.URL, me.Spec.ServiceDNS)
				resolvedModelName = firstNonEmpty(resolvedModelName, me.Spec.Model)
				resolvedModelEndpointName = firstNonEmpty(me.Name, resolvedModelEndpointName)
			}
		}

		jobEnv := []corev1.EnvVar{
			{Name: "KAIGENTS_RUN_ID", Value: string(run.UID)},
			{Name: "KAIGENTS_RUN_NAME", Value: run.Name},
			{Name: "KAIGENTS_RUN_TARGET_KIND", Value: run.Spec.Target.Kind},
			{Name: "KAIGENTS_RUN_TARGET_NAME", Value: run.Spec.Target.Name},
			{Name: "KAIGENTS_RUN_INPUT", Value: run.Spec.Input},
		}

		jobEnv = appendIfValue(jobEnv, "KAIGENTS_MODEL_ENDPOINT_URL", resolvedModelEndpointURL)
		jobEnv = appendIfValue(jobEnv, "KAIGENTS_MODEL_NAME", resolvedModelName)
		jobEnv = appendIfValue(jobEnv, "KAIGENTS_MODEL_ENDPOINT_NAME", resolvedModelEndpointName)
		jobEnv = appendIfValue(jobEnv, "KAIGENTS_AGENT_SYSTEM_PROMPT", resolvedSystemPrompt)
		jobEnv = appendIfValue(jobEnv, "KAIGENTS_MCP_SERVER_URL", resolvedMCPServerURL)
		jobEnv = appendIfValue(jobEnv, "KAIGENTS_MCP_SERVER_NAME", resolvedMCPServerName)
		jobEnv = appendIfValue(jobEnv, "KAIGENTS_SEARCH_TOOL_NAME", resolvedSearchToolName)
		jobEnv = appendIfValue(jobEnv, "KAIGENTS_READ_TOOL_NAME", resolvedReadToolName)

		passThroughEnvNames := []string{
			"KAIGENTS_MODEL_API_KEY",
			"KAIGENTS_STORE",
			"KAIGENTS_STATE_DIR",
			"KAIGENTS_RETHINKDB_HOST",
			"KAIGENTS_RETHINKDB_PORT",
			"KAIGENTS_RETHINKDB_DB",
			"KAIGENTS_RETHINKDB_USER",
			"KAIGENTS_RETHINKDB_PASSWORD",
			"KAIGENTS_MODEL_TIMEOUT_SECS",
			"KAIGENTS_TEMPORAL_ADAPTER_URL",
		}
		for _, name := range passThroughEnvNames {
			if value := os.Getenv(name); value != "" {
				jobEnv = append(jobEnv, corev1.EnvVar{Name: name, Value: value})
			}
		}

		job = &batchv1.Job{
			ObjectMeta: metav1.ObjectMeta{
				Name:      jobName,
				Namespace: req.Namespace,
				Labels: map[string]string{
					"core.kaigents.io/run": req.Name,
				},
			},
			Spec: batchv1.JobSpec{
				BackoffLimit: int32ptr(0),
				Template: corev1.PodTemplateSpec{
					ObjectMeta: metav1.ObjectMeta{
						Labels: map[string]string{
							"core.kaigents.io/run": req.Name,
						},
					},
					Spec: corev1.PodSpec{
						ServiceAccountName: firstNonEmpty(os.Getenv("KAIGENTS_RUNNER_SERVICEACCOUNT"), "default"),
						RestartPolicy:      corev1.RestartPolicyNever,
						Containers: []corev1.Container{
							{
								Name:    "runner",
								Image:   runnerImage,
								Command: []string{"kaigents", "runner"},
								Env:     jobEnv,
							},
						},
					},
				},
			},
		}
		job.SetOwnerReferences([]metav1.OwnerReference{ownerRefForRun(run)})

		if err := r.Client.Create(ctx, job); err != nil {
			_ = r.updateRunStatus(ctx, run, "Failed", "Failed to create execution Job: "+err.Error(), corev1alpha1.ConditionReconcileError, "True")
			return ctrl.Result{}, err
		}

		_ = r.updateRunStatus(ctx, run, "Pending", "Execution Job created", corev1alpha1.ConditionReady, "False")
		return ctrl.Result{RequeueAfter: 1 * time.Second}, nil
	}

	phase, message := phaseFromJob(job)
	conditionType, conditionStatus := conditionForPhase(phase)
	if err := r.updateRunStatus(ctx, run, phase, message, conditionType, conditionStatus); err != nil {
		return ctrl.Result{}, err
	}

	if phase == "Succeeded" || phase == "Failed" {
		return ctrl.Result{}, nil
	}
	return ctrl.Result{RequeueAfter: 2 * time.Second}, nil
}

// SetupWithManager registers the reconciler with the controller manager.
func (r *RunReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1alpha1.Run{}).
		Owns(&batchv1.Job{}).
		Complete(r)
}

func (r *RunReconciler) updateRunStatus(
	ctx context.Context,
	run *corev1alpha1.Run,
	phase string,
	message string,
	conditionType corev1alpha1.ConditionType,
	conditionStatus string,
) error {
	run.Status.ObservedGeneration = run.GetGeneration()
	run.Status.Phase = phase
	run.Status.Message = message

	now := time.Now().Format(time.RFC3339)
	run.Status.Conditions = []corev1alpha1.Condition{
		{
			Type:               conditionType,
			Status:             conditionStatus,
			LastTransitionTime: now,
			Reason:             "Reconciled",
			Message:            message,
		},
	}

	return r.Client.Status().Update(ctx, run)
}

func ownerRefForRun(run *corev1alpha1.Run) metav1.OwnerReference {
	return metav1.OwnerReference{
		APIVersion: corev1alpha1.GroupVersion.String(),
		Kind:       "Run",
		Name:       run.Name,
		UID:        run.UID,
		Controller: boolptr(true),
	}
}

func phaseFromJob(job *batchv1.Job) (string, string) {
	if job.Status.Failed > 0 {
		return "Failed", "Execution Job failed"
	}
	if job.Status.Succeeded > 0 {
		return "Succeeded", "Execution Job succeeded"
	}
	if job.Status.Active > 0 {
		return "Running", "Execution Job running"
	}
	return "Pending", "Execution Job pending"
}

func conditionForPhase(phase string) (corev1alpha1.ConditionType, string) {
	if phase == "Succeeded" {
		return corev1alpha1.ConditionReady, "True"
	}
	if phase == "Failed" {
		return corev1alpha1.ConditionReconcileError, "True"
	}
	return corev1alpha1.ConditionReady, "False"
}

func int32ptr(value int32) *int32 { return &value }

func boolptr(value bool) *bool { return &value }
