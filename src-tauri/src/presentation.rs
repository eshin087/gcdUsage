//! Bounded, plain-text presentation of recorded content. Never interpret markup.
/// Remove only leading, recognized app-injected context blocks. User-authored
/// code and markup elsewhere remain text. Attributes on these wrappers are valid.
pub fn user_text(mut text: &str) -> &str {
    loop {
        text = text.trim_start();
        let Some(tag) = [
            "in-app-browser-context",
            "recommended_plugins",
            "codex_internal_context",
            "environment_context",
            "permissions instructions",
            "app-context",
            "system-reminder",
            "local-command-caveat",
            "command-name",
            "local-command-stdout",
            "task-notification",
        ]
        .into_iter()
        .find(|tag| {
            text.strip_prefix(&format!("<{tag}"))
                .is_some_and(|rest| rest.starts_with('>') || rest.starts_with(char::is_whitespace))
        }) else {
            return text;
        };
        let close = format!("</{tag}>");
        let Some(end) = text.find(&close) else {
            return "";
        };
        text = &text[end + close.len()..];
    }
}
pub fn clean_preview(raw: &str) -> String {
    let mut text = user_text(raw).trim();
    if text.starts_with("<create-pr-command>") {
        return "Create a pull request".into();
    }
    if text.is_empty() && !raw.trim().is_empty() {
        return "App context update (no captured user text)".into();
    }
    if let Some(rest) = text.strip_prefix("<send_user_message_question_reply>") {
        let body = rest
            .split("</send_user_message_question_reply>")
            .next()
            .unwrap_or(rest);
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(body.trim()) {
            let answers = items
                .iter()
                .filter_map(|v| v["answer"].as_str())
                .collect::<Vec<_>>()
                .join("; ");
            if !answers.is_empty() {
                return plain(&format!("Reply: {answers}"), 160);
            }
        }
        return "Reply to an app question (older preview incomplete)".into();
    }
    for tag in ["send_user_message", "user_message"] {
        let start = format!("<{tag}>");
        if let Some(rest) = text.strip_prefix(&start) {
            text = rest.split(&format!("</{tag}>")).next().unwrap_or(rest);
        }
    }
    plain(text, 160)
}
pub fn plain(raw: &str, limit: usize) -> String {
    raw.replace("&#x20;", " ")
        .replace("&nbsp;", " ")
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .filter(
            |c| !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200b}'|'\u{feff}'),
        )
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
}
pub fn project_name(raw: &str) -> Option<String> {
    let name = raw
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("");
    let name = plain(name, 80);
    (!name.is_empty()).then_some(name)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hides_browser_context_attributes_but_preserves_actual_user_code() {
        assert_eq!(clean_preview("<in-app-browser-context source=\"ambient-ui-state\"><div>internal source</div></in-app-browser-context>Fix the toolbar"), "Fix the toolbar");
        assert_eq!(clean_preview("<recommended_plugins>internal list</recommended_plugins>\n<in-app-browser-context source=\"x\">internal</in-app-browser-context>Keep <div> in my code"), "Keep <div> in my code");
        assert!(user_text(
            "<codex_internal_context source=\"goal\">internal</codex_internal_context>"
        )
        .is_empty());
        assert!(
            !clean_preview("<in-app-browser-context source=\"ambient-ui-state\">truncated")
                .contains('<')
        );
        assert_eq!(
            clean_preview("Explain <in-app-browser-context> in this example"),
            "Explain <in-app-browser-context> in this example"
        );
    }
    #[test]
    fn extracts_answers_and_preserves_user_code() {
        assert_eq!(clean_preview("<send_user_message_question_reply>[{\"answer\":\"Last hour\"}]</send_user_message_question_reply>"),"Reply: Last hour");
        assert_eq!(
            clean_preview("<send_user_message>Hello\nworld</send_user_message>"),
            "Hello world"
        );
        assert_eq!(
            clean_preview("Explain <div> & code"),
            "Explain <div> & code"
        );
        assert_eq!(plain("abc\u{202e}xyz\0", 80), "abcxyz");
        assert_eq!(project_name("C:\\private\\Demo"), Some("Demo".into()));
        assert_eq!(clean_preview(&"x".repeat(1000)).len(), 160);
    }
}
