//! Read-only provider adapters. Credentials and raw provider errors never leave this module.
use crate::models::*;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

const CLAUDE_USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const MAX_RESPONSE: usize = 2 * 1024 * 1024;
pub const CATALOG_VERSION: &str = "2026-09-07.1";

#[derive(Debug)]
struct Failure {
    status: ConnectionStatus,
    message: &'static str,
    retry: Option<u64>,
}
impl Failure {
    fn error(message: &'static str) -> Self {
        Self {
            status: ConnectionStatus::Error,
            message,
            retry: None,
        }
    }
    fn auth(message: &'static str) -> Self {
        Self {
            status: ConnectionStatus::NeedsAuth,
            message,
            retry: None,
        }
    }
}

pub fn codex_home(settings: &AppSettings) -> PathBuf {
    settings
        .codex_home
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CODEX_HOME").map(PathBuf::from))
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".codex"))
}

pub fn claude_home(settings: &AppSettings) -> PathBuf {
    settings
        .claude_home
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from))
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".claude"))
}

fn hash_identity(provider: Provider, identity: &str) -> String {
    format!(
        "{}:{:x}",
        provider.key(),
        Sha256::digest(format!("gcd-usage-account-v1:{}:{identity}", provider.key()).as_bytes())
    )
}

fn small_json(path: &Path) -> Option<Value> {
    if std::fs::metadata(path).ok()?.len() > MAX_RESPONSE as u64 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Stable opaque account scope, also used when importing local history. Never returns PII.
pub fn current_account_id(settings: &AppSettings, provider: Provider) -> String {
    let identity = match provider {
        Provider::Codex => small_json(&codex_home(settings).join("auth.json")).and_then(|v| {
            v.pointer("/tokens/account_id")
                .or_else(|| v.get("account_id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        }),
        Provider::Claude => {
            let home = claude_home(settings);
            // Default profile metadata is ~/.claude.json; relocated configurations use
            // <configdir>/.claude.json. Keep the sibling as a compatibility fallback.
            let relocated =
                settings.claude_home.is_some() || std::env::var_os("CLAUDE_CONFIG_DIR").is_some();
            let profiles = if relocated {
                [
                    home.join(".claude.json"),
                    home.with_extension("json"),
                    home.join("settings.json"),
                ]
            } else {
                [
                    home.with_extension("json"),
                    home.join(".claude.json"),
                    home.join("settings.json"),
                ]
            };
            profiles.iter().filter_map(|p| small_json(p)).find_map(|v| {
                v.pointer("/oauthAccount/accountUuid")
                    .or_else(|| v.pointer("/oauthAccount/account_uuid"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
        }
    };
    identity
        .filter(|s| !s.is_empty())
        .map(|s| hash_identity(provider, &s))
        .unwrap_or_else(|| format!("unknown:{}:{}", provider.key(), settings.device_id))
}

fn candidate_executable(path: PathBuf) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    #[cfg(windows)]
    if !path
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    Some(path)
}

fn discover(name: &str, configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|p| !p.trim().is_empty()) {
        return candidate_executable(PathBuf::from(path));
    }
    let executable = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    };
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&path).map(|p| p.join(&executable)));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/bin").join(&executable));
        candidates.push(home.join(".cargo/bin").join(&executable));
        candidates.push(home.join(format!(".{name}/bin")).join(&executable));
    }
    #[cfg(windows)]
    if name == "codex" {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let root = PathBuf::from(local).join("OpenAI/Codex/bin");
            let mut versions: Vec<_> = std::fs::read_dir(root)
                .into_iter()
                .flatten()
                .flatten()
                .collect();
            versions
                .sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));
            candidates.extend(versions.into_iter().map(|e| e.path().join("codex.exe")));
        }
        // npm's launcher is a .cmd script; locate its native vendor binary without invoking a shell.
        if let Some(roaming) = std::env::var_os("APPDATA") {
            let root = PathBuf::from(roaming).join("npm/node_modules/@openai");
            for package in ["codex", "codex-win32-x64", "codex-win32-arm64"] {
                for target in ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"] {
                    candidates.push(
                        root.join(package)
                            .join("vendor")
                            .join(target)
                            .join("codex/codex.exe"),
                    );
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/opt/homebrew/bin").join(name));
        candidates.push(PathBuf::from("/usr/local/bin").join(name));
        if name == "codex" {
            candidates.push(PathBuf::from(
                "/Applications/Codex.app/Contents/Resources/codex",
            ));
            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join("Applications/Codex.app/Contents/Resources/codex"));
            }
        }
    }
    candidates.into_iter().find_map(candidate_executable)
}

pub fn discover_codex(settings: &AppSettings) -> Option<PathBuf> {
    discover("codex", settings.codex_path.as_deref())
}
pub fn discover_claude(settings: &AppSettings) -> Option<PathBuf> {
    discover("claude", settings.claude_path.as_deref())
}

pub async fn poll(provider: Provider, settings: &AppSettings) -> QuotaSnapshot {
    let account = current_account_id(settings, provider);
    let result = match provider {
        Provider::Claude => poll_claude(settings).await,
        Provider::Codex => poll_codex(settings).await,
    };
    let (account_id, status, windows, message, retry_after_seconds) = match result {
        Ok((found_account, windows)) => {
            let message = if windows.is_empty() {
                Some("Connected, but this account did not return usage windows.".into())
            } else {
                None
            };
            (
                found_account.unwrap_or(account),
                ConnectionStatus::Connected,
                windows,
                message,
                None,
            )
        }
        Err(error) => (
            account,
            error.status,
            Vec::new(),
            Some(error.message.into()),
            error.retry,
        ),
    };
    QuotaSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        provider,
        account_id,
        device_id: settings.device_id.clone(),
        fetched_at: Utc::now().timestamp(),
        status,
        windows,
        message,
        retry_after_seconds,
    }
}

fn parse_timestamp(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp())
    })
}

fn valid_percent(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0.0)
        .map(|n| n.min(100.0))
}

fn window_label(pool: &str, minutes: u32) -> String {
    let duration = match minutes {
        300 => "5h".into(),
        10080 => "weekly".into(),
        m if m % 60 == 0 => format!("{}h", m / 60),
        m => format!("{m}m"),
    };
    let name = match pool {
        "codex" => "Codex",
        "claude" => "Claude",
        "claude-sonnet" => "Claude Sonnet",
        "claude-opus" => "Claude Opus",
        _ => pool,
    };
    format!("{name} {duration}")
}

fn parse_codex_windows(value: &Value) -> Vec<QuotaWindow> {
    let mut windows = Vec::new();
    let buckets: Vec<(String, &Value)> = if let Some(map) = value
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
        .filter(|m| !m.is_empty())
    {
        map.iter()
            .map(|(id, bucket)| (id.clone(), bucket))
            .collect()
    } else {
        value
            .get("rateLimits")
            .filter(|v| v.is_object())
            .map(|v| {
                vec![(
                    v.get("limitId")
                        .and_then(Value::as_str)
                        .unwrap_or("codex")
                        .to_owned(),
                    v,
                )]
            })
            .unwrap_or_default()
    };
    for (pool, bucket) in buckets {
        for slot in ["primary", "secondary"] {
            let Some(window) = bucket.get(slot) else {
                continue;
            };
            let (Some(used), Some(duration)) = (
                window.get("usedPercent").and_then(valid_percent),
                window
                    .get("windowDurationMins")
                    .and_then(Value::as_u64)
                    .filter(|d| *d > 0 && *d <= u32::MAX as u64),
            ) else {
                continue;
            };
            let duration = duration as u32;
            let id = format!("{pool}:{duration}");
            if windows.iter().any(|w: &QuotaWindow| w.id == id) {
                continue;
            }
            windows.push(QuotaWindow {
                id,
                label: window_label(&pool, duration),
                duration_minutes: duration,
                used_percent: used,
                resets_at: window.get("resetsAt").and_then(parse_timestamp),
            });
        }
    }
    windows.sort_by_key(|w| {
        (
            !w.id.starts_with("codex:"),
            w.duration_minutes,
            w.id.clone(),
        )
    });
    windows
}

fn parse_claude_windows(value: &Value) -> Vec<QuotaWindow> {
    let mut windows = Vec::new();
    if let Some(map) = value.as_object() {
        for (name, window) in map {
            let (pool, duration) = match name.as_str() {
                "five_hour" => ("claude".to_owned(), 300),
                "seven_day" => ("claude".to_owned(), 10080),
                x if x.starts_with("seven_day_") => (format!("claude-{}", &x[10..]), 10080),
                _ => continue,
            };
            let Some(used) = window.get("utilization").and_then(valid_percent) else {
                continue;
            };
            windows.push(QuotaWindow {
                id: format!("{pool}:{duration}"),
                label: window_label(&pool, duration),
                duration_minutes: duration,
                used_percent: used,
                resets_at: window.get("resets_at").and_then(parse_timestamp),
            });
        }
    }
    windows.sort_by_key(|w| {
        (
            !w.id.starts_with("claude:"),
            w.duration_minutes,
            w.id.clone(),
        )
    });
    windows
}

struct ClaudeCredentials {
    access_token: String,
}
fn parse_claude_credentials(value: Value) -> Option<ClaudeCredentials> {
    value
        .pointer("/claudeAiOauth/accessToken")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|s| ClaudeCredentials {
            access_token: s.to_owned(),
        })
}

#[cfg(any(target_os = "macos", test))]
fn claude_keychain_service(namespace: Option<&str>) -> String {
    use unicode_normalization::UnicodeNormalization;
    match namespace.filter(|s| !s.is_empty()) {
        None => "Claude Code-credentials".into(),
        Some(directory) => {
            // Claude's private secure-storage convention hashes the exact NFC directory string.
            // Do not canonicalize or expand '~': doing so would select a different credential.
            let normalized: String = directory.nfc().collect();
            let hash = format!("{:x}", Sha256::digest(normalized.as_bytes()));
            format!("Claude Code-credentials-{}", &hash[..8])
        }
    }
}

async fn claude_credentials(settings: &AppSettings) -> Option<ClaudeCredentials> {
    #[cfg(target_os = "macos")]
    {
        let namespace = settings
            .claude_home
            .clone()
            .or_else(|| std::env::var("CLAUDE_CONFIG_DIR").ok());
        let service = claude_keychain_service(namespace.as_deref());
        let mut command = Command::new("/usr/bin/security");
        command
            .args(["find-generic-password", "-s", service.as_str(), "-w"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Ok(Ok(output)) = tokio::time::timeout(Duration::from_secs(5), command.output()).await
        {
            if output.status.success() && output.stdout.len() <= MAX_RESPONSE {
                if let Ok(value) = serde_json::from_slice(&output.stdout) {
                    if let Some(credentials) = parse_claude_credentials(value) {
                        return Some(credentials);
                    }
                }
            }
        }
        // The directory-scoped file is Claude's fallback when Keychain is unavailable.
        // Never retry a relocated profile against the default account's Keychain entry.
    }
    small_json(&claude_home(settings).join(".credentials.json")).and_then(parse_claude_credentials)
}
fn http_failure(status: u16, retry: Option<u64>) -> Failure {
    match status {
        401 => Failure::auth("Claude sign-in expired. Open Claude Code and run /login, then refresh here."),
        403 => Failure::auth("Claude usage access was denied. Sign in to your subscription in Claude Code, then refresh here."),
        429 => Failure { status: ConnectionStatus::Error, message: "Claude is rate limiting usage checks. Retrying after a short pause.", retry: Some(retry.unwrap_or(120)) },
        _ => Failure::error("Claude usage could not be read. The private endpoint may have changed; try again later."),
    }
}

async fn claude_request(
    client: &reqwest::Client,
    credentials: &ClaudeCredentials,
) -> Result<reqwest::Response, Failure> {
    client
        .get(CLAUDE_USAGE_URL)
        .bearer_auth(&credentials.access_token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                Failure::error(
                    "Claude usage check timed out. Try again when the connection is available.",
                )
            } else {
                Failure::error("Could not connect to Claude. Check your internet connection.")
            }
        })
}

async fn poll_claude(
    settings: &AppSettings,
) -> Result<(Option<String>, Vec<QuotaWindow>), Failure> {
    let credentials = claude_credentials(settings).await.ok_or_else(|| Failure::auth("No Claude subscription sign-in found. Open Claude Code and run /login, then refresh here."))?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("GCD-Usage/0.1.0")
        .build()
        .map_err(|_| Failure::error("Could not initialize the secure Claude connection."))?;
    let mut response = claude_request(&client, &credentials).await?;
    if response.status().as_u16() == 401 {
        // Claude Code may have refreshed while this request was in flight. We never refresh it ourselves.
        if let Some(updated) = claude_credentials(settings).await {
            if updated.access_token != credentials.access_token {
                response = claude_request(&client, &updated).await?;
            }
        }
    }
    let status = response.status().as_u16();
    let retry = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.parse::<u64>().ok().or_else(|| {
                DateTime::parse_from_rfc2822(s)
                    .ok()
                    .map(|d| d.timestamp().saturating_sub(Utc::now().timestamp()).max(1) as u64)
            })
        });
    if !response.status().is_success() {
        return Err(http_failure(status, retry));
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_RESPONSE as u64)
    {
        return Err(Failure::error(
            "Claude returned an unexpected usage response.",
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| Failure::error("Claude returned an incomplete usage response."))?;
    if bytes.len() > MAX_RESPONSE {
        return Err(Failure::error(
            "Claude returned an unexpected usage response.",
        ));
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| Failure::error("Claude returned an unreadable usage response."))?;
    if !value.is_object() {
        return Err(Failure::error(
            "Claude returned an unexpected usage response.",
        ));
    }
    Ok((
        Some(current_account_id(settings, Provider::Claude)),
        parse_claude_windows(&value),
    ))
}

/// A separate process tree, stopped when this connection is dropped, including on cancellation.
struct OwnedServer {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
    #[cfg(windows)]
    job: windows::Win32::Foundation::HANDLE,
    #[cfg(unix)]
    process_group: i32,
}

#[cfg(windows)]
fn contain_child(child: &Child) -> Result<windows::Win32::Foundation::HANDLE, Failure> {
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::*,
    };
    unsafe {
        let process = child
            .raw_handle()
            .ok_or_else(|| Failure::error("Codex exited before connecting."))?;
        let job = CreateJobObjectW(None, None)
            .map_err(|_| Failure::error("Could not create a contained Codex helper."))?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as _,
            std::mem::size_of_val(&limits) as u32,
        )
        .is_err()
            || AssignProcessToJobObject(job, HANDLE(process)).is_err()
        {
            let _ = CloseHandle(job);
            return Err(Failure::error(
                "Could not contain the Codex helper process.",
            ));
        }
        Ok(job)
    }
}

impl Drop for OwnedServer {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.job);
        }
        #[cfg(unix)]
        unsafe {
            if self.process_group > 0 {
                libc::kill(-self.process_group, libc::SIGKILL);
            }
        }
        let _ = self.child.start_kill();
    }
}

// HANDLE is only closed by the owned guard, and never shared or dereferenced.
#[cfg(windows)]
unsafe impl Send for OwnedServer {}

impl OwnedServer {
    async fn start(settings: &AppSettings) -> Result<Self, Failure> {
        let path = discover_codex(settings).ok_or(Failure {
            status: ConnectionStatus::Unavailable,
            message: "Codex was not found. Install Codex or choose its executable in Settings.",
            retry: None,
        })?;
        let mut command = Command::new(path);
        command
            .args([
                "app-server",
                "--listen",
                "stdio://",
                "-c",
                "analytics.enabled=false",
            ])
            .env("CODEX_HOME", codex_home(settings))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        // Start from home so an arbitrary project's config/hooks cannot affect this account-only session.
        if let Some(home) = dirs::home_dir() {
            command.current_dir(home);
        }
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().process_group(0);
        }
        let mut child = command.spawn().map_err(|_| {
            Failure::error("Could not start Codex. Check the executable selected in Settings.")
        })?;
        #[cfg(windows)]
        let job = contain_child(&child)?;
        #[cfg(unix)]
        let process_group = child
            .id()
            .ok_or_else(|| Failure::error("Codex exited before connecting."))?
            as i32;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| Failure::error("Could not connect to Codex input."))?;
        let output = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| Failure::error("Could not connect to Codex output."))?,
        );
        let mut server = Self {
            child,
            input,
            output,
            next_id: 1,
            #[cfg(windows)]
            job,
            #[cfg(unix)]
            process_group,
        };
        server.rpc("initialize", json!({"clientInfo":{"name":"gcd_usage","title":"GCD Usage","version":env!("CARGO_PKG_VERSION")},"capabilities":{}})).await?;
        server.write(json!({"method":"initialized"})).await?;
        Ok(server)
    }

    async fn write(&mut self, value: Value) -> Result<(), Failure> {
        let mut bytes = serde_json::to_vec(&value)
            .map_err(|_| Failure::error("Could not encode a Codex usage request."))?;
        bytes.push(b'\n');
        self.input
            .write_all(&bytes)
            .await
            .map_err(|_| Failure::error("The Codex helper disconnected."))?;
        self.input
            .flush()
            .await
            .map_err(|_| Failure::error("The Codex helper disconnected."))
    }

    async fn rpc(&mut self, method: &str, params: Value) -> Result<Value, Failure> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(json!({"id":id,"method":method,"params":params}))
            .await?;
        loop {
            let mut line = String::new();
            let length = self
                .output
                .read_line(&mut line)
                .await
                .map_err(|_| Failure::error("Could not read the Codex helper response."))?;
            if length == 0 {
                return Err(Failure::error(
                    "The Codex helper closed before returning usage.",
                ));
            }
            if length > MAX_RESPONSE {
                return Err(Failure::error(
                    "The Codex helper returned an unexpectedly large response.",
                ));
            }
            let value: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if value.get("id").and_then(Value::as_u64) == Some(id) && value.get("method").is_none()
            {
                if let Some(error) = value.get("error") {
                    return Err(codex_rpc_failure(error));
                }
                return value
                    .get("result")
                    .cloned()
                    .ok_or_else(|| Failure::error("Codex returned an unexpected usage response."));
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                // An account-only session must never approve model work or external token refresh requests.
                self.write(json!({"id":value["id"],"error":{"code":-32601,"message":"GCD Usage is a read-only account client"}})).await?;
            }
        }
    }

    async fn stop(mut self) {
        // Closing the containment scope terminates any children too; reap the main helper now.
        #[cfg(windows)]
        unsafe {
            let _ = windows::Win32::System::JobObjects::TerminateJobObject(self.job, 0);
        }
        #[cfg(unix)]
        unsafe {
            if self.process_group > 0 {
                libc::kill(-self.process_group, libc::SIGKILL);
            }
        }
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        #[cfg(unix)]
        {
            self.process_group = 0;
        }
    }
}

fn codex_rpc_failure(error: &Value) -> Failure {
    // Inspect known error shapes without propagating raw errors, which can contain auth data.
    let text = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let status = error
        .pointer("/data/status")
        .or_else(|| error.pointer("/data/httpStatus"))
        .and_then(Value::as_u64);
    if status == Some(401)
        || status == Some(403)
        || text.contains("unauthorized")
        || text.contains("not authenticated")
        || text.contains("not logged in")
        || text.contains("authentication required")
    {
        Failure::auth("Codex needs a ChatGPT sign-in. Sign in through Codex, then refresh here.")
    } else if status == Some(429) || text.contains("429") || text.contains("too many requests") {
        Failure {
            status: ConnectionStatus::Error,
            message: "Codex is rate limiting usage checks. Retrying after a short pause.",
            retry: error
                .pointer("/data/retryAfterSeconds")
                .and_then(Value::as_u64)
                .or(Some(120)),
        }
    } else {
        Failure::error(
            "Codex could not return usage. Check your connection and update Codex if needed.",
        )
    }
}

async fn poll_codex(settings: &AppSettings) -> Result<(Option<String>, Vec<QuotaWindow>), Failure> {
    tokio::time::timeout(Duration::from_secs(25), async {
        let mut server = OwnedServer::start(settings).await?;
        let result = async {
            let account = server.rpc("account/read", json!({"refreshToken":false})).await?;
            let account = account.get("account").filter(|v| v.is_object()).ok_or_else(|| Failure::auth("Codex needs a ChatGPT sign-in. Sign in through Codex, then refresh here."))?;
            if account.get("type").and_then(Value::as_str).is_some_and(|t| !matches!(t, "chatgpt" | "chatgptAuthTokens")) {
                return Err(Failure::auth("Codex is using a non-subscription account. Sign in with ChatGPT to read included usage."));
            }
            let local_id = current_account_id(settings, Provider::Codex);
            let account_id = if local_id.starts_with("unknown:") {
                account.get("email").and_then(Value::as_str).map(|e| hash_identity(Provider::Codex, &e.trim().to_lowercase())).unwrap_or(local_id)
            } else { local_id };
            let limits = server.rpc("account/rateLimits/read", json!({})).await?;
            Ok((Some(account_id), parse_codex_windows(&limits)))
        }.await;
        server.stop().await;
        result
    }).await.map_err(|_| Failure::error("Codex usage check timed out. The temporary helper was stopped."))?
}

/// Catalog availability comes from Codex; quality tiers are explicit advisory rules, not measured quality.
fn codex_tier(id: &str) -> u8 {
    if id.contains("astra") {
        5
    } else if id.contains("sol") {
        4
    } else if id == "gpt-5.5" || id.contains("daybreak") {
        3
    } else if id.contains("mini") || id.contains("spark") || id.contains("luna") {
        1
    } else {
        2
    }
}

fn parse_models(value: &Value) -> Vec<ModelCatalogEntry> {
    value
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|m| m.get("hidden").and_then(Value::as_bool) != Some(true))
        .filter_map(|m| {
            let id = m
                .get("model")
                .or_else(|| m.get("id"))
                .and_then(Value::as_str)?;
            let efforts = m
                .get("supportedReasoningEfforts")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|e| {
                    e.get("reasoningEffort")
                        .and_then(Value::as_str)
                        .or_else(|| e.as_str())
                })
                .map(str::to_owned)
                .collect();
            Some(ModelCatalogEntry {
                id: id.into(),
                provider: Provider::Codex,
                label: m
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or(id)
                    .into(),
                efforts,
                quality_tier: codex_tier(id),
                quota_pool: m
                    .get("limitId")
                    .or_else(|| m.get("rateLimitId"))
                    .and_then(Value::as_str)
                    .unwrap_or("codex")
                    .into(),
                available: true,
                source: format!("codex:model/list;catalog:{CATALOG_VERSION}"),
            })
        })
        .collect()
}

pub fn claude_catalog() -> Vec<ModelCatalogEntry> {
    // Versioned from https://code.claude.com/docs/en/model-config. Fable is omitted because its
    // usage-credit billing cannot be inferred from the included five-hour/weekly allowance.
    [
        (
            "claude-haiku-4-5",
            "Claude Haiku 4.5",
            1,
            Vec::<String>::new(),
        ),
        (
            "claude-sonnet-5",
            "Claude Sonnet 5",
            2,
            vec![
                "low".into(),
                "medium".into(),
                "high".into(),
                "xhigh".into(),
                "max".into(),
            ],
        ),
        (
            "claude-opus-5",
            "Claude Opus 5",
            3,
            vec![
                "low".into(),
                "medium".into(),
                "high".into(),
                "xhigh".into(),
                "max".into(),
            ],
        ),
    ]
    .into_iter()
    .map(|(id, label, tier, efforts)| ModelCatalogEntry {
        id: id.into(),
        provider: Provider::Claude,
        label: label.into(),
        efforts,
        quality_tier: tier,
        quota_pool: "claude".into(),
        available: true,
        source: format!("claude:bundled-catalog:{CATALOG_VERSION};availability-unverified"),
    })
    .collect()
}

pub async fn discover_models(settings: &AppSettings) -> Result<Vec<ModelCatalogEntry>, String> {
    let result = tokio::time::timeout(Duration::from_secs(25), async {
        let mut server = OwnedServer::start(settings).await?;
        let result = async {
            let mut catalog = Vec::new();
            let mut cursor: Option<String> = None;
            let mut seen = HashSet::new();
            for _ in 0..20 {
                let response = server
                    .rpc(
                        "model/list",
                        json!({"limit":100,"includeHidden":false,"cursor":cursor}),
                    )
                    .await?;
                catalog.extend(parse_models(&response));
                cursor = response
                    .get("nextCursor")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                match &cursor {
                    Some(next) if seen.insert(next.clone()) => {}
                    _ => break,
                }
            }
            Ok::<_, Failure>(catalog)
        }
        .await;
        server.stop().await;
        result
    })
    .await;
    // Claude remains usable if Codex is not installed. A failed Codex catalog is not invented.
    let mut catalog = match result {
        Ok(Ok(catalog)) => catalog,
        _ => Vec::new(),
    };
    catalog.extend(claude_catalog());
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_can_be_primary_and_map_wins() {
        let windows = parse_codex_windows(
            &json!({"rateLimits":{"primary":{"usedPercent":99,"windowDurationMins":10080}},"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":23,"windowDurationMins":10080,"resetsAt":123},"secondary":{"usedPercent":12,"windowDurationMins":300}},"spark":{"primary":{"usedPercent":60,"windowDurationMins":10080}}}}),
        );
        assert_eq!(windows.len(), 3);
        assert_eq!(
            windows
                .iter()
                .find(|w| w.id == "codex:10080")
                .unwrap()
                .used_percent,
            23.0
        );
        assert_eq!(
            windows
                .iter()
                .find(|w| w.id == "spark:10080")
                .unwrap()
                .used_percent,
            60.0
        );
    }
    #[test]
    fn missing_windows_are_not_zero() {
        assert!(parse_codex_windows(&json!({"rateLimits":{"primary":{"usedPercent":null,"windowDurationMins":10080},"secondary":null}})).is_empty());
        assert!(
            parse_claude_windows(&json!({"five_hour":null,"seven_day":{"utilization":null}}))
                .is_empty()
        );
    }
    #[test]
    fn claude_model_caps_and_reset_are_retained() {
        let windows = parse_claude_windows(
            &json!({"five_hour":{"utilization":0,"resets_at":"2026-09-07T01:00:00Z"},"seven_day":{"utilization":25},"seven_day_sonnet":{"utilization":101},"extra_usage":{"utilization":5}}),
        );
        assert_eq!(windows.len(), 3);
        assert!(windows[0].resets_at.is_some());
        assert_eq!(windows[0].used_percent, 0.0);
        assert!(windows
            .iter()
            .any(|w| w.id == "claude-sonnet:10080" && w.used_percent == 100.0));
    }
    #[test]
    fn errors_are_sanitized_and_retry_is_preserved() {
        let error = codex_rpc_failure(
            &json!({"message":"Unauthorized secret-token user@example.com","data":{"status":401}}),
        );
        assert_eq!(error.status, ConnectionStatus::NeedsAuth);
        assert!(!error.message.contains("secret-token"));
        assert_eq!(http_failure(429, Some(390)).retry, Some(390));
    }
    #[test]
    fn credential_parser_only_accepts_subscription_token() {
        assert!(parse_claude_credentials(json!({"apiKey":"secret"})).is_none());
        assert_eq!(
            parse_claude_credentials(
                json!({"claudeAiOauth":{"accessToken":"fixture-token","expiresAt":0}})
            )
            .unwrap()
            .access_token,
            "fixture-token"
        );
    }
    #[test]
    fn keychain_namespaces_are_stable_isolated_and_unicode_normalized() {
        assert_eq!(claude_keychain_service(None), "Claude Code-credentials");
        assert_eq!(claude_keychain_service(Some("")), "Claude Code-credentials");
        assert_ne!(
            claude_keychain_service(Some("/Users/example/.claude-work")),
            claude_keychain_service(None)
        );
        assert_ne!(
            claude_keychain_service(Some("~/.claude-work")),
            claude_keychain_service(Some("/Users/example/.claude-work"))
        );
        assert_eq!(
            claude_keychain_service(Some("/Users/caf\u{e9}/.claude")),
            claude_keychain_service(Some("/Users/cafe\u{301}/.claude"))
        );
    }
    #[test]
    fn account_ids_are_stable_and_provider_scoped() {
        assert_eq!(
            hash_identity(Provider::Codex, "account-a"),
            hash_identity(Provider::Codex, "account-a")
        );
        assert_ne!(
            hash_identity(Provider::Codex, "account-a"),
            hash_identity(Provider::Claude, "account-a")
        );
        assert!(!hash_identity(Provider::Codex, "someone@example.com").contains('@'));
    }
    #[test]
    fn model_efforts_are_discovered_and_hidden_entries_skipped() {
        let models = parse_models(
            &json!({"data":[{"model":"gpt-6-astra","displayName":"Astra","supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"high"}]},{"model":"hidden","hidden":true}]}),
        );
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].efforts, vec!["low", "high"]);
        assert_eq!(models[0].quality_tier, 5);
    }
}
