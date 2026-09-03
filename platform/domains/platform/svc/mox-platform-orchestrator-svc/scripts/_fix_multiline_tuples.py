#!/usr/bin/env python3
"""
修复迁移后残留的多行 (StatusCode, api_ok/api_error) 元组模式。
将:
    (
        StatusCode::OK,
        api_ok(...),
    )
转换为:
    api_ok(...)

将:
    return (
        StatusCode::BAD_REQUEST,
        api_error(500, msg),
    );
转换为:
    return api_error(400, msg);
"""

import re
import sys
from pathlib import Path

ORCH_ROOT = Path(__file__).resolve().parent.parent

TARGET_FILES = [
    "src/main.rs",
    "src/handlers/ai_engine.rs",
    "src/handlers/governance.rs",
    "src/handlers/hitl.rs",
    "src/handlers/agent.rs",
    "src/automation.rs",
    "src/market.rs",
    "src/market_dsl.rs",
    "src/market_version.rs",
    "src/routes/ai_engine.rs",
    "src/routes/market.rs",
]

STATUS_MAP = {
    'OK': '0',
    'BAD_REQUEST': '400',
    'UNAUTHORIZED': '401',
    'FORBIDDEN': '403',
    'NOT_FOUND': '404',
    'CONFLICT': '409',
    'UNPROCESSABLE_ENTITY': '422',
    'INTERNAL_SERVER_ERROR': '500',
    'BAD_GATEWAY': '502',
    'SERVICE_UNAVAILABLE': '503',
}


def read_file(path):
    with open(path, "r", encoding="utf-8-sig", newline="") as f:
        return f.read()


def write_file(path, content):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)


def fix_multiline_tuples(content):
    """修复多行 (StatusCode, api_ok/api_error) 元组"""
    fixed = 0

    # 模式1: 多行成功元组
    # (
    #     StatusCode::OK,
    #     api_ok(...),
    # )
    # 或带 return:
    # return (
    #     StatusCode::OK,
    #     api_ok(...),
    # );
    pattern_ok = re.compile(
        r'(return\s+)?\(\s*\n\s*StatusCode::OK\s*,\s*\n\s*(api_ok\(.+?\))\s*,?\s*\n\s*\)\s*;?',
        re.DOTALL
    )

    def repl_ok(m):
        nonlocal fixed
        fixed += 1
        ret = m.group(1) or ''
        expr = m.group(2)
        # 去掉末尾可能的逗号
        expr = expr.rstrip(',')
        if ret:
            return f"{ret}{expr};"
        return expr

    content = pattern_ok.sub(repl_ok, content)

    # 模式2: 多行错误元组
    # (
    #     StatusCode::XXX,
    #     api_error(500, msg),
    # )
    # 或带 return:
    # return (
    #     StatusCode::XXX,
    #     api_error(500, msg),
    # );
    for status_variant, code in STATUS_MAP.items():
        if status_variant == 'OK':
            continue
        pattern_err = re.compile(
            rf'(return\s+)?\(\s*\n\s*StatusCode::{status_variant}\s*,\s*\n\s*api_error\(\d+\s*,\s*(.+?)\)\s*,?\s*\n\s*\)\s*;?',
            re.DOTALL
        )

        def make_repl(c):
            def repl(m):
                nonlocal fixed
                fixed += 1
                ret = m.group(1) or ''
                msg = m.group(2).rstrip(',')
                if ret:
                    return f"{ret}api_error({c}, {msg});"
                return f"api_error({c}, {msg})"
            return repl

        content = pattern_err.sub(make_repl(code), content)

    # 模式3: 单行残留元组 (StatusCode, api_ok/api_error)
    # (StatusCode::OK, api_ok(...))
    content, n = re.subn(
        r'\(\s*StatusCode::OK\s*,\s*(api_ok\(.+?\))\s*\)',
        r'\1',
        content
    )
    fixed += n

    for status_variant, code in STATUS_MAP.items():
        if status_variant == 'OK':
            continue
        content, n = re.subn(
            rf'\(\s*StatusCode::{status_variant}\s*,\s*api_error\(\d+\s*,\s*(.+?)\)\s*\)',
            rf'api_error({code}, \1)',
            content
        )
        fixed += n

    return content, fixed


def fix_remaining_json_in_tuples(content):
    """修复仍然包含 Json( 的 (StatusCode, Json(...)) 元组"""
    fixed = 0

    # 多行 (StatusCode::OK, Json(...))
    pattern = re.compile(
        r'(return\s+)?\(\s*\n\s*StatusCode::OK\s*,\s*\n\s*Json\((.+?)\)\s*,?\s*\n\s*\)\s*;?',
        re.DOTALL
    )

    def repl(m):
        nonlocal fixed
        fixed += 1
        ret = m.group(1) or ''
        expr = m.group(2).rstrip(',')
        if ret:
            return f"{ret}api_ok({expr});"
        return f"api_ok({expr})"

    content = pattern.sub(repl, content)

    return content, fixed


def main():
    total_fixed = 0
    for rel_path in TARGET_FILES:
        filepath = ORCH_ROOT / rel_path
        if not filepath.exists():
            continue
        content = read_file(filepath)
        original = content

        content, n1 = fix_multiline_tuples(content)
        content, n2 = fix_remaining_json_in_tuples(content)

        if content != original:
            write_file(filepath, content)
            print(f"  {rel_path}: 修复 {n1 + n2} 处")
            total_fixed += n1 + n2

    print(f"\n总计修复: {total_fixed} 处")


if __name__ == "__main__":
    main()
