// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 等保三级 hash_chain 独立校验工具
//!
//! # 用法
//! ```bash
//! cargo run --example verify-hash-chain -- chain.json
//! cargo run --example verify-hash-chain -- chain.json --key <hex_root_key>
//! ```
//!
//! # Exit Code
//! - `0` ⇔ integrity=true
//! - `1` ⇔ integrity=false 或参数/IO 错误
//!
//! # stdout
//! 输出单行 JSON：`{"blocks":N,"integrity":true|false,"broken_at":null|u64,"last_ts_ms":u64|null}`

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut file_path: Option<String> = None;
    let mut key_hex: String = String::new();
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--key" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--key missing value");
                    return ExitCode::from(1);
                }
                key_hex = args[i].clone();
            }
            other if file_path.is_none() => file_path = Some(other.to_string()),
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::from(1);
            }
        }
        i += 1;
    }
    let Some(fp) = file_path else {
        eprintln!("usage: verify-hash-chain <chain.json> [--key hex]");
        return ExitCode::from(1);
    };
    let bytes = match std::fs::read(&fp) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read file fail: {e}");
            return ExitCode::from(1);
        }
    };
    let result = match mox_data_standards_core::dengbao_hash_chain::verify_json_file(&bytes, &key_hex) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("verify_json_file fail: {e}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string(&result) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("json serialize fail: {e}");
            return ExitCode::from(1);
        }
    }
    if result.integrity {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}
