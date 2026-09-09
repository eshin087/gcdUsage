use super::*;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
};
impl Store {
    /// One streaming pass per source identity enriches previews without replaying
    /// usage, changing account attribution, or resetting import checkpoints.
    pub(crate) fn enrich_file(
        &mut self,
        path: &Path,
        provider: Provider,
        identity: &str,
    ) -> Result<(), String> {
        let key = path.to_string_lossy();
        let identity = format!("context-v2:{identity}");
        let done: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM enriched_files WHERE path=?1 AND identity=?2)",
                params![key, identity],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if done {
            return Ok(());
        }
        let mut reader = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
        let mut buffer = Vec::new();
        let mut session = String::new();
        let mut project = None;
        let mut title = None;
        let mut previews = HashMap::new();
        loop {
            buffer.clear();
            let n = reader
                .by_ref()
                .take(16 * 1024 * 1024 + 1)
                .read_until(b'\n', &mut buffer)
                .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if buffer.last() != Some(&b'\n') {
                while buffer.last() != Some(&b'\n') {
                    buffer.clear();
                    if reader
                        .by_ref()
                        .take(65536)
                        .read_until(b'\n', &mut buffer)
                        .map_err(|e| e.to_string())?
                        == 0
                    {
                        break;
                    }
                }
                continue;
            }
            let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buffer) else {
                continue;
            };
            let p = &v["payload"];
            if v["type"] == "session_meta" {
                session = p["id"].as_str().unwrap_or("").into();
                project = p["cwd"]
                    .as_str()
                    .and_then(crate::presentation::project_name);
                title = p["title"]
                    .as_str()
                    .map(|s| crate::presentation::plain(s, 120));
            }
            if let Some(s) = v["sessionId"].as_str() {
                session = s.into()
            }
            if let Some(s) = v["cwd"].as_str() {
                project = crate::presentation::project_name(s)
            }
            if let Some(s) = v["customTitle"].as_str() {
                title = Some(crate::presentation::plain(s, 120))
            }
            let body = if v["type"] == "event_msg" && p["type"] == "user_message" {
                Some(crate::history::content_text(&p["message"]))
            } else if v["type"] == "response_item" && p["role"] == "user" {
                Some(crate::history::content_text(&p["content"]))
            } else if provider == Provider::Claude && v["type"] == "user" {
                Some(crate::history::content_text(&v["message"]["content"]))
            } else {
                None
            };
            if let Some(body) = body.filter(|s| !s.is_empty()) {
                let cleaned = crate::presentation::clean_preview(&body);
                let context_only = crate::history::is_context_only(&body);
                if title.is_none() && !context_only {
                    title = Some(crate::presentation::plain(&cleaned, 120))
                }
                if previews.len() < 20000 {
                    let prefix = body.chars().take(160).collect::<String>();
                    for prefix in [
                        prefix.clone(),
                        crate::presentation::plain(&body, 160),
                        crate::presentation::clean_preview(&prefix),
                        cleaned.clone(),
                    ] {
                        let key = (crate::history::event_time(&v), prefix);
                        let value = (cleaned.clone(), context_only);
                        previews
                            .entry(key)
                            .and_modify(|old: &mut Option<(String, bool)>| {
                                if old.as_ref() != Some(&value) {
                                    *old = None;
                                }
                            })
                            .or_insert(Some(value));
                    }
                }
            }
        }
        self.begin()?;
        let result = (|| {
            if !session.is_empty() {
                let rows: Vec<PromptRecord> = self.query_json(
                    "SELECT data FROM prompts WHERE provider=?1 AND session_id=?2",
                    params![provider.key(), session],
                )?;
                for mut p in rows {
                    p.project = project.clone().or(p.project);
                    p.chat_title = title.clone().or(p.chat_title);
                    if let Some(Some((cleaned, context_only))) =
                        previews.get(&(p.timestamp, p.preview.clone()))
                    {
                        p.preview = cleaned.clone();
                        if *context_only {
                            p.kind = ActivityKind::Background;
                        }
                    } else {
                        p.preview = crate::presentation::clean_preview(&p.preview);
                    }
                    self.save_prompt(&p)?;
                }
            }
            self.connection.execute("INSERT INTO enriched_files VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET identity=excluded.identity",params![key,identity]).map_err(|e|e.to_string())?;
            Ok(())
        })();
        match result {
            Ok(()) => self.commit(),
            Err(e) => {
                self.rollback();
                Err(e)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_context_repair_separates_repeated_headers_and_retains_records() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let header = format!(
            "<in-app-browser-context source=\"ambient-ui-state\">{}</in-app-browser-context>",
            "internal markup \n".repeat(30)
        );
        let mut events = vec![
            serde_json::json!({"type":"session_meta","payload":{"id":"context-session","cwd":"/synthetic/Project"}}),
        ];
        for (i, body) in [
            format!("{header}First real prompt"),
            format!("{header}Second real prompt"),
            header.clone(),
        ]
        .into_iter()
        .enumerate()
        {
            let mut p = super::super::tests::prompt();
            p.id = format!("context-{i}");
            p.provider = Provider::Codex;
            p.session_id = "context-session".into();
            p.timestamp = 100 + i as i64;
            p.preview = crate::presentation::plain(&body, 160);
            store.save_prompt(&p).unwrap();
            events.push(serde_json::json!({"type":"event_msg","timestamp":p.timestamp,"payload":{"type":"user_message","message":body}}));
        }
        let file = std::env::temp_dir().join(format!("gcd-context-{}.jsonl", uuid::Uuid::new_v4()));
        std::fs::write(
            &file,
            events.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        store.enrich_file(&file, Provider::Codex, "source").unwrap();
        assert_eq!(
            store.prompt("context-0").unwrap().unwrap().preview,
            "First real prompt"
        );
        assert_eq!(
            store.prompt("context-1").unwrap().unwrap().preview,
            "Second real prompt"
        );
        assert_eq!(
            store.prompt("context-2").unwrap().unwrap().kind,
            ActivityKind::Background
        );
        assert_eq!(store.history(&HistoryFilter::default()).unwrap().total, 2);
        std::fs::remove_file(file).unwrap();
    }
    #[test]
    fn enrichment_recovers_wrapped_answer_without_replaying_usage() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let mut p = super::super::tests::prompt();
        let raw=format!("<send_user_message_question_reply>[{{\"question\":\"{}\",\"answer\":\"Last hour\"}}]</send_user_message_question_reply>","long question ".repeat(30));
        p.preview = raw.chars().take(160).collect();
        p.session_id = "session-enrich".into();
        p.provider = Provider::Codex;
        store.save_prompt(&p).unwrap();
        let file = std::env::temp_dir().join(format!("gcd-enrich-{}.jsonl", uuid::Uuid::new_v4()));
        let content = format!(
            "{}\n{}\n",
            serde_json::json!({"type":"session_meta","payload":{"id":"session-enrich","cwd":"C:/private/ExampleProject","title":"Example chat"}}),
            serde_json::json!({"type":"event_msg","timestamp":p.timestamp,"payload":{"type":"user_message","message":raw}})
        );
        std::fs::write(&file, content).unwrap();
        store
            .enrich_file(&file, Provider::Codex, "identity")
            .unwrap();
        let saved = store.prompt(&p.id).unwrap().unwrap();
        assert_eq!(saved.preview, "Reply: Last hour");
        assert_eq!(saved.project.as_deref(), Some("ExampleProject"));
        assert_eq!(saved.chat_title.as_deref(), Some("Example chat"));
        assert_eq!(saved.account_id, p.account_id);
        assert_eq!(saved.timestamp, p.timestamp);
        assert_eq!(
            store
                .connection
                .query_row("SELECT count(*) FROM requests", [], |r| r.get::<_, u64>(0))
                .unwrap(),
            0
        );
        store
            .enrich_file(&file, Provider::Codex, "identity")
            .unwrap();
        std::fs::remove_file(file).unwrap();
    }
}
