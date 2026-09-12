# GCD Usage

**[Read the six-page user manual](output/pdf/GCD-Usage-Manual.pdf)** - setup, token accounting, history, synchronization and limitations.

[Project status and maintainer references](docs/README.md).

A lightweight desktop companion with usage meters, searchable prompt history, shared-folder synchronization, and local model advice.

## Current local build: 0.5.4
Version 0.5.4 is installed locally on Windows x64. It adds resizing from all four dock corners, persistent Hide/Show with tray restoration, matching native themes and reddish reset labels, recent percentage consumption over the shared adjustable dock duration, Claude Code sign-in renewal, usage charts and allowance forecasts. The approved D1/O1/H2/T2 design remains in place.

The authorized local build passed 99 Rust tests and 17 frontend tests, plus isolated native and dashboard checks. See [validation notes](docs/VALIDATION.md) and the [security review](docs/SECURITY_REVIEW.md) for measurements and remaining limits. GitHub contains the source and documentation; the local installer is not uploaded. Existing release downloads are older versions.

## Display

- **Windows 11:** a readable native terminal dock with JetBrains Mono text. Drag the middle of the dock to move it or an unlocked corner to resize it. Click Hide to hide it until explicitly restored, including after a restart. Double-click the tray icon or right-click it and choose Show dock; the tray menu also opens the dashboard. Hover a meter to browse recorded prompts above the dock; older entries load automatically while scrolling. It does not modify the Windows taskbar.
- **macOS:** compact menu-bar usage meters.
- **Dashboard:** Overview, History, Model advice, and Settings. Closing the dashboard releases its browser while the native meters keep running.

The default is a cream-on-charcoal Site terminal theme with regular JetBrains Mono typography and static ASCII graphics, percentage **left**, and 120% text size. Settings offers five themes, percentage used instead, and text sizes from 90% to 160%. Dock size is independent: use the 80%-160% slider in Settings or right-click the dock for size presets. Dashboard and hover text size remain separate. Saving a theme applies its palette to the Windows dock, hover history and duration picker. Native macOS menu-bar appearance follows macOS. Stale readings are marked; unavailable readings show an em dash and missing reset times show N/A.

Settings groups appearance, connections, sync, and history/advice into separate sections. The Windows dock uses 1120 by 144 logical pixels before DPI scaling in 0.5.4, adding room for recent consumption, with more inner padding, green usage values and fully spelled-out reddish reset labels.

Each allowance card also shows observed recent percentage consumption. A change from 91% to 87% left is 4 percentage points used. Overview, Model advice and the dock share the adjustable token duration, including custom values from 1 minute to 30 days. Resets and gaps mark the observed amount as partial; unavailable readings stay unknown.

The Windows activity card shows logged tokens for the last hour by default, without a model subtitle. Click it, or right-click the dock, to choose a preset or a themed custom duration. The custom picker offers 1-hour, 6-hour, 24-hour and 7-day presets plus a whole-number value in minutes, hours or days, bounded to 1 minute through 30 days. Claude also has a Fable weekly meter when its scoped allowance is available. All Claude meters share the latest Claude history.

Hover history uses one line per prompt: project, cleaned preview, time, a compact model column and lifetime logged tokens. Additional model changes are indicated by a count; full model/reasoning and chat context remain in captured details. Click a row to open captured details. Coding logs without a verified link retain conversation/turn identifiers. Totals cover local and synchronized coding-tool activity. You can hide hover previews in Settings. History reads are paged in the background and are independent of the selected token interval.

## Install and set up

Use a manually validated package for your platform. Existing [Releases](https://github.com/eshin087/gcdUsage/releases) contain older Windows/macOS packages, not the local 0.5.4 installer. Windows installs per user. On macOS, drag the app into Applications.

These are personal preview builds without trusted publisher signing. Your operating system may require first-launch approval. Updates are manual.

Have Claude Code and Codex installed, open GCD Usage, and check the connection status. Existing sign-ins are checked first. A current Claude Code helper renews expired access from its saved subscription refresh token and persists it in Claude Code's credential store. GCD Usage does not copy credentials into its database or sync. Signing out, revocation or missing renewable credentials still requires sign-in; a network failure retries without launching a browser. See [Claude Code's refresh environment variables](https://code.claude.com/docs/en/env-vars). When sign-in is needed, follow the provider's browser flow and the progress shown inside GCD Usage. Organization SSO is available for supported Claude accounts. Setup offers launch-at-login and an optional existing shared folder for combined history across computers.

## Explore usage

Choose last 30 minutes, 1 hour, 6 hours, 24 hours, 7 days, this week, 30 days, this month, all time, or a custom date/time interval. Calendar weeks start Monday in local time. Custom intervals include the start and exclude the end.

Metrics include measured tokens, cache/input/output/reasoning breakdown, prompts, conversations, requests, median/p75 consumption, an activity chart, and model/reasoning breakdowns. History supports search, filters, and CSV export. Usage recorded during an interval can include ongoing prompts started earlier.

Coding-tool history covers local Claude Code and Codex activity. The dashboard uses condensed, single-line prompt rows and compact allowance cards. Manual browser imports have been removed. Ordinary browser conversations and tokens are not collected, including activity on computers without an installed collector. Previously stored browser records are retained for compatibility and excluded from the dock.

### Browser ChatGPT Pro limits

The dashboard includes a **Pro $200 reference**, checked September 8, 2026: GPT-6 Pro permits 200 messages per week; GPT-5.6 Sol Pro permits 170 per day; together they are limited to 200 per day. These are separate from Work/Codex usage. [OpenAI documentation](https://help.openai.com/en/articles/20001354-gpt-56-and-gpt-6-pro-in-chatgpt).

A published cap does not reveal an account balance. **Remaining browser Pro requests are unavailable** because this app has no live browser Chat counter; local Codex logs cannot fill that gap. No manual counter or upload is required. Check ChatGPT for account availability and reset time; published limits may change.

Missing historical fields remain unknown. Quota accounting measures drops in remaining allowance: 82% to 80% left is 2 percentage points consumed. Per-prompt impact is estimated only when suitable snapshots isolate a completed prompt. Overlapping or unseen activity stays unallocated; resets and collection gaps prevent reliable attribution.

Model advice adds 7-, 30- and 90-day trends, a model mix, measured cache reuse, active-day streaks, conversations, request intensity, and a weekday/hour heatmap. Earlier-period comparisons use the same duration. Missing token measurements stay unavailable.

The allowance forecast uses recent provider percentage readings from the same account, device and reset window, independently of log token totals. It requires four readings spanning at least 30 minutes, with no gap over 15 minutes. It reports possible exhaustion before reset or projected remaining allowance at reset; sparse, stale or interrupted data produces a learning/unavailable state. This projection assumes the observed pace continues. Model recommendations remain separate, local and advisory; your model never changes automatically.

## Privacy and shared history

The app retains a short prompt preview and usage metadata. **Previews can contain sensitive text.** Protect exported and synchronized history with the same care as the conversations it summarizes, and use only storage locations you trust.

History synchronization is optional, and no hosted GCD Usage account is required. See [Security](SECURITY.md) for responsible reporting and security guidance.

Select the same shared folder on each computer, give each computer a recognizable name, and keep all installations updated. Settings shows the most recent folder check, pending records, and reported delivery status for other devices. A folder check alone does not confirm delivery to another computer.

## Download size

The packaged app is measured in megabytes. Developer dependencies and compiler caches can occupy several gigabytes while building; they are not included in the installer. End users need only the installer for their computer.

## Development

Build requirements: Node.js 22+, Rust stable, and the platform's native build tools. End users do not need these tools.

~~~sh
npm ci
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
npm run app:dev
~~~

Build Windows with `node node_modules/@tauri-apps/cli/tauri.js build --bundles nsis`, or macOS on a Mac with `node node_modules/@tauri-apps/cli/tauri.js build --bundles app,dmg`. GitHub is used only for file hosting: Actions is disabled and no automatic build or deployment workflow is configured. Builds are manual on a suitable computer; do not enable paid services. See [Validation](docs/VALIDATION.md).

The dashboard bundles JetBrains Mono under the included [SIL Open Font License](public/fonts/JetBrainsMono-OFL.txt). Fonts load locally; no system-wide font installation or external font request is required.
