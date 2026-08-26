# MOX 全自动开发 + 测试验证 · 功能模块全部开发完成执行计划 v2.0

> **文档定位：** 配套《MOX 企业级归一化架构与交付总纲 v2.0》的唯一执行级文件。
> 严格遵守 Experience 1179649 三条纪律：
> 1. **先给审核结论** 再执行；
> 2. **每阶段门禁硬指标写死**，不过就回滚；
> 3. **必须产出报告类交付物**（测试报告 / 验收结果 / 项目总结），不能停留在口头建议。
>
> **周期：** W1–W12（12 周 / 3 个月），T0→T1 档位跑全自动流程，功能模块全部完成即打 CR-002 签字版。
> **配套可视化：** 流水线图见「MOX 全自动端到端开发流水线」；6 层判定矩阵见「MOX 6 层开发完成判定矩阵」。

---

## 0. 审核确认结论（先读 · 和归一化总纲对齐）

### 0.1 合规性审核结论：✅ 条件通过（2 条件）

| 项 | 审核结论 | 和归一化总纲的一致性 |
|---|---|---|
| 架构分层 L5/L4/L3/L3.5/L2/L1 | ✅ 通过 | 100% 复用总纲 §2.1 的 6 层，未加冗余层 |
| 12 阶段 P0–P12 编号 | ✅ 通过 | 100% 复用总纲 §4.1，未加额外阶段 |
| 三大铁律 / Key 同构公式 / 28 项 / 6 域 Schema | ✅ 通过 | 全部只在总纲唯一定义，本计划不重写 |
| 图谱治理三条红线 | ✅ 通过 | 100% 继承总纲 §5.5，本计划把它当 Gate 判定条件 |

**条件通过附带的两个 MUST 条件（W1 之内解决）：**
1. `lib/json-store.js` 改成 tmp+rename 原子写（抄 [expert-alliance-engine.js L40-L48](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/expert-alliance-engine.js#L40-L48) 正确实现），W1D5 前合入主干。
2. CI 加 300MB 混合载荷烟测（§3.6 归一化总纲要求），W1D5 前合入 CI YAML。

---

## 1. 总体节奏（12 周 × 4 大冲刺 × 每冲刺门禁）

```
冲刺1 立即修补 (W1-W2) → 冲刺2 AI核心开发 (W3-W6) → 冲刺3 切换+灰度 (W7-W9) → 冲刺4 验收+交付 (W10-W12)
        Gate A 设计完                        Gate B 质量过                         Gate C 切换过              Gate D CR-002 签字
          ↓                                    ↓                                      ↓                          ↓
      推到 P3 代码生成                   推到 P6 切换灰度                   推到 P9 生产运行               ✅ 全部功能开发完成
```

---

## 2. W1–W12 每周门禁（未过不推进 · 自动回滚）

### 冲刺 1 · 立即修补（W1–W2）· 最终推到 Gate A = 设计完整性

| 周 | 阶段 | 本周自动交付物 | 门禁硬指标（不过就回滚） | 回滚方式 | 责任人 |
|---|---|---|---|---|---|
| **W1** | 需求落图 · P0/P1 | 1) 初始 Project/Tenant/Requirement Node 全部落 L3.5<br/>2) `json-store.js` 原子写补丁 MR<br/>3) 300MB 烟测脚本 CI 合入 | ① JSON 半写混沌测试 10^5 次=0 失败<br/>② 烟测 hashMismatch=0<br/>③ 图谱孤立节点 ≤ 0.5%（红1） | 还原 json-store 旧代码 + 烟测脚本回退 | 后端 Lead |
| **W2** | 架构 & UI 设计 · P2 + Gate A | 1) 6 层架构图 + API 清单自动生成<br/>2) UI 页面清单 × 组件映射表<br/>3) P2 Design Doc PDF（电子签名） | ① Gate A：设计覆盖率 100%（P0 需求到 Design Doc 可追踪）<br/>② 金链 1 第 1 段（Requirement→Design）findPath 非空=100%<br/>③ 前端页面深度 ≤ 4 层 | 回滚到 W1 末快照 + 重跑 P2 AI 设计 | 架构 + 前端 |

### 冲刺 2 · AI 核心开发（W3–W6）· 最终推到 Gate B = 质量门禁

| 周 | 阶段 | 本周自动交付物 | 门禁硬指标（不过就回滚） | 回滚方式 | 责任人 |
|---|---|---|---|---|---|
| **W3** | AI 代码生成 · P3 L5/L4 | 1) L5 前端 AI 工作台页面路由 + 表单组件<br/>2) L4 AI Engine 14 阶段工作流<br/>3) 单测骨架 + E2E 骨架 | ① L5 路由覆盖率 100%<br/>② P0→P5 端到端跑通率 ≥ 80%<br/>③ SAST 扫描 Critical=0（B3） | 回滚 P3 代码生成产物 + 重新 Prompt | AI 开发 Lead |
| **W4** | AI 代码生成 · P3 L3/L3.5 | 1) 云盘 4 Backend FS/S3/MinIO/OSS 全部挂通<br/>2) StorageProvider 4 引擎等价测试<br/>3) `graph_edges` 表 + 6 接口 SQLite/PG 双实现 | ① Key 同构：4 Backend `xx/hash` 公式字节一致（§3.4 唯一处）<br/>② 4 Provider 等价测试 100%<br/>③ 金链 3 条骨架 Node/Edge 可写 | 回滚 P3 末快照 + 关双写 | 后端 + DBA |
| **W5** | 云盘图谱基线 · P4 | 1) SQLite+FS 种子数据 + 初始化脚本<br/>2) 版本管理 + 引用计数 GC 冷/热路径<br/>3) L3+L3.5 冒烟包 Docker Image | ① 秒级版本回滚 0 拷贝 ≥ 100 次通过<br/>② GC 回收准确率 ≥ 99%（向 99.99% 逼近）<br/>③ 单机 SLA ≥ 99.5%（T0 档位） | 重建基线数据库 + 从备份恢复 chunk | SRE + 后端 |
| **W6** | 测试 & 修复 · P5 + Gate B | 1) 单测报告 / 集成测试报告 / E2E 报告（三份 JSON）<br/>2) SAST + DAST 报告<br/>3) 缺陷清单（缺陷 AI 自动修复 PR） | **① Gate B：单测覆盖率 矩阵达标（L5≥80% / L3≥95% / L3.5≥95%）**<br/>**② Critical 缺陷 = 0（B3）；hashMismatch = 0（B4）；WORM 篡改 = 0（B7）**<br/>③ 37 项 SLO 前 12 项 ≥ SLA | 回滚到 W4 末代码基线 + 缺陷批量重修复 | 测试 Lead |

### 冲刺 3 · 切换 + 灰度（W7–W9）· 最终推到 Gate C = 切换门禁

| 周 | 阶段 | 本周自动交付物 | 门禁硬指标（不过就回滚） | 回滚方式 | 责任人 |
|---|---|---|---|---|---|
| **W7** | 切换 · P6 + 预检查 + 双写 | 1) 图谱：SQLite→PG 双写启动 + 存量迁移 Job 报告<br/>2) 云盘：FS→S3/MinIO Backend 热切 env<br/>3) 切换前快照 + 回滚演练录像（**三大铁律第3条先演练成功**） | ① 回滚演练通过率 = 100%（C2 红线）<br/>② 双写一致率：每 5min 采样差异 = 0<br/>③ 存量迁移进度 100% | **关双写 + 回滚 env 到原 Backend**（C2 回滚链） | SRE + 后端 |
| **W8** | 灰度 · P7 四阶段 + 空读回填 | 1) 5%→25%→75%→100% 四阶段灰度报告<br/>2) 空读回填完成率 100%<br/>3) 37 项 SLO 实时看板截图每小时 | ① 任何阶段错误率 > 0.5% → 立即降一级<br/>② 空读回填 7×24 首段窗口差异 = 0<br/>③ GC 回收准确率 ≥ 99.99%（C4 红线） | 灰度百分比回滚 + 清空回填队列 | SRE |
| **W9** | UAT · P8 + Gate C | 1) UAT 用例执行报告（100% 通过截图）<br/>2) 业务方电子签名（CR-002 D1 页）<br/>3) 对账窗口中期报告（第 3 天 × 第 5 天 × 第 7 天） | **① Gate C：对账差异 = 0 连续 7×24h（C3 红线）**<br/>② UAT 通过率 = 100%（D1）<br/>③ 金链 2（切换审计链）findPath 非空 = 100%（红2） | **关双写回切原后端** + 保留新后端作为只读；绝不允许带病推进 | SRE + 业务负责人 |

### 冲刺 4 · 验收 + 交付（W10–W12）· 最终 Gate D = CR-002 签字 = 全部功能开发完成

| 周 | 阶段 | 本周自动交付物 | 门禁硬指标（不过就回滚） | 回滚方式 | 责任人 |
|---|---|---|---|---|---|
| **W10** | 生产运行 · P9/P10 | 1) 生产运行日报 × 7<br/>2) FinOps 容量报告（¥/GB·月 ≤ ¥0.035）<br/>3) 分片 / 升降档评估 | ① MTTR < 5min（D3）；SLA ≥ 99.99%（D5，T3 档位；T0-T2 按 §3.2 档位阈值）<br/>② 告警绑定 Runbook 14/14 = 100%（F1-F14）<br/>③ 加权 FinOps 未超阈值 | 容量预警触发自动升档脚本 | SRE + FinOps |
| **W11** | 审计归档 · P11 | 1) 7/7 证据包（代码 / 测试 / 切换 / 灰度 / 运行 / 告警 / 缺陷）入 hash_chain<br/>2) 合规扫描报告（审计零红项 E2）<br/>3) WORM 归档对象清单 + 篡改 0 通过报告 | **① hash_chain 连续 7 天区块 = 7/7（E1 红线）**<br/>② WORM 篡改测试 0 通过（B7）<br/>③ 合规红项 = 0（E2） | hash_chain 回放到上一合法块 + 重新归档 | 合规 + 后端 |
| **W12** | AAR + 最终签字 · P12 + Gate D | 1) AAR 事后复盘报告 + `improves_next` 落边 → 下次 P0<br/>2) CR-002 28 项验收清单电子签名（业务/架构/安全/SRE 四方）<br/>3) 项目总结报告（本文档第 4 章模板） | **Gate D（最终开发完成）：3 要件同时满足 → 打完成标签**<br/>① CR-002 28 项 = 100%<br/>② 图谱治理红 1/2/3 全部通过（孤立≤0.5%·金链 3/3·Edge tombstone=100%）<br/>③ 7×24 对账差异 = 0 连续 14 天（W9 第 7 天 + W10-W11 运行期） | 回滚 P9 生产运行基线 + 重新开对账窗口；CR-002 不签字不允许宣称「开发完成」 | **四方签字**：业务 + 架构 + 安全合规 + SRE |

---

## 3. 测试报告 / 验收结果 / 项目总结（三份交付物模板 · MUST 输出）

> **Experience 1179649 强提醒：** 必须产出这三份，不能只写"建议下一步"。

### 3.1 测试报告模板（W6D5 / W9D5 / W12D5 输出 3 版）

```markdown
# MOX 全量测试报告 v2.0 · 版本 <build-hash>

## 一、测试范围
- 阶段：P0-P<X> 阶段
- 覆盖层级：L5 / L4 / L3 / L3.5 / L2 / L1
- 执行时间：<开始> → <结束> · 累计 <h>h

## 二、用例统计
| 类型     | 总数 | 通过 | 失败 | 阻塞 | 通过率 | 覆盖率目标 | 实际覆盖率 |
|----------|------|------|------|------|--------|------------|------------|
| 单元测试 |      |      |      |      |        |            |            |
| 集成测试 |      |      |      |      |        |            |            |
| E2E 端到端 |     |      |      |      |        |            |            |
| SAST/DAST 安全 |     |      |      |      |        | Critical=0 |            |
| 混沌/灾备 |      |      |      |      |        | 100%       |            |

## 三、缺陷统计（按严重度）
- Critical：<N> · MUST = 0
- High：<N> · 修复率 ≥ 95%
- Medium：<N> · 修复率 ≥ 80%
- Low：<N> · 遗留清单（必须有责任人 + 日期）

## 四、专项红线验证（必须每项截图附件）
1. hashMismatch = 0 （B4）
2. WORM 篡改 = 0 （B7）
3. 越权 = 0 租户泄漏 = 0 （A6）
4. GC 回收准确率 ≥ 99.99% （C4）
5. 7×24 对账差异 = 0 （C3）
6. 图谱红 1/2/3 全部通过

## 五、风险 & 遗留
- 已关闭风险：<清单>
- 未关闭风险：<清单> · 必须有 <缓解方案> + <责任人> + <截止日期>
```

### 3.2 CR-002 验收结果模板（W12D5 签字版）

```markdown
# MOX CR-002-2026 验收清单 · 验收结果
> 对应归一化总纲 §7 · 28 项 A-E 五大类
> 电子签名系统托管位置：<url>

| 类 | 编号 | 要求条目（简） | 证据附件 | 责任人 | 签字日期 | 通过？ |
|---|---|---|---|---|---|---|
| A | A1 | CR-001 变更申请已签字 | PDF-01 |  |  | ✅ |
| ... | ... | ... | ... | ... | ... | ... |
| E | E2 | 审计零红项 | PDF-28 |  |  | ✅ |

## 汇总
- 通过项：28 / 28
- 未通过项：0 / 28
- 红线警戒项（A6 / B3 / B4 / B7 / C2 / C3 / C4 / E1 / E2 / 红 1/2/3 / D5）：全部通过
- **验收结论：✅ 通过（28/28） / ⚠️ 条件通过（需补 <清单>） / ❌ 不通过**
```

### 3.3 项目总结报告模板（W12D5 最终版）

```markdown
# MOX 企业级全自动开发项目总结 v2.0

## 1. 项目概况
- 周期：W1 YYYY-MM-DD → W12 YYYY-MM-DD · 共 12 周
- 范围：6 层全栈开发 / 12 阶段 / 4 Backend / 4 Provider / T0→T1 档位
- 投入人天：<人天>

## 2. 交付物清单（对照归一化总纲 §8 里程碑）
- 代码：主干标签 <tag-v2.0.0> · 提交 <hash>
- 规范文档：`MOX-Enterprise-Unified-Spec-v2.0.md`
- 执行计划：本文件 `MOX-Fullstack-Auto-Delivery-Plan-v2.0.md`
- 报告：测试报告 × 3 版 · CR-002 签字版 · hash_chain 7/7
- 镜像：Docker Image `<registry>/mox:<v2.0.0>` · Helm 伞图 `<chart-version>`

## 3. 关键指标达成
| 指标 | 目标 | 实际 | 是否达标 |
|---|---|---|---|
| SLA | ≥ 99.99% (T3) / §3.2 档位 |  |  |
| RTO / RPO | RTO<30s · RPO=0 |  |  |
| hash 差异 | =0 |  |  |
| 图谱孤立节点 | ≤0.5% |  |  |
| CR-002 28项 | 100% |  |  |
| ¥/GB·月（T3） | ≤0.035 |  |  |

## 4. 经验教训 + 组织自进化（AAR · improves_next）
- 做得好的：<清单> → 保留在下次 P0
- 做得不好的：<清单> → `improves_next` Edge 关联下次 Project Node（红2金链 3/3）
- 需要沉淀的规范 / 脚本 / Runbook：<清单>

## 5. 后续计划
- 升档 T1→T2：W13–W24（按总纲 §8）
- T2→T3 多 Region：W25–W40（按总纲 §8）
```

---

## 4. 自动回滚机制总表（统一一张，避免各周重复写）

> 原则：三大铁律第 3 条 —— 正式推之前回滚演练必须先成功。

| 触发条件 | 自动执行动作 | 完成判定 | 人工介入阈值 |
|---|---|---|---|
| JSON 半写 > 0 | 回滚 json-store 原子写补丁 | 混沌 10^5 次 = 0 失败 | 30 分钟未通过 → 升级架构 |
| 烟测 hashMismatch > 0 | 停 CI + 回滚近 24h 合入的 chunk-backend 改动 | 重跑烟测 3 次全过 | 1 小时未过 → 升级后端 Lead |
| Gate A 设计不完整 | 回滚 W2 末快照 + 重调 P2 设计 Prompt | 覆盖率 100% + 金链 1 第一段非空 | 2 次重跑仍不过 → 人工审设计 |
| Gate B 覆盖率不达标 / Critical > 0 | 回滚到 W4 末代码 + 缺陷 AI 自动重修复队列 100% 再跑 | 覆盖率达标 + Critical=0 | 重跑 ≥ 3 次 → 测试 Lead 人工筛缺陷 |
| Gate C 对账差异 > 0 | **立即关双写回切原 Backend**（绝不手软） | 差异=0 连续 24h | 48h 差异仍>0 → 发起专项会议 + 三大铁律复盘 |
| Gate D 28 项未过 | 回滚 P9 生产基线 + 重开对账窗口 | 重新到 C3/C4 红线 | 2 次冲刺仍不过 → CR-001 变更延期申请 |

---

## 5. 代码锚点与子规范索引（和归一化总纲保持一致 · 不重复定义）

**代码锚点（唯一引用处均在总纲 §2–§8，本计划只列路径，不重复贴行号）：**
1. [config.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/config.js) —— 环境变量默认值
2. [db.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/db.js) —— SQLite Schema（graph_edges DDL 见总纲 §5.2）
3. [storage/index.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js) —— Provider 抽象 + DualWriteStorage
4. [chunk-backend.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js) —— Key 同构 FS L62 ↔ S3 L164（唯一真源）
5. [file-store.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js) —— 版本 + GC
6. [expert-alliance-engine.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/expert-alliance-engine.js) —— 原子写正确实现（json-store 本周要抄）
7. [ai-engine.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine.js) —— PageRank 复用（L3.5 接口 §5.4）

**子规范索引（操作步骤去附录 B，本计划不展开）：**
- B-01 `deploy/docs/FS-S3-full-lifecycle-ops-guide.md` · 9 日切换节奏操作手册 → W7-W8 执行依据
- B-02 `deploy/docs/storage-cloud-switch-sop.md` · 双写/空读回填脚本 → W7/W8 自动化脚本来源
- B-03 `deploy/docs/filesystem-backend-structure-sop.md` · 目录树 & Key 自检脚本 → W1 烟测 & W4 同构测试依据
- B-04 `deploy/docs/ha-capacity-tco.md` · 容量 & FinOps → W10 FinOps 报告依据
- B-06 `deploy/docs/ops-manual.md` · 巡检 / 备份 / 恢复 → P9 日常运行
- B-07 `deploy/docs/trace-8stages-dashboard.json` · OTel 看板 → W6 P5 性能指标采集

---

> **生效声明：** 本计划是 MOX v2.0 12 周全自动开发的唯一执行级文件。任何与子手册冲突，以归一化总纲 + 本计划先后次序判定；任何一周门禁硬指标未过，**自动回滚，绝不推进下一周**，违者 CR 直接打回。
