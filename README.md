# GCD Usage

A lightweight desktop companion with usage meters, searchable prompt history, shared-folder synchronization, and local model advice.

## Display

- **Windows 11:** a native strip you can drag anywhere in the desktop work area. Lock its position in Settings if desired. It does not modify the Windows taskbar.
- **macOS:** compact menu-bar usage meters.
- **Dashboard:** Overview, History, Model advice, and Settings. Closing the dashboard releases its browser while the native meters keep running.

The default is a black theme, percentage **left**, and 120% text size. Settings offers five themes, percentage used instead, and text sizes from 90% to 160%. Native macOS menu-bar appearance follows macOS. Stale readings are marked; unavailable readings show an em dash and missing reset times show N/A.

## Install and set up

Download the matching Windows x64/ARM64 or macOS Intel/Apple Silicon package from [Releases](https://github.com/eshin087/gcdUsage/releases). Windows installs per user. On macOS, drag the app into Applications.

These are personal preview builds without trusted publisher signing. Your operating system may require first-launch approval. Updates are manual.

Have Claude Code and Codex installed and signed in, open GCD Usage, and review the connection status. Use Reconnect when needed. Setup offers launch-at-login and an optional existing shared folder for combined history across computers.

## Explore usage

Choose last 30 minutes, 1 hour, 6 hours, 24 hours, 7 days, this week, 30 days, this month, all time, or a custom date/time interval. Calendar weeks start Monday in local time. Custom intervals include the start and exclude the end.

Metrics include measured tokens, cache/input/output/reasoning breakdown, prompts, conversations, requests, median/p75 consumption, an activity chart, and model/reasoning breakdowns. History supports search, filters, and CSV export. Usage recorded during an interval can include ongoing prompts started earlier.

V1 covers local Claude Code and Codex history. Ordinary browser conversations are outside this version. Missing historical fields remain unknown. Quota impact is explicitly estimated and may be unavailable when activity overlaps or readings are insufficient.

Model advice runs locally and never changes your model automatically. Personalized forecasts require sufficient completed history; sparse or stale data produces provisional guidance.

## Privacy and shared history

The app retains a short prompt preview and usage metadata. **Previews can contain sensitive text.** CSV exports and shared history are readable by people who can access those files. Choose a shared folder whose membership and permissions you trust. Shared records are not cryptographically authenticated; a folder writer can alter or forge them.

Local preferences and the live database remain on each computer. Use the app's shared-folder option for synchronization. No hosted GCD Usage account is required. See [Security](SECURITY.md) for scope and limitations.

## Development

Build requirements: Node.js 22+, Rust stable, and the platform's native build tools. End users do not need these tools.

~~~sh
npm ci
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
npm run app:dev
~~~

Build Windows with npm run app:build -- --bundles nsis, or macOS on a Mac with npm run app:build -- --bundles app,dmg. Tagged builds run the four supported architecture jobs. See [Validation](docs/VALIDATION.md).
