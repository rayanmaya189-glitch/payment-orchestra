{{/*
Payment Orchestra — Common Helm helpers
*/}}

{{- define "payment-orchestra.name" -}}
{{- .Chart.Name | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "payment-orchestra.fullname" -}}
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

{{- define "payment-orchestra.labels" -}}
helm.sh/chart: {{ .Chart.Name }}-{{ .Chart.Version }}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
environment: {{ .Values.global.environment }}
{{- end }}

{{- define "payment-orchestra.selectorLabels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Common environment variables for all services
*/}}
{{- define "payment-orchestra.env" -}}
- name: RUST_LOG
  value: info
- name: DATABASE_URL
  valueFrom:
    secretKeyRef:
      name: {{ .Values.infrastructure.postgres.secretName }}
      key: url
- name: REDIS_URL
  valueFrom:
    secretKeyRef:
      name: {{ .Values.infrastructure.redis.secretName }}
      key: url
- name: NATS_URL
  value: {{ .Values.infrastructure.nats.url }}
- name: OLLAMA_ENDPOINT
  value: {{ .Values.infrastructure.ollama.endpoint }}
- name: ENVIRONMENT
  value: {{ .Values.global.environment }}
- name: MINIO_ENDPOINT
  value: {{ .Values.infrastructure.minio.endpoint }}
- name: MINIO_BUCKET
  value: {{ .Values.infrastructure.minio.bucket }}
{{- end }}

{{/*
Security context for all services
*/}}
{{- define "payment-orchestra.securityContext" -}}
runAsNonRoot: {{ .Values.security.runAsNonRoot }}
runAsUser: {{ .Values.security.runAsUser }}
readOnlyRootFilesystem: {{ .Values.security.readOnlyRootFilesystem }}
allowPrivilegeEscalation: {{ .Values.security.allowPrivilegeEscalation }}
capabilities:
  drop:
    {{- toYaml .Values.security.capabilities.drop | nindent 4 }}
{{- end }}
