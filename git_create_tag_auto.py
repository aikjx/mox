import subprocess
import datetime
import sys

def run_cmd(cmd: list) -> str:
    try:
        return subprocess.check_output(cmd, stderr=subprocess.STDOUT, encoding="utf-8").strip()
    except subprocess.CalledProcessError as e:
        print(f"ERR: {' '.join(cmd)}\n{e.output}")
        sys.exit(1)

def check_clean():
    if run_cmd(["git", "status", "--porcelain"]):
        print("存在未提交文件，终止打tag")
        sys.exit(2)

def auto_build_tag():
    check_clean()
    t = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    tag = f"tag-{t}"
    msg = f"Auto release tag {t}"
    run_cmd(["git", "tag", "-a", tag, "-m", msg])
    run_cmd(["git", "push", "origin", tag])
    print(f"完成: {tag}")

if __name__ == "__main__":
    auto_build_tag()