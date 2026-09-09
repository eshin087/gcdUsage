use crate::models::TokenUsage;
use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDetail {
    pub id: String,
    pub provider: String,
    pub project: Option<String>,
    pub chat_title: Option<String>,
    pub conversation_id: String,
    pub turn_id: String,
    pub preview: String,
    pub timestamp: Option<i64>,
    pub tokens: TokenUsage,
    pub models: Vec<String>,
    pub request_count: u64,
    pub original_url: Option<String>,
    pub limitation: String,
}
pub fn browser_url(provider: &str, conversation: &str) -> Option<String> {
    let id = uuid::Uuid::parse_str(conversation)
        .ok()?
        .hyphenated()
        .to_string();
    match provider {
        "chatgpt" => Some(format!("https://chatgpt.com/c/{id}")),
        "claude" => Some(format!("https://claude.ai/chat/{id}")),
        _ => None,
    }
}
pub fn launch_original(url: &str) -> Result<(), String> {
    // Reconstruct and compare rather than trusting a saved or frontend-supplied URL.
    let parsed = tauri::Url::parse(url).map_err(|_| "Invalid conversation link")?;
    let provider = match parsed.host_str() {
        Some("chatgpt.com") => "chatgpt",
        Some("claude.ai") => "claude",
        _ => return Err("Unsupported conversation link".into()),
    };
    let id = parsed
        .path_segments()
        .and_then(|p| p.last())
        .ok_or("Invalid conversation link")?;
    if browser_url(provider, id).as_deref() != Some(url) {
        return Err("Invalid conversation link".into());
    }
    launch_url(url)
}
/// No frontend URL input: only this release's reviewed documentation destination.
pub fn launch_pro_documentation() -> Result<(), String> {
    launch_url("https://help.openai.com/en/articles/20001354-gpt-56-and-gpt-6-pro-in-chatgpt")
}
fn launch_url(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        use windows::core::PCWSTR;
        use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let result = ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if result.0 as isize <= 32 {
            return Err("Could not open your browser".into());
        }
    }
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("/usr/bin/open")
            .arg(url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| "Could not open your browser")?;
        if !status.success() {
            return Err("Could not open your browser".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_cannot_escape_the_provider_origin() {
        assert!(browser_url("chatgpt", "../../evil").is_none());
        assert!(browser_url("claude", "123?x=1").is_none());
        assert_eq!(
            browser_url("chatgpt", "12345678-1234-1234-1234-123456789abc").unwrap(),
            "https://chatgpt.com/c/12345678-1234-1234-1234-123456789abc"
        );
        for bad in [
            "file:///tmp/a",
            "https://claude.ai.evil/chat/123",
            "https://chatgpt.com/c/123?evil=1",
        ] {
            assert!(launch_original(bad).is_err());
        }
    }
}
