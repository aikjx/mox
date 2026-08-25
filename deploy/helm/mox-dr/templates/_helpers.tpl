{{/*
Expand the name of the chart.
*/}}
{{- define "mox-dr.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "mox-dr.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "mox-dr.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "mox-dr.labels" -}}
helm.sh/chart: {{ include "mox-dr.chart" . }}
{{ include "mox-dr.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/component: dr
{{- end }}

{{/*
Selector labels
*/}}
{{- define "mox-dr.selectorLabels" -}}
app.kubernetes.io/name: {{ include "mox-dr.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Selector labels for primary role
*/}}
{{- define "mox-dr.primarySelectorLabels" -}}
{{ include "mox-dr.selectorLabels" . }}
app.kubernetes.io/role: primary
{{- end }}

{{/*
Selector labels for secondary role
*/}}
{{- define "mox-dr.secondarySelectorLabels" -}}
{{ include "mox-dr.selectorLabels" . }}
app.kubernetes.io/role: secondary
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "mox-dr.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "mox-dr.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Primary deployment helper
*/}}
{{- define "mox-dr.primaryRole" -}}primary{{- end }}

{{/*
Secondary deployment helper
*/}}
{{- define "mox-dr.secondaryRole" -}}secondary{{- end }}

{{/*
DR region env value
*/}}
{{- define "mox-dr.regionEnv" -}}
{{- if eq . "primary" -}}
{{- printf "primary" -}}
{{- else -}}
{{- printf "secondary" -}}
{{- end }}
{{- end }}
