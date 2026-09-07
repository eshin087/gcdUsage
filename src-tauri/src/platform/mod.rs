use crate::models::{ConnectionStatus, Provider, QuotaSnapshot};
use chrono::Utc;
use tauri::{AppHandle, Manager};

#[cfg(windows)]
mod windows_strip;

pub fn countdown(reset: Option<i64>, now: i64) -> String {
    let Some(reset) = reset else {
        return "reset unknown".into();
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

pub fn cells(snapshots: &[QuotaSnapshot], now: i64) -> Vec<(String, String, bool)> {
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
                        "{}{:.0}%  ·  {}",
                        if stale { "~" } else { "" },
                        w.used_percent,
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
    let cells = cells(snapshots, Utc::now().timestamp());
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
                .map(|((_, value, _), label)| format!("{label} {}", value.replace("  ·  ", " ")))
                .collect::<Vec<_>>()
                .join("  ");
            let _ = tray.set_title(Some(title));
        }
    }
    #[cfg(windows)]
    windows_strip::update(cells);
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
    tauri::WebviewWindowBuilder::new(
        app,
        "dashboard",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("GCD Usage")
    .inner_size(1100.0, 760.0)
    .min_inner_size(760.0, 540.0)
    .center()
    .build()
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_never_becomes_zero() {
        assert!(cells(&[], 0)
            .iter()
            .all(|(_, v, _)| v.contains('—') && !v.contains("0%")));
    }
    #[test]
    fn countdown_boundaries() {
        assert_eq!(countdown(Some(120), 0), "2m");
        assert_eq!(countdown(Some(0), 1), "refreshing");
        assert_eq!(countdown(None, 0), "reset unknown");
    }
}
