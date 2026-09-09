use crate::{history, models::*, platform, providers, recommendation, storage::Store, sync};
use chrono::Utc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, RwLock,
    },
};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    store: Mutex<Store>,
    snapshots: RwLock<Vec<QuotaSnapshot>>,
    stats: RwLock<DashboardStats>,
    advice_stats: RwLock<DashboardStats>,
    catalog: RwLock<Vec<ModelCatalogEntry>>,
    report: Mutex<ImportReport>,
    importing: AtomicBool,
    dirty: AtomicBool,
    force_refresh: AtomicBool,
    refreshing: AtomicBool,
    next_refresh: Mutex<HashMap<Provider, i64>>,
    failures: Mutex<HashMap<Provider, u32>>,
    last_sync: Mutex<Option<i64>>,
    sync_message: Mutex<Option<String>>,
    sync_health: Mutex<sync::SyncHealth>,
    dock: RwLock<crate::dock::DockSummary>,
    pending_prompt: Mutex<Option<String>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
}
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}
fn persist(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let text =
        serde_json::to_vec_pretty(settings).map_err(|_| "Cannot encode settings".to_string())?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, text).map_err(|_| "Cannot write local settings".to_string())?;
    fs::rename(&temporary, path).map_err(|_| "Cannot save local settings".to_string())
}
pub(crate) fn display_settings(app: &tauri::AppHandle) -> (ColorTheme, MeterDisplay, bool, u16) {
    let state = app.state::<AppState>();
    let settings = lock(&state.settings);
    (
        settings.theme,
        settings.meter_display,
        settings.strip_locked,
        settings.font_scale.clamp(90, 160),
    )
}
pub fn strip_position(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    let state = app.state::<AppState>();
    let s = lock(&state.settings);
    s.strip_x.zip(s.strip_y)
}
pub fn save_strip_position(app: &tauri::AppHandle, x: i32, y: i32) {
    let state = app.state::<AppState>();
    let mut s = lock(&state.settings);
    s.strip_x = Some(x);
    s.strip_y = Some(y);
    let _ = persist(&state.settings_path, &s);
}
pub fn request_refresh(app: &tauri::AppHandle) {
    app.state::<AppState>()
        .force_refresh
        .store(true, Ordering::Release);
}

pub(crate) fn dock_summary(app: &tauri::AppHandle) -> (crate::dock::DockSummary, bool) {
    let state = app.state::<AppState>();
    let previews = lock(&state.settings).dock_previews;
    let summary = state.dock.read().unwrap().clone();
    (summary, previews)
}

#[tauri::command]
fn open_pro_documentation() -> Result<(), String> {
    crate::navigation::launch_pro_documentation()
}

#[tauri::command]
fn get_overview(state: tauri::State<'_, AppState>) -> Overview {
    // Drop each guard before acquiring another; native painting and workers
    // access these caches independently while settings are being saved.
    let dock = state.dock.read().unwrap().clone();
    let snapshots = state.snapshots.read().unwrap().clone();
    let stats = state.stats.read().unwrap().clone();
    let settings = lock(&state.settings).clone();
    let import_report = lock(&state.report).clone();
    let last_sync = *lock(&state.last_sync);
    let sync_message = lock(&state.sync_message).clone();
    let sync_health = lock(&state.sync_health).clone();
    Overview {
        dock,
        snapshots,
        stats,
        settings,
        import_report,
        importing: state.importing.load(Ordering::Acquire),
        last_sync,
        sync_message,
        sync_health,
    }
}

pub(crate) fn activate_prompt(app: &tauri::AppHandle, id: String) {
    *lock(&app.state::<AppState>().pending_prompt) = Some(id);
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = platform::show_dashboard(&a);
        let _ = a.emit("prompt-selected", ());
    });
}
#[tauri::command]
fn take_pending_prompt(state: tauri::State<'_, AppState>) -> Option<String> {
    lock(&state.pending_prompt).take()
}
#[tauri::command]
async fn get_prompt_detail(
    app: tauri::AppHandle,
    id: String,
) -> Result<crate::navigation::PromptDetail, String> {
    tauri::async_runtime::spawn_blocking(move || {
        lock(&app.state::<AppState>().store).prompt_detail(&id)
    })
    .await
    .map_err(|_| "History worker stopped".to_string())?
}
#[tauri::command]
async fn open_original_prompt(app: tauri::AppHandle, id: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let detail = lock(&app.state::<AppState>().store).prompt_detail(&id)?;
        if let Some(url) = detail.original_url {
            crate::navigation::launch_original(&url)?;
            Ok(true)
        } else {
            Ok(false)
        }
    })
    .await
    .map_err(|_| "Navigation worker stopped".to_string())?
}
pub(crate) fn set_dock_minutes(app: &tauri::AppHandle, minutes: u32) -> Result<(), String> {
    if !(1..=43200).contains(&minutes) {
        return Err("Use 1 to 43200 minutes".into());
    }
    let state = app.state::<AppState>();
    {
        let mut settings = lock(&state.settings);
        let mut next = settings.clone();
        next.dock_minutes = minutes;
        persist(&state.settings_path, &next)?;
        *settings = next;
    }
    *state.dock.write().unwrap() = crate::dock::DockSummary::default();
    state.dirty.store(true, Ordering::Release);
    let _ = app.emit("settings-updated", ());
    Ok(())
}
#[tauri::command]
async fn get_history(app: tauri::AppHandle, filter: HistoryFilter) -> Result<HistoryPage, String> {
    tauri::async_runtime::spawn_blocking(move || {
        lock(&app.state::<AppState>().store).history(&filter)
    })
    .await
    .map_err(|_| "History worker stopped".to_string())?
}
#[tauri::command]
async fn get_usage_metrics(
    app: tauri::AppHandle,
    range: MetricsRange,
) -> Result<UsageMetrics, String> {
    range.validate()?;
    tauri::async_runtime::spawn_blocking(move || {
        lock(&app.state::<AppState>().store).usage_metrics(range)
    })
    .await
    .map_err(|_| "Statistics worker stopped".to_string())?
}
#[tauri::command]
fn get_recommendations(state: tauri::State<'_, AppState>, task: TaskClass) -> RecommendationSet {
    recommendation::recommend(
        task,
        &state.snapshots.read().unwrap(),
        &state.advice_stats.read().unwrap(),
        &state.catalog.read().unwrap(),
        lock(&state.settings).reserve_percent,
        Utc::now().timestamp(),
    )
}
#[tauri::command]
fn refresh_usage(app: tauri::AppHandle) {
    request_refresh(&app);
}

#[tauri::command]
fn get_signin_status(app: tauri::AppHandle) -> Vec<crate::signin::Progress> {
    crate::signin::statuses(&app)
}
#[tauri::command]
fn signin_input(
    app: tauri::AppHandle,
    provider: Provider,
    code: Option<String>,
) -> Result<(), String> {
    crate::signin::send(&app, provider, code)
}
#[tauri::command]
fn import_history(state: tauri::State<'_, AppState>) {
    state.dirty.store(true, Ordering::Release);
}

fn install_watcher(app: &tauri::AppHandle) {
    let a = app.clone();
    let Ok(mut watcher) =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if let Ok(event) = event {
                if event
                    .paths
                    .iter()
                    .any(|p| p.extension().is_some_and(|s| s == "jsonl"))
                {
                    a.state::<AppState>().dirty.store(true, Ordering::Release);
                }
            }
        })
    else {
        return;
    };
    let settings = lock(&app.state::<AppState>().settings).clone();
    for path in [
        providers::codex_home(&settings).join("sessions"),
        providers::codex_home(&settings).join("archived_sessions"),
        providers::claude_home(&settings).join("projects"),
    ] {
        if path.exists() {
            let _ = watcher.watch(&path, RecursiveMode::Recursive);
        }
    }
    *lock(&app.state::<AppState>().watcher) = Some(watcher);
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, mut settings: AppSettings) -> Result<AppSettings, String> {
    if !settings.reserve_percent.is_finite() || !(0.0..=50.0).contains(&settings.reserve_percent) {
        return Err("Reserve must be between 0 and 50 percent".into());
    }
    if !(60..=3600).contains(&settings.refresh_seconds) {
        return Err("Refresh interval must be between 60 and 3600 seconds".into());
    }
    if !(90..=160).contains(&settings.font_scale) {
        return Err("Font size must be between 90 and 160 percent".into());
    }
    if !(1..=43200).contains(&settings.dock_minutes) {
        return Err("Dock interval must be between 1 minute and 30 days".into());
    }
    if settings.device_name.trim().is_empty() {
        return Err("Enter a computer name".into());
    }
    if let Some(folder) = &settings.sync_folder {
        if !Path::new(folder).is_dir() {
            return Err("Choose an existing local sync folder".into());
        }
    }
    let state = app.state::<AppState>();
    let (connections_changed, history_changed, startup_changed) = {
        let old = lock(&state.settings);
        settings.device_id = old.device_id.clone();
        settings.strip_x = old.strip_x;
        settings.strip_y = old.strip_y;
        let connections = settings.codex_path != old.codex_path
            || settings.claude_path != old.claude_path
            || settings.codex_home != old.codex_home
            || settings.claude_home != old.claude_home;
        (
            connections,
            connections
                || settings.device_name != old.device_name
                || settings.sync_folder != old.sync_folder,
            settings.launch_at_login != old.launch_at_login
                || (!old.setup_complete && settings.setup_complete),
        )
    };
    if startup_changed && std::env::var_os("GCD_USAGE_TEST_MODE").is_none() {
        let launcher = app.autolaunch();
        if settings.launch_at_login {
            launcher.enable()
        } else {
            launcher.disable()
        }
        .map_err(|_| {
            "Could not update launch-at-login. Check this app's startup permissions.".to_string()
        })?;
    }
    {
        let mut stored = lock(&state.settings);
        persist(&state.settings_path, &settings)?;
        *stored = settings.clone();
    }
    if history_changed {
        install_watcher(&app);
        state.dirty.store(true, Ordering::Release);
    }
    if connections_changed {
        request_refresh(&app);
    }
    platform::settings_changed();
    state.dirty.store(true, Ordering::Release);
    // Do not display a previous interval's number while the new summary loads.
    {
        let mut dock = state.dock.write().unwrap();
        if dock.minutes != settings.dock_minutes {
            *dock = crate::dock::DockSummary::default();
        }
    }
    platform::update(&app, &state.snapshots.read().unwrap());
    let _ = app.emit("settings-updated", ());
    Ok(settings)
}

#[tauri::command]
async fn select_sync_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose the same synced GCD Usage folder on each computer")
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| "Folder picker closed unexpectedly".into())
}
fn safe_csv(text: &str) -> String {
    let start = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if text.starts_with(['\t', '\r', '\n'])
        || start.starts_with(['=', '+', '-', '@', '＝', '＋', '－', '＠'])
    {
        format!("'{text}")
    } else {
        text.to_string()
    }
}
#[tauri::command]
async fn export_history(
    app: tauri::AppHandle,
    filter: HistoryFilter,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = app
            .dialog()
            .file()
            .set_title("Export filtered prompt history")
            .set_file_name("gcd-usage-history.csv")
            .add_filter("CSV", &["csv"])
            .blocking_save_file()
            .and_then(|p| p.into_path().ok())
        else {
            return Ok(None);
        };
        let mut csv =
            csv::Writer::from_path(&path).map_err(|_| "Cannot create export file".to_string())?;
        csv.write_record([
            "timestamp",
            "provider",
            "computer",
            "conversation",
            "prompt",
            "status",
            "models",
            "reasoning",
            "requests",
            "input_tokens",
            "cache_read_tokens",
            "cache_write_tokens",
            "output_tokens",
            "reasoning_tokens_subset",
            "total_tokens",
            "estimated_quota_percent",
        ])
        .map_err(|e| e.to_string())?;
        let state = app.state::<AppState>();
        let store = lock(&state.store);
        let mut page_filter = filter.clone();
        page_filter.limit = Some(250);
        page_filter.offset = Some(0);
        loop {
            let page = store.history(&page_filter)?;
            if page.items.is_empty() {
                break;
            }
            for item in &page.items {
                let mut models = item
                    .requests
                    .iter()
                    .map(|r| r.model.clone())
                    .collect::<Vec<_>>();
                models.sort();
                models.dedup();
                let mut efforts = item
                    .requests
                    .iter()
                    .map(|r| r.effort.clone().unwrap_or_else(|| "unknown".into()))
                    .collect::<Vec<_>>();
                efforts.sort();
                efforts.dedup();
                let n = |v: Option<u64>| v.map(|n| n.to_string()).unwrap_or_default();
                csv.write_record([
                    item.prompt.timestamp.to_string(),
                    item.prompt.provider.key().into(),
                    safe_csv(&item.prompt.device_id),
                    safe_csv(&item.prompt.session_id),
                    safe_csv(&item.prompt.preview),
                    safe_csv(&item.prompt.status),
                    safe_csv(&models.join("; ")),
                    safe_csv(&efforts.join("; ")),
                    item.requests.len().to_string(),
                    n(item.tokens.input),
                    n(item.tokens.cache_read),
                    n(item.tokens.cache_write),
                    n(item.tokens.output),
                    n(item.tokens.reasoning),
                    item.tokens.total().to_string(),
                    item.quota_estimate
                        .as_ref()
                        .map(|q| format!("{:.4}", q.percent))
                        .unwrap_or_default(),
                ])
                .map_err(|e| e.to_string())?;
            }
            let next = page_filter.offset.unwrap_or(0) + page.items.len() as u32;
            if next as u64 >= page.total {
                break;
            }
            page_filter.offset = Some(next);
        }
        csv.flush().map_err(|e| e.to_string())?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|_| "Export worker stopped".to_string())?
}

#[tauri::command]
async fn reconnect_provider(
    app: tauri::AppHandle,
    provider: Provider,
    sso: Option<bool>,
) -> Result<(), String> {
    let settings = lock(&app.state::<AppState>().settings).clone();
    crate::signin::start(app, settings, provider, sso.unwrap_or(false))
}

async fn poll_usage(app: tauri::AppHandle, force: bool) {
    let state = app.state::<AppState>();
    let check_now = Utc::now().timestamp();
    if !force
        && [Provider::Claude, Provider::Codex]
            .iter()
            .all(|p| lock(&state.next_refresh).get(p).copied().unwrap_or(0) > check_now)
    {
        return;
    }
    if state.refreshing.swap(true, Ordering::AcqRel) {
        return;
    }
    let settings = lock(&state.settings).clone();
    let now = Utc::now().timestamp();
    let due = |p: Provider| force || lock(&state.next_refresh).get(&p).copied().unwrap_or(0) <= now;
    let (claude, codex) = tokio::join!(
        async {
            if due(Provider::Claude) {
                Some(providers::poll(Provider::Claude, &settings).await)
            } else {
                None
            }
        },
        async {
            if due(Provider::Codex) {
                Some(providers::poll(Provider::Codex, &settings).await)
            } else {
                None
            }
        }
    );
    let mut successful = Vec::new();
    for mut snapshot in [claude, codex].into_iter().flatten() {
        let connected = snapshot.status == ConnectionStatus::Connected;
        let delay = if connected {
            lock(&state.failures).remove(&snapshot.provider);
            settings.refresh_seconds
        } else {
            let mut failures = lock(&state.failures);
            let attempts = failures.entry(snapshot.provider).or_default();
            *attempts = attempts.saturating_add(1);
            snapshot
                .retry_after_seconds
                .unwrap_or(
                    settings
                        .refresh_seconds
                        .saturating_mul(1u64 << (*attempts).min(4)),
                )
                .max(settings.refresh_seconds)
                .min(3600)
        };
        lock(&state.next_refresh).insert(snapshot.provider, now + delay as i64);
        if connected {
            successful.push(snapshot.clone());
        }
        let mut cache = state.snapshots.write().unwrap();
        if let Some(old) = cache.iter_mut().find(|s| s.provider == snapshot.provider) {
            if !connected && !old.windows.is_empty() {
                snapshot.windows = old.windows.clone();
                snapshot.fetched_at = old.fetched_at;
                snapshot.account_id = old.account_id.clone();
            }
            *old = snapshot;
        } else {
            cache.push(snapshot);
        }
    }
    if !successful.is_empty() {
        let a = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let state = a.state::<AppState>();
            let mut store = lock(&state.store);
            for s in successful {
                let _ = store.save_snapshot(&s);
            }
        })
        .await;
    }
    platform::update(&app, &state.snapshots.read().unwrap());
    let _ = app.emit("usage-updated", ());
    state.refreshing.store(false, Ordering::Release);
}

async fn maintenance(app: tauri::AppHandle, import: bool) {
    let state = app.state::<AppState>();
    if state.importing.swap(true, Ordering::AcqRel) {
        return;
    }
    if import {
        let _ = app.emit("history-updated", ());
    }
    let a = app.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let state = a.state::<AppState>();
        let settings = lock(&state.settings).clone();
        // Cached account identity remains useful when sign-in has expired. Unknown
        // identities are explicitly marked and never used for quota attribution.
        let accounts = [Provider::Claude, Provider::Codex]
            .into_iter()
            .map(|provider| (provider, providers::current_account_id(&settings, provider)))
            .collect::<HashMap<_, _>>();
        let mut store = lock(&state.store);
        if import {
            match history::import_all(&mut store, &settings, &accounts) {
                Ok(report) => *lock(&state.report) = report,
                Err(e) => lock(&state.report).warnings = vec![e],
            }
        }
        if let Some(folder) = &settings.sync_folder {
            match sync::synchronize(&mut store, Path::new(folder), &settings.device_id) {
                Ok(()) => {
                    *lock(&state.last_sync) = Some(Utc::now().timestamp());
                    *lock(&state.sync_message) = None;
                    match sync::device_status(
                        &store,
                        Path::new(folder),
                        &settings.device_id,
                        &settings.device_name,
                        Utc::now().timestamp(),
                    ) {
                        Ok(health) => *lock(&state.sync_health) = health,
                        Err(message) => *lock(&state.sync_message) = Some(message),
                    }
                }
                Err(e) => *lock(&state.sync_message) = Some(e),
            }
        } else {
            *lock(&state.sync_health) = sync::SyncHealth::default();
        }
        if let Ok(stats) = store.stats(None, None) {
            *state.stats.write().unwrap() = stats;
        }
        if let Ok(summary) = store.dock_summary(settings.dock_minutes, Utc::now().timestamp()) {
            let still_current = lock(&state.settings).dock_minutes == settings.dock_minutes;
            if still_current {
                *state.dock.write().unwrap() = summary;
            }
        } else {
            *state.dock.write().unwrap() = crate::dock::DockSummary::default();
        }
        if let Ok(stats) = store.stats(Some(Utc::now().timestamp() - 30 * 86400), None) {
            *state.advice_stats.write().unwrap() = stats;
        }
    })
    .await;
    if outcome.is_err() {
        lock(&state.report)
            .warnings
            .push("The history worker stopped; retry importing.".into());
    }
    state.importing.store(false, Ordering::Release);
    platform::update(&app, &state.snapshots.read().unwrap());
    let _ = app.emit("history-updated", ());
}

fn refresh_catalog(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let settings = lock(&app.state::<AppState>().settings).clone();
        if let Ok(catalog) = providers::discover_models(&settings).await {
            *app.state::<AppState>().catalog.write().unwrap() = catalog;
            let _ = app.emit("usage-updated", ());
        }
    });
}
async fn scheduler(app: tauri::AppHandle) {
    poll_usage(app.clone(), true).await;
    refresh_catalog(app.clone());
    let mut last_catalog = Utc::now().timestamp();
    let mut previous = Utc::now().timestamp();
    let mut last_maintenance = 0i64;
    let mut last_draw = 0i64;
    let mut ticks = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        ticks.tick().await;
        let state = app.state::<AppState>();
        let now = Utc::now().timestamp();
        let wake = now - previous > 30 || now < previous;
        previous = now;
        let reset_due = state.snapshots.read().unwrap().iter().any(|s| {
            s.status == ConnectionStatus::Connected
                && s.windows
                    .iter()
                    .any(|w| w.resets_at.is_some_and(|r| r <= now && s.fetched_at < r))
        });
        let requested = state.force_refresh.swap(false, Ordering::AcqRel);
        let force = requested || wake || reset_due;
        if requested || now - last_catalog >= 86400 {
            refresh_catalog(app.clone());
            last_catalog = now;
        }
        if !state.refreshing.load(Ordering::Acquire) {
            let a = app.clone();
            tauri::async_runtime::spawn(async move {
                poll_usage(a, force).await;
            });
        }
        let dirty = state.dirty.swap(false, Ordering::AcqRel);
        if (dirty || now - last_maintenance >= 60) && !state.importing.load(Ordering::Acquire) {
            last_maintenance = now;
            let a = app.clone();
            tauri::async_runtime::spawn(async move {
                maintenance(a, true).await;
            });
        } else if dirty {
            state.dirty.store(true, Ordering::Release);
        }
        if now - last_draw >= 60 {
            platform::update(&app, &state.snapshots.read().unwrap());
            last_draw = now;
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = platform::show_dashboard(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_overview,
            open_pro_documentation,
            get_history,
            get_usage_metrics,
            get_recommendations,
            refresh_usage,
            import_history,
            save_settings,
            select_sync_folder,
            export_history,
            reconnect_provider,
            get_signin_status,
            signin_input,
            take_pending_prompt,
            get_prompt_detail,
            open_original_prompt
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let data_override = std::env::var_os("GCD_USAGE_DATA_DIR");
            let data = data_override
                .clone()
                .map(PathBuf::from)
                .unwrap_or(app.path().app_local_data_dir()?);
            #[cfg(unix)]
            {
                use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
                fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(&data)?;
                // Tighten only our default app directory, never an arbitrary override.
                if data_override.is_none() {
                    fs::set_permissions(&data, fs::Permissions::from_mode(0o700))?;
                }
            }
            #[cfg(not(unix))]
            fs::create_dir_all(&data)?;
            let settings_path = data.join("settings.json");
            let mut settings: AppSettings = if settings_path.exists() {
                serde_json::from_slice(&crate::safety::read_file_limited(
                    &settings_path,
                    1024 * 1024,
                )?)?
            } else {
                AppSettings::default()
            };
            settings.font_scale = settings.font_scale.clamp(90, 160);
            settings.dock_minutes = settings.dock_minutes.clamp(1, 43200);
            persist(&settings_path, &settings).map_err(std::io::Error::other)?;
            let store = Store::open(&data.join("usage.sqlite3")).map_err(std::io::Error::other)?;
            let snapshots = store.latest_snapshots().unwrap_or_default();
            let show = !settings.setup_complete || !std::env::args().any(|a| a == "--background");
            app.manage(AppState {
                settings: Mutex::new(settings),
                settings_path,
                store: Mutex::new(store),
                snapshots: RwLock::new(snapshots),
                stats: RwLock::new(DashboardStats::default()),
                advice_stats: RwLock::new(DashboardStats::default()),
                catalog: RwLock::new(Vec::new()),
                report: Mutex::new(ImportReport::default()),
                importing: AtomicBool::new(false),
                dirty: AtomicBool::new(true),
                force_refresh: AtomicBool::new(false),
                refreshing: AtomicBool::new(false),
                next_refresh: Mutex::new(HashMap::new()),
                failures: Mutex::new(HashMap::new()),
                last_sync: Mutex::new(None),
                sync_message: Mutex::new(None),
                sync_health: Mutex::new(sync::SyncHealth::default()),
                dock: RwLock::new(crate::dock::DockSummary::default()),
                pending_prompt: Mutex::new(None),
                watcher: Mutex::new(None),
            });
            app.manage(crate::signin::SignIns::default());
            app.manage(crate::signin::HelperProcesses::default());
            let menu = tauri::menu::Menu::with_items(
                app,
                &[
                    &tauri::menu::MenuItem::with_id(
                        app,
                        "open",
                        "Open dashboard",
                        true,
                        None::<&str>,
                    )?,
                    &tauri::menu::MenuItem::with_id(
                        app,
                        "refresh",
                        "Refresh usage",
                        true,
                        None::<&str>,
                    )?,
                    &tauri::menu::PredefinedMenuItem::separator(app)?,
                    &tauri::menu::MenuItem::with_id(
                        app,
                        "quit",
                        "Quit GCD Usage",
                        true,
                        None::<&str>,
                    )?,
                ],
            )?;
            let rgba = include_bytes!("../icons/tray.rgba").to_vec();
            tauri::tray::TrayIconBuilder::with_id("usage")
                .icon(tauri::image::Image::new_owned(rgba, 32, 32))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        let _ = platform::show_dashboard(app);
                    }
                    "refresh" => request_refresh(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let _ = platform::show_dashboard(tray.app_handle());
                    }
                })
                .build(app)?;
            platform::create(app.handle());
            platform::update(
                app.handle(),
                &app.state::<AppState>().snapshots.read().unwrap(),
            );
            install_watcher(app.handle());
            if show {
                platform::show_dashboard(app.handle()).map_err(std::io::Error::other)?;
            }
            let a = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                scheduler(a).await;
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Cannot start GCD Usage")
        .run(|_app, event| match event {
            tauri::RunEvent::ExitRequested { code, api, .. } => {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            tauri::RunEvent::Exit => {
                crate::signin::shutdown(_app);
                platform::shutdown();
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                let _ = platform::show_dashboard(_app);
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_neutralizes_formulas() {
        assert_eq!(safe_csv("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(safe_csv("hello"), "hello");
    }
    #[test]
    fn security_csv_neutralizes_whitespace_and_full_width_formulas() {
        for text in ["=1+1", " \t=1+1", "\n=1+1", "＝1+1", "＠SUM(1)"] {
            assert!(
                safe_csv(text).starts_with('\''),
                "formula-like text was not escaped"
            );
        }
    }
}
