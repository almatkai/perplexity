#!/usr/bin/env python3
"""Validate and smoke an extracted package using an actual fixture host.

No Tezbar/app source, SDK source, Cargo or Bun is used by the executable.
This fixture acknowledges shell calls; it never opens a real browser.
"""
import argparse
import hashlib
import json
import os
import pathlib
import queue
import stat
import subprocess
import tempfile
import threading
import uuid
import zipfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=pathlib.Path, required=True)
    parser.add_argument("--target", choices=["aarch64-apple-darwin", "x86_64-pc-windows-msvc"], required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="perplexity-smoke-") as directory:
        root = pathlib.Path(directory)
        with zipfile.ZipFile(args.archive) as package:
            manifest = json.loads(package.read("extension.json"))
            assert set(package.namelist()) == set(manifest["files"]) | {"extension.json"}
            for entry in package.infolist():
                relative = pathlib.PurePosixPath(entry.filename)
                assert not relative.is_absolute() and all(part not in ("..", ".") and ":" not in part and "\\" not in part for part in relative.parts)
                assert not stat.S_ISLNK(entry.external_attr >> 16)
                data = package.read(entry)
                if entry.filename != "extension.json":
                    assert hashlib.sha256(data).hexdigest() == manifest["files"][entry.filename]
                path = root.joinpath(*relative.parts)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
        executable = root / "bin" / ("tezbar-perplexity.exe" if args.target.startswith("x86_64-pc-windows") else "tezbar-perplexity")
        executable.chmod(0o755)
        # Preserve OS essentials, but remove developer/runtime lookup from PATH.
        env = {name: value for name, value in os.environ.items() if name in ("SystemRoot", "WINDIR", "TEMP", "TMP")}
        env["PATH"] = os.path.join(env.get("SystemRoot", "C:/Windows"), "System32") if os.name == "nt" else "/usr/bin:/bin"
        info = json.loads(subprocess.check_output([str(executable), "--protocol-info"], env=env, cwd=root, timeout=5))
        assert info["id"] == manifest["id"] and info["version"] == manifest["version"]
        messages = queue.Queue(maxsize=128)
        process = subprocess.Popen([str(executable)], cwd=root, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)

        def read_output():
            for line in process.stdout:
                messages.put(line)
            messages.put(None)

        reader = threading.Thread(target=read_output, daemon=True)
        reader.start()

        def receive():
            line = messages.get(timeout=5)
            assert line is not None, "Unexpected process EOF"
            assert len(line) <= 16 * 1024 * 1024 + 1
            return json.loads(line)

        def send(value):
            process.stdin.write((json.dumps(value, ensure_ascii=False) + "\n").encode())
            process.stdin.flush()

        def request(method, params):
            value = {"type": "request", "id": "h:" + str(uuid.uuid4()), "method": method, "params": params}
            send(value)
            return value["id"]

        def invoke(method, params):
            identity = request(method, params)
            response = receive()
            assert response["id"] == identity
            return response

        try:
            hello = receive()
            assert hello["type"] == "hello" and hello["id"] == manifest["id"] and hello["protocol"] == 1
            assert invoke("initialize", {"protocol": 1, "extensionId": manifest["id"], "platform": "Windows" if os.name == "nt" else "macOS",
                                         "packageDir": str(root), "dataDir": str(root / "data"), "tempDir": str(root / "temp"),
                                         "grantedCapabilities": ["shell.open"], "reason": "package-smoke", "migrationPaths": {}})["result"]["ready"]
            assert invoke("session.open", {"sessionId": "smoke", "commandName": "ask-perplexity", "preferences": {}, "argumentValues": {}})["result"]["mode"] == "view"
            assert "error" in invoke("session.call", {"sessionId": "smoke", "operation": "extension:invoke-action", "payload": {"actionId": "perplexity:submit", "formValues": {"query": ""}}})
            identity = request("session.call", {"sessionId": "smoke", "operation": "extension:invoke-action", "payload": {"actionId": "perplexity:submit", "formValues": {"query": "日本 😀 +"}}})
            reverse = receive()
            assert reverse["method"] == "host.shell.open" and reverse["params"]["target"] == "https://www.perplexity.ai/search?q=%E6%97%A5%E6%9C%AC+%F0%9F%98%80+%2B"
            send({"type": "response", "id": reverse["id"], "result": None})
            response = receive()
            assert response["id"] == identity and response["result"]["mode"] == "no-view"
            assert "result" in invoke("session.dispose", {"sessionId": "smoke"})
            assert "error" in invoke("session.call", {"sessionId": "smoke", "operation": "extension:refresh-session", "payload": {}})
            assert "result" in invoke("shutdown", {})
            # Shutdown must finish even while the host write end stays open.
            assert process.wait(timeout=6) == 0
            process.stdin.close()
            reader.join(timeout=1)
            print(json.dumps({"passed": True, "target": args.target, "fixtureHost": True, "liveShellOpen": False,
                              "checks": ["inventory", "isolated-package", "identity", "form", "validation", "unicode-url", "reverse-shell-rpc", "dispose", "shutdown"]}))
        finally:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)
            if not process.stdin.closed:
                process.stdin.close()
            process.stdout.close()


if __name__ == "__main__":
    main()
