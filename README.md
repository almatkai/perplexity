# Perplexity — independent Rust extension

Ask Perplexity from a Tezbar form or a `query` command argument. The original
`ask-perplexity` command metadata and `perplexityApp` preference are retained.
No API key is needed: this opens the browser search URL, or the installed
Perplexity app's `perplexity-app://search` URL. It does not fetch AI answers.

A missing/empty argument opens the form. Empty submitted text is rejected;
whitespace is preserved, and Unicode uses the original URLSearchParams encoding.
Opening failures reach the host as RPC errors. Overlapping submissions are
rejected; refresh is static. No stored drafts/fallback text have been added.

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked
CARGO_BUILD_JOBS=2 cargo test --locked -- --test-threads=1
./target/release/tezbar-perplexity --protocol-info
./target/release/tezbar-perplexity --self-test
```

The SDK is pinned to immutable commit `9f91afcc071c3f301414dc62b33cfeac7d89a0cf`.
CI checks this pin and builds this repository independently. No app crate,
Tezbar checkout, Tauri, Bun or JavaScript runtime is required.

Without flags the executable speaks frozen UTF-8 JSONL protocol 1 on stdin/stdout.
It sends hello, validates initialize identity and private paths, accepts session
open/call/dispose and shutdown. The only requested host capability is `shell.open`;
the wire call is `host.shell.open {target: URL}`. Diagnostics go to stderr.
The SDK reader remains active while that call waits for its reply. Unit tests
capture calls; separate child-process tests exercise real pipes and a fixture host
without opening a live browser/app. Host owns package verification, user grants,
session identity annotation, crash cleanup and process-tree termination.

Supported release targets are macOS ARM (`aarch64-apple-darwin`) and Windows x64
(`x86_64-pc-windows-msvc`). CI builds/tests/packages on native runners; presence of
workflow files does not claim either workflow has run. The app URL requires an
installed handler on each OS. Live browser/app opening remains an integration gate.
The source manifest retains `verification: not-run` for real platform integration.

```sh
python3 scripts/package.py --target aarch64-apple-darwin \
  --binary target/release/tezbar-perplexity
```

Run `python3 scripts/smoke.py --target aarch64-apple-darwin --archive
dist/perplexity-aarch64-apple-darwin.zip` for an isolated extracted-package smoke
with source absent and developer tools removed from PATH. It uses a fixture host
and never opens a browser. CI runs the equivalent on both native targets.

Use the Windows executable and triple on its native runner. ZIP root contains
extension.json, bin/tezbar-perplexity[.exe], resources, LICENSE, NOTICE, licenses,
and this README. `files` in the archive manifest maps every other file to SHA256;
extension.json excludes itself to avoid a circular hash. Archive/source artifacts
remain empty; the generated `.artifact.json` is the external catalog fragment
containing URL, archive SHA256, executable path and executable SHA256. Release maintainers
merge target fragments into the trusted release catalog and verifies the whole
ZIP before installation. Local verified artifact import needs no GitHub token.
No publish/remote-creation/merge step is part of this repository workflow.

Attribution and the exact source commit are in NOTICE. LICENSE covers the Tezbar
Rust adapters; the Raycast MIT notice and upstream icon are preserved separately.
