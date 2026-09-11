# T10 云盘 M4 产出物目录

> **归属**：Batch A (T10) 验收产出物  
> **代码/数据分离约束**：本目录只放业务数据与报告，架构代码在 platform/

## 产出格式

| 文件 | 说明 |
|------|------|
| lifecycle_report.json | 冷热分层迁移统计（对象数/字节数/平均迁移动作耗时） |
| iam10_matrix.json | 10 条 Policy × 10 场景判定矩阵（{"sid":[{"action","resource","principal","allow":bool}]}） |
| sts_ttl900_report.json | STS AssumeRole TTL=900s 校验结果（过期/签名/并发） |
| quota429_limits.json | Quota 维度（bytes/objects/reqs）× 租户 的当前阈值 |
| dengbao_chain_sample.json | 等保三级 hash_chain 样例（≥100 block + integrity 字段） |
| ubric_evidence.md | T10 自检 rubric 证据（IAM/WORM/Quota429） |
