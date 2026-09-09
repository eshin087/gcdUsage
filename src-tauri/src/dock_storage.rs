use super::*;
use crate::dock::{DockModel, DockPrompt, DockSummary};

impl Store {
    pub fn dock_summary(&self, minutes: u32, now: i64) -> Result<DockSummary, String> {
        if !(1..=43200).contains(&minutes) || now <= 0 {
            return Err("Invalid dock interval".into());
        }
        let from = (now - minutes as i64 * 60).max(0);
        // Reasoning is a subset of output, never an additional summand.
        let total = "coalesce(json_extract(data,'$.tokens.input'),0)+coalesce(json_extract(data,'$.tokens.cacheRead'),0)+coalesce(json_extract(data,'$.tokens.cacheWrite'),0)+coalesce(json_extract(data,'$.tokens.output'),0)";
        let measured = "json_extract(data,'$.tokens.input') IS NOT NULL OR json_extract(data,'$.tokens.cacheRead') IS NOT NULL OR json_extract(data,'$.tokens.cacheWrite') IS NOT NULL OR json_extract(data,'$.tokens.output') IS NOT NULL";
        let (tokens, unknown_requests) = self.connection.query_row(
            &format!("SELECT coalesce(sum({total}),0),coalesce(sum(CASE WHEN {measured} THEN 0 ELSE 1 END),0) FROM requests WHERE timestamp>=?1 AND timestamp<?2"),
            params![from, now], |r| Ok((r.get::<_,u64>(0)?, r.get::<_,u64>(1)?))) .map_err(|e| e.to_string())?;
        let mut statement = self.connection.prepare(&format!("SELECT provider,model,effort,coalesce(sum({total}),0),count(*) FROM requests WHERE timestamp>=?1 AND timestamp<?2 GROUP BY provider,model,effort ORDER BY sum({total}) DESC,provider,model,effort LIMIT 64")).map_err(|e|e.to_string())?;
        let models = statement
            .query_map(params![from, now], |r| {
                Ok(DockModel {
                    provider: r.get(0)?,
                    model: r.get(1)?,
                    effort: r.get(2)?,
                    tokens: r.get(3)?,
                    requests: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut statement=self.connection.prepare(&format!("SELECT provider,coalesce(sum({total}),0) FROM requests WHERE timestamp>=?1 AND timestamp<?2 GROUP BY provider")).map_err(|e|e.to_string())?;
        let provider_tokens = statement
            .query_map(params![from, now], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let mut prompts = vec![];
        for provider in [Provider::Claude, Provider::Codex] {
            let recent: Vec<PromptRecord> = self.query_json("SELECT data FROM prompts WHERE provider=?1 AND kind='user' ORDER BY timestamp DESC,id LIMIT 10",params![provider.key()])?;
            for p in recent {
                let requests:Vec<RequestUsage> = self.query_json("SELECT data FROM requests WHERE prompt_id=?1 AND provider=?2 AND account_id=?3 ORDER BY timestamp",params![p.id,provider.key(),p.account_id])?;
                let mut tokens = TokenUsage::default();
                let mut models = std::collections::BTreeSet::new();
                for r in &requests {
                    tokens.add(&r.tokens);
                    models.insert(format!(
                        "{} · {}",
                        r.model,
                        r.effort.as_deref().unwrap_or("unknown")
                    ));
                }
                let known = [
                    tokens.input,
                    tokens.cache_read,
                    tokens.cache_write,
                    tokens.output,
                ]
                .iter()
                .any(Option::is_some);
                prompts.push(DockPrompt {
                    context: format!(
                        "{} / {}",
                        p.project.as_deref().unwrap_or("Project unknown"),
                        p.chat_title
                            .as_deref()
                            .unwrap_or(&p.session_id.chars().take(8).collect::<String>())
                    ),
                    id: format!("local:{}", p.id),
                    provider: provider.key().into(),
                    preview: crate::presentation::clean_preview(&p.preview),
                    timestamp: Some(p.timestamp),
                    tokens: known.then(|| tokens.total()),
                    models: if models.is_empty() {
                        "Unknown model / reasoning".into()
                    } else {
                        models.into_iter().take(8).collect::<Vec<_>>().join(" / ")
                    },
                    status: p.status,
                    browser: false,
                });
            }
            let browser_provider = if provider == Provider::Codex {
                "chatgpt"
            } else {
                "claude"
            };
            let browser:Vec<crate::browser_history::BrowserPrompt> = self.query_json("SELECT data FROM browser_prompts WHERE provider=?1 ORDER BY timestamp DESC,id LIMIT 10",params![browser_provider])?;
            for p in browser {
                prompts.push(DockPrompt {
                    context: format!(
                        "Browser / {}",
                        p.chat_title
                            .as_deref()
                            .unwrap_or(&p.conversation_id.chars().take(8).collect::<String>())
                    ),
                    id: format!("browser:{}", p.id),
                    provider: provider.key().into(),
                    preview: crate::presentation::clean_preview(&p.preview),
                    timestamp: p.timestamp,
                    tokens: None,
                    models: format!(
                        "{} · {}",
                        if p.models.is_empty() {
                            "Unknown model".into()
                        } else {
                            p.models.join(" / ")
                        },
                        p.effort.as_deref().unwrap_or("unknown")
                    ),
                    status: "imported".into(),
                    browser: true,
                });
            }
        }
        prompts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then_with(|| a.id.cmp(&b.id)));
        // Keep enough records for each provider's last ten as well as combined last ten.
        let mut counts = HashMap::new();
        prompts.retain(|p| {
            let n = counts.entry(p.provider.clone()).or_insert(0);
            *n += 1;
            *n <= 10
        });
        Ok(DockSummary {
            updated_at: Some(now),
            minutes,
            tokens,
            unknown_requests,
            provider_tokens,
            models,
            prompts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dock_interval_counts_cache_once_and_excludes_end_and_other_accounts() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let prompt = serde_json::json!({"id":"p","provider":"claude","accountId":"a","deviceId":"d","sessionId":"s","turnId":"t","timestamp":50,"preview":"hello","status":"completed","kind":"user"});
        store.connection.execute("INSERT INTO prompts(id,provider,account_id,device_id,session_id,turn_id,timestamp,preview,status,kind,data) VALUES('p','claude','a','d','s','t',50,'hello','completed','user',?1)",params![prompt.to_string()]).unwrap();
        for (id, time, account) in [
            ("start", 40, "a"),
            ("inside", 99, "a"),
            ("end", 100, "a"),
            ("other", 99, "b"),
        ] {
            let request = serde_json::json!({"id":id,"promptId":"p","provider":"claude","accountId":account,"deviceId":"d","sessionId":"s","timestamp":time,"model":"test-model","effort":null,"tokens":{"input":10,"cacheRead":20,"cacheWrite":3,"output":5,"reasoning":4},"kind":"user","source":"fixture","sourceEventId":id});
            store.connection.execute("INSERT INTO requests(id,prompt_id,provider,account_id,device_id,session_id,timestamp,model,effort,kind,data) VALUES(?1,'p','claude',?2,'d','s',?3,'test-model',NULL,'user',?4)",params![id,account,time,request.to_string()]).unwrap();
        }
        let s = store.dock_summary(1, 100).unwrap();
        assert_eq!(s.tokens, 114); // Three interval requests across accounts; no reasoning duplication.
        assert_eq!(s.models[0].requests, 3);
        assert_eq!(s.prompts[0].tokens, Some(114)); // Three lifetime requests of account a only.
        assert!(s.prompts[0].models.contains("unknown"));
        assert_eq!(
            store.dock_summary(0, 100).unwrap_err(),
            "Invalid dock interval"
        );
        assert_eq!(store.dock_summary(1, 200).unwrap().tokens, 0);
    }
    #[test]
    fn dock_returns_ten_prompts_per_provider_and_preserves_browser_unknowns() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        for provider in ["claude", "chatgpt"] {
            for i in 0..14 {
                let message = format!("m{i}");
                let p = crate::browser_history::BrowserPrompt {
                    chat_title: None,
                    id: stable_id(&["browser", provider, "a", "s", &message]),
                    provider: provider.into(),
                    account_id: "a".into(),
                    account_label: "Test".into(),
                    conversation_id: "s".into(),
                    message_id: message,
                    timestamp: Some(i),
                    preview: "<private & literal>".into(),
                    models: vec![],
                    effort: None,
                    replies: 1,
                };
                store
                    .apply_event(&SyncEvent::BrowserPrompt(p), true)
                    .unwrap();
            }
        }
        let s = store.dock_summary(60, 100).unwrap();
        assert_eq!(s.prompts.len(), 20);
        assert_eq!(s.recent(Some("claude")).len(), 10);
        assert_eq!(s.recent(None).len(), 10);
        assert!(s.prompts.iter().all(|p| p.tokens.is_none() && p.browser));
        assert_eq!(s.tokens, 0);
        assert_eq!(s.prompts[0].timestamp, Some(13));
    }
}
