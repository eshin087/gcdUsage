# GCD Usage: project guidance

Read this file before changing the app. Read `HISTORY_LESSONS.md`, `docs/VALIDATION.md`, and `docs/SECURITY_REVIEW.md` for decisions and verification limits. User instructions take precedence over this guidance.

## Product and architecture

GCD Usage is a per-user Windows/macOS desktop companion for provider allowance meters, local coding-tool prompt/request history, optional shared-folder sync, and local model advice. It is not an automatic browser collector or a hosted cloud service.

- Tauri 2 + Rust + SQLite own collection, validation, accounting and OS integration.
- Svelte/TypeScript owns an on-demand dashboard. Destroy its webview on close.
- Windows uses native GDI dock/hover windows and a native duration picker; no resident webview. macOS uses tray/menu-bar text.
- `src-tauri/src/providers.rs`: bounded provider/helper connections and quota parsing.
- `history.rs`, `enrichment.rs`, `presentation.rs`: streaming import, optional context enrichment, plain-text cleanup. Enrichment must not replay token usage or reassign accounts.
- `storage.rs`, `metrics.rs`, `dock_storage.rs`, `details.rs`: persistence and aggregation. Keep measured tokens separate from quota estimates.
- `sync.rs`: immutable event batches and device receipts. Local database files and credentials never sync.
- `navigation.rs`: validated legacy conversation links and one fixed Pro documentation destination; coding logs use captured details.
- `platform/`: native display, geometry, pointer state and duration UI.
- `public/gcd-logo.png` is the generated monochrome lowercase GCD mark. `scripts/create-icons.mjs` generates all packaged icons and raw tray pixels from it.

## Invariants

- Unknown is null/unavailable, never a fabricated zero. Reasoning overlaps output and must not be added twice.
- Group tool loops/subagents only when the logs establish attribution. Preserve model changes and unknown historical reasoning.
- Deduplicate requests using stable source identity. Prevent cross-provider/account replacement and linking.
- Time intervals are inclusive at the start, exclusive at the end. The hover list is latest-ten history; its row tokens are lifetime totals, independent of the selected rolling interval.
- Fable uses the provider's labeled scoped weekly limit; never relabel the general five-hour or weekly window as Fable.
- Keep previews bounded, render them as text, and strip only recognized application wrappers. Never render log text as HTML or execute it.
- Never hold multiple cache guards across callbacks or native painting. Clone each cache in its own statement and release its lock.
- New Tauri commands require explicit build-manifest and dashboard capability entries. Do not grant generic filesystem, shell or unrestricted URL access.
- Logs/exports/sync/public docs must exclude credentials and source-device paths. Project basenames and chat titles are optional sensitive history metadata.

## Development and validation

Use Node 22.12+ and current Rust stable. `npm ci`, `npm run check`, `npm test`, and `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked` are baseline checks. Build Windows with `node node_modules/@tauri-apps/cli/tauri.js build --bundles nsis`. Build macOS on a Mac or the existing four-architecture GitHub workflow.

Use an isolated data directory for native smoke tests; never run tests that mutate real history or sign-in state. `scripts/native-smoke.cjs`, `dock-smoke.cjs`, `security-smoke.cjs`, and `interval-smoke.cjs` contain focused checks. Wait for native startup and completed cache refresh; an async browser predicate alone can produce false positives. Compare totals with independent SQLite calculations and require nonempty fixtures.

Keep builds to two Cargo workers on the local host. Measure idle memory/CPU with the dashboard closed separately from helpers/imports. Visual changes require screenshots or native render tests plus an interactive check where available. No test suite proves absence of all vulnerabilities; record remaining limitations.

## Delivery

Use the existing `gcdUsage` repository. Do not create a replacement repo or rewrite remote history unless explicitly requested. Commit source and sanitized documentation only; generated tools, caches, private fixtures and installer intermediates remain ignored. Update version fields consistently in package/lock files and Tauri/Cargo configuration.

Maintain the short manual at `output/pdf/GCD-Usage-Manual.pdf` using `scripts/build-manual.py`. Keep it at six pages or fewer, visually review all pages, and link it near the top of README. Keep security reports high-level and credential-free.

After QA, preserve compact release artifacts and remove only verified generated workspace folders. Never delete real user data, original provider logs, or unrelated files. Check absolute cleanup targets. Document what was installed, measured, published and left unverified.

Publish CI-built packages. Local binaries may retain personal compiler source paths even when debug symbols are stripped; do not upload locally built installers without checking that boundary.

## 0.5 product decisions

Use static Matrix green/black terminal styling and condensed one-line dashboard history. Preserve alternate themes and the larger font setting. Browser imports are retired: no parser, UI or IPC command. Legacy record types/tables remain solely for retention and sync compatibility. Do not restore manual uploads. The Pro $200 browser panel lists dated public caps; remaining requests stay unavailable until a genuine account counter exists. Never infer browser allowance from Codex logs or substitute a cap for remaining balance.

Dock size uses the local `dockScale` setting (80-160%, default 100%) independently from dashboard/hover `fontScale`. Keep native dock paint, cell hit testing and sizing on the same effective DPI/size scale, with monitor fitting. The slim base geometry is 800 by 56 logical pixels. Keep the generated pixel-style logo, bundled light/regular IBM Plex Mono with its OFL notice, native regular monospace drawing and static ASCII graphics.

Small icon frames use an optical g crop from the generated mark to preserve legibility. The crop in `scripts/create-icons.mjs` is tied to the current raster asset; review 16/32-pixel output whenever replacing the logo. Do not smooth thin letters into unreadable small icons.
