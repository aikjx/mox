# PrimiFlow 六维溯源矩阵（由关联图谱自动生成）

| 需求 | 功能 | 业务 | 算法 | 任务 | 代码 | 数据设计 |
|------|------|------|------|------|------|----------|
| 自然语言需求→可渲染DAG画布 | canvas 可视化可编辑画布 / topology_operator 需求→DAG | 拓扑涌现 / 需求结构化 | κτ 调度 | asr_transcribe / emerge_topology / parse_requirement / regularize | AsrClient / Orchestrator / Scheduler / TopologyOperator | Conversation / Project / Topology / TraceLink |
| ℛ̂ 合规裁剪 | scheduler κ/τ+ℛ̂ | 正则化裁剪 | κτ 调度 / ℛ̂ 正则化 | asr_transcribe / edit_canvas / emerge_topology / parse_requirement / regularize / smoke_test | AsrClient / CanvasState / Orchestrator / Scheduler / SmokeTester / TopologyOperator | Artifact / Conversation / Project / Topology / TraceLink |
| 八份说明书自动生成 | doc_generator 八文档 | 导出工程 | 六维溯源绑定 | bind_trace / export_project / generate_docs | DocGenerator / Orchestrator | Artifact / Project / TraceLink |
| 六维溯源绑定 | orchestrator 编排状态机 | 六维溯源 / 导出工程 | 六维溯源绑定 | bind_trace / export_project / generate_docs | DocGenerator / Orchestrator | Artifact / Project / TraceLink |
| κ 复用资产 Q | asset 资产检索/冻结 | 资产冻结复用 | pgvector 检索 | freeze_asset | AssetService | Asset |
| 冒烟兜底主链路 | smoke_tester 校验/冒烟 | 正则化裁剪 | κτ 调度 / ℛ̂ 正则化 | asr_transcribe / edit_canvas / emerge_topology / parse_requirement / regularize / smoke_test | AsrClient / CanvasState / Orchestrator / Scheduler / SmokeTester / TopologyOperator | Artifact / Conversation / Project / Topology / TraceLink |
