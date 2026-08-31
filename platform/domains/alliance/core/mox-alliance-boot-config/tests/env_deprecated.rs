//! 环境变量归一化测试（独立进程 + 单函数串行，避免并行共享 env 竞态）
//! 验证旧 `EXECUTOR_MODE` 兼容（deprecated）且新 `MOX_ALLIANCE_EXECUTOR_MODE` 优先。
//! 通过 `load_executor`（不存在的路径 → 内置默认 + env 覆盖）走完整加载链路。
use mox_alliance_boot_config::load_executor;

const NO_FILE: &str = "config/__bootcfg_nonexistent__.yml";

fn clean_env() {
    std::env::remove_var("EXECUTOR_MODE");
    std::env::remove_var("MOX_ALLIANCE_EXECUTOR_MODE");
}

#[test]
fn env_normalization_rules() {
    // 1) 新变量优先于旧变量
    clean_env();
    std::env::set_var("EXECUTOR_MODE", "mock");
    std::env::set_var("MOX_ALLIANCE_EXECUTOR_MODE", "expert");
    let ec = load_executor(NO_FILE).unwrap();
    assert_eq!(ec.executor.mode, "expert", "新 MOX_ALLIANCE_EXECUTOR_MODE 应优先于旧 EXECUTOR_MODE");
    clean_env();

    // 2) 仅设旧变量 → 兼容生效（deprecated）
    std::env::set_var("EXECUTOR_MODE", "mock");
    let ec = load_executor(NO_FILE).unwrap();
    assert_eq!(ec.executor.mode, "mock", "仅设旧 EXECUTOR_MODE 时应兼容生效");
    clean_env();

    // 3) 均未设置 → 保持内置默认 expert
    let ec = load_executor(NO_FILE).unwrap();
    assert_eq!(ec.executor.mode, "expert", "未设置环境变量时保持默认 expert");
}
