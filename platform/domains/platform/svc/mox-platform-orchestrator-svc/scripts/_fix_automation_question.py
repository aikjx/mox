#!/usr/bin/env python3
"""
修复 automation.rs 中的 ? 操作符模式。
将 Result<Json<T>, (StatusCode, String)> 迁移到 ApiResponse<T> 后，
? 操作符不再可用，需要转换为显式 match / if let。
"""

import re
from pathlib import Path

ORCH_ROOT = Path(__file__).resolve().parent.parent

STATUS_MAP = {
    'BAD_REQUEST': '400', 'UNAUTHORIZED': '401', 'FORBIDDEN': '403',
    'NOT_FOUND': '404', 'CONFLICT': '409', 'INTERNAL_SERVER_ERROR': '500',
}


def read_file(path):
    with open(path, "r", encoding="utf-8-sig", newline="") as f:
        return f.read()


def write_file(path, content):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)


def find_matching_paren(text, open_pos):
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


def convert_question_marks(content):
    """转换所有 ? 操作符模式"""
    result = []
    i = 0
    n = len(content)
    fixed = 0

    while i < n:
        # 查找 .map_err(|e| (StatusCode::XXX, ...))?
        map_err_match = re.search(r'\.map_err\(\|e\|\s*\(\s*StatusCode::(\w+)\s*,\s*', content[i:])
        if map_err_match and map_err_match.start() == 0:
            # 找到 map_err 模式
            status_variant = map_err_match.group(1)
            code = STATUS_MAP.get(status_variant, '500')

            # 找到 map_err 闭包的结束 (the ) before ?)
            # map_err(|e| (StatusCode::XXX, expr))
            paren_start = i + map_err_match.end() - 1  # position of ( after comma
            # Actually, let me find the outer ( of map_err(
            map_err_paren = i + map_err_match.start() + len('.map_err')  # position of (
            close_paren = find_matching_paren(content, map_err_paren)

            if close_paren > 0:
                # 提取错误消息表达式 (inside the inner tuple)
                inner_start = i + map_err_match.end()  # after "StatusCode::XXX, "
                # find the matching ) for the inner tuple
                inner_paren = paren_start
                inner_close = find_matching_paren(content, inner_paren)
                if inner_close > 0:
                    err_expr = content[inner_start:inner_close].strip()
                else:
                    err_expr = 'e.to_string()'

                # 检查 ? 后面是否有 ; 或 .ok_or( 或其他
                after_close = close_paren + 1
                while after_close < n and content[after_close] in ' \t\n':
                    after_close += 1

                # 检查是否是 let x = expr?...; 模式
                # 向前查找 let
                let_match = re.search(r'let\s+(\w+)\s*=\s*$', content[max(0,i-200):i], re.MULTILINE)

                if content[after_close] == '?':
                    # 有 ? 操作符
                    q_pos = after_close
                    after_q = q_pos + 1
                    while after_q < n and content[after_q] in ' \t\n':
                        after_q += 1

                    # 检查 ? 后面是否跟 .ok_or(
                    if content[after_q:after_q+7] == '.ok_or(':
                        # 链式 .ok_or()? 模式
                        okor_paren = after_q + 6  # position of (
                        okor_close = find_matching_paren(content, okor_paren)
                        if okor_close > 0:
                            # 提取 ok_or 的参数
                            okor_content = content[okor_paren+1:okor_close].strip()
                            # 解析 (StatusCode::XXX, msg)
                            okor_status = re.search(r'StatusCode::(\w+)', okor_content)
                            okor_code = STATUS_MAP.get(okor_status.group(1), '404') if okor_status else '404'
                            okor_msg_match = re.search(r',\s*(.+)$', okor_content, re.DOTALL)
                            okor_msg = okor_msg_match.group(1).strip() if okor_msg_match else '"not found"'

                            # 检查 ? 后面
                            after_okor = okor_close + 1
                            while after_okor < n and content[after_okor] in ' \t\n':
                                after_okor += 1

                            if content[after_okor] == '?':
                                final_q = after_okor
                                after_final = final_q + 1
                                while after_final < n and content[after_final] in ' \t\n':
                                    after_final += 1

                                if let_match:
                                    var_name = let_match.group(1)
                                    # let x = expr.map_err(...).ok_or(...)?;
                                    # → let x = match expr { Ok(v) => v, Err(e) => return api_error(code, err_expr) };
                                    #   let x = match x { Some(v) => v, None => return api_error(okor_code, okor_msg) };
                                    expr_before = content[let_match.end():i].strip()
                                    replacement = f"match {expr_before} {{\n                        Ok(v) => v,\n                        Err(e) => return api_error({code}, {err_expr}),\n                    }};\n                    let {var_name} = match {var_name} {{\n                        Some(v) => v,\n                        None => return api_error({okor_code}, {okor_msg}),\n                    }}"
                                    # 移除原来的 let x =
                                    result = result[:let_match.start()] if let_match.start() > 0 else []
                                    # 这太复杂了，让我用更简单的方法
                                    pass

                    # 简单情况: expr.map_err(...)?;
                    if let_match:
                        var_name = let_match.group(1)
                        expr_before = content[let_match.end():i].strip()
                        # 找到语句结束
                        stmt_end = after_q
                        while stmt_end < n and content[stmt_end] != ';':
                            stmt_end += 1
                        stmt_end += 1  # include ;

                        replacement = f"let {var_name} = match {expr_before} {{\n                        Ok(v) => v,\n                        Err(e) => return api_error({code}, {err_expr}),\n                    }};"
                        result.append(replacement)
                        i = stmt_end
                        fixed += 1
                        continue
                    else:
                        # 语句级: expr.map_err(...)?;
                        stmt_end = after_q
                        while stmt_end < n and content[stmt_end] != ';':
                            stmt_end += 1
                        stmt_end += 1

                        expr_before = content[i:i]  # empty, expr is before map_err
                        # 找到表达式开始（向前找语句开始）
                        # 简单处理: 整个语句是 expr.map_err(...)?;
                        # 转换为 if let Err(e) = expr { return api_error(code, err_expr); }
                        # 但 expr 可能很复杂，让我直接用 match
                        replacement = f"match {{}} {{}}"  # placeholder
                        # 实际上，对于语句级的 ?，我们需要保留表达式
                        # 让我找到表达式开始
                        expr_start = i
                        while expr_start > 0 and content[expr_start-1] not in ';\n{':
                            expr_start -= 1
                        full_expr = content[expr_start:close_paren+1].strip()
                        # 移除 .map_err(...) 部分
                        base_expr = full_expr[:full_expr.index('.map_err')].strip()
                        replacement = f"if let Err(e) = {base_expr} {{\n                        return api_error({code}, {err_expr});\n                    }}"
                        result.append(replacement)
                        i = stmt_end
                        fixed += 1
                        continue

        result.append(content[i])
        i += 1

    return "".join(result), fixed


def main():
    filepath = ORCH_ROOT / "src/automation.rs"
    content = read_file(filepath)

    # 先统计有多少个 ? 操作符
    q_count = content.count('?')
    print(f"automation.rs 中共有 {q_count} 个 ? 操作符")

    # 这个自动转换太复杂了，让我用更直接的方法
    # 直接读取文件，找到所有 handler，手动重写

    print("automation.rs 的 ? 模式需要手动修复，正在生成修复方案...")

    # 让我用正则替换简单的模式
    # 模式1: .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    # 模式2: .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?

    # 对于 let x = expr?...; 模式
    # 我们需要转换为 match

    # 让我先看看所有包含 ? 的行
    lines = content.split('\n')
    for i, line in enumerate(lines):
        if '?' in line and 'map_err' in line:
            print(f"  行 {i+1}: {line.strip()[:100]}")

    print(f"\n需要修复的行数: {sum(1 for l in lines if '?' in l and 'map_err' in l)}")


if __name__ == "__main__":
    main()
