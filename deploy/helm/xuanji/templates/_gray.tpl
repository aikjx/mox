{{/*
灰度/金丝雀百分比与路由选择辅助函数。
Xuanji Umbrella Chart - Gray Release Helpers.
*/}}

{{/*
计算当前灰度阶段百分比。
输入: stageIndex (0..N-1)
输出: 该阶段的 canary 百分比数值
*/}}
{{- define "xuanji.gray.percent" -}}
{{- $stages := .Values.global.gray.stages -}}
{{- $idx := int . -}}
{{- if lt $idx (len $stages) -}}
{{- index $stages $idx -}}
{{- else -}}
100
{{- end -}}
{{- end }}

{{/*
根据灰度权重返回 canary vs stable 的路由权重对。
Usage: {{ include "xuanji.gray.weights" 1 }} -> "canary=10,stable=90"
*/}}
{{- define "xuanji.gray.weights" -}}
{{- $stageIndex := int . -}}
{{- $weight := int (include "xuanji.gray.percent" $stageIndex) -}}
{{- $stable := sub 100 $weight -}}
canary={{ $weight }},stable={{ $stable }}
{{- end }}

{{/*
返回当前灰度阶段的标签名。
*/}}
{{- define "xuanji.gray.stageLabel" -}}
{{- $stageIndex := int . -}}
gray-stage-{{ $stageIndex }}
{{- end }}

{{/*
路由选择函数：基于指定百分比返回流量应该命中 canary 的概率判断键。
用于 VirtualService / Ingress annotations 的加权路由。
*/}}
{{- define "xuanji.gray.routeSelector" -}}
{{- $stageIndex := int . -}}
{{- $weight := int (include "xuanji.gray.percent" $stageIndex) -}}
{{- if ge $weight 100 -}}
all
{{- else if eq $weight 0 -}}
none
{{- else -}}
{{- $r := randAlphaNum 8 -}}
{{- $v := sha1sum (printf "%s-%s" $r (now | unixEpoch)) | substr 0 4 | printf "0x%s" | int -}}
{{- if lt (mod $v 100) $weight -}}
canary
{{- else -}}
stable
{{- end -}}
{{- end -}}
{{- end }}

{{/*
获取 VirtualService 中对 canary subset 的 HTTPRouteDestination weight。
输入: stageIndex
*/}}
{{- define "xuanji.gray.canaryWeight" -}}
{{- $stageIndex := int . -}}
{{- if .Values.global.gray.enabled -}}
{{- include "xuanji.gray.percent" $stageIndex -}}
{{- else -}}
0
{{- end -}}
{{- end }}

{{/*
获取 VirtualService 中对 stable subset 的 HTTPRouteDestination weight。
输入: stageIndex
*/}}
{{- define "xuanji.gray.stableWeight" -}}
{{- $stageIndex := int . -}}
{{- if .Values.global.gray.enabled -}}
{{- $c := int (include "xuanji.gray.canaryWeight" $stageIndex) -}}
{{- sub 100 $c -}}
{{- else -}}
100
{{- end -}}
{{- end }}

{{/*
返回健康检查阈值（百分比字符串）。
*/}}
{{- define "xuanji.gray.healthThreshold" -}}
{{- default 95 .Values.global.gray.healthThresholdPercent -}}
{{- end }}

{{/*
返回按当前百分比过滤的 Kubernetes deployment 金丝雀 annotations。
输入: stageIndex
*/}}
{{- define "xuanji.gray.canaryAnnotations" -}}
{{- $stageIndex := int . -}}
{{- if .Values.global.gray.enabled -}}
traffic.kubernetes.io/canary: "true"
traffic.kubernetes.io/canary-weight: "{{ include "xuanji.gray.percent" $stageIndex }}"
{{- end -}}
{{- end }}
