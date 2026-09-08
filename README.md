# GCD Usage

A small desktop companion for Claude Code and Codex: glanceable quota meters, local prompt history, shared-folder synchronization, and offline model advice.

## What it shows

- **Windows 11:** a freely movable native strip. Drag the grip at the left end to place it anywhere in the desktop work area; its position is saved. Click a meter to open the dashboard. Settings → Appearance offers **Lock strip position** to prevent accidental movement. No browser stays running for the strip.
- **macOS 13+:** compact menu-bar labels for Claude's five-hour and weekly limits and Codex's weekly limit. Keep the macOS menu bar visible if you want to glance without hovering.
- **Dashboard:** Overview, filtered prompt History, Recommendations, and Settings. Closing it destroys the webview while collection continues. Quit completely from the tray menu.

Percentages show **left** by default. Settings → Appearance lets you choose percentage used instead. The preference applies to the native meters and dashboard progress bars. A `~` marks a last-known/stale reading; `—` means no reading is available. An unavailable reset time is shown as `N/A`. Reset countdowns update locally. The app refreshes usage every two minutes, on wake, and when requested, with backoff on failures.

Ordinary ChatGPT and Claude browser conversations are outside v1. ChatGPT Work and Codex share a quota; an ordinary ChatGPT chat counter is not represented as Codex usage.

The default theme is **Black**. Choose Black, Slate, Midnight, Light, or System in Settings → Appearance. Themes apply to the dashboard and native Windows strip; macOS menu-bar colors follow macOS. Text starts at **120%**; the **Font size** slider adjusts it from 90% to 160%. The Windows strip resizes with its text. macOS menu-bar text size follows macOS; the dashboard slider works on both platforms. Appearance preferences stay on each computer.

Windows 11’s supported [widget API](https://learn.microsoft.com/en-us/windows/apps/develop/widgets/) places third-party content in the Widgets board, not a persistent custom text area in the taskbar. Literal in-taskbar meters require undocumented Explorer integration (see [implementation limitations](https://github.com/pfcdev/TaskbarWidgets/blob/main/docs/windows-private-api-risks.md)). GCD Usage uses the movable-strip fallback and does not modify Explorer. Upgrading from 0.1.1 removes the old default edge anchoring.

## Downloads

Get the personal preview from the [private GitHub release](https://github.com/eshin087/gcdUsage/releases/tag/v0.1.2), signed in as an account with repository access. Choose Windows x64 or ARM64, or macOS Intel or Apple Silicon. Windows packages install per user; Mac DMGs contain an application bundle to drag into Applications. Updates are manual.

## First setup

1. Install the package for your OS/processor. Initial packages are personal builds without trusted publisher signing. Windows may request a first-launch confirmation. On macOS, approve the app through **System Settings → Privacy & Security** if Gatekeeper blocks the personal build.
2. Have **Claude Code** and **Codex** installed and signed in. The app detects normal CLI locations and the Codex desktop app's bundled CLI. Custom executable/config locations are available in Settings.
3. Review connection status. **Reconnect** opens the provider's own sign-in flow. GCD Usage does not request your password or refresh subscription tokens itself.
4. Enable launch-at-login if desired (selected by default), and choose an existing shared folder if you want combined history across computers.
5. Finish setup. Existing local logs import in the background; the history continues to update as you work.

No API key, server, app account, or model call is required. Recommendations do not spend AI allowance. Provider logins remain separate on each computer.

## Choose a usage interval

In Overview, use **Time range**: last 30 minutes, 1 hour, 6 hours, 24 hours, 7 days, this week, 30 days, this month, all time, or a custom start/end date and time. Selection is remembered on this computer. Rolling intervals update every 30 seconds while the dashboard is open; they do not trigger provider calls. Calendar weeks start Monday in local time. Custom intervals include the start and exclude the end.

The interval updates measured tokens, cache/input/output/reasoning breakdown, new prompt counts, active conversations, model requests, median/p75 consumption, the activity chart, and the complete model/reasoning table. Requests from prompts started earlier are counted when recorded, including attributable subagents; they are not mislabeled as background work. Per-prompt figures use only the tokens inside the interval, across active user prompts. Automatic reviews and unmatched requests remain separate background activity. Counts reflect the records currently imported, so missing logs can leave incomplete coverage.

**Explore history** carries the interval into the prompt start-time filters, which accept dates and times down to seconds. History rows continue to show each matching prompt's complete recorded request usage; continuing prompts started earlier appear in interval metrics but will not match these start-time filters. The current allowance meters, all-time quota accounting, and 30-day model-advice calibration keep their own explicit periods.

## History and interpretation

The importer reads `CODEX_HOME/sessions`, `CODEX_HOME/archived_sessions`, and `CLAUDE_CONFIG_DIR/projects`, defaulting to `~/.codex` and `~/.claude`. It never edits provider logs or settings. It preserves only the first **160 Unicode characters** of each user prompt, plus usage metadata; it does not archive complete answers or tool output.

The database distinguishes user prompts, conversations, underlying model requests, attributable subagents, and automatic/background work. Streaming content blocks and copied request identifiers are deduplicated. Mixed-model turns retain their individual model/effort breakdown. Missing historical metadata remains unknown. Provider log formats are internal and can change, so unsupported records are not invented or silently converted to zero.

Token input is normalized into uncached input, cache reads, and cache writes. Reasoning tokens, when available, are a subset of output and are not added again to the total. Token counts are observed log data, not a subscription invoice.

Quota impact is an **estimate**, available only when snapshots safely bound a completed prompt within one unchanged quota window. Overlapping work, unavailable snapshots, resets, and unknown account identity prevent confident attribution. Even isolated estimates may include unobserved activity elsewhere. History imported from before the app collected snapshots generally has no quota-impact estimate.

Recommendations use task suitability, current limits, a 10% reserve, recent usage, and model/effort history. Personalized token forecasts require at least 20 completed prompts; quota affordability additionally needs usable calibration for the matching account and quota window. Sparse or stale data produces provisional advice, not a claim that a task is affordable. Advice never switches your model automatically.

## Multiple computers

Choose the same folder through your existing OneDrive, Dropbox, or other file-sync client on each computer. The app creates `gcd-usage-v1` inside it and exchanges small immutable, versioned batches. Delayed and duplicate deliveries merge idempotently; each computer remains usable offline.

**The shared folder contains prompt previews and usage metadata in readable JSON.** Choose a folder whose sharing/access settings you trust. Credentials, local source paths, device settings, and the SQLite database are not synchronized. Do not place the live SQLite database in a cloud-synced folder.

Local application data is stored under `%LOCALAPPDATA%/com.gcd.usage` on Windows and `~/Library/Application Support/com.gcd.usage` on macOS. `GCD_USAGE_DATA_DIR` overrides this directory for development/testing. `GCD_USAGE_TEST_MODE=1` prevents test settings from altering launch-at-login.

## Development

Requirements: Node.js 22+, Rust stable, and platform build tools. Windows needs Microsoft's C++ Build Tools plus the Windows SDK. macOS needs Xcode Command Line Tools. End users do not need these developer tools.

```sh
npm ci
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run app:dev
```

On this Windows workspace, `scripts/windows.ps1` also discovers the isolated Rust toolchain under `.tools`:

```powershell
.\scripts\windows.ps1 -Action test
.\scripts\windows.ps1 -Action build
```

To build manually:

```sh
# Windows, per-user setup.exe
npm run app:build -- --bundles nsis
# macOS, run on the target Mac
npm run app:build -- --bundles app,dmg
```

Artifacts appear under `src-tauri/target/release/bundle`. The GitHub workflow runs on manual dispatch and version tags, building Windows x64/ARM64 and macOS Intel/Apple Silicon packages on matching runners. It uploads private build artifacts and smoke-tests packaged macOS background launch. Release assets are published manually after validation.

The optional diagnostic binary performs read-only provider probes and imports into an explicitly selected local test directory. It reports totals without printing prompt previews or provider credentials. See `docs/VALIDATION.md` for release verification.

## Maintenance

Claude quota polling uses its private `/api/oauth/usage` endpoint and may need adaptation after provider changes. The application leaves credential refresh to Claude Code. If its saved token expires, reconnect through Claude Code and refresh the meter.

Codex uses its documented app-server account and model interfaces. An incompatible CLI or provider response is surfaced as unavailable; missing quotas are never interpreted as unlimited.

Updates are manual. Installing a newer version preserves local data. Uninstalling the app should remove startup registration; retaining or deleting local history and the shared folder is a separate choice.
