# Validation

See [current status](PROJECT_STATUS.md) for the latest source, installed version and pending delivery. Results below are dated evidence; older CI entries are historical and do not authorize new GitHub builds.

## Historical release: 0.5.0

The Windows installer is approximately 2.8 MiB and its executable 6.6 MiB. With the dashboard closed, this host measured 35.6-37.0 MiB working set and about 0.04% of total host CPU over a 59-second sample. Helper/import peaks are separate; this is one machine and interval, not a universal guarantee.

[Four-platform CI](https://github.com/eshin087/gcdUsage/actions/runs/34321479246) passed: 81 Rust tests on each Windows architecture, 80 on each macOS architecture, 13 frontend tests and Svelte/TypeScript checks. Both macOS background-launch checks passed. Packages use the CI-built binaries; no personal compiler path was found in the Windows executable.

The installed Windows package passed synthetic native checks with 36 prompts: context cleanup, captured details, unavailable browser Pro balances and rejection of retired browser commands. Dock totals matched independent SQLite sums across seven durations; invalid inputs and preview persistence passed. Dashboard command restrictions, external-document isolation and popup rejection passed. No JavaScript errors were observed.

The compact UI was visually reviewed. Layout checks passed at 760, 900, 1000, 1100 and 1280 pixels, with 160% text and alternate themes. The six-page manual was rendered and visually checked on all pages. Post-upgrade database integrity passed, aggregate history counts were preserved, and both provider connections were healthy.

The quota regression covers a poll during an active prompt followed by a post-completion reading, while retaining overlap rejection. Browser Pro caps are dated documentation references, not a verified remaining account balance. Manual import UI, parser and commands are removed; legacy stored records are retained and excluded from dock history. Physical mixed-DPI behavior and interactive macOS use still require device checks.


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

## 0.5.1 Windows validation

The user requested thinner terminal typography, a sharper pixel-style logo, embedded ASCII art and independent dock resizing. Frontend checks pass. All 81 local Windows Rust tests now pass, including native render checks at 80%, 100% and 160% dock scale. Rendered dock images were visually checked. The new four-platform run was blocked before any jobs started by GitHub account billing/spending restrictions. The local Windows x64 installer passed and was installed successfully (3.23 MB). Native checks passed with 36 synthetic prompts: context cleanup, detail navigation, restricted commands/navigation/popups, seven intervals matched against SQLite, independent dock sizing at 80/100/160%, invalid size rejection and persisted settings. No JavaScript errors were reported. The six-page manual was rendered and visually checked. macOS and Windows ARM64 packages remain blocked; previous 0.5.0 CI results do not validate this revision. The local installer is for personal testing and is not published.

## 0.5.2 Site terminal Windows validation

Frontend type/Svelte checks and 13 tests pass. Synthetic browser checks cover responsive History, alternate themes, larger fonts, all Settings sections and unknown Browser Pro counts. After explicit user authorization, a two-worker local Windows x64 build passed 81 Rust tests, including process-private JetBrains Mono registration and native render tests at 80%, 100% and 160% dock scale. The native dock and hover renders were visually reviewed.

The local Windows x64 installer is 3,329,219 bytes. It passed native smoke checks with 36 synthetic prompts: wrapper cleanup, captured details, restricted commands/navigation/popups, seven independently checked token intervals, settings persistence, and dock resizing from 80% to 160%. The exact tested installer was installed as version 0.5.2. The real database passed a read-only integrity check with 5,483 prompts and 17,768 requests, and the background process used about 37 MB. No LSASS application fault was logged during the compile or installation. macOS and Windows ARM64 remain unbuilt.

The earlier four-platform attempt was rejected before any steps executed. GitHub Actions is now disabled by policy. Dashboard Overview, Settings and Model advice screenshots were visually reviewed. The updated six-page manual was rendered and reviewed on every page. The locally built installer remains a personal artifact and is not uploaded to GitHub Releases because local compiler paths may be embedded.

GitHub policy update: Actions and automated builds are now disabled at the user's request. Future native validation requires a suitable manually operated build environment; resolving GitHub billing is no longer a proposed next step. Existing CI entries above are historical evidence only.

## September 10 design source preview (unreleased)
D1 dock, O1 allowance overview, and H2 single-line hover history are applied in source. The hover uses native-only background cursor paging in batches of 40, preserving provider/account-scoped lifetime token totals. The compact model column retains an indication of additional models; full information remains in captured details. T2 is applied in source with themed native buttons and edit controls, a themed unit menu, inline validity feedback and whole-number minute/hour/day conversion bounded to 1-43,200 minutes.

Frontend checks and all 13 tests pass. Synthetic browser regression checks pass, with overview layout checks at 760, 900, 1100, 1280 and 1600 pixels, including 160% text, and visual checks in all four themes. The existing full-page layout still overflows at 600 pixels, below the tested supported range; the new overview itself fits. The six-page source-preview manual is regenerated and all pages visually reviewed. The actual paging and aggregation SQL passes an independent in-memory check using 105 synthetic prompts, covering equal timestamps, provider filters, new imports, cross-account isolation and unknown versus zero tokens; this does not substitute for executing the Rust tests.

Rust paging, cell-boundary and duration-conversion regression tests are added but not executed. All changed Rust files pass a syntax-parser check, and new Win32 symbols were checked against the pinned windows-sys 0.59.0 bindings. These checks do not validate types, linking, native painting or keyboard/mouse behavior. Native compilation, native rendering, scrollbar interaction, mixed-DPI behavior and installation are unverified because this host requires fresh explicit native-build authorization while the LSASS issue remains unresolved. No native build, installation, GitHub action, deployment or publication was performed.

## 0.5.3 local Windows update

The user's instruction to proceed with the installed app explicitly authorized this local native build. Rust 1.98.1 and a two-worker Cargo limit produced the Windows x64 release and NSIS installer. Frontend checks report zero errors/warnings; all 13 frontend tests and 85 Rust tests pass. The added native duration test covers preset values, invalid input and disabled Apply, the 30-day boundary, Cancel/Close, reopening and native rendering. The test harness needs common-controls v6; the build configuration now supplies its manifest while retaining Tauri's packaged manifest.

Native render screenshots were reviewed for the dock, single-line hover rows, hidden previews, 80/100/160% dock scaling and the T2 dialog. The packaged dashboard passed isolated checks with 135 synthetic prompts: search and one-line history, unknown provider values, four themes at 160% text, independent 80/100/160% dock sizes, invalid duration rejection and six token intervals independently matched to SQLite. Native command-denial, external-document isolation and popup checks pass. No dashboard JavaScript errors were observed. The six-page 0.5.3 manual was rendered and all pages visually reviewed.

After the isolated dashboard closed, no child processes remained. Over 57.8 seconds, the main process consumed 0.016 CPU seconds, with 34.6 MiB working set and 9.9 MiB private memory. This is an isolated idle observation, not a peak or a measurement of a large personal history.

The 3,340,622-byte installer completed successfully and the installed app reports 0.5.3. Its executable matches the tested release byte-for-byte except Tauri's documented three-byte UNK-to-NSS bundle marker. Before normal startup, existing settings and history record counts were unchanged; the real database passed read-only quick_check. A local rollback copy was retained. The installed app was launched normally. No LSASS Application Error event was found during this update.

The computer-use helper could not start because of its sandbox setup error. Native message/control tests and render review passed, but end-to-end mouse-wheel paging, unit-menu keyboard interaction and multi-monitor DPI transitions still need manual review. SQLite cursor tests cover 105 records, tied timestamps, new imports, provider filtering and duplicate prevention. macOS and Windows ARM64 were not built. No installer was uploaded, no GitHub build ran, and Actions remains disabled.

## 0.5.4 source and frontend verification - September 11
The requested corner resizing, persistent hide/show, native theme palettes, Claude renewal, usage insights and allowance forecasting are implemented in source. Source version fields agree on 0.5.4; the installed app remains 0.5.3. No native build or installation was started for this update.

Svelte/TypeScript checks report zero errors or warnings, all 15 frontend tests pass, and the production frontend bundle builds. Tests include preserving tray hiding and corner size changes while the dashboard has unrelated unsaved settings. Synthetic browser checks passed at 1600, 1100 and 760 pixels across Black, Light, Slate and Midnight, plus enlarged text. They exercised 7/30/90-day periods, token/prompt chart switching, 168 heatmap cells, model mix, forecasts, unknown totals, stale and learning states, and load errors. No page JavaScript errors were observed. Screenshots were reviewed; the six-page manual was regenerated and every page visually reviewed.

Nine changed Rust files parse without syntax errors. This is not Rust type checking or execution. Native tests were added for opposite-corner anchoring and monitor-fitted persistence, hidden-setting serialization, renewal input validation, forecast rate/reset/account/gap boundaries, and unknown token handling. Native render cases now include all four themes. These Rust tests and renders remain unexecuted until an authorized native build.

Still required: Rust compilation and tests, isolated native hide/tray restoration and restart checks, corner pointer/DPI behavior, shared theme rendering, and a controlled helper renewal test. Actual provider renewal has not been exercised, and tests must not mutate real credentials or history. macOS remains unbuilt. No release package, GitHub Action or paid service was used.


## September 11 recent consumption and project continuity extension
Recent allowance consumption now shares the dock token duration and is presented in Overview, Model advice and the native dock source. The calculation is scoped to the current provider/account/device/window, sums only comparable observed percentage-point changes, distinguishes zero from unavailable, and labels incomplete coverage. The dock's common geometry is now 1120 by 144 logical pixels to retain padding for its new row.

The final frontend checks report zero Svelte/TypeScript errors or warnings; all 17 tests pass and the production bundle builds. Synthetic browser interaction checks passed for 30-minute, 1-hour, 90-minute custom, 1-day and 1-minute durations, persistence between Overview and Model advice, partial coverage, unavailable values and invalid custom input. All four themes fit at 1600, 1100 and 760 pixels; 160% text also passed. No page JavaScript errors were observed. Overview and Model advice screenshots were visually reviewed.

Recent-usage Rust regressions are prepared for interval edges, duplicate readings, account/device isolation, zero, resets, gaps and duration bounds. Changed Rust sources pass syntax parsing only. The new native row, its four-theme render fixtures and the 144-pixel resize geometry have not been compiled, rendered or exercised in the app. These remain part of the pending authorized native checks.

The local gcd-usage-resume, gcd-usage-accounting and gcd-usage-verify-release skills passed the skill creator's validator. The project reference index, status, architecture, data contracts, decisions and release checklist were added; relative documentation links resolve. The updated six-page manual was rendered and every page visually reviewed.

Installed executable metadata and the Git remote were checked on September 11: the installed app is 0.5.3 and main still points to the validated 0.5.3 source commit 2e0f6f5. No 0.5.4 native build, installation, source push or package publication has occurred. Fresh native-build authorization remains pending under AGENTS.md.


## 0.5.4 authorized Windows update - September 11, 2026
This entry supersedes the source-only and pending-build checkpoints earlier on this date. The user authorized the local build, installation, security/performance audit and GitHub source update. Native work used two Cargo workers. Final Windows x64 packaging passed; its optimized compilation took approximately 2 minutes 4 seconds. All 99 Rust tests and 17 frontend tests pass, with zero Svelte/TypeScript errors or warnings and a successful production bundle. One standalone child fixture is intentionally ignored by the normal runner and exercised by its parent helper tests.

Native window-message checks at 96 DPI passed for all four corners, opposite-corner anchoring, persisted scale/position, Hide, tray double-click and Show dock. Hidden state survived a process restart and restored from the tray. Earlier attempts overlapped live mouse input; the recorded trace exposed that interference and the agreed input-free repeat passed. Physical Explorer input and mixed-DPI transitions remain unverified.

The final rebuilt binary passed populated dashboard tests with 5,036 synthetic prompts: one-line history/search, four themes at enlarged text, independent dock sizes, unknown provider values, six token intervals independently matched to SQLite, 7/30/90-day insights, recent-duration bounds and shared-duration persistence. The 30-minute interval had legitimately become empty; longer intervals remained populated. No JavaScript errors were observed. Native command-denial, external-document isolation and popup checks passed again on the final binary.

One hundred hidden native duration-dialog create/change/close cycles left GDI and USER counts unchanged at 13 and 5. Actual native render images were reviewed for the four palettes, padding, recent-consumption row and reddish reset text; dashboard layouts and all six manual pages were also reviewed.

The audit hardened Claude renewal through a narrow child environment, bounded tokens, fixed arguments, sanitized errors, TLS verification, deadlines, process-tree cleanup and rejection of an in-flight account change. Synthetic helper tests passed. No real-account renewal round trip was forced; revocation, missing refresh credentials and compromised local environments remain limitations. Dependency review and upstream maintenance notices are recorded in SECURITY_REVIEW.md.

Performance work clones only visible hover rows, coalesces insights requests and uses scoped indexes. Native diagnostics identified an expensive link-query plan even with zero pending links. The new query starts from link hints and preserves provider/account/turn/time matching. A regression with 5,000 unrelated prompt/request pairs checks bounded SQLite work, latest eligible attribution, excluded identities and unchanged-repeat behavior. The same fixture's link lookup decreased from 11.317 seconds to 0.000097 seconds; unchanged import repetition fell from 9.38 seconds to 0.000851 seconds. Temporary diagnostic code was removed before packaging.

Over 180 seconds with 5,036 synthetic prompts, the dashboard closed and the dock hidden, the final app consumed 0.422 CPU seconds (0.0073% of this 32-logical-CPU host). The sampled peak working set was 41.0 MiB and peak private memory 13.3 MiB; private memory ended at 11.6 MiB. GDI/USER counts stayed at 12/32, handles did not grow, and no child processes remained. Three final-package dashboard cycles also released all WebView children. Ten earlier cycles passed before the lookup-only rebuild. These are bounded synthetic observations, not peak guarantees or proof against every leak.

The 3,375,367-byte NSIS installer exited with code 0 and installed version 0.5.4. Its executable matches the tested release except exactly Tauri's UNK-to-NSS three-byte marker. Before normal startup, settings bytes and history counts were unchanged and the real database passed read-only quick_check. A rollback copy was retained. The installed app was launched normally without test mode or debugging overrides.

The local installer remains private; GitHub delivery contains source and sanitized documentation only. Actions stays disabled and no hosted build or paid service was used. macOS and Windows ARM64 remain unbuilt. These checks do not establish absence of every vulnerability or memory leak.

The normal installed process returned fresh connected readings for both Claude and Codex. This confirms saved-sign-in use after the update, not a forced OAuth refresh round trip. No matching LSASS/RPCRT4 Application Error was found from the start of this update through the final installation check. The underlying host issue remains unresolved.
