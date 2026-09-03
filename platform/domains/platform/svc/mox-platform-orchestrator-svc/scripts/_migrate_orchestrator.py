#!/usr/bin/env python3
"""
mox-platform-orchestrator-svc HTTP handler 迁移脚本 v2
将旧的 Json<T> / (StatusCode, Json<T>) / Result<Json<T>, (StatusCode, String)>
返回模式迁移到统一的 ApiResponse<T> 协议。

使用正确的括号匹配处理嵌套括号（format!(), json!() 等）。

用法: python scripts/_migrate_orchestrator.py
"""

import re
import os
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

PROTOCOL_IMPORT = "use mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};"

stats = {"files": 0, "return_types": 0, "json_to_api_ok": 0, "tuple_to_api": 0,
         "result_to_api": 0, "imports_added": 0, "manual_fix_needed": []}


def read_file(path: Path) -> str:
    with open(path, "r", encoding="utf-8-sig", newline="") as f:
        return f.read()


def write_file(path: Path, content: str):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)


def find_matching_paren(text: str, open_pos: int) -> int:
    """找到与 open_pos 处的 '(' 匹配的 ')' 位置，处理嵌套括号和字符串。"""
    depth = 0
    i = open_pos
    in_string = False
    string_char = None
    while i < len(text):
        c = text[i]
        if in_string:
            if c == '\\':
                i += 2
                continue
            if c == string_char:
                in_string = False
        else:
            if c in ('"', "'"):
                in_string = True
                string_char = c
            elif c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
                if depth == 0:
                    return i
        i += 1
    return -1


def add_import(content: str) -> str:
    global stats
    if "mox_api_protocol" in content:
        return content

    lines = content.split("\n")
    last_use_idx = -1
    for i, line in enumerate(lines):
        stripped = line.strip()
        # 只识别顶层 use（列0，无缩进），排除函数体内的局部 use
        if stripped.startswith("use ") and ";" in stripped:
            if not line.startswith(" ") and not line.startswith("\t"):
                last_use_idx = i

    if last_use_idx >= 0:
        lines.insert(last_use_idx + 1, PROTOCOL_IMPORT)
        stats["imports_added"] += 1
        return "\n".join(lines)
    else:
        insert_idx = 0
        for i, line in enumerate(lines):
            stripped = line.strip()
            if stripped and not stripped.startswith("//") and not stripped.startswith("/*") and not stripped.startswith("*"):
                insert_idx = i
                break
        lines.insert(insert_idx, PROTOCOL_IMPORT)
        lines.insert(insert_idx + 1, "")
        stats["imports_added"] += 1
        return "\n".join(lines)


def replace_return_types(content: str) -> str:
    """替换函数返回类型（使用正则，类型中不会有复杂嵌套）"""
    global stats

    # -> ApiResult<Json<T>>  ->  -> ApiResponse<T>
    # (ApiResult<T> = Result<T, ApiError>, used in governance.rs/hitl.rs)
    content, n0 = re.subn(
        r'->\s*ApiResult<Json<([^>]+)>>',
        r'-> ApiResponse<\1>',
        content
    )

    # -> (StatusCode, Json<T>)  ->  -> ApiResponse<T>
    content, n1 = re.subn(
        r'->\s*\(\s*StatusCode\s*,\s*Json<([^>]+)>\s*\)',
        r'-> ApiResponse<\1>',
        content
    )

    # -> Result<Json<T>, (StatusCode, String)>  ->  -> ApiResponse<T>
    content, n2 = re.subn(
        r'->\s*Result<Json<([^>]+)>\s*,\s*\(\s*StatusCode\s*,\s*String\s*\)\s*>',
        r'-> ApiResponse<\1>',
        content
    )

    # -> Json<T>  ->  -> ApiResponse<T>
    content, n3 = re.subn(
        r'->\s*Json<([^>]+)>',
        r'-> ApiResponse<\1>',
        content
    )

    stats["return_types"] += n0 + n1 + n2 + n3
    return content


def extract_error_message(json_expr: str) -> str | None:
    """从 json!({...}) 或 serde_json::json!({...}) 表达式中提取 "error" 字段的值。"""
    # 匹配 "error": value （value 可能是字符串、format!() 调用等）
    m = re.search(r'"error"\s*:\s*', json_expr)
    if not m:
        return None
    start = m.end()
    # 提取值：直到遇到逗号（在顶层括号深度0）或闭合括号
    depth = 0
    i = start
    in_string = False
    string_char = None
    while i < len(json_expr):
        c = json_expr[i]
        if in_string:
            if c == '\\':
                i += 2
                continue
            if c == string_char:
                in_string = False
        else:
            if c in ('"', "'"):
                in_string = True
                string_char = c
            elif c in '([{':
                depth += 1
            elif c in ')]}':
                if depth == 0:
                    break
                depth -= 1
            elif c == ',' and depth == 0:
                break
        i += 1
    return json_expr[start:i].strip()


def is_error_json(expr: str) -> bool:
    """判断 Json(expr) 中的 expr 是否是错误响应（包含 "error" 或 "success": false）。"""
    return '"error"' in expr or '"success": false' in expr or '"success":false' in expr


def transform_json_calls(content: str) -> str:
    """
    转换函数体中的 Json(...) 调用。
    使用正确的括号匹配提取完整表达式。
    只转换处于返回位置的 Json(...) 调用。
    """
    global stats
    result = []
    i = 0
    n = len(content)

    while i < n:
        # 查找 "Json(" 
        if content[i:i+5] == "Json(":
            # 检查前面的上下文，判断是否是返回位置
            # 获取前面的非空白字符
            j = i - 1
            while j >= 0 and content[j] in ' \t':
                j -= 1
            prev_char = content[j] if j >= 0 else ''
            prev_chars = content[max(0,j-10):j+1]

            # 排除函数参数中的 Json(req): 模式（提取器）
            # 提取器模式: Json(ident): Json<Type>
            # 检查后面是否有 ": Json<"
            after_close = find_matching_paren(content, i + 4)
            if after_close > 0:
                rest = content[after_close+1:after_close+10].strip()
                if rest.startswith(':'):
                    # 这是提取器模式 Json(req): Json<Type>，不转换
                    result.append(content[i:after_close+1])
                    i = after_close + 1
                    continue

            # 判断是否是返回位置:
            # 1. 前面是 "return " 
            # 2. 前面是 "=> " (match arm)
            # 3. 行首（函数体最后一行或独立语句）
            # 4. 前面是 "," (元组中的第二个元素)
            is_return_pos = False
            is_error = False

            # 检查前面的文本
            before_text = content[max(0,i-20):i]

            if re.search(r'return\s+$', before_text):
                is_return_pos = True
            elif re.search(r'=>\s*$', before_text):
                is_return_pos = True
            elif prev_char in '\n' or (prev_char == '' and i == 0):
                # 行首，可能是返回值
                is_return_pos = True
            elif prev_char == ',':
                # 元组中的元素，检查是否是 (StatusCode, Json(...)) 模式
                # 向前找 StatusCode
                tuple_before = content[max(0,i-60):i]
                if re.search(r'StatusCode::\w+\s*,\s*$', tuple_before):
                    is_return_pos = True
                    is_error = True  # 假设非 OK 的状态码都是错误

            if not is_return_pos:
                result.append(content[i])
                i += 1
                continue

            # 提取完整的 Json(expr)
            open_paren = i + 4  # "Json(" 中 '(' 的位置
            close_paren = find_matching_paren(content, open_paren)
            if close_paren < 0:
                result.append(content[i])
                i += 1
                continue

            inner_expr = content[open_paren+1:close_paren].strip()

            # 判断是否是错误响应
            if not is_error:
                is_error = is_error_json(inner_expr)

            if is_error:
                # 尝试提取 error 消息
                msg = extract_error_message(inner_expr)
                if msg:
                    result.append(f"api_error(500, {msg})")
                    stats["tuple_to_api"] += 1
                else:
                    # 无法提取，保留为 api_ok（需要手动检查）
                    result.append(f"api_ok({inner_expr})")
                    stats["json_to_api_ok"] += 1
                    stats["manual_fix_needed"].append(f"可能的错误返回未正确转换: {inner_expr[:80]}")
            else:
                result.append(f"api_ok({inner_expr})")
                stats["json_to_api_ok"] += 1

            i = close_paren + 1
        else:
            result.append(content[i])
            i += 1

    return "".join(result)


def transform_statuscode_tuples(content: str) -> str:
    """
    转换 (StatusCode::XXX, Json(...)) 元组为 api_ok / api_error。
    使用正确的括号匹配。
    """
    global stats
    result = []
    i = 0
    n = len(content)

    # 状态码到数字的映射
    status_map = {
        'OK': '0',  # 成功
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

    while i < n:
        # 查找 "(StatusCode::"
        if content[i:i+1+12] == "(StatusCode::" or \
           (content[i] == '(' and content[i+1:i+13] == 'StatusCode::'):
            # 找到 StatusCode::XXX
            m = re.match(r'\(\s*StatusCode::(\w+)\s*,\s*', content[i:])
            if m:
                status_variant = m.group(1)
                code = status_map.get(status_variant, '500')
                comma_end = i + m.end()

                # 查找 Json(
                j = comma_end
                while j < n and content[j] in ' \t\n':
                    j += 1

                if content[j:j+5] == "Json(":
                    open_paren = j + 4
                    close_paren = find_matching_paren(content, open_paren)
                    if close_paren > 0:
                        inner_expr = content[open_paren+1:close_paren].strip()
                        # 跳过闭合括号后的空白和逗号
                        k = close_paren + 1
                        while k < n and content[k] in ' \t\n,':
                            k += 1
                        # 检查是否有外层 ')'
                        if k < n and content[k] == ')':
                            k += 1  # 跳过外层 ')'

                        if status_variant == 'OK':
                            result.append(f"api_ok({inner_expr})")
                            stats["tuple_to_api"] += 1
                        else:
                            # 错误响应，尝试提取 error 消息
                            msg = extract_error_message(inner_expr)
                            if msg:
                                result.append(f"api_error({code}, {msg})")
                            else:
                                result.append(f"api_error({code}, {inner_expr})")
                            stats["tuple_to_api"] += 1

                        i = k
                        continue

        result.append(content[i])
        i += 1

    return "".join(result)


def transform_result_patterns(content: str) -> str:
    """转换 Ok(Json(...)) 和 Err((StatusCode::XXX, msg)) 模式。"""
    global stats
    result = []
    i = 0
    n = len(content)

    status_map = {
        'BAD_REQUEST': '400', 'UNAUTHORIZED': '401', 'FORBIDDEN': '403',
        'NOT_FOUND': '404', 'CONFLICT': '409', 'INTERNAL_SERVER_ERROR': '500',
    }

    while i < n:
        # Ok(Json(...))
        if content[i:i+3] == "Ok(" and i+3 < n and content[i+3:i+7] == "Json":
            # Ok(Json(expr))
            ok_open = i + 2  # '(' after Ok
            json_open = i + 7  # '(' after "Json" (Ok(Json() -> positions 0:O 1:k 2:( 3:J 4:s 5:o 6:n 7:()
            json_close = find_matching_paren(content, json_open)
            if json_close > 0:
                inner_expr = content[json_open+1:json_close].strip()
                # 检查 Ok 的闭合括号
                k = json_close + 1
                while k < n and content[k] in ' \t\n':
                    k += 1
                if k < n and content[k] == ')':
                    k += 1
                result.append(f"api_ok({inner_expr})")
                stats["result_to_api"] += 1
                i = k
                continue

        # Err((StatusCode::XXX, msg))
        if content[i:i+4] == "Err(":
            err_open = i + 3
            # 查找内层 '('
            j = err_open + 1
            while j < n and content[j] in ' \t\n':
                j += 1
            if content[j] == '(':
                inner_open = j
                # 查找 StatusCode::
                m = re.match(r'\(\s*StatusCode::(\w+)\s*,\s*', content[inner_open:])
                if m:
                    status_variant = m.group(1)
                    code = status_map.get(status_variant, '500')
                    msg_start = inner_open + m.end()
                    # 找到匹配的内层 ')'
                    inner_close = find_matching_paren(content, inner_open)
                    if inner_close > 0:
                        msg_expr = content[msg_start:inner_close].strip()
                        # 去掉末尾逗号
                        if msg_expr.endswith(','):
                            msg_expr = msg_expr[:-1].strip()
                        # 跳过 Err 的闭合括号
                        k = inner_close + 1
                        while k < n and content[k] in ' \t\n':
                            k += 1
                        if k < n and content[k] == ')':
                            k += 1
                        result.append(f"api_error({code}, {msg_expr})")
                        stats["result_to_api"] += 1
                        i = k
                        continue

        result.append(content[i])
        i += 1

    return "".join(result)


def migrate_file(filepath: Path) -> bool:
    global stats
    print(f"\n处理: {filepath.relative_to(ORCH_ROOT)}")

    content = read_file(filepath)
    original = content

    # 1. 添加 import
    content = add_import(content)

    # 2. 替换返回类型
    content = replace_return_types(content)

    # 3. 转换 (StatusCode, Json(...)) 元组
    content = transform_statuscode_tuples(content)

    # 4. 转换 Ok(Json(...)) 和 Err((StatusCode, msg))
    content = transform_result_patterns(content)

    # 5. 转换剩余的 Json(...) 返回值
    content = transform_json_calls(content)

    if content != original:
        write_file(filepath, content)
        stats["files"] += 1
        print(f"  ✓ 已迁移")
        return True
    else:
        print(f"  - 无变化")
        return False


def main():
    print("=" * 60)
    print("mox-platform-orchestrator-svc ApiResponse 迁移脚本 v2")
    print("=" * 60)

    migrated = []
    for rel_path in TARGET_FILES:
        filepath = ORCH_ROOT / rel_path
        if not filepath.exists():
            print(f"跳过（不存在）: {rel_path}")
            continue
        if migrate_file(filepath):
            migrated.append(rel_path)

    print("\n" + "=" * 60)
    print("迁移统计")
    print("=" * 60)
    print(f"迁移文件数: {stats['files']}")
    print(f"添加 import: {stats['imports_added']}")
    print(f"返回类型替换: {stats['return_types']}")
    print(f"Json→api_ok: {stats['json_to_api_ok']}")
    print(f"元组→api_ok/api_error: {stats['tuple_to_api']}")
    print(f"Result→api_ok/api_error: {stats['result_to_api']}")
    if stats["manual_fix_needed"]:
        print(f"\n需手动检查的项 ({len(stats['manual_fix_needed'])}):")
        for item in stats["manual_fix_needed"][:20]:
            print(f"  - {item}")
    print(f"\n迁移的文件:")
    for f in migrated:
        print(f"  - {f}")


if __name__ == "__main__":
    main()
