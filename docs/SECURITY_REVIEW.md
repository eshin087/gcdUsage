# Security and performance review - 0.5.0

Reviewed provider response handling and helper execution, dashboard command permissions/navigation, history and export processing, synchronization, native drawing and new conversation links. Checks cover current source and dependency versions; this is not a guarantee that every vulnerability has been found.

## Changes

- Reject conflicting provider/account ownership when an incoming event reuses an existing prompt, request or browser-record identifier. Existing records cannot be silently replaced across these identities.
- Reconstruct browser-conversation links from validated UUIDs and fixed provider origins. The dashboard cannot supply an arbitrary command, path or destination to the opener. Coding logs without a verified route use local details.
- Clean recognized internal message wrappers and remove invisible directional/control formatting from displayed previews. Continue rendering user text as text; do not interpret markup.
- Upgrade Vitest to a patched release. The npm audit reports no known advisories after the upgrade.
- Add indexed provider/time and request/account lookups. Enrich historical context once per file identity without replaying token accounting.
- Keep native meters and duration controls free of resident webviews or animated effects. Preserve bounded reads, helper deadlines, disabled HTTP redirects, local-only dashboard permissions and CSV formula neutralization.

## Dependency triage

An OSV check covered 535 public Rust package/version pairs. `glib` 0.18.5 matches RUSTSEC-2024-0429, but target-specific dependency trees show it is not included in the supported Windows or macOS builds. Linux is not a supported release target. Do not enable Linux packaging without resolving its dependencies.

The lockfile also includes maintenance notices for `proc-macro-error` and the `unic-*` family. These are unmaintained-package notices, not a demonstrated exploit in this app. The Unicode dependency arrives through Tauri's URL-pattern stack and remains an upstream maintenance dependency. Do not claim a completely clean Rust advisory scan.

References: [OSV](https://osv.dev/), [GLib advisory](https://osv.dev/vulnerability/RUSTSEC-2024-0429), [Vitest advisory](https://github.com/advisories/GHSA-82fw-gwwq-j7x9).

## Trust boundaries and limits

Shared-folder sync relies on trusted writers and the storage service's access controls; it is not authenticated, end-to-end-encrypted cloud sync. Previews and context labels can contain sensitive information. Local privileged malware is outside the app's protection boundary. Provider-private interfaces, optional upstream libraries, export formats, OS webviews and unsigned personal builds remain dependencies or limitations.

Record native smoke-test results, platform builds, memory/CPU measurements and interactive verification in `VALIDATION.md`. Measure helpers/imports separately from idle use. Public reports and test outputs must not contain credentials or private history.

Release packages are built on isolated CI runners. Stripping debug symbols does not necessarily remove compiler source paths from local binaries; locally built installers are kept out of public release uploads.

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
