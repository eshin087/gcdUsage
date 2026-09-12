fn main() {
    // Native control tests also link dialogs that require common-controls v6.
    // Tauri embeds its resource only in binaries, not the library test harness.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Packaged binaries already receive Tauri's manifest resource.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_overview",
            "open_pro_documentation",
            "get_prompt_detail",
            "open_original_prompt",
            "take_pending_prompt",
            "get_history",
            "get_usage_metrics",
            "get_usage_insights",
            "get_recent_allowance",
            "set_activity_duration",
            "get_recommendations",
            "refresh_usage",
            "import_history",
            "save_settings",
            "select_sync_folder",
            "export_history",
            "reconnect_provider",
            "get_signin_status",
            "signin_input",
        ]),
    ))
    .expect("Cannot build application permissions");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Keep Tauri's complete manifest and avoid a duplicate generated resource.
        println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    }
}
