---
name: src-tauri-not-rustfmt-clean
description: src-tauri HEAD is not rustfmt-1.8.0-clean; global cargo fmt rewrites ~30 unrelated files
metadata: 
  node_type: memory
  type: project
  originSessionId: 1378d90e-c255-497a-a0f2-56f1cceaeb46
---

As of 2026-07-20 (branch `v.0.12.0`), the `src-tauri/` tree at HEAD is **not** rustfmt-1.8.0-clean. Running a global `cargo fmt --manifest-path src-tauri/Cargo.toml` reformats ~30 pre-existing files (import reflow, signature collapsing, etc.) that are unrelated to whatever you're changing - producing a huge noisy diff. No `rustfmt.toml` exists (default config).

**Why:** The committed code was formatted with a different rustfmt version or by hand; CI evidently doesn't enforce `cargo fmt --check` on src-tauri.

**How to apply:** Do NOT run global `cargo fmt` on this repo. If you do (or did), revert untouched files with `git checkout HEAD -- src-tauri/` and re-apply only your logical edits minimally - match the surrounding (non-rustfmt) style. Verify with `cargo check` / `cargo clippy` / `cargo test`, not `cargo fmt --check` (the latter fails at HEAD regardless). The frontend (`src/`) is a normal Vue/TS tree and is unaffected.
