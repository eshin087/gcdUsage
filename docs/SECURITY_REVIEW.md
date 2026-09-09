# Security and performance review - 0.4.1

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
