import re

# Fix meta_redis.rs
fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\cloud\svc\mox-cloud-filer-svc\src\meta_redis.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Strategy: find all async fn methods that use self.con, and add `let mut con = self.con.clone();`
# then replace self.con. with con. within those methods.
# Simpler approach: replace all `self.con.` with `con.` and add clone at the start of each method.

# First, replace all self.con. with con.
n_con = content.count('self.con.')
content = content.replace('self.con.', 'con.')

# Now we need to add `let mut con = self.con.clone();` before each use of `con.`
# The simplest way: find each async fn in impl RealRedisStore or impl MetaBackend for RealRedisStore
# and add the clone line after the function signature.

# Actually, a simpler approach: replace the first `con.` in each method with `{ let mut con = self.con.clone(); con.`
# But this is complex. Let's use a different approach:
# Add `let mut con = self.con.clone();` right after every `async fn ...` line in the impl blocks.

lines = content.split('\n')
new_lines = []
in_impl = False
for i, line in enumerate(lines):
    new_lines.append(line)
    # Detect async fn methods (not trait definitions)
    if re.match(r'\s*async fn \w+', line) and '{' in line:
        # Check if this method uses con. in its body
        # Look ahead for con.
        uses_con = False
        for j in range(i+1, min(i+30, len(lines))):
            if 'con.' in lines[j]:
                uses_con = True
                break
            if lines[j].strip() == '}' and j > i+1:
                break
        if uses_con:
            indent = len(line) - len(line.lstrip())
            new_lines.append(' ' * (indent + 4) + 'let mut con = self.con.clone();')
    elif re.match(r'\s*async fn \w+', line) and '{' not in line:
        # Multi-line signature, look for { on next lines
        # Check if this method uses con. in its body
        uses_con = False
        for j in range(i+1, min(i+40, len(lines))):
            if 'con.' in lines[j]:
                uses_con = True
                break
            if '{' in lines[j]:
                break
        if uses_con:
            # Find the { line and add clone after it
            pass  # We'll handle this case separately

content = '\n'.join(new_lines)

# Handle the case where async fn signature spans multiple lines
# Find patterns where con. is used but no clone line was added
# This is a fallback: if we see `con.` without a preceding `let mut con`, add it
# Actually, let's just do a simpler fix: replace all occurrences of the pattern
# where a method body starts with con. usage

# Write back
with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print(f'meta_redis.rs: replaced {n_con} self.con. with con.')
print('Added clone lines where possible')
