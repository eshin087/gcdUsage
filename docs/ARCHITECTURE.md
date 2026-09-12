# Architecture
GCD Usage is a per-user Tauri desktop companion with local SQLite and optional shared-folder delivery, without a hosted GCD Usage service.

| Responsibility | Source | Boundary |
| --- | --- | --- |
| Lifecycle, cached state, IPC, tray | app.rs | Clone caches separately; validate each command |
| Provider polling and renewal | providers.rs, signin.rs | Provider helper owns credentials; bounded I/O and deadlines |
| Streaming import and preview repair | history.rs, presentation.rs, enrichment.rs | No replay of usage or reassignment of accounts |
| Storage and identity validation | storage.rs, safety.rs | Stable IDs; provider/account ownership |
| Request-time analytics | metrics.rs | Requests inside the interval may belong to earlier prompts |
| Recent allowance changes | recent_usage.rs | Comparable observed segments, explicit coverage |
| Usage insights and future forecast | insights.rs | Percentage forecasts are independent of tokens |
| Model advice calibration | recommendation.rs, storage.rs | Completed-prompt population differs from interval metrics |
| Native summary and paged history | dock.rs, dock_storage.rs | Cached paint data, cursor paging, lifetime row totals |
| Windows UI and geometry | platform/windows_strip.rs, geometry.rs, duration_picker.rs | GDI, process-private font, matching DPI paint/hit testing |
| Details and supported links | details.rs, navigation.rs | Plain text and validated destinations |
| Immutable sync batches | sync.rs | No database or credential synchronization |
| Dashboard | src/App.svelte, src/lib, src/console.css | On-demand Svelte webview destroyed on close |

Rust paths are relative to src-tauri/src unless shown otherwise.

## Flow and lifetime
Provider readings become snapshots; imports become prompts and requests. SQLite retains normalized records and immutable sync events. Aggregators separately expose measured tokens, current allowance, observed recent consumption, future forecasts and attributed prompt impact.

Native paint reads cached summaries. Database queries and helper processes run off the window thread. Recent percentage usage is added to the summary by maintenance. Older hover history loads on background workers. Each repaint clones only visible rows; closing the popup releases its accumulated pages. Insights requests allow one active query and coalesce rapid changes to the newest pending selection.

The dashboard webview exists only while open. Hiding the dock saves visibility while collection and the tray continue; Quit exits. macOS menu-bar text is a separate implementation from Windows dock/hover controls.

## Settings and commands
dockScale is independent of fontScale. The dock uses 1120 by 144 logical pixels in 0.5.4 to fit the requested recent-usage row; the original D1 base was 1120 by 120. dockMinutes is shared by dock tokens and recent allowance use. dockHidden persists across restarts. These are local preferences.

Save the same palette to dashboard, dock, hover and duration UI. Merge unchanged dock fields into a dashboard draft so an unrelated save does not undo native hiding or resizing.

Every new dashboard command needs handler registration in app.rs, declaration in src-tauri/build.rs, and its specific permission in src-tauri/capabilities/dashboard.json.
