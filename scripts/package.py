#!/usr/bin/env python3
"""Stage a deterministic ZIP and external catalog fragment; never publish."""
import argparse
import hashlib
import json
import pathlib
import stat
import subprocess
import zipfile

TARGETS = {"aarch64-apple-darwin": "tezbar-perplexity", "x86_64-pc-windows-msvc": "tezbar-perplexity.exe"}
ROOT = pathlib.Path(__file__).resolve().parents[1]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--binary", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "dist")
    args = parser.parse_args()
    manifest = json.loads((ROOT / "extension.json").read_text(encoding="utf-8"))
    # Run the actual built target on its native runner before packaging.
    info = json.loads(subprocess.check_output([str(args.binary.resolve()), "--protocol-info"], timeout=5))
    assert info["id"] == manifest["id"] and info["version"] == manifest["version"] and info["protocol"] == 1
    executable = "bin/" + TARGETS[args.target]
    files = {executable: args.binary.read_bytes()}
    for name in ["LICENSE", "NOTICE", "README.md"]:
        files[name] = (ROOT / name).read_bytes()
    for directory in ["resources", "licenses"]:
        for path in sorted((ROOT / directory).rglob("*")):
            if path.is_symlink():
                raise ValueError(f"Symlink assets are forbidden: {path}")
            if path.is_file():
                files[path.relative_to(ROOT).as_posix()] = path.read_bytes()
    manifest["files"] = {name: sha(data) for name, data in sorted(files.items())}
    # Archive SHA cannot be included inside itself. Full artifact metadata is
    # external and trusted/pinned by host catalog; source/archive artifacts stay {}.
    files["extension.json"] = (json.dumps(manifest, indent=2, ensure_ascii=False) + "\n").encode()
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f"perplexity-{args.target}.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as package:
        for name, data in sorted(files.items()):
            entry = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            entry.create_system = 3
            entry.external_attr = ((stat.S_IFREG | (0o755 if name == executable else 0o644)) << 16)
            entry.compress_type = zipfile.ZIP_DEFLATED
            package.writestr(entry, data)
    artifact = {"url": f"{manifest['repository']}/releases/download/v{manifest['version']}/{archive.name}",
                "sha256": sha(archive.read_bytes()), "executable": executable,
                "executableSha256": manifest["files"][executable]}
    fragment = {"id": manifest["id"], "version": manifest["version"], "artifacts": {args.target: artifact}}
    metadata = args.output / f"perplexity-{args.target}.artifact.json"
    metadata.write_text(json.dumps(fragment, indent=2) + "\n", encoding="utf-8")
    with zipfile.ZipFile(archive) as package:
        assert set(package.namelist()) == set(files)
        for name, digest in manifest["files"].items():
            assert sha(package.read(name)) == digest
    print(json.dumps({"archive": str(archive), "artifact": str(metadata), "sha256": artifact["sha256"]}))


if __name__ == "__main__":
    main()
