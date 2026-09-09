# Validation

Version 0.4.1 addresses the user-reported browser-context leak. Leading recognized context blocks with attributes are removed before preview extraction; context-only historical records are retained as background activity. Timestamp-scoped enrichment prevents repeated headers from mixing distinct prompts. All 82 Rust tests pass, including repeated-prefix repair and preservation of ordinary user code.


The 0.4.1 native checks passed with browser-context fixtures, and the user confirmed that the reported history issue was fixed. Windows x64/ARM64 and macOS Intel/Apple Silicon test and package-build steps passed; both Mac background-launch checks passed. The CI-built Windows x64 installer was installed and its local database integrity check passed. The release binary was checked for absence of this developer machine's profile path.

The Windows installer is approximately 2.7 MB and its executable approximately 6.8 MB. Background observations with the dashboard closed were approximately 35–47 MB; helpers and initial history repair are separate peaks. Temporary local build tools, dependency folders and test fixtures were removed, then the installed app was checked again. Some older captured previews remain unavailable; actual user code is not removed just because it resembles markup.

Version 0.4.0 adds the Fable scoped weekly allowance, contextual prompt details, direct native dock duration controls, a new GCD identity and a larger dashboard. Local checks pass 80 Rust tests, 13 frontend tests, and Svelte/TypeScript validation. The live Claude response was checked for the explicitly labeled Fable window; missing general limits are never substituted with that scoped allowance.

Synthetic native tests cover 36 prompts across multiple projects, wrapper cleanup, captured-details navigation, rejected invalid identifiers, and the 18 px default dashboard font. History filters fit the default window without horizontal overflow. Actual native drawing tests cover five dock cards, ten-row context, unavailable values, privacy mode, restricted screen space and several font scales. Screenshots use synthetic data only.

The Windows x64 0.4.0 installer built successfully at approximately 2.7 MB. Native dock totals matched independent SQLite calculations across seven interval changes; invalid durations were rejected, preview settings persisted, and closing the dashboard removed its webview. The final executable passed context/detail regression and command, external-document navigation, and popup isolation checks. A speculative request may occur before native navigation cancellation; this is document isolation, not an outbound firewall.

The six-page PDF manual was rendered and visually reviewed on every page. The dependency review is documented in [Security review](SECURITY_REVIEW.md), including remaining upstream maintenance notices and the unsupported Linux-only advisory. The npm audit reports no known advisories.

Interactive checks of the new native duration menu, clickable hover rows, physical mixed-DPI monitors and macOS sign-in remain device checks; automated drawing and command tests do not substitute for those interactions. Historical release results below are not claims that the new version has passed the same platform checks.
Version 0.3.0 adds the Windows native activity dock and prompt hover cards, with a one-hour default interval and configurable duration. Local checks pass 74 Rust tests, 12 frontend tests, and Svelte/TypeScript validation. Native rendering tests cover multiple font scales, ten-row history, restricted screen space, hidden previews, and unavailable data. Geometry tests cover hover delay, pointer travel into the popup, scrolling layout, drag thresholds, and monitor bounds.

Version 0.3.0 platform tests and package builds completed for Windows x64/ARM64 and macOS Intel/Apple Silicon; both macOS background-launch checks passed. The release contains all four packages and verified SHA-256 checksums. The installed Windows app used approximately 39–45 MB with its dashboard closed, and its history database passed an integrity check after cleanup.

The Windows 0.3.0 dock smoke test passed with populated history: totals matched independent SQLite arithmetic across seven interval changes, invalid durations were rejected, and preview settings persisted. Native dashboard regression, permission, navigation, and popup checks passed; both providers connected successfully. The Windows x64 installer is approximately 2.7 MB. The user confirmed that the hover panel appears above the dock and stays open when moving into it. Live pointer automation was unavailable; drag behavior and mixed-DPI transitions still require an interactive device check. The rendering tests exercise the actual native drawing code with synthetic data.

Version 0.2.0 adds browser-export import, in-app sign-in progress, and device delivery reports for shared history.

Local validation covers 67 Rust tests, 12 frontend tests, and Svelte/TypeScript checks. Coverage includes duplicate exports, missing metadata, bounded inputs, account separation, date boundaries, delayed sync delivery, safe exports, and dashboard permissions.

Windows 0.2.0 native checks passed for existing provider connections, browser-history search and pagination, interval filtering, HTML escaping, sync receipts, settings persistence, and dashboard closure. A harmless helper fixture verified hidden execution, cancellation, and verification-code submission. Organization SSO requires an eligible account and was not exercised end to end.

Native interval totals matched independent SQLite calculations across six time ranges. Font sizes at 90%, 120%, and 160% passed. The Windows x64 executable was approximately 6.6 MB, and its background process used approximately 40 MB in the local check with the dashboard closed. Temporary helpers and import peaks are separate from that reading.

Platform packages are built for Windows x64/ARM64 and macOS Intel/Apple Silicon. macOS packages also receive a background-launch smoke test. Interactive macOS sign-in and physical mixed-DPI monitor behavior require checks on the target device.

All four platform jobs passed for version 0.2.0. The [preview release](https://github.com/eshin087/gcdUsage/releases/tag/v0.2.0) includes packages of approximately 2.4–3.5 MB and SHA-256 checksums. The installed Windows release was checked again after generated development files were removed; its local database remained healthy and both provider connections returned current readings.

Browser-import counts describe exported messages. Exact historical token consumption, hidden reasoning, and original-device attribution are not inferred. Shared-folder receipts describe what each device reported receiving.

Performance depends on history size and temporary import or provider helpers. Packages are personal previews without trusted publisher signing.
