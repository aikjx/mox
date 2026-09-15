"""Real-process deployment check; requires a built mox-server. Uses isolated temporary data."""
import argparse
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def request(port, path, data=None, token=None, method=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}{path}",
        data=None if data is None else json.dumps(data).encode(), headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode()


def run(binary):
    secret = secrets.token_urlsafe(48)
    processes, logs = {}, []
    with tempfile.TemporaryDirectory(prefix="mox-host-check-") as work:
        def start(role):
            directory = Path(work) / role
            directory.mkdir(exist_ok=True)
            with socket.socket() as sock:
                sock.bind(("127.0.0.1", 0))
                port = sock.getsockname()[1]
            env = os.environ.copy()
            env.update(JWT_SECRET=secret, MOX_DEV_MODE="0", MOX_HOST_ROLE=role,
                       MOX_STORE_DATA_DIR=str(directory / "data/store"),
                       MOX_STORAGE_ROOT=str(directory / "data/storage"))
            log = open(directory / "process.log", "a", encoding="utf-8")
            logs.append(log)
            process = subprocess.Popen([str(binary), "--bind", "127.0.0.1", "--port", str(port)],
                cwd=directory, env=env, stdout=log, stderr=log,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            processes[role] = process
            for _ in range(120):
                if process.poll() is not None:
                    raise AssertionError(f"{role} exited: {(directory / 'process.log').read_text()[-3000:]}")
                try:
                    if request(port, "/health")[0] == 200:
                        return port
                except (OSError, TimeoutError):
                    pass
                time.sleep(0.25)
            raise AssertionError(f"{role} did not become ready")

        def stop(role):
            process = processes.pop(role)
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()

        credentials = {"username": "hostcheck", "email": "hostcheck@example.test",
                       "password": secrets.token_urlsafe(24), "tenant_id": "T001"}
        try:
            iam = start("iam")
            status, _ = request(iam, "/api/auth/register", credentials)
            assert status == 200, "registration failed"
            status, login = request(iam, "/api/auth/login", credentials)
            assert status == 200 and "access_token" in login.get("data", {}), login
            token = login["data"]["access_token"]
            assert request(iam, "/api/kb/documents", token=token)[0] == 404
            stop("iam")
            iam = start("iam")
            status, login = request(iam, "/api/auth/login", credentials)
            assert status == 200 and "access_token" in login.get("data", {}), "IAM restart lost users"
            print("PASS IAM registration/login/restart persistence and route isolation", flush=True)
            for role, path in [("kg", "/kg/v1/stats"), ("cloud", "/cloud/v1/buckets"),
                               ("kb", "/api/kb/documents")]:
                port = start(role)
                assert request(port, path)[0] == 401, role
                assert request(port, path, token="dev-token")[0] == 401, role
                status, _ = request(port, path, token=token)
                assert status == 200, (role, status)
                assert request(port, "/api/auth/me", token=token)[0] == 404, role
                if role == "cloud":
                    assert request(port, path, {"name": "hostcheck"}, token)[0] == 200
                    assert request(port, path + "/hostcheck/objects/check.json", {"persisted": True}, token, "PUT")[0] == 200
                if role == "kb":
                    status, created = request(port, path, {"title": "hostcheck", "content": "persistent document"}, token)
                    assert status == 200 and created.get("data"), created
                stop(role)
                if role in ("cloud", "kb"):
                    port = start(role)
                    if role == "cloud":
                        assert request(port, path + "/hostcheck/objects/check.json", token=token)[1] == {"persisted": True}
                    else:
                        status, listing = request(port, path, token=token)
                        assert status == 200 and listing["data"]["total"] == 1, listing
                    stop(role)
                print(f"PASS {role}: shared IAM token, anonymous/dev token rejected, unrelated routes absent", flush=True)
            stop("iam")
            fused = start("all")
            for path in ["/kg/v1/stats", "/cloud/v1/buckets", "/api/kb/documents"]:
                assert request(fused, path, token=token)[0] == 200, path
            print("PASS fused host exposes the same three domain APIs", flush=True)
        finally:
            for role in list(processes):
                stop(role)
            for log in logs:
                log.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    run(args.binary.resolve(strict=True))
