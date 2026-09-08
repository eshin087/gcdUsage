# Validation

## 0.1.3 security update

Security regression coverage includes spreadsheet export escaping, absolute executable discovery, bounded reads, shared-record account boundaries, stable record timestamps, migration of earlier records, numeric validation, and dashboard navigation restrictions.

The local security update passed 61 Rust tests, 12 frontend tests, and Svelte/TypeScript checks. Native Windows checks verified dashboard command restrictions, blocked external document replacement and popups, and interval totals against independent database queries. The Windows installer built successfully. Four architecture builds run in the release workflow.

## Earlier releases

Version 0.1.2 added configurable usage intervals, larger adjustable text, and a freely movable Windows strip. Its four architecture builds and both macOS background-launch checks passed. Version 0.1.1 added appearance themes and percentage-left display. Version 0.1.0 established the desktop meters and history workflow.

Interactive macOS behavior and physical mixed-DPI monitor arrangements remain target-device checks. Personal packages are not signed by a trusted publisher. Performance depends on history size and temporary provider helpers.
