# Validation record

Personal v0.1 build, September 2026. This file distinguishes executed checks from checks that still require a target device.

## Executed on Windows 11 x64

- Rust integration/unit suite: provider parsing, unavailable/expired sessions, quota window selection, token overlap arithmetic, streaming offsets, incomplete JSONL records, mixed models, duplicate requests, forks, background work, account separation, immutable sync delivery, recommendation reserves and sparse/stale history.
- Frontend: TypeScript/Svelte check, seven unit tests, production build, and headless browser interaction checks of all four pages at desktop and compact widths in light/dark appearance.
- Live Codex connection: documented app-server read succeeded in 658 ms. The temporary helper exited after the read.
- Live Claude connection: saved Code sign-in returned HTTP 401; the app reports a reconnect requirement. A successful authenticated Claude response still requires a current Claude Code sign-in.
- Initial local-log diagnostic: 241 files, 13,398 requests, about 32 seconds. Next incremental import completed in 287 ms without warnings. Counts can change while coding tools remain active.
- First Windows NSIS package: 2.8 MB (decimal), below the 30 MB goal. Final package and runtime measurements are recorded after smoke testing.

## Repeatable checks

```sh
npm ci
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
cargo run --manifest-path src-tauri/Cargo.toml --features diagnostics --bin diagnostics -- --providers
cargo run --manifest-path src-tauri/Cargo.toml --features diagnostics --bin diagnostics -- --import --data-dir /absolute/local/test-directory
```

Diagnostics print only connection status and aggregate counts, never tokens or prompt text. The diagnostic executable is feature-gated and excluded from end-user packages. Use a new local test directory for a full-import benchmark and repeat it for the incremental benchmark.

For UI smoke tests, set `GCD_USAGE_DATA_DIR` to a separate local directory and `GCD_USAGE_TEST_MODE=1` before launching. This avoids changing actual launch-at-login settings. Environment overrides should be limited to the launched process.

## Target-device checks

GitHub Actions builds and runs the test suite independently for Windows x64, Windows ARM64, macOS Intel, and macOS Apple Silicon. Cross-platform compilation does not establish native UI behavior.

On each supported OS/architecture, check:

1. Install, first-launch approval, tool discovery, reconnect, import, shared-folder setup, and launch-at-login.
2. Readable meters in light/dark mode and at common DPI/scaling settings; move the Windows strip between monitors, disconnect a monitor, and restart.
3. Sleep/wake, reset boundaries, offline stale readings, and recovery without accumulating helper processes.
4. Close the dashboard and verify its webview is destroyed. Measure idle resident/private memory and CPU separately from imports and temporary provider helpers. Goals: below 75 MB with the dashboard closed and negligible idle CPU.
5. Two real computers using the same synced folder, including offline edits, delayed/duplicate delivery, account switching, and copied histories. Unit tests cover these merge conditions; real sync-client timing needs this smoke test.
6. Install an upgrade over the previous version, verify data remains, then uninstall and verify startup registration is gone. Deleting local history and shared history is a separate deliberate operation.

Physical monitor/DPI, native macOS Keychain/menu-bar, sleep/wake, and first-launch security approval checks require the corresponding device. They are not inferred from a passing build.
