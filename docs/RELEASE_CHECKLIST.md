# Manual validation and delivery
Select checks for the requested change rather than repeating every historical check. Follow current user authorization and AGENTS.md. GitHub is file hosting only.

## Establish state
Compare package/lock versions with Cargo/lock and Tauri configuration. Distinguish source, tested package, installed binary and remote commit. A version string alone does not establish that the running app was updated.

Read current build authorization. scripts/windows.ps1 -Action check invokes Cargo and is a native compile. Prior one-time approval is not permission for the next update.

## Verify affected behavior
- Frontend: npm run check, npm test, production bundle, screenshots and interaction checks for changed themes/layouts/large text.
- Native, once authorized: CARGO_BUILD_JOBS=2 and cargo test --manifest-path src-tauri/Cargo.toml --lib --locked. Avoid overlapping native jobs.
- Accounting: populated fixtures, independent SQLite totals, boundaries, zero/unknown, ownership, duplicates, resets and gaps.
- Native UI: actual render review plus pointer/keyboard checks where possible. Geometry tests do not prove tray interaction or monitor transitions.
- Auth: isolated credentials and fake helpers for logout, missing refresh credentials, timeout and competing renewal. Parsing alone does not prove provider renewal.
- IPC/navigation: focused command-denial and external-document checks when their boundaries change.

Inspect scripts/native-smoke.cjs, dock-smoke.cjs, interval-smoke.cjs, security-smoke.cjs and display-smoke.cjs before use. Wait for startup and completed populated caches; an async browser predicate can falsely pass.

## Package and install
Build Windows with node node_modules/@tauri-apps/cli/tauri.js build --bundles nsis only on an authorized suitable host. Build macOS on a suitable Mac. Never substitute GitHub Actions or paid build services.

Use test mode and an isolated GCD_USAGE_DATA_DIR. Never mutate real history or credentials through a test. Preserve a local rollback copy before an authorized installed update; start background helpers with hidden windows.

Verify installer completion, installed version, settings/history preservation and read-only database integrity. Tauri's NSIS bundle marker changes UNK to NSS: this can explain that exact difference from the pre-bundle executable, not arbitrary binary differences.

Library control tests require common-controls v6 while packaged binaries already have Tauri's manifest. Preserve build.rs's distinction.

## Record and deliver
Measure idle memory/CPU with the dashboard closed separately from helpers/imports when relevant. State actual observation duration and limitations. After manual edits, render and inspect all pages, retaining the six-page limit.

Commit sanitized source/docs only. Check local binaries for personal compiler paths before any upload. Source pushed is not a package published. Verify the remote commit after an authorized push.

Keep compact local artifacts and evidence. Before removing generated directories, resolve each target and confirm workspace containment. Preserve real data, original logs, unrelated files and rollback copies.

Update PROJECT_STATUS.md and VALIDATION.md with actual checks, installation, publication and remaining limits.
