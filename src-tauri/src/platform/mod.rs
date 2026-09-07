use crate::models::{ColorTheme, ConnectionStatus, MeterDisplay, Provider, QuotaSnapshot};
use chrono::Utc;
use tauri::{AppHandle, Manager};

#[cfg(any(windows, test))]
mod geometry;
#[cfg(windows)]
mod windows_strip;

pub fn countdown(reset: Option<i64>, now: i64) -> String {
    let Some(reset) = reset else {
        return "N/A".into();
    };
    let seconds = reset - now;
    if seconds <= 0 {
        return "refreshing".into();
    }
    if seconds >= 86400 {
        format!("{}d {}h", seconds / 86400, (seconds % 86400) / 3600)
    } else if seconds >= 3600 {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    } else {
        format!("{}m", (seconds + 59) / 60)
    }
}

pub fn cells(
    snapshots: &[QuotaSnapshot],
    now: i64,
    display: MeterDisplay,
) -> Vec<(String, String, bool)> {
    [
        (Provider::Claude, 300, "Claude · 5h"),
        (Provider::Claude, 10080, "Claude · week"),
        (Provider::Codex, 10080, "Codex · week"),
    ]
    .iter()
    .map(|(provider, duration, label)| {
        let snapshot = snapshots.iter().find(|s| s.provider == *provider);
        let general_id = format!("{}:{}", provider.key(), duration);
        let window = snapshot.and_then(|s| {
            s.windows.iter().find(|w| w.id == general_id).or_else(|| {
                s.windows.iter().find(|w| {
                    w.duration_minutes == *duration
                        && !w.id.contains("spark")
                        && !w.id.contains("sonnet")
                        && !w.id.contains("opus")
                        && !w.id.contains("fable")
                        && !w.id.contains("bengalfox")
                })
            })
        });
        match (snapshot, window) {
            (Some(s), Some(w)) => {
                let stale = s.status != ConnectionStatus::Connected
                    || now - s.fetched_at > 300
                    || w.resets_at.is_some_and(|r| r <= now);
                (
                    label.to_string(),
                    format!(
                        "{}{} {} · {}",
                        if stale { "~" } else { "" },
                        display
                            .percent(w.used_percent)
                            .map(|p| format!("{p:.0}%"))
                            .unwrap_or_else(|| "—".into()),
                        display.label(),
                        countdown(w.resets_at, now)
                    ),
                    stale,
                )
            }
            (Some(s), _) => (
                label.to_string(),
                match s.status {
                    ConnectionStatus::NeedsAuth => "—  ·  sign in",
                    ConnectionStatus::Error => "—  ·  offline",
                    _ => "—  ·  unavailable",
                }
                .into(),
                true,
            ),
            _ => (label.to_string(), "—  ·  connecting".into(), true),
        }
    })
    .collect()
}

pub fn create(app: &AppHandle) {
    #[cfg(windows)]
    windows_strip::create(app.clone());
    #[cfg(not(windows))]
    let _ = app;
}
pub fn update(app: &AppHandle, snapshots: &[QuotaSnapshot]) {
    let (theme, display, _) = crate::app::display_settings(app);
    let cells = cells(snapshots, Utc::now().timestamp(), display);
    if let Some(window) = app.get_webview_window("dashboard") {
        let _ = window.set_theme(window_theme(theme));
    }
    if let Some(tray) = app.tray_by_id("usage") {
        let full = cells
            .iter()
            .map(|(label, value, _)| format!("{label}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = tray.set_tooltip(Some(full));
        #[cfg(target_os = "macos")]
        {
            let title = cells
                .iter()
                .zip(["C5", "CW", "OW"])
                .map(|((_, value, _), label)| {
                    format!(
                        "{label} {}",
                        value
                            .replace(" · ", " ")
                            .replace(" left", "")
                            .replace(" used", "")
                    )
                })
                .collect::<Vec<_>>()
                .join("  ");
            let _ = tray.set_title(Some(format!("{} · {title}", display.label())));
        }
    }
    #[cfg(windows)]
    windows_strip::update(cells);
}
fn window_theme(theme: ColorTheme) -> Option<tauri::Theme> {
    match theme {
        ColorTheme::System => None,
        ColorTheme::Light => Some(tauri::Theme::Light),
        _ => Some(tauri::Theme::Dark),
    }
}
pub fn settings_changed() {
    #[cfg(windows)]
    windows_strip::settings_changed();
}
pub fn shutdown() {
    #[cfg(windows)]
    windows_strip::shutdown();
}

pub fn show_dashboard(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("dashboard") {
        let _ = window.show();
        let _ = window.unminimize();
        return window.set_focus().map_err(|e| e.to_string());
    }
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "dashboard",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("GCD Usage")
    .theme(window_theme(crate::app::display_settings(app).0))
    .inner_size(1100.0, 760.0)
    .min_inner_size(760.0, 540.0)
    .center()
    .data_directory(
        std::env::var_os("GCD_USAGE_DATA_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or(app.path().app_local_data_dir().map_err(|e| e.to_string())?)
            .join("webview"),
    )
    .build()
    .map_err(|e| e.to_string())?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            // Release the webview before its last native window so its browser
            // controller and environment cannot keep running after close.
            if let Some(window) = handle.get_webview_window("dashboard") {
                let webview: &tauri::Webview = window.as_ref();
                let _ = webview.close();
                let _ = window.destroy();
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_never_becomes_zero() {
        assert!(cells(&[], 0, MeterDisplay::Remaining)
            .iter()
            .all(|(_, v, _)| v.contains('—') && !v.contains("0%")));
    }
    #[test]
    fn countdown_boundaries() {
        assert_eq!(countdown(Some(120), 0), "2m");
        assert_eq!(countdown(Some(0), 1), "refreshing");
        assert_eq!(countdown(None, 0), "N/A");
    }
}
