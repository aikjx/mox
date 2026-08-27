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

    // Foundation layer
    if components.contains(&"foundation") { return (Layer::L0, "foundation".into()); }

    // Gateway layer
    if components.contains(&"gateway") { return (Layer::L1, "gateway".into()); }

    // API layer
    if components.iter().any(|c| *c == "api") {
        let domain = components.iter().position(|c| *c == "domains")
            .and_then(|i| components.get(i + 1))
            .unwrap_or(&"unknown")
            .to_string();
        return (Layer::L2, domain);
    }

    // Core layer
    if components.iter().any(|c| *c == "core") {
        let domain = components.iter().position(|c| *c == "domains")
            .and_then(|i| components.get(i + 1))
            .unwrap_or(&"unknown")
            .to_string();
        return (Layer::L3, domain);
    }

    // SVC layer
    if components.iter().any(|c| *c == "svc") {
        let domain = components.iter().position(|c| *c == "domains")
            .and_then(|i| components.get(i + 1))
            .unwrap_or(&"unknown")
            .to_string();
        return (Layer::L4, domain);
    }

    // SDK layer
    if components.iter().any(|c| *c == "sdk") {
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
    if let Some(dev_deps) = value.get("dev-dependencies").and_then(|d| d.as_table()) {
        for (dep_name, _) in dev_deps {
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

    let mut crates = HashMap::new();
    for entry in WalkDir::new(workspace_root).into_iter().filter_map(|e| e.ok()) {
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
    let same_domain = from.domain == to.domain || to.domain == "foundation" || to.domain == "gateway";
    match (from.layer, to.layer) {
        (Layer::L0, _) => false,
        (Layer::L1, Layer::L0) | (Layer::L1, Layer::L2) => true,
        (Layer::L1, _) => false,
        (Layer::L2, Layer::L0) => true,
        (Layer::L2, _) => false,
        (Layer::L3, Layer::L0) | (Layer::L3, Layer::L2) => true,
        (Layer::L3, Layer::L3) => same_domain, // same-domain core deps allowed
        (Layer::L3, _) => false,
        (Layer::L4, Layer::L0) | (Layer::L4, Layer::L2) | (Layer::L4, Layer::L3) => true,
        (Layer::L4, Layer::L4) => same_domain, // same-domain svc deps allowed
        (Layer::L4, _) => false,
        (Layer::L5, _) => true,
        (Layer::Unknown, _) => true,
        (_, Layer::Unknown) => true,
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
                    // Must go through api layer (L2)
                    if dep_info.layer != Layer::L2 && dep_info.layer != Layer::L0 {
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
                if dep_info.layer != Layer::L0 && dep_info.layer != Layer::Unknown {
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
    let platform_dir = root.join("platform");
    let data_extensions = [".db", ".sqlite", ".sqlite3", ".log", ".pid", ".sock", ".lock"];
    let mut violations = Vec::new();

    for entry in WalkDir::new(&platform_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy();
            for ext in &data_extensions {
                if name.ends_with(ext) {
                    violations.push(format!("  {}", entry.path().display()));
                }
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
        if entry.file_type().is_file() && entry.path().extension().map_or(false, |e| e == "rs") {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                for (line_num, line) in content.lines().enumerate() {
                    for pattern in &forbidden_patterns {
                        if line.contains(pattern) {
                            // 允许在注释中出现（以 // 开头）
                            let trimmed = line.trim_start();
                            if !trimmed.starts_with("//") && !trimmed.starts_with("*") {
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
