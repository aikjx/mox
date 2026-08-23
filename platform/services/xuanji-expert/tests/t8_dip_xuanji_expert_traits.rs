//! T8 · DIP 反转验证（xuanji-expert 对外抽象 trait + 下游只依赖 trait）
//!
//! 四个测试：
//! - tr_08_01_hermes_only_use_traits：静态扫描 hermes-flow-bridge/src 所有 .rs，
//!   凡 `use xuanji_expert::X` 形式只允许 X ∈ {expert_traits, types}。
//! - tr_08_02_catalog_only_use_traits：同上对 business-catalog/src。
//! - tr_08_03_mock_consultant：最小 MockExpert impl ExpertConsultant trait，
//!   脱离 xuanji-expert concrete 引擎运行，给出 DIP 证据。
//! - tr_08_04_build_and_unit_test：调用 cargo 在三个 crate 上跑 `--lib` 单元测试，
//!   必须全部 exit 0，证明 DIP 改造不破坏既有行为。

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use xuanji_expert::expert_traits::ExpertConsultant;
use xuanji_expert::types::{ConsultQuery, ConsultReport};

// ============================================================================
// 工具：递归枚举 .rs 文件（纯 std，不依赖 walkdir）
// ============================================================================

fn list_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    walk(dir, &mut out);
    out.sort();
    out
}

/// 以 `CARGO_MANIFEST_DIR` 为锚，定位工作区根目录（platform/services/xuanji-expert → ../../..）。
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest = .../infotopograph/platform/services/xuanji-expert
    manifest
        .parent().unwrap()   // services
        .parent().unwrap()   // platform
        .parent().unwrap()   // infotopograph (workspace root)
        .to_path_buf()
}

/// 检查一个 Rust 源文件中的所有 `use xuanji_expert::X`，仅允许 X ∈ {expert_traits, types}。
/// 返回违规列表（(path, 违规use片段)）。
fn scan_xuanji_use_violations(file: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => return vec![format!("无法读取文件 {}: {}", file.display(), e)],
    };
    let mut violations = Vec::new();
    // 逐行扫描（我们的 use 语句都是单行）
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // 允许的前缀： use / pub(crate) use / pub use
        if !(trimmed.starts_with("use ") || trimmed.starts_with("pub use ") || trimmed.starts_with("pub(crate) use ")) {
            continue;
        }
        // 寻找 "xuanji_expert::" 在 use 行内的出现
        // use 形式： use xuanji_expert::expert_traits::X; / use xuanji_expert::types::{A, B};
        // 不合法：use xuanji_expert::pipeline::xuanji_optimize; / use xuanji_expert::context::GovernContext;
        let rest_after_keyword = match trimmed.find("xuanji_expert::") {
            Some(p) => &trimmed[p..],
            None => continue,
        };
        // 取 "xuanji_expert::" 之后的首个 identifier（到下一个 :: 或 { 或 空格 或 ;）
        let after = &rest_after_keyword["xuanji_expert::".len()..];
        let end = after
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let mod_name = &after[..end];
        if mod_name != "expert_traits" && mod_name != "types" {
            violations.push(format!(
                "{}:{} 非白名单模块 use xuanji_expert::{} (期望 expert_traits 或 types) —— 行内容：{}",
                file.display(),
                idx + 1,
                mod_name,
                line.trim()
            ));
        }
    }
    violations
}

// ============================================================================
// tr_08_01 / tr_08_02：静态扫描
// ============================================================================

#[test]
fn tr_08_01_hermes_only_use_traits() {
    let ws = workspace_root();
    let dir = ws.join("platform/services/hermes-flow-bridge/src");
    assert!(dir.is_dir(), "找不到 hermes-flow-bridge src: {}", dir.display());
    let files = list_rs_files(&dir);
    assert!(!files.is_empty(), "hermes src 目录至少应有一个 .rs 文件");

    let mut all_violations: Vec<String> = Vec::new();
    for f in &files {
        all_violations.extend(scan_xuanji_use_violations(f));
    }
    assert!(
        all_violations.is_empty(),
        "hermes-flow-bridge 存在 DIP 违规 use 语句（禁止直接引入 xuanji_expert concrete struct/模块）：\n{}",
        all_violations.join("\n")
    );
}

#[test]
fn tr_08_02_catalog_only_use_traits() {
    let ws = workspace_root();
    let dir = ws.join("platform/services/business-catalog/src");
    assert!(dir.is_dir(), "找不到 business-catalog src: {}", dir.display());
    let files = list_rs_files(&dir);
    assert!(!files.is_empty(), "catalog src 目录至少应有一个 .rs 文件");

    let mut all_violations: Vec<String> = Vec::new();
    for f in &files {
        all_violations.extend(scan_xuanji_use_violations(f));
    }
    assert!(
        all_violations.is_empty(),
        "business-catalog 存在 DIP 违规 use 语句（禁止直接引入 xuanji_expert concrete struct/模块）：\n{}",
        all_violations.join("\n")
    );
}

// ============================================================================
// tr_08_03：MockExpert 证明 DIP（trait 可脱离 concrete 运行）
// ============================================================================

/// 最小 Mock 实现 ExpertConsultant，不依赖 xuanji-expert 任何 concrete 引擎结构，
/// 仅基于 trait 默认实现提供 consult_blocking 同步路径（用于测试/演示）。
struct MockExpertEmpty;

#[async_trait]
impl ExpertConsultant for MockExpertEmpty {
    async fn consult(&self, _q: &ConsultQuery) -> xuanji_expert::types::Result<ConsultReport> {
        unreachable!("sync-only mock，不应进入 async consult 路径")
    }

    fn consult_blocking(&self, q: &ConsultQuery) -> xuanji_expert::types::Result<ConsultReport> {
        Ok(ConsultReport {
            report_id: q.id.clone(),
            steps: vec!["[MockExpertEmpty] 空咨询报告（DIP 证据：无璇玑 concrete）".into()],
            score: 1.0,
            vetoed: false,
            reason: None,
        })
    }
}

#[test]
fn tr_08_03_mock_consultant() {
    // 构造：把 Mock 挂到 Arc<dyn ExpertConsultant>
    let consultant: Arc<dyn ExpertConsultant> = Arc::new(MockExpertEmpty);
    let query = ConsultQuery {
        id: "mock-q".into(),
        query: "Hello, 这是一个不涉及 concrete 引擎的测试".into(),
        ctx: HashMap::new(),
    };
    // 用 trait object 调用（consult_blocking 是 sync 默认实现，不触发 tokio）
    let rep = consultant.consult_blocking(&query).expect("mock consult 必成功");
    // 断言返回"空报告"特征，证明运行未走到 xuanji-expert concrete 引擎
    assert_eq!(rep.report_id, "mock-q");
    assert!((rep.score - 1.0).abs() < 1e-9);
    assert!(!rep.vetoed);
    assert_eq!(rep.steps.len(), 1);
    assert!(rep.steps[0].contains("DIP 证据"));
    assert!(rep.reason.is_none());

    // 同时构造通过 Arc 传参给下游的典型模式（供下游容器注入）
    fn downstream_api(consultant: Arc<dyn ExpertConsultant>) -> ConsultReport {
        let q = ConsultQuery { id: "d".into(), query: String::new(), ctx: HashMap::new() };
        consultant.consult_blocking(&q).unwrap()
    }
    let r2 = downstream_api(consultant.clone());
    assert_eq!(r2.report_id, "d");
    assert!(!r2.vetoed);
}

// ============================================================================
// tr_08_04：cargo test 全量单元测试（三个 crate --lib）
// ============================================================================

#[test]
fn tr_08_04_build_and_unit_test() {
    let ws = workspace_root();
    // 为了让 CI/本地 环境都一致，显式在工作区根目录执行 cargo。
    // 同时显式 CARGO_TARGET_DIR 避免与已有 target 冲突（可选，让 cargo 自动处理即可）。
    // 本测试若本地已经开着 cargo check 会锁，因此设置环境 CARGO_NET_OFFLINE 等都不强制。
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&ws)
        .arg("test")
        .arg("-p")
        .arg("xuanji-expert")
        .arg("-p")
        .arg("hermes-flow-bridge")
        .arg("-p")
        .arg("business-catalog")
        .arg("--lib")
        // 降低噪声：只显示一条 pass/fail；也可去掉以便调试
        .arg("--quiet");

    // 允许较长执行时间：cargo 可能首次需要构建
    let status = cmd
        .status()
        .expect("无法启动 cargo 子进程（cargo 必须在 PATH 中）");

    assert!(
        status.success(),
        "DIP 改造破坏了现有单元测试：cargo test -p xuanji-expert -p hermes-flow-bridge -p business-catalog --lib 返回 {:?}\n\
         工作目录：{}\n\
         请用此命令手动重跑以查看失败输出。",
        status.code(),
        ws.display()
    );
}
