import re, pathlib

F = [
    pathlib.Path(r"d:\a10\aikjx\gitcode\infotopograph\platform\crates\xiaobai-operators\src\app.rs"),
    pathlib.Path(r"d:\a10\aikjx\gitcode\infotopograph\platform\crates\xiaobai-operators\src\file.rs"),
    pathlib.Path(r"d:\a10\aikjx\gitcode\infotopograph\platform\crates\xiaobai-operators\src\volume.rs"),
    pathlib.Path(r"d:\a10\aikjx\gitcode\infotopograph\platform\crates\xiaobai-operators\src\input.rs"),
    pathlib.Path(r"d:\a10\aikjx\gitcode\infotopograph\platform\crates\xiaobai-operators\src\server_3717.rs"),
]

total_changes = 0
for fp in F:
    if not fp.exists():
        print(f"SKIP missing {fp}")
        continue
    s = fp.read_text(encoding="utf-8")
    orig = s

    # Fix fallbacks_used: vec!["literal"] into Vec<String> required type:
    # each element inside vec![...] add .to_string()
    def fix_vec(matchobj):
        inner = matchobj.group(1)
        # split by commas, trim, convert each "xxx" to "xxx".to_string()
        parts = [p.strip() for p in inner.split(",") if p.strip()]
        fixed_parts = []
        for p in parts:
            if p.startswith('"') and p.endswith('"'):
                fixed_parts.append(p + ".to_string()")
            else:
                fixed_parts.append(p + ".to_string()")  # identifiers, shouldn't happen
        return "vec![" + ", ".join(fixed_parts) + "]"

    s = re.sub(r"fallbacks_used:\s*vec!\[(.*?)\]", lambda mm: f"fallbacks_used: {fix_vec(mm)}", s)

    # Fix InvalidArgument inside {...} blocks: param: "xxx" → param: "xxx".into()
    # and param: identifier → param: identifier.to_string(); same for hint
    def fix_invalid_arg(m):
        block = m.group(0)
        block_new = block
        block_new = re.sub(r'(param:\s*)"([^"]*)"', lambda mm: f'{mm.group(1)}"{mm.group(2)}".into()', block_new)
        block_new = re.sub(r'(hint:\s*)"([^"]*)"', lambda mm: f'{mm.group(1)}"{mm.group(2)}".into()', block_new)
        block_new = re.sub(r'(param:\s*)([A-Za-z_][A-Za-z0-9_]*)\s*,', lambda mm: f'{mm.group(1)}{mm.group(2)}.to_string(),', block_new)
        block_new = re.sub(r'(hint:\s*)([A-Za-z_][A-Za-z0-9_]*)\s*,', lambda mm: f'{mm.group(1)}{mm.group(2)}.to_string(),', block_new)
        return block_new

    s = re.sub(r'(?:xiaobai_core::XiaobaiError::|XiaobaiError::|XiErr::from\(XiaobaiError::)?InvalidArgument\s*\{[^}]*\}', fix_invalid_arg, s)

    changed_count = 0
    if s != orig:
        # Approximate count of changes
        changed_count = (
            s.count(".to_string()") - orig.count(".to_string()") +
            s.count(".into()") - orig.count(".into()")
        )
        fp.write_text(s, encoding="utf-8")
        total_changes += changed_count
        print(f"{fp.name}: ~{changed_count} changes applied")
    else:
        print(f"{fp.name}: no changes")

print(f"TOTAL approximate changes: {total_changes}")
print("fix_pass2.py DONE")
