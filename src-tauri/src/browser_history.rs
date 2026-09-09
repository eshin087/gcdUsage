//! Explicit imports of user-selected conversation exports. No browser sessions are read.
use crate::storage::{stable_id, Store, SyncEvent};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserImportReport {
    pub added: u64,
    pub duplicates: u64,
    pub skipped: u64,
}
fn timestamp(value: &Value) -> Option<i64> {
    value
        .as_f64()
        .filter(|t| t.is_finite() && *t >= 0. && *t <= 253_402_300_799.)
        .map(|t| t as i64)
        .or_else(|| {
            value
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|t| t.timestamp())
        })
}
fn text(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        return s.chars().take(160).collect();
    }
    if let Some(s) = value.get("text").and_then(Value::as_str) {
        return s.chars().take(160).collect();
    }
    let parts = value
        .get("parts")
        .and_then(Value::as_array)
        .or_else(|| value.as_array());
    parts
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().or_else(|| v.get("text").and_then(Value::as_str)))
                .collect::<Vec<_>>()
                .join("\n")
                .chars()
                .take(160)
                .collect()
        })
        .unwrap_or_default()
}
fn parse_conversation(
    value: &Value,
    account: &str,
    label: &str,
) -> Result<(Vec<BrowserPrompt>, u64), String> {
    let (provider, conversation_id) = if value.get("mapping").is_some() {
        (
            "chatgpt",
            value.get("id").or_else(|| value.get("conversation_id")),
        )
    } else if value.get("chat_messages").is_some() {
        ("claude", value.get("uuid").or_else(|| value.get("id")))
    } else {
        return Err("This file does not contain a supported conversation export".into());
    };
    let Some(conversation_id) = conversation_id.and_then(Value::as_str) else {
        return Ok((vec![], 1));
    };
    let account_id = stable_id(&[provider, account]);
    let mut records = vec![];
    let mut skipped = 0;
    let messages: Vec<(&str, &Value, Vec<&Value>)> = if provider == "chatgpt" {
        let mapping = value["mapping"]
            .as_object()
            .ok_or("Invalid conversation mapping")?;
        mapping
            .iter()
            .filter_map(|(id, node)| {
                let message = node.get("message")?;
                if message.pointer("/author/role")?.as_str()? != "user" {
                    return None;
                }
                let children = node
                    .get("children")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|id| mapping.get(id.as_str()?))
                    .filter_map(|n| n.get("message"))
                    .filter(|m| {
                        m.pointer("/author/role").and_then(Value::as_str) == Some("assistant")
                    })
                    .collect();
                Some((
                    message.get("id").and_then(Value::as_str).unwrap_or(id),
                    message,
                    children,
                ))
            })
            .collect()
    } else {
        let messages = value["chat_messages"]
            .as_array()
            .ok_or("Invalid conversation messages")?;
        messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m["sender"].as_str() == Some("human"))
            .map(|(index, m)| {
                let following = messages
                    .iter()
                    .skip(index + 1)
                    .take_while(|m| m["sender"].as_str() != Some("human"))
                    .filter(|m| m["sender"].as_str() == Some("assistant"))
                    .collect();
                (
                    m.get("uuid")
                        .or_else(|| m.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    m,
                    following,
                )
            })
            .collect()
    };
    for (message_id, message, replies) in messages {
        if message_id.is_empty() {
            skipped += 1;
            continue;
        }
        let preview = text(
            message
                .get("content")
                .filter(|v| !v.is_null())
                .unwrap_or(&message["text"]),
        );
        let preview = if preview.is_empty() {
            text(&message["text"])
        } else {
            preview
        };
        let mut models: Vec<String> = replies
            .iter()
            .filter_map(|m| m.pointer("/metadata/model_slug").or_else(|| m.get("model")))
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        models.sort();
        models.dedup();
        let effort = replies
            .iter()
            .filter_map(|m| {
                m.pointer("/metadata/reasoning_effort")
                    .and_then(Value::as_str)
            })
            .next()
            .map(str::to_owned);
        let row = BrowserPrompt {
            chat_title: value
                .get("title")
                .or_else(|| value.get("name"))
                .and_then(Value::as_str)
                .map(|s| crate::presentation::plain(s, 120)),
            id: stable_id(&[
                "browser",
                provider,
                &account_id,
                conversation_id,
                message_id,
            ]),
            provider: provider.into(),
            account_id: account_id.clone(),
            account_label: label.into(),
            conversation_id: conversation_id.into(),
            message_id: message_id.into(),
            timestamp: timestamp(
                message
                    .get("create_time")
                    .or_else(|| message.get("created_at"))
                    .unwrap_or(&Value::Null),
            ),
            preview,
            models,
            effort,
            replies: replies.len() as u32,
        };
        row.validate()?;
        records.push(row);
    }
    Ok((records, skipped))
}
pub fn import_file(
    store: &mut Store,
    path: &Path,
    account_label: &str,
) -> Result<BrowserImportReport, String> {
    let label = account_label.trim();
    if label.is_empty() || label.chars().count() > 80 || label.chars().any(char::is_control) {
        return Err("Enter an account label of 1–80 characters".into());
    }
    // Imports are explicit and bounded; no archive is unpacked into the filesystem.
    let bytes = crate::safety::read_file_limited(path, 64 * 1024 * 1024)
        .map_err(|_| "Choose a conversations JSON export smaller than 64 MB")?;
    import_bytes(store, &bytes, label)
}
fn import_bytes(
    store: &mut Store,
    bytes: &[u8],
    label: &str,
) -> Result<BrowserImportReport, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| "The selected file is not valid JSON")?;
    let conversations = value
        .as_array()
        .or_else(|| value.get("conversations").and_then(Value::as_array))
        .ok_or("Choose the conversations JSON file from your export")?;
    if conversations.len() > 50000 {
        return Err("This export has too many conversations for one import".into());
    }
    let account = label.to_lowercase();
    let mut report = BrowserImportReport::default();
    store.begin()?;
    let result = (|| {
        for conversation in conversations {
            let (rows, skipped) = parse_conversation(conversation, &account, label)?;
            report.skipped += skipped;
            for row in rows {
                if store.apply_event(&SyncEvent::BrowserPrompt(row), true)? {
                    report.added += 1;
                } else {
                    report.duplicates += 1;
                }
            }
        }
        store.commit()
    })();
    if result.is_err() {
        store.rollback();
    }
    result?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn exports_deduplicate_across_imports_and_preserve_unknowns() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let data = json!([{"id":"chat", "mapping":{"a":{"message":{"id":"m","author":{"role":"user"},"create_time":100,"content":{"parts":["hello"]}},"children":["b"]},"b":{"message":{"author":{"role":"assistant"},"metadata":{"model_slug":"example-model"}}}}}, {"uuid":"c", "chat_messages":[{"uuid":"u","sender":"human","created_at":"2026-01-01T00:00:00Z","text":"hello"},{"uuid":"v","sender":"assistant","text":"answer"}]}]);
        let bytes = serde_json::to_vec(&data).unwrap();
        assert_eq!(
            import_bytes(&mut store, &bytes, "Personal").unwrap().added,
            2
        );
        assert_eq!(
            import_bytes(&mut store, &bytes, "Personal")
                .unwrap()
                .duplicates,
            2
        );
        assert_eq!(import_bytes(&mut store, &bytes, "Work").unwrap().added, 2);
        let page = store.browser_history(&BrowserFilter::default()).unwrap();
        assert_eq!(page.total, 4);
        assert!(page.items.iter().all(|r| r.effort.is_none()));
        assert_eq!(store.stats(None, None).unwrap().request_count, 0);
    }
    #[test]
    fn malformed_export_rolls_back_and_preview_is_bounded() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let good = json!({"uuid":"c","chat_messages":[{"uuid":"u","sender":"human","text":"💡".repeat(300)}]});
        let bad = serde_json::to_vec(&json!([good, {"unrecognized":true}])).unwrap();
        assert!(import_bytes(&mut store, &bad, "Personal").is_err());
        assert_eq!(
            store
                .browser_history(&BrowserFilter::default())
                .unwrap()
                .total,
            0
        );
        import_bytes(
            &mut store,
            &serde_json::to_vec(&json!([good])).unwrap(),
            "Personal",
        )
        .unwrap();
        let row = store
            .browser_history(&BrowserFilter::default())
            .unwrap()
            .items
            .remove(0);
        assert_eq!(row.preview.chars().count(), 160);
        assert!(row.timestamp.is_none());
    }
}
