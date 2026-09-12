# Security and performance review

Reviewed provider response handling and helper execution, dashboard command permissions/navigation, history and export processing, synchronization, native drawing and new conversation links. Checks cover current source and dependency versions; this is not a guarantee that every vulnerability has been found.

## Changes

- Reject conflicting provider/account ownership when an incoming event reuses an existing prompt, request or browser-record identifier. Existing records cannot be silently replaced across these identities.
- Reconstruct browser-conversation links from validated UUIDs and fixed provider origins. The dashboard cannot supply an arbitrary command, path or destination to the opener. Coding logs without a verified route use local details.
- Clean recognized internal message wrappers and remove invisible directional/control formatting from displayed previews. Continue rendering user text as text; do not interpret markup.
- Upgrade Vitest to a patched release. The npm audit reports no known advisories after the upgrade.
- Add indexed provider/time and request/account lookups. Enrich historical context once per file identity without replaying token accounting.
- Keep native meters and duration controls free of resident webviews or animated effects. Preserve bounded reads, helper deadlines, disabled HTTP redirects, local-only dashboard permissions and CSV formula neutralization.

## Historical 0.5.0 dependency triage

The earlier OSV check covered 535 public Rust package/version pairs. `glib` 0.18.5 matches RUSTSEC-2024-0429, but target-specific dependency trees show it is not included in the supported Windows or macOS builds. Linux is not a supported release target. Do not enable Linux packaging without resolving its dependencies.

The lockfile also includes maintenance notices for `proc-macro-error` and the `unic-*` family. These are unmaintained-package notices, not a demonstrated exploit in this app. The Unicode dependency arrives through Tauri's URL-pattern stack and remains an upstream maintenance dependency. Do not claim a completely clean Rust advisory scan.

References: [OSV](https://osv.dev/), [GLib advisory](https://osv.dev/vulnerability/RUSTSEC-2024-0429), [Vitest advisory](https://github.com/advisories/GHSA-82fw-gwwq-j7x9).

## Trust boundaries and limits

Shared-folder sync relies on trusted writers and the storage service's access controls; it is not authenticated, end-to-end-encrypted cloud sync. Previews and context labels can contain sensitive information. Local privileged malware is outside the app's protection boundary. Provider-private interfaces, optional upstream libraries, export formats, OS webviews and unsigned personal builds remain dependencies or limitations.

Record native smoke-test results, platform builds, memory/CPU measurements and interactive verification in `VALIDATION.md`. Measure helpers/imports separately from idle use. Public reports and test outputs must not contain credentials or private history.

Historical note: some earlier releases were built on CI runners. GitHub Actions and hosted builds are now disabled; the current policy permits GitHub file hosting only with no spending. Native builds require the current host authorization described in AGENTS.md. Stripping debug symbols does not necessarily remove compiler source paths from local binaries; do not upload locally built installers without checking that boundary.

## 0.5.0 validation

The manual browser import UI, export parser and dashboard IPC commands are removed. Legacy record validation and storage remain for retention and versioned sync compatibility. The documentation opener takes no destination argument and opens one fixed HTTPS source. Browser Pro caps are dated reference data; no browser credentials, scraping or inferred account counters were introduced. Static terminal styling adds no animation loop or remote font. Quota attribution now selects a post-completion reading and retains conservative window/overlap checks. Existing dependency caveats above remain applicable.

The final native package passed command-denial, retired-import rejection, external-document isolation and popup tests with synthetic data. Both full and production npm audits reported zero known advisories. Rust dependency versions are unchanged from the reviewed baseline; the existing upstream caveats remain. Sanitized source and generated package checks found no local-profile paths or credential patterns. These checks do not prove the absence of all vulnerabilities.

## 0.5.1 changes

Dock scaling is a bounded local setting (80-160%, default 100%); native size and hit-testing use the same monitor-fitted scale. It does not add generic shell or filesystem access. The new font files are unmodified, pinned IBM Plex assets bundled with their OFL license and loaded locally. Native meters use an existing system monospace font. Legacy settings default the new field without losing history.

## 0.5.2 Site terminal

This presentation change adds no IPC command, network endpoint or collection capability. JetBrains Mono v2.304 is pinned to its official source and bundled with its OFL license. The Windows font is registered privately in memory for the process lifetime; no font installation or registry writes are added. Frontend dependency audit reports zero known advisories; Rust dependency versions are unchanged. Native compilation and runtime verification remain required in a suitable manual build environment. GitHub Actions is disabled by policy.

## September 10 history paging source preview
Older hover history is fetched on a background worker using bounded cursor queries. The native window receives cached plain-text records; no webview command, shell permission or external endpoint is added. Lifetime aggregation continues to match both provider and account, and unknown token values remain unavailable. Model details stay in the existing captured-details flow. The popup releases its accumulated history when closed, and generation checks discard results from obsolete hover requests. Native runtime behavior remains unverified pending an authorized build.

The T2 duration picker remains a native window. It uses existing settings persistence and adds no IPC or network access. Whole units convert through checked multiplication with the existing 1-43,200 minute bound; invalid entries cannot be applied. Fonts and brushes are owned by the dialog and released on destruction.

## 0.5.3 local verification
The authorized local Windows build passed 85 Rust tests, including bounded cursor paging, provider/account isolation, unknown values and native duration validation. The packaged dashboard passed command-denial, external-document isolation and popup checks with synthetic data. Closing the dashboard left no child processes in the isolated idle check. A native menu result is ignored if its dialog was destroyed while the menu was open. Existing settings and database integrity were checked before normal installed startup. These checks are limited validation, not a guarantee against every vulnerability; manual native pointer/keyboard and multi-monitor coverage remains incomplete. Locally built artifacts remain private and GitHub Actions stays disabled.

## 0.5.4 source review
The new insights command accepts only 7, 30 or 90 days and is explicitly listed in the build manifest and dashboard capability. It queries existing local records without additional collection or network endpoints. Forecast inputs are scoped to provider, account, device and reset window; unknown/stale values remain unavailable. Dock visibility and geometry are local settings, not synchronized credentials.

Claude renewal delegates to the installed Claude Code executable using its documented refresh environment variables. Before use, the executable is checked for support; commands have fixed arguments, null input/output, a deadline and a hidden Windows process. Tokens enter only the child environment, never command arguments, app history, exports or sync. The provider helper retains ownership of credential storage. Credentials are reread before starting and after completion to handle logout or a competing renewal. Missing credentials never trigger automatic interactive sign-in. Privileged local processes remain outside this boundary.

At the initial source review, native compilation, isolated runtime coverage and a provider renewal round trip were pending. The user subsequently authorized the local 0.5.4 build and installation on September 11. Record completed native checks in VALIDATION.md; frontend checks alone do not establish native or provider behavior. Tests must not mutate a real account's sign-in state.

Recent allowance consumption is exposed through a bounded 1-43,200-minute read command and a narrow duration setter, both explicitly registered and permitted. The query streams at most 50,000 snapshots scoped to provider/account/device, checks payload identity/time, and exposes coverage rather than fabricating missing usage. Native painting consumes the cached result. The duration setter changes only the existing local display interval.


## 0.5.4 security audit - September 11, 2026

### Dependency and dashboard boundaries

The current npm audit reported zero known advisories across 130 dependencies: no critical, high, moderate, low or informational findings. A downloaded public RustSec database containing 1,223 advisory files was matched locally against 535 public Cargo package/version pairs. The private dependency inventory was not sent to OSV. Seven advisories match the lockfile: GLib unsoundness RUSTSEC-2024-0429, the proc-macro-error maintenance notice, and five unic-* maintenance notices. These matching advisories have no assigned CVSS severity; maintenance notices are not evidence of an exploitable flaw in this app.

Current offline Cargo target trees confirm that glib and proc-macro-error are absent from Windows x64/ARM64 and macOS x64/ARM64 dependency trees. The five unic-* packages remain through Tauri's URL-pattern dependencies. No supported-target exploitable advisory was identified in this scan, but the upstream maintenance debt remains. Linux is unsupported and must not be enabled without resolving its dependency findings. Sources: [RustSec advisory database](https://github.com/RustSec/advisory-db), [GLib advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).

The three new commands, get_usage_insights, get_recent_allowance and set_activity_duration, are explicitly registered in the invocation handler, build manifest and local-only dashboard capability. Their inputs are bounded and database queries use parameters. The response types expose usage aggregates, labels, time windows and coverage; they do not expose provider credential objects. No generic shell, filesystem or arbitrary URL permission was added. Existing content-security policy, exact local-origin navigation checks, popup denial, fixed conversation destinations, plain-text history rendering and CSV formula neutralization remain in place. Shared-folder sync continues to validate normalized events and bound batch sizes; trusted writers and the folder's access controls remain necessary.

### Claude credential hardening

Renewal invokes the resolved Claude Code executable with fixed auth login arguments. The helper support check is a compatibility check, not signature verification. Its environment is cleared and rebuilt from a narrow list of OS, user-directory and locale variables, plus the required credential namespace and refresh inputs. Ambient preload, debug, routing, proxy, telemetry and TLS-override variables are not inherited; the helper receives an explicit setting requiring TLS certificate verification and a restricted system PATH. This reduces accidental credential exposure through inherited process configuration. It can require reconnecting or changing the environment on networks that depend on custom proxy or certificate configuration.

Access and refresh tokens must be nonempty printable ASCII without whitespace or control characters and at most 8,192 bytes. Scopes have separate character, length and count limits. Refresh secrets are passed in the helper environment rather than command arguments; standard input/output/error are closed, and application errors use fixed messages. The app does not serialize credentials into its usage database, dashboard, history exports or sync batches. The usage request targets a fixed HTTPS provider endpoint, disables redirects, bounds responses and has connection/request deadlines.

The renewal helper has a 30-second deadline and process-tree cleanup on completion, timeout or cancellation: a Windows job object or Unix process group limits orphaned descendants. Credentials are reread before and after renewal; a missing saved sign-in does not launch automatic interactive login. Usage results are rejected if the locally observed account changes during the check, preventing that response from being attributed to the earlier account.

### Resource safeguards and verification limits

The hover renderer clones only visible history rows for each paint instead of copying the entire accumulated scroll session. Older rows remain available while the popup is open and are released when it closes. Model advice coalesces refreshes into one running insights query and at most one newest pending request; closing the component discards queued work and obsolete responses. A provider/account/device/time snapshot index supports scoped recent-consumption queries; snapshot reads and provider responses remain bounded. Link reconciliation now starts from the link table and uses a provider/account/turn index, avoiding scans of unrelated prompt/request pairs while preserving account ownership, timestamps and token data. On a 5,036-record synthetic fixture, diagnostic link resolution decreased from 11.317 seconds to 0.000097 seconds, and an unchanged repeat import decreased from 9.38 seconds to 0.000851 seconds. These are fixture measurements, not universal timing guarantees. The changes reduce unnecessary allocation and query work; they do not by themselves prove the absence of a memory leak.

The full authorized Windows native suite reports 99 passed, zero failed and one ignored standalone child fixture, which is exercised indirectly by the parent helper tests. Helper lifecycle, environment and token-validation tests passed. One hundred hidden native duration-dialog create/change/close cycles left GDI and USER object counts unchanged at 13 and 5. Ten dashboard webview open/close cycles left zero child processes after closure and stable handle counts. Packaged IPC restrictions, external-navigation isolation and popup checks also passed. No real-account renewal round trip was performed; synthetic helper tests do not establish provider authentication. The final rebuilt package passed a 180-second isolated idle check with a 41.0 MiB sampled peak working set, 0.0073% host CPU, stable GDI/USER counts and no child processes. Installation then passed executable correspondence, unchanged settings/history counts and database integrity checks. See VALIDATION.md for conditions and limits. These bounded checks do not guarantee that every vulnerability or leak has been found.

Same-user or administrator malware, a compromised/replaced Claude Code helper, and provider-owned credential-storage compromise are outside this app's protection boundary. Secrets necessarily exist in process memory while in use. External logout can race with an already-started renewal, and provider revocation or an unusable refresh token can still require sign-in. The app cannot promise permanent authentication or that an access token can never be abused. GitHub remains file hosting only with no Actions, hosted builds or paid services.
