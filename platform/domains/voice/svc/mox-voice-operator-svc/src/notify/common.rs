// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 共享工具函数：字符串转义

pub(crate) fn escape_ps(s: &str) -> String {
    // PowerShell single-quote escape: ' → ''
    s.replace('\'', "''")
}

pub(crate) fn escape_osa(s: &str) -> String {
    // AppleScript: \ → \\, " → \"
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
