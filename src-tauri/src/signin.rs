//! Runs the provider's unmodified login helper without a console window.
use crate::{models::*, providers};
use serde::Serialize;
use std::{collections::HashMap, process::Stdio, sync::Mutex, time::Duration};
use tauri::{Emitter, Manager};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::mpsc,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub provider: Provider,
    pub phase: String,
    pub message: String,
}
struct Session {
    progress: Progress,
    input: mpsc::Sender<Option<String>>,
}
#[derive(Default)]
pub struct SignIns(Mutex<HashMap<Provider, Session>>);
#[derive(Default)]
pub struct HelperProcesses(Mutex<std::collections::HashSet<u32>>);
pub fn shutdown(app: &tauri::AppHandle) {
    for pid in app.state::<HelperProcesses>().0.lock().unwrap().drain() {
        #[cfg(windows)]
        unsafe {
            use windows::Win32::System::Threading::{
                OpenProcess, TerminateProcess, PROCESS_TERMINATE,
            };
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                let _ = TerminateProcess(handle, 1);
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
    }
}
pub fn statuses(app: &tauri::AppHandle) -> Vec<Progress> {
    app.state::<SignIns>()
        .0
        .lock()
        .unwrap()
        .values()
        .map(|s| s.progress.clone())
        .collect()
}
fn progress(app: &tauri::AppHandle, provider: Provider, phase: &str, message: &str) {
    if let Some(s) = app.state::<SignIns>().0.lock().unwrap().get_mut(&provider) {
        s.progress.phase = phase.into();
        s.progress.message = message.into();
    }
    let _ = app.emit("signin-updated", ());
}
pub fn send(
    app: &tauri::AppHandle,
    provider: Provider,
    code: Option<String>,
) -> Result<(), String> {
    let code = code.map(|s| s.trim().to_owned());
    if code
        .as_ref()
        .is_some_and(|s| s.is_empty() || s.len() > 4096 || s.chars().any(char::is_control))
    {
        return Err("Enter the code shown by the provider".into());
    }
    let state = app.state::<SignIns>();
    let sessions = state.0.lock().unwrap();
    let session = sessions.get(&provider).ok_or("No sign-in is active")?;
    session
        .input
        .try_send(code)
        .map_err(|_| "Sign-in is no longer waiting, or a code is already being checked".into())
}
async fn discard(mut output: impl AsyncRead + Unpin) -> Result<(), ()> {
    let mut buffer = [0u8; 4096];
    let mut total = 0;
    loop {
        let count = output.read(&mut buffer).await.map_err(|_| ())?;
        if count == 0 {
            return Ok(());
        }
        total += count;
        if total > 1024 * 1024 {
            return Err(());
        }
    }
}
pub fn start(
    app: tauri::AppHandle,
    settings: AppSettings,
    provider: Provider,
    sso: bool,
) -> Result<(), String> {
    let path = match provider {
        Provider::Claude => providers::discover_claude(&settings),
        Provider::Codex => providers::discover_codex(&settings),
    }
    .ok_or("Install the coding tool or choose its executable in Settings")?;
    let (sender, mut receiver) = mpsc::channel(2);
    {
        let state = app.state::<SignIns>();
        let mut sessions = state.0.lock().unwrap();
        if sessions
            .get(&provider)
            .is_some_and(|s| ["checking", "waiting"].contains(&s.progress.phase.as_str()))
        {
            return Ok(());
        }
        sessions.insert(
            provider,
            Session {
                input: sender,
                progress: Progress {
                    provider,
                    phase: "checking".into(),
                    message: "Checking your existing sign-in…".into(),
                },
            },
        );
    }
    tauri::async_runtime::spawn(async move {
        if !sso {
            let snapshot = providers::poll(provider, &settings).await;
            if matches!(receiver.try_recv(), Ok(None)) {
                progress(&app, provider, "cancelled", "Sign-in cancelled.");
                return;
            }
            if snapshot.status == ConnectionStatus::Connected {
                progress(
                    &app,
                    provider,
                    "connected",
                    "Your existing sign-in is connected. No login needed.",
                );
                crate::request_refresh(&app);
                return;
            }
            if snapshot.status != ConnectionStatus::NeedsAuth {
                progress(
                    &app,
                    provider,
                    "failed",
                    "Could not verify the connection. Check your network, then try again.",
                );
                return;
            }
        }
        if matches!(receiver.try_recv(), Ok(None)) {
            progress(&app, provider, "cancelled", "Sign-in cancelled.");
            return;
        }
        let mut command = Command::new(path);
        match provider {
            Provider::Claude => {
                command.args(["auth", "login"]);
                if sso {
                    command.arg("--sso");
                }
                if let Some(home) = &settings.claude_home {
                    command.env("CLAUDE_CONFIG_DIR", home);
                }
            }
            Provider::Codex => {
                command.arg("login");
                if let Some(home) = &settings.codex_home {
                    command.env("CODEX_HOME", home);
                }
            }
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let Ok(mut child) = command.spawn() else {
            progress(
                &app,
                provider,
                "failed",
                "Could not start the provider's sign-in. Check its installation.",
            );
            return;
        };
        let mut input = child.stdin.take().unwrap();
        let pid = child.id().unwrap();
        app.state::<HelperProcesses>().0.lock().unwrap().insert(pid);
        let mut output = tokio::spawn(discard(child.stdout.take().unwrap()));
        let mut errors = tokio::spawn(discard(child.stderr.take().unwrap()));
        progress(
            &app,
            provider,
            "waiting",
            "Complete the provider's sign-in in your browser. If it shows a code, enter it below.",
        );
        let deadline = tokio::time::sleep(Duration::from_secs(300));
        tokio::pin!(deadline);
        let mut output_done = false;
        let mut errors_done = false;
        let result = loop {
            tokio::select! {
                status = child.wait() => { app.state::<HelperProcesses>().0.lock().unwrap().remove(&pid); break if status.is_ok_and(|s| s.success()) { "complete" } else { "failed" } },
                _ = &mut deadline => break "timeout",
                value = receiver.recv() => match value {
                    Some(Some(code)) => { if input.write_all(format!("{code}\n").as_bytes()).await.is_err() { break "failed"; } },
                    _ => break "cancelled",
                },
                result = &mut output, if !output_done => { output_done = true; if !matches!(result, Ok(Ok(()))) { break "failed"; } },
                result = &mut errors, if !errors_done => { errors_done = true; if !matches!(result, Ok(Ok(()))) { break "failed"; } },
            }
        };
        let _ = child.kill().await;
        app.state::<HelperProcesses>()
            .0
            .lock()
            .unwrap()
            .remove(&pid);
        output.abort();
        errors.abort();
        if result == "complete" {
            let snapshot = providers::poll(provider, &settings).await;
            if snapshot.status == ConnectionStatus::Connected {
                progress(
                    &app,
                    provider,
                    "connected",
                    "Connected. Your meters are refreshing.",
                );
            } else {
                progress(
                    &app,
                    provider,
                    "failed",
                    "Sign-in finished, but usage is not available yet. Try Refresh.",
                );
            }
            crate::request_refresh(&app);
        } else {
            progress(&app, provider, result, match result { "cancelled" => "Sign-in cancelled.", "timeout" => "Sign-in timed out. Try again when ready.", _ => "Sign-in did not complete. Open the provider's app to sign in, then check the connection again." });
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn login_output_is_discarded_and_bounded() {
        assert!(discard(&b"provider sign-in output"[..]).await.is_ok());
        assert!(discard(&vec![0u8; 1024 * 1024 + 1][..]).await.is_err());
    }
}
