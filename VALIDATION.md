# Perplexity validation

Both production modules copied from frozen app commit
fe71ca87f21ac4ff31505010c228f8abda0f2a89; changes are SDK imports/context + typed
host.shell.open. Original manifest command/preference metadata, Apache license,
Raycast MIT notice and upstream Perplexity logo retained (provenance in NOTICE).

Local environment: macOS 26.3 (25D2125), Darwin arm64,
Rust 1.98.0 / Cargo 1.98.0. All commands passed with CARGO_BUILD_JOBS=2:

- `cargo test --offline --locked -- --test-threads=1`: **7 tests passed**
  (3 adapter + 4 stdio/executable integration; 0 ignored).
- `cargo build --release --offline --locked`.
- `cargo clippy --offline --locked --all-targets -- -D warnings`.
- `cargo fmt --check`.
- `python3 scripts/package.py --target aarch64-apple-darwin --binary target/release/tezbar-perplexity`.
- `python3 scripts/smoke.py --target aarch64-apple-darwin --archive dist/perplexity-aarch64-apple-darwin.zip`.

Process evidence: real independent executable hello/identity, initialize, form,
malformed form/argument rejection, exact UTF-8 + whitespace URL encoding, browser
and app preference snapshots, spoofed payload preference/session override rejection,
typed reverse shell.open while handler is pending, duplicate submit rejection,
open failure/retry, dispose during an action, reverse cancellation, refresh/stale
session rejection, shutdown with host stdin still open, restart and EOF with pending reverse RPC.

Package smoke verifies inventory, extracts into a fresh temporary directory, runs
with source absent and developer tools removed from PATH, then performs the real
stdio form/action/dispose/shutdown roundtrip. Its fixture host acknowledges the
shell call; it does NOT open a live browser or Perplexity app. Original app-side
registry/fallback/uninstall/profile tests remain host-owned, not silently moved
into this independent extension.

Local development artifact: dist/perplexity-aarch64-apple-darwin.zip and matching
.artifact.json. The JSON records current archive/executable SHA256; URL is the
intended future release location, not proof of upload. The package carries files
inventory excluding extension.json itself, root licenses/notices and original icon.
ZIP source/archive artifacts remain empty; host trusted catalog pins full target
artifact metadata outside the archive (no circular archive self-hash).

Logs: /tmp/tezbar-external-rust/perplexity-test.log, perplexity-build.log,
perplexity-clippy.log, perplexity-format.log, perplexity-package.log,
perplexity-package-smoke.log. SDK has separate sdk-*.log evidence.

NOT verified: Windows MSVC/native tests, GitHub CI run, host app artifact install,
live browser/app opening, signed/notarized distribution. Source manifest preserves
platform verification=not-run for real product integration.

Remaining gates:

1. SDK pinned to `9f91afcc071c3f301414dc62b33cfeac7d89a0cf`; Cargo.lock
   resolves the public Git source. Locked tests (7), release build, strict Clippy,
   formatting and regenerated package smoke passed after pinning. Logs use the
   `/tmp/tezbar-external-rust/perplexity-pinned-*.log` prefix.
2. Run native macOS ARM + Windows x64 MSVC CI build/tests/package/package smoke.
3. Publish verified target releases, regenerate final ZIPs,
   merges generated target artifact JSON into trusted catalog and pins hashes.
4. Host supports frozen protocol, shell.open {target}, scoped session lifecycle,
   renderer error/view annotation, rejects stale generations, disposes no-view
   sessions, and kills process trees/resources on crash. Exact generic integration
   requirements: https://github.com/almatkai/tezbar-extension-sdk-rust/blob/9f91afcc071c3f301414dc62b33cfeac7d89a0cf/HOST_INTEGRATION.md.
5. Run clean-profile artifact installation and a real browser/app preference action
   on both OSes before product verification changes from not-run.
