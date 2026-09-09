# Validation

Version 0.2.0 adds browser-export import, in-app sign-in progress, and device delivery reports for shared history.

Local validation covers 67 Rust tests, 12 frontend tests, and Svelte/TypeScript checks. Coverage includes duplicate exports, missing metadata, bounded inputs, account separation, date boundaries, delayed sync delivery, safe exports, and dashboard permissions.

Platform packages are built for Windows x64/ARM64 and macOS Intel/Apple Silicon. macOS packages also receive a background-launch smoke test. Interactive macOS sign-in and physical mixed-DPI monitor behavior require checks on the target device.

Browser-import counts describe exported messages. Exact historical token consumption, hidden reasoning, and original-device attribution are not inferred. Shared-folder receipts describe what each device reported receiving.

Performance depends on history size and temporary import or provider helpers. Packages are personal previews without trusted publisher signing.
