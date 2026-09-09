fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_overview",
            "open_pro_documentation",
            "get_prompt_detail",
            "open_original_prompt",
            "take_pending_prompt",
            "get_history",
            "get_usage_metrics",
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
}
