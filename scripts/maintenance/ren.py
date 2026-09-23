import os, sys
old = sys.argv[1]
new = sys.argv[2]
os.rename(old, new)
print(f"OK: {old} -> {new}")
