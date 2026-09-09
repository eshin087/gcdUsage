# GCD Usage

**[Read the six-page user manual](output/pdf/GCD-Usage-Manual.pdf)** - setup, token accounting, history, synchronization and limitations.

A lightweight desktop companion with usage meters, searchable prompt history, shared-folder synchronization, and local model advice.

## Display

- **Windows 11:** a native rounded dock with colored usage cards. Drag anywhere on the dock to move it; click to open the dashboard. Hover a meter to see its last 10 recorded prompts above the dock. It does not modify the Windows taskbar.
- **macOS:** compact menu-bar usage meters.
- **Dashboard:** Overview, History, Model advice, and Settings. Closing the dashboard releases its browser while the native meters keep running.

The default is a black Matrix-style theme, percentage **left**, and 120% text size. Settings offers five themes, percentage used instead, and text sizes from 90% to 160%. Native macOS menu-bar appearance follows macOS. Stale readings are marked; unavailable readings show an em dash and missing reset times show N/A.

The Windows activity card shows logged tokens and recent models for the last hour by default. Click it, or right-click the dock, to choose a preset or custom duration of 1–43,200 minutes. Claude also has a Fable weekly meter when its scoped allowance is available. All Claude meters share the latest Claude history.

Hover cards show project/chat context, cleaned prompt previews, model/reasoning, time, status and lifetime logged tokens. Click a row to open captured details. Coding logs without a verified link retain conversation/turn identifiers. Totals cover local and synchronized coding-tool activity. You can hide hover previews in Settings. Hover cards scroll when the screen cannot fit all ten prompts.

## Install and set up

Download the matching Windows x64/ARM64 or macOS Intel/Apple Silicon package from [Releases](https://github.com/eshin087/gcdUsage/releases). Windows installs per user. On macOS, drag the app into Applications.

These are personal preview builds without trusted publisher signing. Your operating system may require first-launch approval. Updates are manual.

Have Claude Code and Codex installed, open GCD Usage, and check the connection status. Existing sign-ins are checked first. When sign-in is needed, follow the provider's browser flow and the progress shown inside GCD Usage. Organization SSO is available for supported Claude accounts. Setup offers launch-at-login and an optional existing shared folder for combined history across computers.

## Explore usage

Choose last 30 minutes, 1 hour, 6 hours, 24 hours, 7 days, this week, 30 days, this month, all time, or a custom date/time interval. Calendar weeks start Monday in local time. Custom intervals include the start and exclude the end.

Metrics include measured tokens, cache/input/output/reasoning breakdown, prompts, conversations, requests, median/p75 consumption, an activity chart, and model/reasoning breakdowns. History supports search, filters, and CSV export. Usage recorded during an interval can include ongoing prompts started earlier.

Coding-tool history covers local Claude Code and Codex activity. The dashboard uses condensed, single-line prompt rows and compact allowance cards. Manual browser imports have been removed. Ordinary browser conversations and tokens are not collected, including activity on computers without an installed collector. Previously stored browser records are retained for compatibility and excluded from the dock.

### Browser ChatGPT Pro limits

The dashboard includes a **Pro $200 reference**, checked September 8, 2026: GPT-6 Pro permits 200 messages per week; GPT-5.6 Sol Pro permits 170 per day; together they are limited to 200 per day. These are separate from Work/Codex usage. [OpenAI documentation](https://help.openai.com/en/articles/20001354-gpt-56-and-gpt-6-pro-in-chatgpt).

A published cap does not reveal an account balance. **Remaining browser Pro requests are unavailable** because this app has no live browser Chat counter; local Codex logs cannot fill that gap. No manual counter or upload is required. Check ChatGPT for account availability and reset time; published limits may change.

Missing historical fields remain unknown. Quota accounting measures drops in remaining allowance: 82% to 80% left is 2 percentage points consumed. Per-prompt impact is estimated only when suitable snapshots isolate a completed prompt. Overlapping or unseen activity stays unallocated; resets and collection gaps prevent reliable attribution.

Model advice runs locally and never changes your model automatically. Personalized forecasts require sufficient completed history; sparse or stale data produces provisional guidance.

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

Build Windows with `node node_modules/@tauri-apps/cli/tauri.js build --bundles nsis`, or macOS on a Mac with `node node_modules/@tauri-apps/cli/tauri.js build --bundles app,dmg`. Tagged builds run the four supported architecture jobs. See [Validation](docs/VALIDATION.md).
