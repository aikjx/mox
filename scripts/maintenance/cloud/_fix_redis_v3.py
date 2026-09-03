import re

fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\cloud\svc\mox-cloud-filer-svc\src\meta_redis.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Step 1: Merge multi-line self.con. into single line
# Pattern: self\n<whitespace>.con\n<whitespace>.method
content = re.sub(r'self\s*\n\s*\.con\s*\n\s*\.', 'self.con.', content)

# Step 2: Replace all self.con. with con.
n = content.count('self.con.')
content = content.replace('self.con.', 'con.')

# Step 3: Add clone line at start of each async fn method that uses con.
lines = content.split('\n')
result = []
i = 0
while i < len(lines):
    line = lines[i]
    result.append(line)
    # Detect async fn with opening brace on same or next line
    if re.search(r'async fn \w+', line):
        # Find opening brace
        brace_idx = i
        while brace_idx < len(lines) and '{' not in lines[brace_idx]:
            brace_idx += 1
            result.append(lines[brace_idx])
        if brace_idx < len(lines) and '{' in lines[brace_idx]:
            # Check if body uses con.
            uses_con = False
            depth = 0
            for j in range(brace_idx, len(lines)):
                depth += lines[j].count('{') - lines[j].count('}')
                if j > brace_idx and 'con.' in lines[j]:
                    uses_con = True
                    break
                if depth <= 0 and j > brace_idx:
                    break
            if uses_con:
                indent = len(lines[brace_idx]) - len(lines[brace_idx].lstrip())
                result.append(' ' * (indent + 4) + 'let mut con = self.con.clone();')
        i = brace_idx + 1
        continue
    i += 1

content = '\n'.join(result)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print(f'Merged and replaced {n} self.con. patterns')
print('Added clone lines to methods using con.')
