# Validation

Version 0.3.0 adds the Windows native activity dock and prompt hover cards, with a one-hour default interval and configurable duration. Local checks pass 74 Rust tests, 12 frontend tests, and Svelte/TypeScript validation. Native rendering tests cover multiple font scales, ten-row history, restricted screen space, hidden previews, and unavailable data. Geometry tests cover hover delay, pointer travel into the popup, scrolling layout, drag thresholds, and monitor bounds.

The Windows 0.3.0 dock smoke test passed with populated history: totals matched independent SQLite arithmetic across seven interval changes, invalid durations were rejected, and preview settings persisted. Native dashboard regression, permission, navigation, and popup checks passed; both providers connected successfully. The Windows x64 installer is approximately 2.7 MB. Live pointer automation was unavailable in the local test environment; physical hover/drag behavior and mixed-DPI transitions still require an interactive device check. The rendering tests exercise the actual native drawing code with synthetic data.

Version 0.2.0 adds browser-export import, in-app sign-in progress, and device delivery reports for shared history.

Local validation covers 67 Rust tests, 12 frontend tests, and Svelte/TypeScript checks. Coverage includes duplicate exports, missing metadata, bounded inputs, account separation, date boundaries, delayed sync delivery, safe exports, and dashboard permissions.

Windows 0.2.0 native checks passed for existing provider connections, browser-history search and pagination, interval filtering, HTML escaping, sync receipts, settings persistence, and dashboard closure. A harmless helper fixture verified hidden execution, cancellation, and verification-code submission. Organization SSO requires an eligible account and was not exercised end to end.

Native interval totals matched independent SQLite calculations across six time ranges. Font sizes at 90%, 120%, and 160% passed. The Windows x64 executable was approximately 6.6 MB, and its background process used approximately 40 MB in the local check with the dashboard closed. Temporary helpers and import peaks are separate from that reading.

Platform packages are built for Windows x64/ARM64 and macOS Intel/Apple Silicon. macOS packages also receive a background-launch smoke test. Interactive macOS sign-in and physical mixed-DPI monitor behavior require checks on the target device.

All four platform jobs passed for version 0.2.0. The [preview release](https://github.com/eshin087/gcdUsage/releases/tag/v0.2.0) includes packages of approximately 2.4–3.5 MB and SHA-256 checksums. The installed Windows release was checked again after generated development files were removed; its local database remained healthy and both provider connections returned current readings.

Browser-import counts describe exported messages. Exact historical token consumption, hidden reasoning, and original-device attribution are not inferred. Shared-folder receipts describe what each device reported receiving.

Performance depends on history size and temporary import or provider helpers. Packages are personal previews without trusted publisher signing.
