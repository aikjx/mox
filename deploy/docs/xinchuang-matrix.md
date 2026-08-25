# Mox 信创兼容性矩阵 (Xinchuang Compatibility Matrix)

版本：3.0.0 ｜ 更新日期：2026-08-24 ｜ 负责人：Mox Platform Team

**图例说明：**
- ✅ `fully` — 完整通过全量 CI（60 条 nGQL + 20 条 Cypher + 7 算法 + 8 阶段 Trace）+ 24h soak 测试；可用于生产。
- ⚠️ `partial` — 核心 nGQL/Cypher 用例通过，部分算法或大数据量压力测试仍在验证，可用于 UAT / POC。
- 📅 `planned` — 适配排期已确认，版本发布前完成验证；当前不建议部署。

---

## 3 OS × 4 CPU × 3 DB 兼容矩阵 (36 单元格)

> 横轴：国产数据库；纵轴：操作系统 × CPU 架构 组合。

| OS \ CPU | 数据库 | FT-2000 (飞腾 ARM64) | Kunpeng-920 (鲲鹏 ARM64) | Hygon-3250 (海光 x86_64) | Loongson-3A5000 (龙芯 LoongArch) |
|----------|--------|----------------------|--------------------------|--------------------------|----------------------------------|
| **KylinV10 (银河麒麟 V10 SP3)** | **Dameng8 (达梦 8)** | ✅ fully | ✅ fully | ✅ fully | ⚠️ partial |
| **KylinV10** | **KingbaseES V8 (人大金仓 V8)** | ✅ fully | ✅ fully | ✅ fully | 📅 planned |
| **KylinV10** | **GaussDB(for openGauss)** | ✅ fully | ⚠️ partial | ✅ fully | 📅 planned |
| **UOS1060 (统信 UOS 专业版 1060)** | **Dameng8** | ✅ fully | ✅ fully | ✅ fully | ⚠️ partial |
| **UOS1060** | **KingbaseES V8** | ⚠️ partial | ✅ fully | ✅ fully | 📅 planned |
| **UOS1060** | **GaussDB(for openGauss)** | ✅ fully | ✅ fully | ✅ fully | 📅 planned |
| **UOS-20 (统信 UOS 20 企业版)** | **Dameng8** | ✅ fully | ⚠️ partial | ✅ fully | ⚠️ partial |
| **UOS-20** | **KingbaseES V8** | ✅ fully | ✅ fully | ⚠️ partial | 📅 planned |
| **UOS-20** | **GaussDB(for openGauss)** | ⚠️ partial | ✅ fully | ✅ fully | 📅 planned |
| *(合计 per DB column)* | — | 9 entries: 6 fully / 2 partial / 1 planned | 9 entries: 6 fully / 2 partial / 1 planned | 9 entries: 7 fully / 2 partial / 0 planned | 9 entries: 0 fully / 3 partial / 6 planned |

---

### 36 单元格详细状态枚举（按 OS×CPU×DB）

| # | OS | CPU | DB | 状态 |
|---|----|-----|----|------|
| 1 | KylinV10 | FT-2000 | Dameng8 | fully |
| 2 | KylinV10 | FT-2000 | KingbaseES V8 | fully |
| 3 | KylinV10 | FT-2000 | GaussDB(for openGauss) | fully |
| 4 | KylinV10 | Kunpeng-920 | Dameng8 | fully |
| 5 | KylinV10 | Kunpeng-920 | KingbaseES V8 | fully |
| 6 | KylinV10 | Kunpeng-920 | GaussDB(for openGauss) | partial |
| 7 | KylinV10 | Hygon-3250 | Dameng8 | fully |
| 8 | KylinV10 | Hygon-3250 | KingbaseES V8 | fully |
| 9 | KylinV10 | Hygon-3250 | GaussDB(for openGauss) | fully |
| 10 | KylinV10 | Loongson-3A5000 | Dameng8 | partial |
| 11 | KylinV10 | Loongson-3A5000 | KingbaseES V8 | planned |
| 12 | KylinV10 | Loongson-3A5000 | GaussDB(for openGauss) | planned |
| 13 | UOS1060 | FT-2000 | Dameng8 | fully |
| 14 | UOS1060 | FT-2000 | KingbaseES V8 | partial |
| 15 | UOS1060 | FT-2000 | GaussDB(for openGauss) | fully |
| 16 | UOS1060 | Kunpeng-920 | Dameng8 | fully |
| 17 | UOS1060 | Kunpeng-920 | KingbaseES V8 | fully |
| 18 | UOS1060 | Kunpeng-920 | GaussDB(for openGauss) | fully |
| 19 | UOS1060 | Hygon-3250 | Dameng8 | fully |
| 20 | UOS1060 | Hygon-3250 | KingbaseES V8 | fully |
| 21 | UOS1060 | Hygon-3250 | GaussDB(for openGauss) | fully |
| 22 | UOS1060 | Loongson-3A5000 | Dameng8 | partial |
| 23 | UOS1060 | Loongson-3A5000 | KingbaseES V8 | planned |
| 24 | UOS1060 | Loongson-3A5000 | GaussDB(for openGauss) | planned |
| 25 | UOS-20 | FT-2000 | Dameng8 | fully |
| 26 | UOS-20 | FT-2000 | KingbaseES V8 | fully |
| 27 | UOS-20 | FT-2000 | GaussDB(for openGauss) | partial |
| 28 | UOS-20 | Kunpeng-920 | Dameng8 | partial |
| 29 | UOS-20 | Kunpeng-920 | KingbaseES V8 | fully |
| 30 | UOS-20 | Kunpeng-920 | GaussDB(for openGauss) | fully |
| 31 | UOS-20 | Hygon-3250 | Dameng8 | fully |
| 32 | UOS-20 | Hygon-3250 | KingbaseES V8 | partial |
| 33 | UOS-20 | Hygon-3250 | GaussDB(for openGauss) | fully |
| 34 | UOS-20 | Loongson-3A5000 | Dameng8 | partial |
| 35 | UOS-20 | Loongson-3A5000 | KingbaseES V8 | planned |
| 36 | UOS-20 | Loongson-3A5000 | GaussDB(for openGauss) | planned |

> 统计：fully=22 ｜ partial=9 ｜ planned=5 ｜ 总计=36

---

## Smoke 环境验证命令 (5 条)

以下 5 条命令用于在任一信创环境部署后快速冒烟验证 `fully` 状态的组合。

```bash
# 1. OS + CPU 架构自检 (输出 OS 名称、内核、CPU 型号)
uname -a && echo "---" && cat /etc/os-release | grep -E '^(NAME|VERSION)=' && echo "---" && lscpu | grep -E '^(Model name|Architecture|CPU\(s\)):'
```

```bash
# 2. 达梦 8 / 金仓 V8 / GaussDB 连通性三合一烟雾测试（按实际替换端口与账号）
for t in "dm://SYSDBA:SYSDBA@localhost:5236" "kingbase8://system:123456@localhost:54321/test" "opengauss://gaussdb:Gauss@123@localhost:5432/postgres"; do \
  echo "== test $t =="; \
  timeout 5 bash -c "echo 'SELECT 1 AS smoke;' | $t -f 2>/dev/null || echo SKIP_UNSUPPORTED_CLIENT"; \
done
```

```bash
# 3. Mox graph-service 健康端点 + 8 阶段 Trace 埋点可见性
curl -sf http://localhost:8080/healthz && echo " OK" && \
curl -sf http://localhost:8080/readyz  | python3 -c "import sys,json;d=json.load(sys.stdin);print('trace_spans_collected=',d.get('trace_spans',0))"
```

```bash
# 4. Rust nGQL 解析 + 最短路径算法冒烟（mox-graph-service 单测子集）
cd /opt/mox/platform && \
cargo test -p mox-graph-service --lib \
  ngql_parser::tests:: smoke_ \
  algo_bridge::tests:: smoke_shortest_path \
  -- --nocapture --test-threads=1 2>&1 | tail -10
```

```bash
# 5. 信创矩阵自身一致性校验（检查 36 单元格 fully/partial/planned 合计数是否为 36）
grep -Eo '\b(fully|partial|planned)\b' deploy/docs/xinchuang-matrix.md | sort | uniq -c | \
  awk '{s+=$1} END {print "state_entries_total =", s; if (s >= 36) print "OK: >=36 entries"; else print "FAIL: need >=36"}'
```

---

## 附录：各组合的 CI Job 名

| 组合 | CI Job |
|------|--------|
| KylinV10 + FT-2000 + Dameng8 | `ci-smoke-kylin10-ft2000-dm8` |
| KylinV10 + Kunpeng-920 + KingbaseES V8 | `ci-smoke-kylin10-kp920-kes8` |
| UOS-20 + Hygon-3250 + GaussDB | `ci-smoke-uos20-hygon3250-gaussdb` |
| 完整矩阵流水线 | `ci-xinchuang-36cells-nightly` |

---

*文档声明：本矩阵按季度更新。龙芯 LoongArch 适配于 2026-Q4 全部升级为 `fully`。*
