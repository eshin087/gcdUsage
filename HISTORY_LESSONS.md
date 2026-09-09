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

This file records project-specific preferences and lessons, not private conversation transcripts or machine identifiers.

10. Internal browser context wrappers can carry attributes and precede actual user text. Strip recognized leading context blocks before truncation, retain context-only records as background activity, and key preview repair by timestamp as well as preview so repeated headers cannot mix prompts.
