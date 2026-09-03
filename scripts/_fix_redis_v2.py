fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\cloud\svc\mox-cloud-filer-svc\src\meta_redis.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# First replace all self.con. with con.
n = content.count('self.con.')
content = content.replace('self.con.', 'con.')

# Now find all async fn methods and add clone line if body uses con.
# Strategy: split by lines, track function bodies by brace counting.
lines = content.split('\n')
result = []
i = 0
while i < len(lines):
    line = lines[i]
    result.append(line)
    # Check if this line starts an async fn (with or without {)
    if 'async fn ' in line and ('{' in line or any('{' in lines[j] for j in range(i+1, min(i+5, len(lines))))):
        # Find the opening brace
        brace_line = i
        while brace_line < len(lines) and '{' not in lines[brace_line]:
            brace_line += 1
        if brace_line < len(lines):
            # Check if body uses con.
            uses_con = False
            depth = 0
            for j in range(brace_line, len(lines)):
                depth += lines[j].count('{') - lines[j].count('}')
                if 'con.' in lines[j] and j > brace_line:
                    uses_con = True
                    break
                if depth <= 0 and j > brace_line:
                    break
            if uses_con:
                # Add clone line after the brace line
                indent = len(lines[brace_line]) - len(lines[brace_line].lstrip())
                # If brace is at end of line, add after; if brace is alone, add after
                result.append(' ' * (indent + 4) + 'let mut con = self.con.clone();')
                i = brace_line + 1
                continue
    i += 1

content = '\n'.join(result)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print(f'Replaced {n} self.con. with con.')
print('Added clone lines to methods using con.')
