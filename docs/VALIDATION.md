# Validation record

## v0.1.2 intervals and readability

- 52 Rust tests and 12 frontend tests passed, with zero Svelte/TypeScript errors or warnings. Interval tests cover exclusive end boundaries, carry-in prompts, subagents, duplicate requests, model/effort changes, account boundaries, overlapping reasoning tokens, empty ranges, and bounded chart allocation. Frontend tests cover rolling durations, local calendar weeks/months, custom seconds, invalid dates, and history boundary conversion.
- The isolated Windows app queried a copy of real history containing 14,467 requests. Six intervals (30 minutes through all time) matched independent SQLite request counts and token sums. Model-group totals and chart totals matched the same records. Queries took 7–83 ms on this computer.
- Native UI checks exercised all ten interval choices, invalid and empty custom ranges, history navigation, persisted interval selection, and font sizes 90%, 120%, and 160%. No browser errors were observed. Screenshots were inspected locally; they are excluded from the repository.
- Existing 0.1.1 settings migrate to an unlocked strip at 120% text size while preserving theme, percentage mode, device identity, and saved position. Lock position is a separate optional setting; it does not snap to the taskbar. Native fonts, strip size, margins, and drag grip share the same DPI and text scale.
- After explicit dashboard close, its browser child processes exited. The isolated background app measured 34.9 MiB working set and 11.8 MiB private memory, with no children. Provider helpers and imports remain temporary costs.
- Repeat the native interval checks with `scripts/interval-smoke.cjs`; repeat theme, percentage, and position-lock checks with `scripts/display-smoke.cjs`. Use the private isolated test environment described below.
- The 2.59 MB Windows x64 installer upgraded the normal per-user installation to 0.1.2. Installed-app checks passed for all five themes, both percentage modes, both position-lock settings, and preference persistence. The normal app was reopened without debugging/test overrides; both providers returned fresh connected readings. Device identity, login startup preference, sync folder, connection paths, history, and unrelated display preferences were retained.
- All four dashboard pages fit the minimum 760-pixel window width at 160% text size. Physical dragging across mixed-DPI monitors and interactive macOS menu-bar checks remain target-device checks.


- All four architectures passed the required tests and package builds in [the 0.1.2 release workflow](https://github.com/eshin087/gcdUsage/actions/runs/34172380827); both Mac packages passed background launch. The [private release](https://github.com/eshin087/gcdUsage/releases/tag/v0.1.2) contains four installers and SHA-256 checksums. Uploaded asset sizes and digests were verified: Windows x64 2.59 MB, Windows ARM64 2.31 MB, macOS Intel 3.43 MB, and macOS Apple Silicon 3.14 MB.

## v0.1.1 display update

- 50 Rust tests and eight frontend tests pass, with zero Svelte/TypeScript errors or warnings. New checks cover migration of existing settings, remaining-versus-used arithmetic, missing measurements, taskbar edges, scaled geometry, and secondary displays with negative coordinates.
- The Windows 0.1.1 installer upgraded the existing per-user installation. An isolated native-app smoke test selected all five themes, exercised both percentage modes and both anchor settings, and verified saved preferences after reloading. Black is RGB (0, 0, 0); all four explicit palettes are distinct. System followed the Windows light preference on this computer.
- The dashboard and native meters default to percentage left. Unknown reset time uses N/A. Theme changes do not trigger provider polling or rewrite launch-at-login unless those settings change.
- An explicit dashboard-close event was confirmed; all child processes exited. The background app used approximately 43.2 MiB after the display checks. The local Windows installer is 2.58 MB.
- Taskbar anchoring uses the monitor work area and locks dragging. Geometry tests cover bottom/top/left/right taskbars and DPI-sized strips; physical monitor rearrangement and auto-hide remain device checks. macOS colors in the system menu bar continue to follow macOS.
- Repeat the native display checks with `scripts/display-smoke.cjs` using the isolated test environment described below.
- All four target platforms passed the required test/build steps, and both Mac packages passed background launch in [the release build](https://github.com/eshin087/gcdUsage/actions/runs/34170544795). The [private 0.1.1 release](https://github.com/eshin087/gcdUsage/releases/tag/v0.1.1) contains four installers; uploaded SHA-256 digests were verified. Sizes are 2.58 MB (Windows x64), 2.30 MB (Windows ARM64), 3.40 MB (macOS Intel), and 3.11 MB (macOS Apple Silicon).

## v0.1.0 baseline

Personal v0.1.0 preview, September 7, 2026. Executed checks are listed separately from checks requiring the user's signed-in account or physical devices.

## Executed on Windows 11 x64

- **45 Rust tests passed**, covering provider parsing, expired/missing sign-ins, quota-window selection, token overlap arithmetic, streaming offsets, incomplete records, mixed models, duplicate requests, forks, subagents/background work, account separation, sync delivery, and recommendation budgets.
- **Seven frontend tests passed**; Svelte/TypeScript reported zero errors or warnings. Production and headless UI checks cover all four pages, compact/desktop widths, and light/dark appearance.
- Live Codex account/rate-limit/model reads succeeded through its temporary app-server. The provider helper exits after the read.
- Live Claude polling succeeded after renewing the Claude Code sign-in: the private usage endpoint returned both five-hour and weekly usage in 206 ms. Before reconnect, the app correctly detected local token expiry without repeatedly contacting the endpoint. The five-hour response omitted a reset timestamp; that remains unavailable rather than being invented.
- A fresh real-log diagnostic imported 265 files and 13,563 requests in 14.3 seconds, with zero warnings. It measured about 1.912 billion tokens, mostly repeated cache reads. The next incremental scan took 229 ms (575 ms including statistics), added no requests/prompts, and preserved the same totals. Counts grow while coding tools are active.
- Regression tests cover mirrored modern/legacy token records after compaction, inherited initial cumulative totals, and repeated identical prompts in different turns. These prevent historical counter rebasing from inflating token totals.
- The corrected **installed application** imported 272 files, 263 user prompts, 30 conversations, and 13,650 requests with no warnings. Its native WebView smoke test exercised history filters/details, all three recommendation choices, settings persistence/reload, and dashboard close; no browser errors occurred.
- Closing the dashboard releases its WebView2 processes. After the corrected installed-app smoke test, the background process used **41.0 MiB working set / 18.3 MiB private memory**, with zero child processes. A cold background run measured **20.8 MiB working set**. Both are below the 75 MB background target.
- Temporary provider helpers are measured separately: a sampled Codex helper peak was **103.2 MiB**, with a combined main/helper peak of 119.0 MiB. They are not permanent background memory.
- A 25-second background sample used **3.06% of one logical CPU**, approximately **0.096% total CPU on this 32-thread machine**, including maintenance while the coding tools remained active. This is a sample, not a guarantee for every computer or import.
- Windows per-user install, upgrade, uninstall, shortcut creation, and retention of local application data passed an isolated lifecycle test. The final corrected build was then installed in the normal per-user location. Installer changes to Tauri's bundle marker were accounted for when comparing installed and build executables.
- The Windows x64 NSIS installer is about **2.57 MB**, below the 30 MB target. End users do not need Rust, Node.js, or C++ build tools.

## Cross-platform checks

GitHub Actions builds and tests Windows x64, Windows ARM64, macOS Intel, and macOS Apple Silicon on their corresponding runners. All four package builds passed. The macOS jobs additionally start the packaged app in background mode with isolated local data, confirm it remains alive and creates its SQLite database, then terminate their own test process. The [final tagged release workflow](https://github.com/eshin087/gcdUsage/actions/runs/34165888175) passed all 45 Rust tests and seven frontend tests on each architecture. macOS background RSS was about 31.7 MiB on Intel and 70.0 MiB on Apple Silicon in CI, with no provider accounts or user history configured.

Compilation and background launch do not establish interactive menu-bar behavior, macOS Keychain access, or first-launch approval on a personal Mac.

The [private release](https://github.com/eshin087/gcdUsage/releases/tag/v0.1.0) contains four installers and SHA-256 checksums. Uploaded asset sizes and digests were verified: Windows x64 2.57 MB, Windows ARM64 2.30 MB, macOS Intel 3.40 MB, and macOS Apple Silicon 3.11 MB.

## Repeatable checks

```sh
npm ci
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
cargo run --manifest-path src-tauri/Cargo.toml --features diagnostics --bin diagnostics -- --providers
cargo run --manifest-path src-tauri/Cargo.toml --features diagnostics --bin diagnostics -- --import --data-dir /absolute/local/test-directory
```

Diagnostics print only connection status and aggregate counts, never access tokens or prompt text. The diagnostic executable is feature-gated and excluded from end-user packages. Use a fresh local directory for a full-import benchmark and repeat it for the incremental benchmark.

For the installed Windows WebView smoke test, launch with process-scoped `GCD_USAGE_DATA_DIR` pointing to an isolated local directory, `GCD_USAGE_TEST_MODE=1`, and `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223`. Then run `node scripts/native-smoke.cjs` with Playwright available. Set `GCD_QA_EXPECT_HISTORY=1` to wait for a populated import. Test mode prevents startup-registration changes. Screenshots can contain private prompt previews and must stay local. Do not use the remote-debugging argument for normal operation.

## Remaining target-device checks

- Inspect native meters in both appearances and at common DPI settings; move the Windows strip between monitors, disconnect a display, and restart.
- Verify sleep/wake, offline recovery, reset boundaries, and stale-state readability during actual device use. Unit tests cover calculation boundaries and provider failures.
- On a personal Mac, complete Gatekeeper approval, Keychain discovery, menu-bar interaction, launch-at-login, and a normal install/upgrade/uninstall.
- Use two physical computers and the same existing synced folder, including offline work and delayed delivery. Automated tests cover duplicate/delayed batches, account separation, and copied histories; real sync-client timing remains a device check.

The display uses a native Windows strip above the taskbar. It does not embed a custom toolbar inside Windows 11's taskbar. Native macOS text lives in the menu bar. No shell replacement or injection is installed.
