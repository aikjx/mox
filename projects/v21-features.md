# XUANJI v2.1 Feature Flag Reference (v20260825)

本文件列出 XUANJI 仓库中通过 Cargo `[features]` 暴露的、会影响编译产物
算法/协议/数据平面对齐行为的开关。每个 feature 记录默认值、作用域、
构建命令、兼容性说明与风险等级，供发布/回归/SRE 在冻结窗口核对。

---

## 1. `simd`

| 项 | 值 |
| --- | --- |
| 默认 | 关闭 (off by default) |
| 声明位置 | `platform/services/xuanji-cloud-drive-volume/Cargo.toml` |
| 作用 | 启用 xuanji-cloud-drive-volume 的 AVX2 / NEON 向量化 GF(2^8) 乘法，用于 2+1 XOR 纠删码与本地卷的块校验加速。 |
| 回退策略 | 无对应 CPU 特性的编译目标 / 主机自动 fallback 到标量实现；功能一致，仅吞吐下降。 |
| 构建命令 | `cargo build --release -p xuanji-cloud-drive-volume --features simd` |
| 兼容性 | 与 `glacier` / `gm-sm` 正交；可叠加使用。 |
| 风险等级 | **低** —— 纯计算密集路径；算法正确性由 scalar 路径镜像单测覆盖。 |

---

## 2. `gm-sm`

| 项 | 值 |
| --- | --- |
| 默认 | 关闭 (off by default) |
| 声明位置 | `platform/services/xuanji-standards/Cargo.toml`（下游 `xuanji-server` 通过 `cargo --features gm-sm` 透传） |
| 作用 | 启用国密算法族：<br>• 审计 HashChain 用 **SM3** 替换 SHA-256<br>• STS 临时凭证追加 **SM2** 签名<br>• 对象 PUT/GET 数据面走 **SM4-GCM** 块加/解密 |
| 回退策略 | 开启后所有兼容层走国密；不开启则沿用 FIPS/AES-GCM/SHA256。迁移窗口可与 `dual_chain` 同时打开。 |
| 构建命令 | `cargo build --release -p xuanji-server --features gm-sm,glacier,simd` |
| 兼容性 | 与 `glacier` / `simd` 兼容；**与已写入的 SHA-256 链不兼容** —— 新建集群前决定，不支持热切换。若需要迁移窗口请同时启用 `dual_chain`。 |
| 风险等级 | **高** —— 影响对象不可变数据面与审计链哈希算法；上线前需跑完整 T10 等保矩阵 + 跨区域复制校验。 |

---

## 3. `glacier`

| 项 | 值 |
| --- | --- |
| 默认 | 关闭 (off by default) |
| 声明位置 | `platform/services/xuanji-cloud-drive-s3/Cargo.toml`（`xuanji-server` 聚合 features 时透传） |
| 作用 | 启用 AWS S3 `StorageClass=GLACIER` 支持：<br>• GlacierAdapter：冷存储 PUT/GET/HEAD 走 4 小时 Restore 状态机<br>• Tiered-lifecycle：对象年龄 > N 天自动 CLS→GLACIER<br>• RestoreJob API：Expedited/Standard/Bulk 三档恢复 |
| 回退策略 | 关闭后 Glacier 存储类被视为非法，写入 400；不影响 STANDARD/IA。 |
| 构建命令 | `cargo build --release -p xuanji-server --features gm-sm,glacier,simd` |
| 兼容性 | 与 `gm-sm` / `simd` 正交；**对已归档对象的取回必须开启该 feature**，否则状态机 handler 未编译。 |
| 风险等级 | **中** —— 状态机失败会落 DLQ；不会丢失对象但可能产生 24h 级的取回延迟。 |

---

## 4. `dual_chain`

| 项 | 值 |
| --- | --- |
| 默认 | 关闭 (off by default) |
| 声明位置 | `platform/services/xuanji-standards/Cargo.toml` |
| 作用 | 审计链 **同时写入 SHA-256 + SM3 两条哈希**。适用于等保整改 / 国密迁移窗口期，允许新集群读 SHA-2 旧链的同时生成 SM3 新链，供 Cut-over 比对。 |
| 回退策略 | 关闭后仅写入默认算法（`gm-sm` 关 → SHA-256，`gm-sm` 开 → SM3）。 |
| 构建命令 | 建议与 `gm-sm` 同时开启：`cargo build --release -p xuanji-server --features gm-sm,dual_chain` |
| 兼容性 | 独立使用无意义；**必须**与 `gm-sm` 或纯 SHA-2 默认中的一个同时启用。写入体积翻倍，磁盘 / 对象大小预算需 × 2。 |
| 风险等级 | **中** —— 迁移窗口结束后应在一次冷维护中关闭；残留双写会无谓消耗 I/O 与存储。 |

---

## 矩阵速查

| 组合 | 典型场景 |
| --- | --- |
| `--features simd` | 纯数据面性能敏感，无合规要求 |
| `--features gm-sm,dual_chain` | 等保整改迁移中（老链 SHA-2 → 新链 SM3） |
| `--features gm-sm,glacier,simd` | 生产全量：国密合规 + 冷归档 + SIMD 吞吐 |
| (默认, 无 features) | 开发 / CI 回归基准 |

> 文档版本号 `v20260825` 与创建日期一致；每次新增或修改 feature flag
> 需同步 bump 本文件顶部版本号并在 `t19-regression/runs/` 下记录回归矩阵。
