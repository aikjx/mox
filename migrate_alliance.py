#!/usr/bin/env python3
"""Migrate alliance.rs handlers from (StatusCode, Json(json!())) tuples to ApiResponse.
Uses a line-by-line state machine to handle multi-line tuples.
"""
import re

path = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    lines = f.readlines()

original_line_count = len(lines)
result = []
i = 0
migrated_handlers = 0
success_tuples = 0
error_tuples = 0

def get_indent(line):
    return line[:len(line) - len(line.lstrip())]

while i < len(lines):
    line = lines[i]
    stripped = line.strip()

    # Detect start of a tuple: line is just "(" possibly with trailing whitespace
    # and the next non-empty line contains StatusCode::
    if stripped == '(' and i + 1 < len(lines):
        # Look ahead for StatusCode
        j = i + 1
        while j < len(lines) and lines[j].strip() == '':
            j += 1
        if j < len(lines) and 'StatusCode::' in lines[j]:
            status_line = lines[j].strip()
            indent = get_indent(line)

            if 'StatusCode::OK' in status_line:
                # === SUCCESS TUPLE ===
                # Pattern:
                #   indent(
                #       indent    StatusCode::OK,
                #       indent    Json(json!({
                #       indent        "ok": true,
                #       indent        ...fields...
                #       indent    })),
                #   indent)
                success_tuples += 1
                result.append(f'{indent}api_ok(json!({{\n')
                # Skip: opening (, StatusCode::OK,, Json(json!({
                i = j + 1  # now at Json(json!({ line or next
                # Skip Json(json!({ line
                while i < len(lines) and 'Json(json!({' not in lines[i]:
                    i += 1
                i += 1  # skip Json(json!({
                # Now process fields until })),
                while i < len(lines):
                    field_line = lines[i]
                    field_stripped = field_line.strip()
                    if field_stripped == '"ok": true,' or field_stripped == '"ok":true':
                        i += 1
                        continue
                    if field_stripped.startswith('})),'):
                        # Closing of json - replace with }))
                        inner_indent = get_indent(field_line)
                        result.append(f'{inner_indent}}}))\n')
                        i += 1
                        # Skip closing ) line
                        while i < len(lines) and lines[i].strip() != ')':
                            # There might be trailing comma on ) line like "),"
                            if lines[i].strip() == '),' or lines[i].strip() == ')':
                                break
                            result.append(lines[i])
                            i += 1
                        i += 1  # skip )
                        break
                    result.append(field_line)
                    i += 1
                continue

            elif 'StatusCode::INTERNAL_SERVER_ERROR' in status_line or 'StatusCode::NOT_FOUND' in status_line:
                # === ERROR TUPLE ===
                # Pattern:
                #   indent(
                #       indent    StatusCode::XXX,
                #       indent    Json(json!({
                #       indent        "ok": false,
                #       indent        "error": <expr>,
                #       indent    })),
                #   indent),
                error_tuples += 1
                code = 500 if 'INTERNAL_SERVER_ERROR' in status_line else 404

                # Find the "error": line
                error_expr = None
                k = j + 1
                while k < len(lines) and k < j + 10:
                    if '"error":' in lines[k]:
                        # Extract expression after "error":
                        m = re.search(r'"error":\s*(.+?),?\s*$', lines[k].strip())
                        if m:
                            error_expr = m.group(1).strip()
                        break
                    k += 1

                if error_expr:
                    # Check if the opening paren was preceded by "=> " (match arm)
                    # The line before the tuple might end with "=> ("
                    # We need to check if result's last line ends with "=>"
                    if result and result[-1].strip().endswith('=>'):
                        # Replace the "=>" line's trailing content
                        result[-1] = result[-1].rstrip() + f' api_error({code}, {error_expr}),\n'
                    else:
                        result.append(f'{indent}api_error({code}, {error_expr}),\n')
                else:
                    # Fallback: keep original
                    result.append(line)
                    i += 1
                    continue

                # Skip entire tuple: from opening ( to closing )
                i = j + 1
                paren_depth = 1
                while i < len(lines) and paren_depth > 0:
                    if '(' in lines[i]:
                        # Count parens (rough)
                        for ch in lines[i]:
                            if ch == '(':
                                paren_depth += 1
                            elif ch == ')':
                                paren_depth -= 1
                    elif ')' in lines[i]:
                        for ch in lines[i]:
                            if ch == ')':
                                paren_depth -= 1
                    i += 1
                continue

    # Default: keep line
    result.append(line)
    i += 1

# Now change return types: -> impl IntoResponse -> -> ApiResponse<Value>
content = ''.join(result)
content = content.replace('-> impl IntoResponse', '-> ApiResponse<Value>')

# Add import
if 'mox_api_protocol' not in content:
    # Insert after 'use serde_json::{json, Value};'
    content = content.replace(
        'use serde_json::{json, Value};\n',
        'use serde_json::{json, Value};\nuse mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};\n'
    )

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

# Verify
remaining = len(re.findall(r'Json\s*\(\s*json!', content))
impl_into = content.count('impl IntoResponse')
api_ok_count = content.count('api_ok(')
api_err_count = content.count('api_error(')
api_response_count = len(re.findall(r'->\s*ApiResponse<', content))

print(f"alliance.rs migration complete")
print(f"  Original lines: {original_line_count}, New lines: {len(content.splitlines())}")
print(f"  Success tuples migrated: {success_tuples}")
print(f"  Error tuples migrated: {error_tuples}")
print(f"  Remaining Json(json!): {remaining}")
print(f"  impl IntoResponse remaining: {impl_into}")
print(f"  api_ok calls: {api_ok_count}")
print(f"  api_error calls: {api_err_count}")
print(f"  ApiResponse return types: {api_response_count}")
print(f"  Import added: {'mox_api_protocol' in content}")
