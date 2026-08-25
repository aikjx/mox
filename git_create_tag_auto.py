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

def get_remotes() -> list:
    out = run_cmd(["git", "remote"])
    return [r for r in out.splitlines() if r.strip()]

def auto_build_tag():
    check_clean()
    t = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    tag = f"tag-{t}"
    msg = f"Auto release tag {t}"
    run_cmd(["git", "tag", "-a", tag, "-m", msg])

    remotes = get_remotes()
    if not remotes:
        print("未配置任何 git 远程，跳过 push")
        print(f"完成(仅本地tag): {tag}")
        return

    failed = []
    for remote in remotes:
        try:
            run_cmd(["git", "push", remote, tag])
            print(f"已备份到远程 [{remote}]: {tag}")
        except SystemExit:
            failed.append(remote)

    if failed:
        print(f"以下远程推送失败: {', '.join(failed)}")
        sys.exit(3)

    print(f"完成: {tag} (已同步至 {len(remotes)} 个远程: {', '.join(remotes)})")

if __name__ == "__main__":
    auto_build_tag()