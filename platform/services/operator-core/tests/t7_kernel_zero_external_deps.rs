//! T7 验收：kernel 零外部依赖 + cargo test 回归通过。
//!
//! 【AC-04 规范】L6 kernel.rs 仅允许 use std/alloc/core；以下 7 个 crate 必须为 0 命中：
//!   `serde` / `nalgebra` / `ndarray` / `thiserror` / `anyhow` / `tracing` / `uuid`
//!
//! - tr_07_01_a_kernel_file_no_external_use：对 `src/kernel.rs` 做字符串级 use 前缀检查
//!   （语义等价正则 `^\s*use\s+([a-zA-Z_][a-zA-Z0-9_]*)`，仅放行 std/alloc/core/crate/super/self）
//! - tr_07_01_b_kernel_no_forbidden_derive_attrs：检查 `#[derive(Serialize, ...)]` /
//!   `#[derive(thiserror::Error)]` / `#[serde(...)]` / `#[tracing::...]` 等属性宏对
//!   7 crate 的引用；即使未显式 `use`，derive 宏也会把外部 crate 拉进内核（违反零依赖原则）
//! - tr_07_01_c_kernel_no_cfg_attr_extern：检查 `#[cfg_attr(feature = "...", derive(...))]`
//!   中是否出现 7 个禁用 crate 的 derive 名（如 Serialize/Deserialize/Error）
//! - tr_07_01_d_kernel_only_allow_std_alloc_core_crate：确认所有 use 前缀仅 ∈
//!   {std, alloc, core, crate, super, self}，任何外部 crate 前缀都算违规
//! - tr_07_01_e_kernel_ext_file_allows_external（正对照）：kernel_ext.rs 命中至少 2 个
//!   外部前缀（serde + nalgebra），证明 grep 逻辑不松
//! - tr_07_03 ～ tr_07_07：对 7 个 crate 单 crate 精确 grep 断言各为 0 匹配

use std::fs;
use std::path::Path;
use std::process::Command;

/// AC-04 指定的 7 个禁止出现在 kernel.rs 的 crate
const FORBIDDEN_PREFIXES: &[&str] = &[
    "serde",
    "nalgebra",
    "ndarray",
    "thiserror",
    "anyhow",
    "tracing",
    "uuid",
];

/// derive 属性中常见的来自 7 个禁用 crate 的 derive 标记名（不含 Debug/Clone 等 std 内置）
const FORBIDDEN_DERIVE_NAMES: &[&str] = &[
    "Serialize",
    "Deserialize",
    "Error",          // thiserror::Error
];

/// 属性宏路径（出现这些前缀=显式调用了禁用 crate 的宏）
const FORBIDDEN_ATTR_PATHS: &[&str] = &[
    "serde::",
    "thiserror::",
    "tracing::",
    "uuid::",
    "nalgebra::",
    "ndarray::",
    "anyhow::",
];

const GUARD_ENV: &str = "T7_REENTRY_GUARD";

/// 手动实现前缀解析，语义等价正则 `^\s*use\s+([a-zA-Z_][a-zA-Z0-9_]*)`。
fn extract_use_prefix(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' {
            i += 1;
        } else {
            break;
        }
    }
    if i + 3 >= bytes.len() {
        return None;
    }
    if &bytes[i..i + 3] != b"use" {
        return None;
    }
    i += 3;
    if i >= bytes.len() {
        return None;
    }
    let sep = bytes[i];
    if !(sep == b' ' || sep == b'\t' || sep == b';') {
        return None;
    }
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' {
            i += 1;
        } else {
            break;
        }
    }
    if i >= bytes.len() {
        return None;
    }
    let start = bytes[i];
    if !(start.is_ascii_alphabetic() || start == b'_') {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len() {
        let b = bytes[j];
        if b.is_ascii_alphanumeric() || b == b'_' {
            j += 1;
        } else {
            break;
        }
    }
    let prefix = &line[i..j];
    Some(prefix.to_string())
}

/// 从 `#[derive(A, B, C)]` / `#[derive(A, B(c), C)]` 行中提取 derive 的标识符列表。
fn extract_derive_names(line: &str) -> Vec<String> {
    const NEEDLE: &[u8] = b"#[derive(";
    const NLEN: usize = 9; // "#[derive(" 的字节长度
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + NLEN <= bytes.len() {
        if &bytes[i..i + NLEN] == NEEDLE {
            i += NLEN;
            let mut j = i;
            let mut depth = 1usize;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            // derive_content: 从 derive( 内部开始，到最后一个闭合 ')' 之前（不含）
            let end_exclusive = j.saturating_sub(1);
            let derive_content = if end_exclusive > i {
                &line[i..end_exclusive]
            } else {
                ""
            };
            for raw in derive_content.split(',') {
                let trimmed = raw.trim();
                let cleaned: String = trimmed
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                    .collect();
                let last_segment = cleaned
                    .rsplit("::")
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !last_segment.is_empty() {
                    out.push(last_segment);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn assert_kernel_no_external_use(path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "tr_07: 无法读取 kernel.rs 文件 {}: {}",
            path.display(),
            e
        )
    });

    let mut line_no = 0usize;
    for raw_line in content.lines() {
        line_no += 1;
        if let Some(prefix) = extract_use_prefix(raw_line) {
            let allowed = ["std", "alloc", "core", "crate", "super", "self"];
            if allowed.iter().any(|a| *a == prefix) {
                continue;
            }
            if FORBIDDEN_PREFIXES.iter().any(|f| *f == prefix) {
                panic!(
                    "tr_07_01 FAIL: {}:{} 发现禁用外部 crate `use {}`（AC-04 7 crate 禁止）;\n行内容: `{}`",
                    path.display(),
                    line_no,
                    prefix,
                    raw_line.trim()
                );
            }
            // 未知前缀（既非 std 族也非 crate/super/self 也非 7 禁用 → 仍视为违规，保证零外部依赖）
            panic!(
                "tr_07_01 FAIL: {}:{} 非白名单 use 前缀 `{}`（仅允许 std/alloc/core/crate/super/self）;\n行内容: `{}`",
                path.display(),
                line_no,
                prefix,
                raw_line.trim()
            );
        }
    }
}

fn assert_no_forbidden_derives(path: &Path) {
    let content = fs::read_to_string(path).unwrap();
    let mut line_no = 0usize;
    for raw_line in content.lines() {
        line_no += 1;
        let t = raw_line.trim_start();
        // 1) 检查显式属性宏路径 `#[serde(...)]` / `#[thiserror::...]`
        for ap in FORBIDDEN_ATTR_PATHS.iter() {
            if t.starts_with("#[") && t.contains(ap) {
                panic!(
                    "tr_07_01_b FAIL: {}:{} 属性宏使用禁用 crate `{}`;\n行内容: `{}`",
                    path.display(),
                    line_no,
                    ap,
                    raw_line.trim()
                );
            }
        }
        // 2) 检查 derive(...) 中是否出现 Serialize/Deserialize/Error（thiserror/serde 独有）
        let derives = extract_derive_names(raw_line);
        for d in derives.iter() {
            if FORBIDDEN_DERIVE_NAMES.iter().any(|f| f == d) {
                panic!(
                    "tr_07_01_b FAIL: {}:{} #[derive(...)] 包含禁用 crate 标记 `{}`（应改为 kernel_ext.rs SerdeWrapper newtype）;\n行内容: `{}`",
                    path.display(),
                    line_no,
                    d,
                    raw_line.trim()
                );
            }
        }
    }
}

// ============================================================================
// TR-07-01 系列：AC-04 零外部依赖 5 个维度
// ============================================================================

#[test]
fn tr_07_01_a_kernel_file_no_external_use() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("src").join("kernel.rs");
    assert!(
        path.exists(),
        "tr_07_01: 找不到 kernel.rs，期望路径: {}",
        path.display()
    );
    assert_kernel_no_external_use(&path);
}

#[test]
fn tr_07_01_b_kernel_no_forbidden_derive_attrs() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("src").join("kernel.rs");
    assert_no_forbidden_derives(&path);
}

#[test]
fn tr_07_01_c_kernel_no_cfg_attr_conditional_derive() {
    // 查找形如 #[cfg_attr(..., derive(Serialize))] 的条件属性
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("src").join("kernel.rs");
    let content = fs::read_to_string(&path).unwrap();
    let mut line_no = 0usize;
    for raw_line in content.lines() {
        line_no += 1;
        let t = raw_line.trim_start();
        if t.starts_with("#[cfg_attr") && t.contains("derive(") {
            // 从 cfg_attr 中提取 derive(...) 段
            if let Some(start) = t.find("derive(") {
                let rest = &t[start..];
                let sub: String = rest
                    .chars()
                    .take_while(|c| !matches!(c, ']'))
                    .collect();
                // 7 个禁用 derive 名关键字
                for bad in FORBIDDEN_DERIVE_NAMES.iter() {
                    if sub.contains(bad) {
                        panic!(
                            "tr_07_01_c FAIL: {}:{} cfg_attr 条件 derive 仍包含 `{}`（外部 crate）;\n行内容: `{}`",
                            path.display(),
                            line_no,
                            bad,
                            raw_line.trim()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn tr_07_01_d_kernel_prefixes_only_std_family() {
    // 扫描所有 use 前缀，仅允许 {std,alloc,core,crate,super,self}（集合语义）
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("src").join("kernel.rs");
    let content = fs::read_to_string(&path).unwrap();
    let allowed: std::collections::HashSet<&str> =
        ["std", "alloc", "core", "crate", "super", "self"].iter().copied().collect();
    let mut line_no = 0usize;
    for raw in content.lines() {
        line_no += 1;
        if let Some(p) = extract_use_prefix(raw) {
            assert!(
                allowed.contains(p.as_str()),
                "tr_07_01_d FAIL: {}:{} use 前缀 `{}` 不在白名单 std/alloc/core/crate/super/self；原始行: `{}`",
                path.display(),
                line_no,
                p,
                raw.trim()
            );
        }
    }
}

#[test]
fn tr_07_01_e_kernel_ext_contains_external_positive_control() {
    // 正对照：kernel_ext.rs 应含有 serde 与 nalgebra use（证明 grep 不产生假阴性）
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("src").join("kernel_ext.rs");
    let content = fs::read_to_string(&path).unwrap_or_default();
    let has_serde = content.lines().filter_map(extract_use_prefix).any(|p| p == "serde");
    let has_nalgebra = content.lines().filter_map(extract_use_prefix).any(|p| p == "nalgebra");
    assert!(
        has_serde && has_nalgebra,
        "tr_07_01_e 正对照失败：kernel_ext.rs 应至少含 use serde + use nalgebra（当前 serde={}, nalgebra={}）",
        has_serde, has_nalgebra
    );
}

// ============================================================================
// TR-07-03 ～ TR-07-09：对 7 个 crate 逐个精确 grep（AC-04 要求）
// ============================================================================

macro_rules! per_crate_test {
    ($fname:ident, $crate_name:expr, $tr:literal) => {
        #[test]
        fn $fname() {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let path = Path::new(manifest_dir).join("src").join("kernel.rs");
            let content = fs::read_to_string(&path).unwrap();
            let hits: Vec<_> = content
                .lines()
                .enumerate()
                .filter(|(_, ln)| {
                    // use 前缀命中 OR `#[crate::` 属性命中
                    let trimmed = ln.trim_start();
                    let prefix_hit = extract_use_prefix(ln)
                        .map(|p| p == $crate_name)
                        .unwrap_or(false);
                    let attr_hit = trimmed.starts_with("#[")
                        && trimmed.contains(&format!("{}::", $crate_name));
                    let derive_hit = {
                        // 本 crate 贡献的 FORBIDDEN_DERIVE_NAMES 是否出现
                        match $crate_name {
                            "serde" => extract_derive_names(ln)
                                .iter()
                                .any(|d| d == "Serialize" || d == "Deserialize"),
                            "thiserror" => extract_derive_names(ln)
                                .iter()
                                .any(|d| d == "Error"),
                            _ => false,
                        }
                    };
                    prefix_hit || attr_hit || derive_hit
                })
                .map(|(i, ln)| format!("  L{}: {}", i + 1, ln.trim()))
                .collect();
            assert!(
                hits.is_empty(),
                "TR-07 {} FAIL: kernel.rs 仍命中 `{}` crate（AC-04 禁止）。\n命中行：\n{}",
                $tr,
                $crate_name,
                hits.join("\n")
            );
        }
    };
}

per_crate_test!(tr_07_03_per_crate_serde, "serde", "serde");
per_crate_test!(tr_07_04_per_crate_nalgebra, "nalgebra", "nalgebra");
per_crate_test!(tr_07_05_per_crate_ndarray, "ndarray", "ndarray");
per_crate_test!(tr_07_06_per_crate_thiserror, "thiserror", "thiserror");
per_crate_test!(tr_07_07_per_crate_anyhow, "anyhow", "anyhow");
per_crate_test!(tr_07_08_per_crate_tracing, "tracing", "tracing");
per_crate_test!(tr_07_09_per_crate_uuid, "uuid", "uuid");

// ============================================================================
// 回归测试入口（防递归：显式 --ignored 才触发 cargo test -p operator-core）
// ============================================================================

#[test]
#[ignore = "需要显式调用：cargo test -p operator-core --test t7_kernel_zero_external_deps tr_07_02 -- --ignored；避免全量递归"]
fn tr_07_02_cargo_test_operator_core() {
    let in_reentry = std::env::var(GUARD_ENV).is_ok();

    if in_reentry {
        return;
    }

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .arg("test")
        .arg("-p")
        .arg("operator-core")
        .env(GUARD_ENV, "1")
        .status()
        .expect("tr_07_02: 启动 cargo test 失败");

    assert!(
        status.success(),
        "tr_07_02 FAIL: cargo test -p operator-core 非零退出: {:?}",
        status
    );
}

// ============================================================================
// Self-sanity：辅助函数自检
// ============================================================================

#[cfg(test)]
mod sanity {
    use super::*;

    #[test]
    fn extract_std() {
        assert_eq!(
            extract_use_prefix("    use std::any::TypeId;"),
            Some("std".to_string())
        );
    }

    #[test]
    fn extract_serde() {
        assert_eq!(
            extract_use_prefix("use serde::{Serialize, Deserialize};"),
            Some("serde".to_string())
        );
    }

    #[test]
    fn extract_ndarray_indented() {
        assert_eq!(
            extract_use_prefix("\t\tuse ndarray::Array2;"),
            Some("ndarray".to_string())
        );
    }

    #[test]
    fn skip_non_use_lines() {
        assert_eq!(extract_use_prefix("foo use bar;"), None);
        assert_eq!(extract_use_prefix("#[cfg(test)] use std::x;"), None);
    }

    #[test]
    fn extract_crate_prefix() {
        assert_eq!(
            extract_use_prefix("use crate::kernel::TypeIdentifier;"),
            Some("crate".to_string())
        );
    }

    #[test]
    fn derive_extracts_serialize_and_deserialize() {
        let names = extract_derive_names("#[derive(Debug, Clone, Serialize, Deserialize)]");
        assert!(names.iter().any(|n| n == "Serialize"), "names: {:?}", names);
        assert!(names.iter().any(|n| n == "Deserialize"), "names: {:?}", names);
        assert!(names.iter().any(|n| n == "Debug"));
        assert!(names.iter().any(|n| n == "Clone"));
    }

    #[test]
    fn derive_extracts_qualified_thiserror_error() {
        let names = extract_derive_names("#[derive(Debug, thiserror::Error)]");
        assert!(names.iter().any(|n| n == "Error"), "names: {:?}", names);
        assert!(names.iter().any(|n| n == "Debug"));
    }
}
