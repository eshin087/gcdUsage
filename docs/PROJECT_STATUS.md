# Current status
Reconciled September 11, 2026 after the authorized 0.5.4 update. Verify live state before future delivery.

| Layer | State |
| --- | --- |
| Source | 0.5.4 on main; final app source matches the validated Windows package |
| Windows package | Local Windows x64 release and NSIS installer; two Cargo workers |
| Installed app | 0.5.4 verified, exact tested executable except Tauri's three-byte bundle marker |
| Native validation | 99 Rust tests passed; one standalone child fixture exercised by its parent test |
| Frontend | 17 tests passed; zero Svelte/TypeScript errors/warnings; production bundle passed |
| Runtime | Populated native UI/accounting/security checks; corners, tray restoration and hidden restart passed |
| GitHub delivery | Source and sanitized documentation only; local installer retained privately; Actions disabled |

## Current update
0.5.4 includes corner resizing, persistent Hide/Show and tray restore, shared native themes, reddish reset labels and times, Claude Code renewal, usage insights and allowance forecasting. Recent observed percentage consumption appears on the dock, Overview cards and Model advice with the same adjustable dock duration. Unknown and partial coverage remain explicit.

The approved D1/O1/H2/T2 design remains in place. The extra usage row uses a 144-pixel logical dock height to retain padding. The six-page manual and native/dashboard screenshots were visually reviewed.

The security audit tightened helper environments, token validation, deadlines, process cleanup and account-change checks. A slow link-reconciliation query now uses scoped indexed lookups. The actual native diagnostic fell from 11.317 seconds to under a millisecond on the same fixture. See SECURITY_REVIEW.md and VALIDATION.md.

## Resource and installation evidence
Over 180 seconds with 5,036 synthetic prompts, the dashboard closed and the dock hidden, the final app consumed 0.422 CPU seconds (0.0073% of this 32-logical-CPU host). The sampled peak working set was 41.0 MiB and peak private memory 13.3 MiB; private memory ended at 11.6 MiB. GDI/USER counts stayed at 12/32, handles did not grow, and no child processes remained. Three final-package dashboard cycles also released all WebView children. Ten earlier cycles passed before the lookup-only rebuild. These are bounded synthetic observations, not peak guarantees or proof against every leak.

The 3,375,367-byte installer exited successfully. Before normal startup, settings bytes and history counts were unchanged, the real database passed read-only quick_check, and installed executable metadata and correspondence passed. A rollback copy was retained and the installed app was started normally. Claude and Codex returned fresh connected readings. No matching LSASS/RPCRT4 Application Error was observed during this update.

## Remaining limits
- Native window-message checks covered all four corners at 96 DPI; physical Explorer tray interaction, end-to-end wheel/unit-menu keyboard behavior and mixed-DPI monitor transitions still need manual device checks.
- No real-account OAuth renewal round trip was forced. Synthetic helper tests do not prove permanent provider authentication; logout, revocation or unusable refresh credentials can still require sign-in.
- macOS and Windows ARM64 were not built for this update.
- Dependency maintenance notices remain documented. Local-account/admin compromise and a compromised helper remain outside the app's guarantees.
- The host LSASS/RPCRT4 issue is not considered resolved. September 11 authorization covers this update, not future native builds.
- Forecasts and recent consumption require comparable readings. Do not substitute token totals for allowance percentages.

Use docs/README.md for architecture, data contracts, decisions and release checks. Reconcile installed, source and remote state again when resuming.
