# -*- coding: utf-8 -*-
"""A-1: 修复 platform_config.json 并移除明文 admin123 密码。"""
import io
import json

f = r"D:\a10\aikjx\gitcode\infotopograph\platform_config.json"

with io.open(f, "r", encoding="utf-8-sig") as fh:
    raw = fh.read()

# 修复 PowerShell 写入的字面 \\r\\n（反斜杠+反斜杠+r+反斜杠+反斜杠+n → 真实换行）
fixed = raw.replace(r"\r\n", "\n")

# 验证解析
try:
    data = json.loads(fixed)
    print("JSON PARSE OK after escape fix")
except Exception as e:
    print("STILL BROKEN:", e)
    # 打印问题区域以便诊断
    idx = str(e).find("char")
    if idx >= 0:
        pos = int(str(e)[idx + 5 :].strip().rstrip(")"))
        print("problem area:", repr(fixed[max(0, pos - 60) : pos + 60]))
    raise SystemExit(1)

# A-1: 移除明文密码
if "admin" in data and "password" in data["admin"]:
    del data["admin"]["password"]
    print("removed password from admin")

# 规范化写回（utf-8 无 BOM，indent=2，与仓库其他 json 风格一致）
with io.open(f, "w", encoding="utf-8", newline="\n") as fh:
    json.dump(data, fh, ensure_ascii=False, indent=2)
    fh.write("\n")

# 复验
with io.open(f, "r", encoding="utf-8") as fh:
    check = json.load(fh)
flat = json.dumps(check, ensure_ascii=False)
print("RELOAD OK, top keys:", len(check))
print("admin section:", json.dumps(check["admin"], ensure_ascii=False))
print("admin123 present:", "admin123" in flat)
print("file bytes:", len(flat))
