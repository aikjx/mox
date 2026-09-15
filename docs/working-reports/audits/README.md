# 审计与评估快照 — Audits

> **层定位**：L7 报告与验证层 → 审计子目录（🟡 证据，非权威）。
> 上层入口：[工作报告索引](../README.md) · [文档中心](../../README.md)

---

## 一、内容清单

| 文件 | 说明 | 时效 |
|------|------|------|
| [`PRODUCTION_READINESS_ASSESSMENT_v3.4.md`](./PRODUCTION_READINESS_ASSESSMENT_v3.4.md) | 生产就绪度评估 v3.4（维度评分、阻断项、放行结论） | 一次性快照 |
| [`SECURITY_AUDIT_v3.4.md`](./SECURITY_AUDIT_v3.4.md) | 安全审计 v3.4（面/风险/整改项） | 一次性快照 |

## 二、使用约定

1. **快照性质**：版本号（`vX.Y`）与日期绑定，结论只对当时代码状态有效；不得当作"当前状态"引用。
2. 整改项闭环后，应把**状态**写回 L6 企业级层（[`../../enterprise/00-INDEX.md`](../../enterprise/00-INDEX.md)）或对应 ADR，本目录保留原始结论用于追溯。
3. 新一轮评估：`<主题>_v<版本>.md` 新增文件，并在上表登记，旧版移入 [`../../_archive/`](../../_archive/)。
