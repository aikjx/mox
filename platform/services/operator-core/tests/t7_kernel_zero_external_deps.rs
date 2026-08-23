//! T7 验收：kernel 零外部依赖 + cargo test 回归通过。
//!
//! - tr_07_01_kernel_file_no_external_use：对 `src/kernel.rs` 做字符串级检查：
//!   按正则意图 `^\s*use\s+([a-zA-Z_][a-zA-Z0-9_]*)` 提取前缀，
//!   禁止命中 `serde` / `nalgebra` / `anyhow` / `petgraph` / `chrono` / `tokio` / `thiserror`，
//!   前缀为 `std` 时放行。
//! - tr_07_02_cargo_test_operator_core：跑子进程 `cargo test -p operator-core`，
//!   靠 `T7_REENTRY_GUARD=1` 环境变量防无限递归。

use std::fs;
use std::path::Path;
use std::process::Command;

const FORBIDDEN_PREFIXES: &[&str] = &[
    "serde",
    "nalgebra",
    "anyhow",
    "petgraph",
    "chrono",
    "tokio",
    "thiserror",
];

const GUARD_ENV: &str = "T7_REENTRY_GUARD";

/// 手动实现前缀解析，语义等价正则 `^\s*use\s+([a-zA-Z_][a-zA-Z0-9_]*)`。
fn extract_use_prefix(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0;
    // 跳过前导空白
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' {
            i += 1;
        } else {
            break;
        }
    }
    // 接下来必须是 "use" + 空白
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
    // 跳过分隔空白
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' {
            i += 1;
        } else {
            break;
        }
    }
    // 提取首段标识符 [a-zA-Z_][a-zA-Z0-9_]*
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

fn assert_kernel_no_external_use(path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "tr_07_01: 无法读取 kernel.rs 文件 {}: {}",
            path.display(),
            e
        )
    });

    let mut line_no = 0usize;
    for raw_line in content.lines() {
        line_no += 1;
        if let Some(prefix) = extract_use_prefix(raw_line) {
            if prefix == "std" {
                continue;
            }
            if FORBIDDEN_PREFIXES.iter().any(|f| *f == prefix) {
                panic!(
                    "tr_07_01 FAIL: {}:{} 发现禁用外部前缀 `use {}`;\n行内容: `{}`",
                    path.display(),
                    line_no,
                    prefix,
                    raw_line.trim()
                );
            }
            // crate / super / self / 本项目模块 → 放行
        }
    }
}

#[test]
fn tr_07_01_kernel_file_no_external_use() {
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

// ----- 辅助：对 extract_use_prefix 的简单自检 -----

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
    fn extract_nalgebra_indented() {
        assert_eq!(
            extract_use_prefix("\t\tuse nalgebra::DVector;"),
            Some("nalgebra".to_string())
        );
    }

    #[test]
    fn skip_commented_use() {
        // 我们不解析注释，命中也没关系（kernel.rs 里不会有带 use 前缀的注释行
        // 含禁用 crate 名；另外本规则是 AST 字符串级的宽松检查）。
        // 此处仅校验行首没有 use 关键词时返回 None：
        assert_eq!(extract_use_prefix("foo use bar;"), None);
        assert_eq!(extract_use_prefix("#[cfg(test)] use std::x;"), None); // 行首是 #[...]
    }

    #[test]
    fn extract_crate_prefix() {
        assert_eq!(
            extract_use_prefix("use crate::kernel::TypeIdentifier;"),
            Some("crate".to_string())
        );
    }
}
