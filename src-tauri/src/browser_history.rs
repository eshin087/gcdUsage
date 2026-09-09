//! Legacy records retained for existing databases and versioned sync compatibility.
//! Manual browser import was retired; no export parser or import entry point remains.
use crate::storage::stable_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPrompt {
    #[serde(default)]
    pub chat_title: Option<String>,
    pub id: String,
    pub provider: String,
    pub account_id: String,
    pub account_label: String,
    pub conversation_id: String,
    pub message_id: String,
    pub timestamp: Option<i64>,
    pub preview: String,
    pub models: Vec<String>,
    pub effort: Option<String>,
    pub replies: u32,
}
impl BrowserPrompt {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .chat_title
            .as_ref()
            .is_some_and(|s| s.chars().count() > 120 || s.chars().any(char::is_control))
        {
            return Err("Invalid conversation title".into());
        }
        if !["claude", "chatgpt"].contains(&self.provider.as_str())
            || self.id
                != stable_id(&[
                    "browser",
                    &self.provider,
                    &self.account_id,
                    &self.conversation_id,
                    &self.message_id,
                ])
            || [&self.account_id, &self.conversation_id, &self.message_id]
                .iter()
                .any(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            || self.account_label.is_empty()
            || self.account_label.chars().count() > 80
            || self.preview.chars().count() > 160
            || self.models.len() > 32
            || self.models.iter().any(|s| s.len() > 256)
            || self.effort.as_ref().is_some_and(|s| s.len() > 128)
            || self
                .timestamp
                .is_some_and(|t| !(0..=253_402_300_799).contains(&t))
        {
            return Err("Invalid browser history record".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowserFilter {
    pub query: String,
    pub provider: String,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub offset: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPage {
    pub items: Vec<BrowserPrompt>,
    pub total: u64,
    pub conversations: u64,
    pub replies: u64,
}
