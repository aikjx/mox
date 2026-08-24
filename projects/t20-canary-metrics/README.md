# T20 Helm 灰度 Metrics

## 产出格式

| 文件 | 说明 |
|------|------|
| canary_phase_1.json ~ canary_phase_4.json | 四阶段 warmup 100×healthz + 10×metrics 汇总（success/error_rate/p95_ms） |
