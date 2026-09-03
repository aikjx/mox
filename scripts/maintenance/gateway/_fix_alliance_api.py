import re

fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

fixes = []

# Pattern: (StatusCode::XXX,\n            Json(json!(...)),\n        ),
# Convert to api_ok(json!(...)) or api_error(code, msg)

# First, handle the multi-line tuple pattern
# Pattern: Err(e) => (\n StatusCode::XXX,\n Json(json!(...)),\n ),
# We need to find each match arm and convert

# Strategy: use regex to find (StatusCode::XXX, Json(json!(...))) multi-line tuples
# and replace based on StatusCode

def convert_match(match):
    status_code = match.group(1)
    json_content = match.group(2)

    # Check if it's an error response (contains "ok": false)
    if '"ok": false' in json_content or '"ok":false' in json_content:
        # Extract error message
        err_match = re.search(r'"error":\s*(.+?)(?:,|\})', json_content, re.DOTALL)
        if err_match:
            err_msg = err_match.group(1).strip()
            # Convert StatusCode to error code
            code_map = {
                'NOT_FOUND': '404',
                'INTERNAL_SERVER_ERROR': '500',
                'BAD_REQUEST': '400',
                'UNAUTHORIZED': '401',
                'FORBIDDEN': '403',
                'CONFLICT': '409',
            }
            code = code_map.get(status_code, '500')
            return f'api_error({code}, {err_msg})'
        return f'api_error(500, "operation failed")'
    else:
        # Success response
        return f'api_ok(json!({json_content}))'

# Match multi-line: (StatusCode::XXX,\n<whitespace>Json(json!(...)),\n<whitespace>)
# This is complex, let's use a different approach: find each (StatusCode::...) and its matching Json(json!(...))

# Simpler approach: replace specific patterns
# Pattern 1: (StatusCode::OK, Json(json!(...)))
content = re.sub(
    r'\(\s*StatusCode::OK\s*,\s*Json\(json!\((.*?)\)\)\s*,?\s*\)',
    lambda m: f'api_ok(json!({m.group(1)}))',
    content,
    flags=re.DOTALL
)
fixes.append('StatusCode::OK -> api_ok')

# Pattern 2: (StatusCode::CREATED, Json(json!(...)))
content = re.sub(
    r'\(\s*StatusCode::CREATED\s*,\s*Json\(json!\((.*?)\)\)\s*,?\s*\)',
    lambda m: f'api_ok(json!({m.group(1)}))',
    content,
    flags=re.DOTALL
)
fixes.append('StatusCode::CREATED -> api_ok')

# Pattern 3: Error responses with "ok": false
# NOT_FOUND
content = re.sub(
    r'\(\s*StatusCode::NOT_FOUND\s*,\s*Json\(json!\(\s*\{\s*"ok":\s*false\s*,\s*"error":\s*(.+?)\s*\}\s*\)\)\s*,?\s*\)',
    lambda m: f'api_error(404, {m.group(1).strip()})',
    content,
    flags=re.DOTALL
)
fixes.append('StatusCode::NOT_FOUND error -> api_error(404)')

# INTERNAL_SERVER_ERROR
content = re.sub(
    r'\(\s*StatusCode::INTERNAL_SERVER_ERROR\s*,\s*Json\(json!\(\s*\{\s*"ok":\s*false\s*,\s*"error":\s*(.+?)\s*\}\s*\)\)\s*,?\s*\)',
    lambda m: f'api_error(500, {m.group(1).strip()})',
    content,
    flags=re.DOTALL
)
fixes.append('StatusCode::INTERNAL_SERVER_ERROR error -> api_error(500)')

# BAD_REQUEST
content = re.sub(
    r'\(\s*StatusCode::BAD_REQUEST\s*,\s*Json\(json!\(\s*\{\s*"ok":\s*false\s*,\s*"error":\s*(.+?)\s*\}\s*\)\)\s*,?\s*\)',
    lambda m: f'api_error(400, {m.group(1).strip()})',
    content,
    flags=re.DOTALL
)
fixes.append('StatusCode::BAD_REQUEST error -> api_error(400)')

# FORBIDDEN
content = re.sub(
    r'\(\s*StatusCode::FORBIDDEN\s*,\s*Json\(json!\(\s*\{\s*"ok":\s*false\s*,\s*"error":\s*(.+?)\s*\}\s*\)\)\s*,?\s*\)',
    lambda m: f'api_error(403, {m.group(1).strip()})',
    content,
    flags=re.DOTALL
)
fixes.append('StatusCode::FORBIDDEN error -> api_error(403)')

# UNAUTHORIZED
content = re.sub(
    r'\(\s*StatusCode::UNAUTHORIZED\s*,\s*Json\(json!\(\s*\{\s*"ok":\s*false\s*,\s*"error":\s*(.+?)\s*\}\s*\)\)\s*,?\s*\)',
    lambda m: f'api_error(401, {m.group(1).strip()})',
    content,
    flags=re.DOTALL
)
fixes.append('StatusCode::UNAUTHORIZED error -> api_error(401)')

# Catch-all: any remaining (StatusCode::XXX, Json(json!(...))) without "ok": false
content = re.sub(
    r'\(\s*StatusCode::\w+\s*,\s*Json\(json!\((.*?)\)\)\s*,?\s*\)',
    lambda m: f'api_ok(json!({m.group(1)}))',
    content,
    flags=re.DOTALL
)
fixes.append('remaining StatusCode tuples -> api_ok')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('alliance.rs fixes:')
for f in fixes:
    print(f'  - {f}')
print('Done')
