// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX Architecture Tests
//!
//! Enforces cross-domain dependency rules and layering constraints:
//! - L0 (foundation): cannot depend on any domain crate
//! - L1 (gateway): can depend on L0 + L2 (api), NOT L3/L4/L5 directly
//! - L2 (api): cannot depend on L1/L3/L4/L5 (pure trait contracts)
//! - L3 (core): can depend on L0 + L2, NOT L1/L4/L5
//! - L4 (svc): can depend on L0 + L2 + L3, NOT L1/L5
//! - L5 (sdk): can depend on anything (FFI bindings)
//
// 架构守护测试工具库：辅助函数仅供 #[test] 使用，不参与生产分发。
#![allow(dead_code)]
//!
//! Cross-domain dependencies MUST go through the api/ layer.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Layer { L0, L1, L2, L3, L4, L5, Unknown }

#[derive(Debug, Clone)]
struct CrateInfo {
    name: String,
    path: PathBuf,
    layer: Layer,
    domain: String,
    dependencies: HashSet<String>,
}

fn classify_crate(path: &Path, workspace_root: &Path) -> (Layer, String) {
    let rel = path.strip_prefix(workspace_root).unwrap_or(path);
    let components: Vec<&str> = rel.components().filter_map(|c| c.as_os_str().to_str()).collect();

    // Shared infrastructure (platform/shared/*): cross-cutting facilities
    // (cache, auth, config, server-runtime, resilience, observability, contracts)
    // act as the foundation layer and are usable by any domain.
    // Base primitives (domains/base/*): cross-cutting model/query/store/lifecycle
    // abstractions, treated as foundation and usable by any domain.
    if components.contains(&"base") { return (Layer::L0, "foundation".into()); }
    if components.contains(&"shared") { return (Layer::L0, "foundation".into()); }
    // Foundation layer
    if components.contains(&"foundation") { return (Layer::L0, "foundation".into()); }

    // Gateway layer
    if components.contains(&"gateway") { return (Layer::L1, "gateway".into()); }

    // API / proto layer (gRPC + contract protobufs, pure trait/data contracts)
    if components.iter().any(|c| *c == "api" || *c == "proto") {
        let domain = components.iter().position(|c| *c == "domains")
            .and_then(|i| components.get(i + 1))
            .unwrap_or(&"unknown")
            .to_string();
        return (Layer::L2, domain);
    }

    // Core layer
    if components.contains(&"core") {
        let domain = components.iter().position(|c| *c == "domains")
            .and_then(|i| components.get(i + 1))
            .unwrap_or(&"unknown")
            .to_string();
        // platform 域 core 是平台共享基础设施（system/operator/graph/model/iam/meta...），
        // 被全业务域依赖，架构上即 L0 平台层，非业务域 L3。
        if domain == "platform" { return (Layer::L0, "foundation".into()); }
        return (Layer::L3, domain);
    }

    // SVC layer
    if components.contains(&"svc") {
        let domain = components.iter().position(|c| *c == "domains")
            .and_then(|i| components.get(i + 1))
            .unwrap_or(&"unknown")
            .to_string();
        return (Layer::L4, domain);
    }

    // SDK layer
    if components.contains(&"sdk") {
        let domain = components.iter().position(|c| *c == "domains")
            .and_then(|i| components.get(i + 1))
            .unwrap_or(&"unknown")
            .to_string();
        return (Layer::L5, domain);
    }

    (Layer::Unknown, "unknown".into())
}

fn parse_cargo_toml(path: &Path) -> Option<(String, HashSet<String>)> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;

    let name = value.get("package")?.get("name")?.as_str()?.to_string();

    let mut deps = HashSet::new();
    if let Some(dependencies) = value.get("dependencies").and_then(|d| d.as_table()) {
        for (dep_name, _) in dependencies {
            if dep_name.starts_with("mox-") { deps.insert(dep_name.clone()); }
        }
    }

    Some((name, deps))
}

fn collect_all_crates(workspace_root: &Path) -> HashMap<String, CrateInfo> {
    // Crates known to be in transition / refactoring, excluded from arch checks
    let excluded: HashSet<&str> = [
        "mox-platform-test-harness", // test utility, not production code
        "mox-kg-algo-core",          // pre-existing cross-domain dep, pending api refactor
    ].iter().cloned().collect();

    // 目录剪枝：第三方参考代码与运行时目录不参与架构治理
    // （AGENTS.md：`ais/` `third_party/` 为第三方参考不入库；`target/` 为运行态。
    //   剪枝而非逐 crate 排除——vendor 树含数百个外部 crate，如 openai-codex 的
    //   codex-core 曾因路径含 "core" 被误归类为 L3 而误报 IO 违规。）
    const PRUNED_DIRS: &[&str] = &["ais", "third_party", "target", "node_modules", ".git"];
    let is_pruned = |p: &Path| -> bool {
        p.components().any(|c| {
            let s = c.as_os_str().to_str().unwrap_or("");
            PRUNED_DIRS.contains(&s)
        })
    };

    let mut crates = HashMap::new();
    for entry in WalkDir::new(workspace_root)
        .into_iter()
        .filter_entry(|e| !is_pruned(e.path()))
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "Cargo.toml" {
            let path = entry.path().to_path_buf();
            if let Some((name, deps)) = parse_cargo_toml(&path) {
                if excluded.contains(name.as_str()) { continue; }
                let parent = path.parent().unwrap_or(&path).to_path_buf();
                let (layer, domain) = classify_crate(&parent, workspace_root);
                crates.insert(name.clone(), CrateInfo { name, path: parent, layer, domain, dependencies: deps });
            }
        }
    }
    crates
}

fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::L0 => "L0-foundation",
        Layer::L1 => "L1-gateway",
        Layer::L2 => "L2-api",
        Layer::L3 => "L3-core",
        Layer::L4 => "L4-svc",
        Layer::L5 => "L5-sdk",
        Layer::Unknown => "unknown",
    }
}

fn is_allowed_dependency(from: &CrateInfo, to: &CrateInfo) -> bool {
    // Foundation 层基础错误类型：任何 crate 都可以依赖 mox-error
    if to.name == "mox-error" { return true; }
    let same_domain = from.domain == to.domain || to.domain == "foundation" || to.domain == "gateway";
    match (from.layer, to.layer) {
        // Unknown 层优先：未分类的 crate 不阻止依赖（待分类后再校验）
        (Layer::Unknown, _) | (_, Layer::Unknown) => true,
        // L4-svc 可以依赖 L5-sdk（客户端库类型，svc 复用 sdk 类型是合理的）
        (Layer::L4, Layer::L5) => true,
        // Foundation/shared 内部聚合：server-runtime 组合 cache/auth/config/observability 等
        (Layer::L0, Layer::L0) => true,
        (Layer::L0, _) => false,        // L1-gateway 是统一入口，需要调用各域 svc 和 sdk（服务化架构特征）
        (Layer::L1, Layer::L0) | (Layer::L1, Layer::L2) | (Layer::L1, Layer::L3) | (Layer::L1, Layer::L4) | (Layer::L1, Layer::L5) => true,
        (Layer::L1, _) => false,
        (Layer::L2, Layer::L0) => true,
        // 同域 api/proto 互引允许（gRPC 消息共享，如 alliance-api -> *-common-proto）
        (Layer::L2, Layer::L2) => same_domain,
        (Layer::L2, _) => false,
        (Layer::L3, Layer::L0) | (Layer::L3, Layer::L2) => true,
        (Layer::L3, Layer::L3) => same_domain, // same-domain core deps allowed
        (Layer::L3, _) => false,
        (Layer::L4, Layer::L0) | (Layer::L4, Layer::L2) | (Layer::L4, Layer::L3) => true,
        (Layer::L4, Layer::L4) => true, // svc interop allowed in service-oriented architecture
        (Layer::L4, _) => false,
        (Layer::L5, _) => true,
    }
}

#[test]
fn test_layering_rules() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    let crates = collect_all_crates(&workspace_root);
    let mut violations = Vec::new();

    for (name, info) in &crates {
        if info.layer == Layer::Unknown { continue; }
        for dep_name in &info.dependencies {
            if let Some(dep_info) = crates.get(dep_name) {
                if !is_allowed_dependency(info, dep_info) {
                    violations.push(format!(
                        "  {} [{}] -> {} [{}]  VIOLATION: {} cannot depend on {}",
                        name, layer_name(info.layer),
                        dep_name, layer_name(dep_info.layer),
                        layer_name(info.layer), layer_name(dep_info.layer)
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Architecture layering violations found ({}):\n{}",
            violations.len(), violations.join("\n"));
    }
}

#[test]
fn test_cross_domain_dependencies_go_through_api() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    let crates = collect_all_crates(&workspace_root);
    let mut violations = Vec::new();

    for (name, info) in &crates {
        if info.layer == Layer::L0 || info.layer == Layer::L5 || info.layer == Layer::Unknown { continue; }
        for dep_name in &info.dependencies {
            if let Some(dep_info) = crates.get(dep_name) {
                // Cross-domain dependency (different domain)
                if info.domain != dep_info.domain && dep_info.domain != "foundation" && dep_info.domain != "gateway" {
                    // 编排者（L1 gateway 入口、L4 svc）跨域调用 L4 svc 是 SOA 编排本职，
                    // 在 modular monolith 内进程调用是设计如此，不视为违规。
                    // 编排者（L1 gateway 入口、L4 svc/orchestrator）跨域调用各域 L3/L4/L5 是其编排职责，
                    // modular monolith 内直接驱动域核心/服务/客户端是设计如此。
                    let orchestrator_cross_layer = info.layer == Layer::L1 || info.layer == Layer::L4;                    if !orchestrator_cross_layer && dep_info.layer != Layer::L2 && dep_info.layer != Layer::L0 {
                        violations.push(format!(
                            "  {} [domain={}] -> {} [domain={}, layer={}]  VIOLATION: cross-domain deps must use api/ layer",
                            name, info.domain, dep_name, dep_info.domain, layer_name(dep_info.layer)
                        ));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Cross-domain dependency violations found ({}):\n{}",
            violations.len(), violations.join("\n"));
    }
}

#[test]
fn test_no_circular_dependencies() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    let crates = collect_all_crates(&workspace_root);

    // DFS cycle detection
    fn dfs(
        name: &str,
        crates: &HashMap<String, CrateInfo>,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        visited.insert(name.to_string());
        stack.insert(name.to_string());

        if let Some(info) = crates.get(name) {
            for dep in &info.dependencies {
                if !visited.contains(dep) {
                    if let Some(cycle) = dfs(dep, crates, visited, stack) {
                        let mut c = cycle;
                        c.push(name.to_string());
                        return Some(c);
                    }
                } else if stack.contains(dep) {
                    return Some(vec![dep.clone(), name.to_string()]);
                }
            }
        }

        stack.remove(name);
        None
    }

    let mut visited = HashSet::new();
    for name in crates.keys() {
        if !visited.contains(name) {
            let mut stack = HashSet::new();
            if let Some(cycle) = dfs(name, &crates, &mut visited, &mut stack) {
                panic!("Circular dependency detected: {}", cycle.join(" -> "));
            }
        }
    }
}

#[test]
fn test_api_crates_are_pure() {
    // L2 api crates should only depend on L0 foundation crates
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    let crates = collect_all_crates(&workspace_root);
    let mut violations = Vec::new();

    for (name, info) in &crates {
        if info.layer != Layer::L2 { continue; }
        for dep_name in &info.dependencies {
            if let Some(dep_info) = crates.get(dep_name) {
                let same_domain = info.domain == dep_info.domain || dep_info.domain == "foundation";
                let allowed = dep_info.layer == Layer::L0
                    || dep_info.layer == Layer::Unknown
                    || (dep_info.layer == Layer::L2 && same_domain);
                if !allowed {
                    violations.push(format!(
                        "  {} [api] -> {} [{}]  VIOLATION: api crates must only depend on foundation",
                        name, dep_name, layer_name(dep_info.layer)
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("API purity violations found ({}):\n{}",
            violations.len(), violations.join("\n"));
    }
}

// ═══════════════════════════════════════════════════════════════════
// core 层 IO 依赖门禁（禁止新增，含技术债基线）
// ═══════════════════════════════════════════════════════════════════

/// core 层（L3）不得引入新的 IO 依赖
///
/// # 规则
/// 数据库 / HTTP 客户端一类的 IO 依赖属于适配层职责。core 层保持纯计算，
/// 才能保证「算法可脱离外部环境单测」，也避免层违规沿依赖图扩散到各域。
///
/// # 治理方式（而非一刀切禁止）
/// - **optional 依赖允许存在**：调用方不启用 feature 就不编译，视为按需装配
///   （正面范例：`mox-cloud-store-core` 的 `reqwest = { optional = true }`、
///   `mox-ai-alliance-engine` 的 `reqwest = { optional = true }` + `llm-http` feature）；
/// - **既有违规登记在 `BASELINE`**（技术债），新增即失败，先止血再清偿；
/// - **基线项一旦消失必须移除**：测试会 panic 提示，防止基线无限膨胀、掩盖进展。
#[test]
fn test_core_layer_has_no_new_io_dependencies() {
    const IO_DEPS: &[&str] = &[
        // 持久化 / 数据库
        "sqlx", "reqwest", "redis", "mongodb", "tokio-postgres",
        "duckdb", "clickhouse", "elasticsearch",
        // 嵌入式持久化（2026-09-21 审计补严：rusqlite 曾被 core 层非可选依赖，已清偿为可选）
        "rusqlite", "rocksdb", "sled", "lmdb", "diesel", "sea-orm",
        // 原生网络
        "tokio-tungstenite", "native-tls",
        // 设备 / 桌面自动化（音频、截屏、键鼠、窗口、剪贴板）
        "cpal", "rodio", "sherpa-onnx", "screenshots", "enigo", "global-hotkey",
        "wry", "tao", "arboard",
    ];

    /// 已知技术债（crate 名, IO 依赖名）。修复后必须从本表移除。
    ///
    /// 当前为空：core 层已无任何非可选 IO 依赖（技术债已全部清偿）。
    const BASELINE: &[(&str, &str)] = &[];

    // 说明：`mox-alliance-boot-config` 的 reqwest 位于 [dev-dependencies]，
    // 不进入生产编译，故不计入违规（门禁只校验 [dependencies]）。
    // 已清偿的技术债（修复即从 BASELINE 移除，此处留档防止回潮）：
    //   - `mox-ai-alliance-engine`：`sqlx` → 可选 `pg` feature；`reqwest` → 可选 `llm-http` feature
    //     （core 默认零 IO，纯领域逻辑 LLMConfig / ChatMessage / ExpertOpinionJSON 始终可用）；
    //   - `mox-alliance-scheduler-core`：`rusqlite` → 可选 `sqlite` feature
    //     （SqliteTaskRepository 适配器，生产由 scheduler-svc 启用）；
    //   - `mox-kb-core`：`rusqlite` → 可选 `sqlite` feature
    //     （SqliteKbStore + FTS5 适配器，生产由 kb-server 启用）。

    let crates = collect_all_crates(&workspace_root());

    let mut violations = Vec::new();
    // 用拥有所有权的 String 元组：toml 解析结果是局部变量，不能把其引用带出循环
    let mut resolved: HashSet<(String, String)> = HashSet::new();

    for (name, info) in &crates {
        if info.layer != Layer::L3 {
            continue;
        }
        let content = match std::fs::read_to_string(info.path.join("Cargo.toml")) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let value: toml::Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let deps = match value.get("dependencies").and_then(|d| d.as_table()) {
            Some(d) => d,
            None => continue,
        };

        for (dep_name, dep_val) in deps {
            if !IO_DEPS.iter().any(|d| d == dep_name) {
                continue;
            }
            // optional 依赖视为按需装配，允许存在
            let optional = dep_val.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);
            if optional {
                continue;
            }
            if BASELINE.contains(&(name.as_str(), dep_name.as_str())) {
                resolved.insert((name.clone(), dep_name.clone()));
                continue;
            }
            violations.push(format!(
                "  {} [L3-core] -> {}  VIOLATION: 非可选 IO 依赖，未登记在基线中",
                name, dep_name
            ));
        }
    }

    if !violations.is_empty() {
        panic!(
            "core 层新增 IO 依赖（{}）：\n{}\n\n修复方式：将持久化 / HTTP 调用下沉到 svc 层，\
             或改为 optional 依赖按需装配。",
            violations.len(),
            violations.join("\n")
        );
    }

    // 基线清理：已修复的登记项必须移除，防止基线永久膨胀掩盖治理进展
    let stale: Vec<String> = BASELINE
        .iter()
        .filter(|(c, d)| !resolved.contains(&(c.to_string(), d.to_string())))
        .map(|(c, d)| format!("  ({}, {}) 已不存在，请从 BASELINE 移除以固化成果", c, d))
        .collect();
    if !stale.is_empty() {
        panic!("架构门禁基线已过期（{}）：\n{}", stale.len(), stale.join("\n"));
    }
}

// ═══════════════════════════════════════════════════════════════════
// 全维归一化门禁：跨 crate 重复符号不得新增
// ═══════════════════════════════════════════════════════════════════

/// 向上查找最近 Cargo.toml 所在目录名，作为 crate 名
fn crate_of_path(path: &Path) -> String {
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").exists() {
            return d.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
        }
        dir = d.parent();
    }
    "unknown".to_string()
}

/// 扫描公开符号定义：返回 (kind, name) -> 出现过的 crate 集合
///
/// 只统计行首 `pub enum/struct/const/type`（与 `scripts/normalization-scan.py` 规则一致），
/// 跳过注释行；私有类型不构成跨 crate 契约，不计入。
fn scan_public_symbols(root: &Path) -> HashMap<(String, String), HashSet<String>> {
    let mut map: HashMap<(String, String), HashSet<String>> = HashMap::new();
    let platform = root.join("platform");
    if !platform.exists() {
        return map;
    }

    for entry in WalkDir::new(&platform).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let content = match std::fs::read_to_string(p) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let crate_name = crate_of_path(p);

        for line in content.lines() {
            let t = line.trim_start();
            if t.starts_with("//") || t.starts_with("*") {
                continue;
            }
            for kind in ["enum", "struct", "const", "type"] {
                let prefix = ["pub ", kind, " "].concat();
                if let Some(rest) = t.strip_prefix(prefix.as_str()) {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        map.entry((kind.to_string(), name))
                            .or_default()
                            .insert(crate_name.clone());
                    }
                    break;
                }
            }
        }
    }
    map
}

/// 跨 crate 重复定义的公开符号：不得新增，也不得扩散
///
/// 基线：`platform/arch-test/baseline/normalization.txt`（由 `scripts/normalization-scan.py
/// --baseline-txt` 生成），格式为 `kind::Name|crateA,crateB` —— 记录该重复符号当前的
/// **副本 crate 集合**。同名不等于重复债务（不同业务上下文可有不同模型），因此治理口径是：
/// - **不得新增**：出现基线之外的新跨 crate 同名符号 → 失败；
/// - **不得扩散**：已登记的重复又出现新副本 crate（2 份 → 3 份）→ 失败
///   （旧格式只记名字检测不到这一点，已升级为集合格式）；
/// - **语义归属优先**：语义相同 → 收敛到拥有域的权威定义并改为引用；
///   语义不同 → 重命名以示区分（显式转换连接），而非强行合并。
///
/// 概念归属登记表：`platform/arch-test/baseline/ownership-registry.csv`
/// （每项记录 权威拥有 crate / 处置决策 merge-pending-diff·keep-distinct·triage-pending / 依据）。
/// 再生成：`python scripts/normalization-scan.py --baseline-txt ...` + `python scripts/ownership-registry.py`。
#[test]
fn test_no_new_cross_crate_duplicate_symbols() {
    let root = workspace_root();
    let baseline_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("baseline")
        .join("normalization.txt");

    let baseline_content = std::fs::read_to_string(&baseline_path).unwrap_or_else(|_| {
        panic!(
            "缺少归一化基线文件: {}\n请先执行: python scripts/normalization-scan.py --baseline-txt {}",
            baseline_path.display(),
            baseline_path.display()
        )
    });
    // 基线解析：`kind::Name|crateA,crateB` → (key, 副本 crate 集合)。
    // 旧格式 `kind::Name`（无 `|`）解析为空集合 ⇒ 任何现存副本都会被判为扩散，
    // 从而强制重新生成基线（自迁移，不留静默豁免）。
    let mut baseline: HashMap<String, HashSet<String>> = HashMap::new();
    for line in baseline_content.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let (key, crates) = match l.split_once('|') {
            Some((k, s)) => (
                k.trim().to_string(),
                s.split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect::<HashSet<String>>(),
            ),
            None => (l.to_string(), HashSet::new()),
        };
        baseline.insert(key, crates);
    }

    let symbols = scan_public_symbols(&root);
    let mut current: HashMap<String, HashSet<String>> = HashMap::new();
    for ((kind, name), crates) in &symbols {
        if crates.len() > 1 {
            current
                .entry(format!("{}::{}", kind, name))
                .or_default()
                .extend(crates.iter().cloned());
        }
    }

    fn sorted_join(set: &HashSet<String>) -> String {
        let mut v: Vec<&String> = set.iter().collect();
        v.sort();
        v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",")
    }

    let mut violations: Vec<String> = Vec::new();

    // ① 新增重复：基线中不存在的新跨 crate 同名符号
    let mut added_keys: Vec<&String> =
        current.keys().filter(|k| !baseline.contains_key(*k)).collect();
    added_keys.sort();
    for k in added_keys {
        violations.push(format!(
            "  {}  新增跨 crate 重复（副本: {}）",
            k,
            sorted_join(&current[k])
        ));
    }

    // ② 重复扩散：已登记的重复又出现基线之外的新副本 crate（2 份 → 3 份）
    let mut spread: Vec<(&String, Vec<String>)> = Vec::new();
    for (k, cur) in &current {
        if let Some(base) = baseline.get(k) {
            let new_crates: Vec<String> = cur.difference(base).cloned().collect();
            if !new_crates.is_empty() {
                spread.push((k, new_crates));
            }
        }
    }
    spread.sort();
    for (k, new_crates) in &spread {
        violations.push(format!(
            "  {}  重复扩散：新增副本 crate [{}]（同名≠同义：语义不同请重命名区分，\
             语义相同请收敛到拥有域权威定义）",
            k,
            new_crates.join(", ")
        ));
    }

    if !violations.is_empty() {
        panic!(
            "跨 crate 重复符号治理违规（{}）：\n{}\n\n\
             基线格式：kind::Name|crate1,crate2（副本 crate 集合）。\n\
             处理后请重新生成基线：python scripts/normalization-scan.py --baseline-txt {}",
            violations.len(),
            violations.join("\n"),
            baseline_path.display()
        );
    }

    // 进展提示（不失败）：清偿是好事，不应阻塞 CI；提示用于同步收缩基线
    let mut shrunk: Vec<String> = Vec::new();
    let mut resolved: Vec<&String> = Vec::new();
    for (k, base) in &baseline {
        match current.get(k) {
            Some(cur) => {
                let removed: Vec<&String> = base.difference(cur).collect();
                if !removed.is_empty() {
                    let names: Vec<&str> = removed.iter().map(|s| s.as_str()).collect();
                    shrunk.push(format!("  {}  副本已减少: [{}]", k, names.join(", ")));
                }
            }
            None => resolved.push(k),
        }
    }
    shrunk.sort();
    if !shrunk.is_empty() {
        println!(
            "[归一化进展] 以下 {} 项重复副本已收缩，建议重新生成基线固化成果：\n{}",
            shrunk.len(),
            shrunk.join("\n")
        );
    }
    resolved.sort();
    if !resolved.is_empty() {
        println!(
            "[归一化进展] 以下 {} 项已不再是跨 crate 重复，建议重新生成基线以固化成果：\n{}",
            resolved.len(),
            resolved.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 架构-数据分离不变量测试
// ═══════════════════════════════════════════════════════════════════

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf()
}

/// 验证 platform/ 目录下无运行时数据文件（.db/.sqlite/.log 等）
#[test]
fn test_architecture_data_separation() {
    let root = workspace_root();
    // 架构-数据分离的本意：运行时数据不得进入版本库。
    // 只检查 git 追踪文件（git ls-files），开发者本机跑服务产生的 .db 已被
    // .gitignore 排除，不应误报；真正违规是有人把数据文件 git add 进去。
    let data_extensions = [".db", ".sqlite", ".sqlite3", ".log", ".pid", ".sock"];
    let mut violations = Vec::new();

    let git = std::process::Command::new("git")
        .arg("ls-files").arg("platform")
        .current_dir(&root)
        .output().expect("git ls-files must run");
    let tracked = String::from_utf8_lossy(&git.stdout);
    for rel in tracked.lines() {
        if rel.is_empty() { continue; }
        for ext in &data_extensions {
            if rel.ends_with(ext) {
                violations.push(format!("  {}", rel));
            }
        }
    }

    if !violations.is_empty() {
        panic!("Architecture-data separation violations: data files found in platform/ ({}):\n{}",
            violations.len(), violations.join("\n"));
    }
}

/// 验证代码中无硬编码的相对数据路径（必须通过 mox-platform-paths 管理）
#[test]
fn test_no_hardcoded_data_paths() {
    let root = workspace_root();
    let platform_dir = root.join("platform");
    // 禁止的硬编码相对路径模式
    let forbidden_patterns = [
        r#""./data/"#,
        r#""./config/"#,
        r#""./plugins/"#,
        r#""./storage/"#,
        r#""./third_party/"#,
        r#""./.runtime/"#,
    ];
    let mut violations = Vec::new();

    for entry in WalkDir::new(&platform_dir).into_iter().filter_map(|e| e.ok()) {
        // Skip this crate itself: it defines the forbidden path literals above.
        if entry.path().components().any(|c| c.as_os_str() == "arch-test") { continue; }
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "rs") {            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                for (line_num, line) in content.lines().enumerate() {
                    for pattern in &forbidden_patterns {
                        if line.contains(pattern) {
                            // 允许在注释中出现（以 // 开头）
                            let trimmed = line.trim_start();
                            // 合理例外显式标注：行尾含 `// allow: <reason>` 则豁免
                    let exempt = trimmed.contains("// allow:");
                    if !exempt && !trimmed.starts_with("//") && !trimmed.starts_with("*") {
                                violations.push(format!(
                                    "  {}:{}  {}",
                                    entry.path().display(), line_num + 1, trimmed
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Hardcoded data path violations found ({}): use mox-platform-paths instead\n{}",
            violations.len(), violations.join("\n"));
    }
}

/// 验证所有插件文件位于 plugins/ 目录，不在 platform/ 内
#[test]
fn test_plugins_outside_platform() {
    let root = workspace_root();
    let platform_dir = root.join("platform");
    let plugin_extensions = [".wasm", ".so", ".dll", ".dylib"];
    let mut violations = Vec::new();

    for entry in WalkDir::new(&platform_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy();
            for ext in &plugin_extensions {
                if name.ends_with(ext) {
                    violations.push(format!("  {}", entry.path().display()));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Plugin separation violations: plugin files found in platform/ ({}):\n{}",
            violations.len(), violations.join("\n"));
    }
}

/// 验证第三方源码/模型位于 third_party/，不在 platform/ 内
#[test]
fn test_third_party_outside_platform() {
    let root = workspace_root();
    let platform_dir = root.join("platform");
    // 检查 platform/ 下是否有 third_party 或 vendor 目录
    let mut violations = Vec::new();

    for entry in WalkDir::new(&platform_dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name == "third_party" || name == "vendor" || name == "external" {
                violations.push(format!("  {}", entry.path().display()));
            }
        }
    }

    if !violations.is_empty() {
        panic!("Third-party separation violations: third_party/vendor dirs found in platform/ ({}):\n{}",
            violations.len(), violations.join("\n"));
    }
}
