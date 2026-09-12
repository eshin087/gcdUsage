# Project history and lessons

## User preferences established in this project

- Keep the app lightweight, simple to install, and comfortable to use across computers.
- Default to black, percentage left, readable larger text and an unlocked Windows dock. Avoid anchor/grip symbols.
- A futuristic original GCD identity is preferred; do not imitate another company's logo.
- Allow token intervals directly from the dock. Default to the last hour, with presets and a custom duration.
- Show readable project/chat/prompt context. Open the original conversation when supported, with captured details as the fallback.
- Use the original GitHub repository and keep public README/manual content sanitized. Do not publish credentials, local paths, raw histories or private QA screenshots.
- Keep documentation concise; the user manual should be five or six pages maximum.
- Ask focused questions where the answer affects behavior, and continue independent authorized work while waiting.

## Constraints and product decisions

Browser extensions and desktop/sync-client installation may not be permitted on a work computer. A browser-only device is not automatically observed. Explicit provider exports are the available ingestion route where permitted; exact browser token consumption remains unknown. Google Drive is usable only when an approved mechanism actually delivers the corresponding local shared folder. App-managed cloud sync is not implemented.

Windows taskbar embedding was not adopted. The supported display is a movable native dock; macOS uses menu-bar text. Do not describe the dock as being inside the Windows taskbar.

## Releases

- 0.2: browser export import, in-app sign-in progress and device delivery reports.
- 0.3: rounded native dock, latest-ten hover history, editable activity interval and preview toggle. Interactive hover behavior was confirmed by the user.
- 0.4: Fable scoped weekly meter, original GCD logo, larger futuristic dashboard, cleaned/contextual history, conversation/details navigation, native dock duration picker, security fixes and the concise manual. See validation notes for actual completed checks.

## Lessons for implementation

1. Provider response schemas evolve. The labeled `limits` array exposes Fable even when legacy per-model fields do not. Use explicit labels and nullable values, not guessed mappings from opaque names.
2. Save cleaned previews before truncation. A truncated internal wrapper can hide the actual answer; recover it from original local logs without replaying usage or changing identities.
3. Release each cache lock before acquiring another. Struct-expression temporaries and read-to-write transitions can extend lock lifetimes and stall a refresh.
4. Native smoke tests must wait for non-null cache timestamps and the requested interval. Empty totals alone are not a successful populated-history test.
5. Source folders can grow by gigabytes while Rust dependencies and caches are present. Installer size, idle memory and temporary build size are separate measurements. Cleanup after validated delivery keeps the working folder small.
6. Do not attribute a host restart to the app or build without system evidence. Avoid restart commands, global toolchain changes, and excessive parallel build load.
7. Some automated computer-control environments fail before interaction begins. Report that limitation, use actual native drawing tests and app-specific smoke tests, and ask for a focused manual check rather than claiming mouse behavior was tested.
8. An authorization denial is not a reason to bypass a restriction. A previous duplicate-repository deletion was blocked despite confirmation; do not retry through a different mechanism.
9. Dependency audits need target-aware triage. Linux-only advisories are not Windows/macOS shipping code; unmaintained dependencies still deserve disclosure and upstream tracking.


10. Internal browser context wrappers can carry attributes and precede actual user text. Strip recognized leading context blocks before truncation, retain context-only records as background activity, and key preview repair by timestamp as well as preview so repeated headers cannot mix prompts.
11. Publish CI-built installers. Symbol stripping does not remove every embedded compiler source path from local binaries.

This file records project-specific preferences and lessons, not private conversation transcripts or machine identifiers.

## 0.5 preferences and boundaries

The user prefers a single-color lowercase gcd identity generated with the image tool, Matrix-style static black/green surfaces, compact Overview cards and single-line history for skimming. The user confirmed Browser ChatGPT Pro on the $200 tier. Public caps are reference data, not observed request counts. Browser work cannot be counted from local Codex activity. Remove manual browser uploads; retain prior data without adding manual workflows. Remaining-quota deltas measure interval consumption; per-prompt attribution still requires isolated snapshots and stays explicitly estimated.

The user rejected the first 0.5 logo as blurry and thick/wobbly, and wanted thinner terminal typography, more embedded ASCII art, and a slimmer dock with independent size controls. Avoid heavy bold lettering and rounded dock cards. Use the pixel-style generated mark, locally bundled light/regular terminal fonts in the dashboard, regular native monospace text and static ASCII instrumentation. Dock scaling is separate from dashboard/hover font size.

The user approved design 01 / Site terminal from the September design review: their website uses JetBrains Mono and cream-on-charcoal, not bright Matrix green. Use quiet slash navigation, unboxed meter columns, grouped settings, restrained ASCII, and a larger dock. A design approval is required before changing to a different visual direction. The user explicitly authorized one local 0.5.2 compile; it passed with two workers and no new LSASS fault. Future native builds require fresh explicit authorization while the restart issue remains unresolved. GitHub Actions is prohibited.

The user authorizes zero GitHub spending and wants file hosting only. Actions is disabled, workflows removed, and generated remote artifacts/caches cleared. Do not ask for billing increases or reintroduce paid services. See docs/GITHUB_POLICY.md.

## September 10 design selections (source preview)
The user selected D1 quiet-divider dock, O1 quiet-divider allowance overview, and H2 inset single-line hover history. Keep a compact model column on each history row and load older records automatically while scrolling. Remove only the dock token model subtitle; lifetime row accounting remains unchanged. The user also selected T2: a themed native duration dialog with presets and a custom whole-number value plus unit selector. These source changes have not been natively built or installed.

## September 10 installed 0.5.3 update
The user explicitly authorized proceeding with the installed app after approving D1/O1/H2/T2. The local Windows x64 release was built, tested with synthetic data and installed as 0.5.3. All 85 Rust and 13 frontend tests pass. Native control tests require their own common-controls v6 manifest; avoid duplicating Tauri's manifest in packaged binaries. The installed executable differs from the pre-bundle build only by Tauri's UNK-to-NSS marker. No new LSASS application fault was observed. This authorization covers this update, not future native builds; keep GitHub Actions disabled. See docs/VALIDATION.md for the remaining interactive checks.

## September 11 controls and insights
The user chose usage forecasting and persistent hidden state. Windows dock corners resize proportionally within 80-160% and monitor bounds, preserving the opposite corner. Persist the fitted size so release does not reposition the dock. Hide is restored only explicitly, including after restart; tray double-click and Show dock restore it. Save native control changes without discarding unrelated dashboard edits.

Claude access tokens can expire while a refresh token remains valid. Renew through the installed helper's documented non-interactive path and provider-owned credential store; do not create a second credential store. Honor missing credentials and manual logout, serialize local refresh attempts, and keep secret values out of logs and arguments.

Model advice adds local activity insights and a forecast from actual provider percentage changes. Forecasts require sufficient continuous readings from the same account/device/window. Never derive allowance exhaustion from token counts. This source update does not authorize a new native build.

The user also selected recent consumption directly on the dock, with the same duration as dock tokens. Keep the value on dashboard cards and in Model advice too. Preserve padding by extending the dock height to 144 logical pixels and share its geometry constants between sizing, resizing and paint. Recent consumption is observed percentage-point change, with partial coverage for resets/gaps; it is separate from forecasting and request token totals. Project references now have a maintained index in docs/README.md.


## September 11 security and resource review
The user explicitly authorized the local 0.5.4 build, installation and GitHub source update. Reset labels and times use muted red, with darker red on the light theme. Renewal helpers receive a narrow environment, bounded tokens, fixed arguments and process-tree cleanup. Do not expose secrets in command arguments or audit evidence; provider-owned storage and compromised local accounts remain trust boundaries.

Native control automation must settle queued window messages and verify the actual pointer/capture. User mouse movement can interrupt a synthetic drag; record the trace and repeat during an agreed brief input-free window. Four-corner and tray checks passed under that condition. Closing dashboard WebViews and native resource counts require runtime observation; source review cannot prove universal freedom from leaks.
