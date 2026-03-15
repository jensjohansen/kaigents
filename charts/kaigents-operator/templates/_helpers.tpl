{{/*
File: charts/kaigents-operator/templates/_helpers.tpl
Purpose: Defines shared Helm template helper functions for naming Kaigents operator resources.
Product/business importance: Ensures consistent, predictable resource naming across operator chart deployments.

Copyright (c) 2026 John K Johansen
License: MIT (see LICENSE)
*/}}

{{- define "kaigents-operator.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "kaigents-operator.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s" $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}

{{- define "kaigents-operator.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "kaigents-operator.fullname" .) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}
