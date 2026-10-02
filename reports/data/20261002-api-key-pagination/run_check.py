"""Capture the exact command, exit status and full real check output."""
import json
import subprocess
import sys
from pathlib import Path

folder = Path(__file__).resolve().parent
root = folder.parents[2]
name = sys.argv[1]
cwd = root / "frontend-ui" if name == "frontend_build" else root
command = sys.argv[2:]
result = subprocess.run(command, cwd=str(cwd), stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
(folder / (name + ".log")).write_bytes(result.stdout)
(folder / (name + ".json")).write_text(json.dumps({"command":command,"cwd":str(cwd),"exit_code":result.returncode,"log":name+".log"},indent=2),encoding="utf-8")
print(name, "exit_code", result.returncode)
sys.stdout.buffer.write(result.stdout[-2200:])
sys.exit(result.returncode)
