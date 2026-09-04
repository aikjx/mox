# VAL-INDEX · 验证报告归一化（VAL-xx）

> 编号：**DOC-NORM-VAL-V1.0** · 归属：[README.md](README.md)（SSoT 枢纽）
> 内容：mox 出码/运行验证矩阵——verify 5 项、治理闸门 G1~G8、页面/API 验证、无 Mock 策略。

---

## 1. 出码治理门禁（生成即合规）

| 门 | 名称 | 判定 | 事实来源 |
|----|------|------|----------|
| verify 5 项 | mox-expert::verify 验证网关 | 合规/安全/权限/质量/可追溯 | `docs/modules/algorithm-verification.md`（AV-STD） |
| 治理闸门 G1~G8 | primiflow-fusion::full_gate | 8 项治理规则全过 | `docs/DOC-NORMALIZATION-REPORT.md` §FIX-10 |
| evidence 入图 | kg 投影器 | 产出带 `evidence_id` 方可入可信层 | `docs/database/mox_sys/relation-model.md` §5 |

> AI 优化闭环（L4）每次产出必须回 verify 门禁；优化结果挂 evidence 入图，形成可溯源演进链。

---

## 2. 页面可访问性验证（38 页面，事实来源：功能图谱 §10.1）

| 状态 | 页面数 |
|------|:--:|
| ✅ 正常渲染 | 38（含 5 个 admin 子面板） |
| ❌ 空白/报错 | 0 |

---

## 3. API 连通性验证（191 接口，事实来源：功能图谱 §10.2）

按分组（系统健康 5 / 知识图谱 18 / AI 核心 18 / 专家 15 / 大模型 15 / 项目 12 / 任务 6 / 工作流 11 / 知识库 8 / 其他 83）逐项探活，`getHealth`/`getGraphStats`/`aiChat` 等核心端点返回正常。

---

## 4. 无 Mock 策略

- 强制：`docs/NO-MOCK-POLICY.md`——生产路径禁止 Mock 兜底；降级用 `MockData.js` 必须独立存放（见 `MODULE-MANIFEST.md` §8）。
- 连接状态：页面加载即探测后端；断连须明确提示，不得静默假成功。

---

## 5. 验证矩阵模板（VAL-xx 登记范式）

| 维度 | 验证点 | 工具/端点 | 判定标准 |
|------|--------|-----------|----------|
| 功能 | 页面渲染/API 连通 | 浏览器/HTTP 探活 | 200 + 业务字段非空 |
| 合规 | verify 5 项 | 验证网关 | 全过 |
| 安全 | RBAC/ABAC/ReBAC | iam | 任一层拒绝不可放宽 |
| 可追溯 | evidence_id | kg 投影 | 入可信层须带证据 |
| 性能 | 基准/对比 | InfiniteOptimizer | 优于基线方可应用 |

---

## 6. 登记规则

- 验证报告 `VAL-{两位序号}-{中文短名}.md` 放 `docs/normalization/verify/`，须含矩阵表与判定标准。
- 跨文档引用 `docs/normalization/VAL-INDEX.md#章节`。
