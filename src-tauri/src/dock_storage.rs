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
            prompts.extend(self.dock_prompt_page(Some(provider.key()), None, 10)?.prompts);
        }
        prompts.sort_by(|a,b| b.timestamp.cmp(&a.timestamp).then_with(|| a.id.cmp(&b.id)));
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
    /// Bounded history reads, independent of the selected token interval.
    pub fn dock_prompt_page(
        &self, provider: Option<&str>, before: Option<&crate::dock::DockCursor>, limit: usize,
    ) -> Result<crate::dock::DockPage, String> {
        if !(1..=100).contains(&limit) || provider.is_some_and(|v| v != "claude" && v != "codex") {
            return Err("Invalid history page".into());
        }
        let timestamp = before.map(|c| c.timestamp);
        let id = before.map(|c| c.id.as_str()).unwrap_or("");
        let mut recent: Vec<PromptRecord> = self.query_json(
            "SELECT data FROM prompts WHERE kind='user' AND provider IN ('claude','codex')
             AND (?1 IS NULL OR provider=?1)
             AND (?2 IS NULL OR timestamp<?2 OR (timestamp=?2 AND id>?3))
             ORDER BY timestamp DESC,id ASC LIMIT ?4",
            params![provider,timestamp,id,(limit+1) as i64])?;
        let more = recent.len() > limit;
        recent.truncate(limit);
        let next = if more { recent.last().map(|p| crate::dock::DockCursor {
            timestamp:p.timestamp,id:p.id.clone()
        }) } else { None };
        let mut prompts=Vec::with_capacity(recent.len());
        for p in recent {
            // Aggregate in SQLite rather than materializing every tool-loop request.
            // Reasoning overlaps output. Provider/account identity must match.
            let totals: (Option<u64>, Option<u64>, Option<u64>, Option<u64>) =
                self.connection.query_row(
                    "SELECT sum(json_extract(data,'$.tokens.input')),
                            sum(json_extract(data,'$.tokens.cacheRead')),
                            sum(json_extract(data,'$.tokens.cacheWrite')),
                            sum(json_extract(data,'$.tokens.output'))
                     FROM requests WHERE prompt_id=?1 AND provider=?2 AND account_id=?3",
                    params![p.id,p.provider.key(),p.account_id],
                    |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e| e.to_string())?;
            let tokens=[totals.0,totals.1,totals.2,totals.3];
            let mut statement=self.connection.prepare(
                "SELECT DISTINCT model,effort FROM requests
                 WHERE prompt_id=?1 AND provider=?2 AND account_id=?3
                 ORDER BY model,effort LIMIT 8").map_err(|e|e.to_string())?;
            let models=statement.query_map(params![p.id,p.provider.key(),p.account_id], |r| {
                Ok(format!("{} · {}",r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?.unwrap_or("unknown".into())))
            }).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
            prompts.push(DockPrompt {
                context:format!("{} / {}",p.project.as_deref().unwrap_or("Project unknown"),
                    p.chat_title.as_deref().unwrap_or(&p.session_id.chars().take(8).collect::<String>())),
                id:format!("local:{}",p.id),provider:p.provider.key().into(),
                preview:crate::presentation::clean_preview(&p.preview),
                timestamp:Some(p.timestamp),
                tokens:tokens.iter().any(Option::is_some).then(|| tokens.iter().flatten().fold(0u64,|sum,n|sum.saturating_add(*n))),
                models:if models.is_empty() {"Unknown model / reasoning".into()} else {models.join(" / ")},
                status:p.status,browser:false,
            });
        }
        Ok(crate::dock::DockPage {prompts,next})
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_page_prompt(store: &Store, id: &str, provider: &str, timestamp: i64) {
        let data=serde_json::json!({"id":id,"provider":provider,"accountId":"a","deviceId":"d",
            "sessionId":"s","turnId":id,"timestamp":timestamp,"preview":"Synthetic prompt",
            "status":"completed","kind":"user"});
        store.connection.execute(
            "INSERT INTO prompts(id,provider,account_id,device_id,session_id,turn_id,timestamp,preview,status,kind,data)
             VALUES(?1,?2,'a','d','s',?1,?3,'Synthetic prompt','completed','user',?4)",
            params![id,provider,timestamp,data.to_string()]).unwrap();
    }
    #[test]
    fn history_pages_pass_ten_without_duplicates_across_equal_times_and_new_imports() {
        let store=Store::open(Path::new(":memory:")).unwrap();
        for i in 0..105 {
            insert_page_prompt(&store,&format!("p{i:03}"),if i%2==0 {"claude"} else {"codex"},100-i/3);
        }
        for provider in [None,Some("claude"),Some("codex")] {
            let mut cursor=None;
            let mut ids=std::collections::BTreeSet::new();
            loop {
                let page=store.dock_prompt_page(provider,cursor.as_ref(),17).unwrap();
                assert!(page.prompts.len()<=17);
                for p in page.prompts {
                    assert!(provider.is_none_or(|v|v==p.provider));
                    assert_eq!(p.tokens,None);
                    assert!(ids.insert(p.id),"A prompt appeared on more than one page");
                }
                if page.next.is_none() {break;}
                cursor=page.next;
            }
            assert_eq!(ids.len(),match provider {None=>105,Some("claude")=>53,_=>52});
        }
        let first=store.dock_prompt_page(None,None,17).unwrap();
        insert_page_prompt(&store,"newer","codex",200);
        let next=store.dock_prompt_page(None,first.next.as_ref(),17).unwrap();
        assert!(next.prompts.iter().all(|p|p.id!="local:newer" &&
            first.prompts.iter().all(|old|old.id!=p.id)));
        assert!(store.dock_prompt_page(Some("other"),None,17).is_err());
        assert!(store.dock_prompt_page(None,None,101).is_err());
    }
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
    fn dock_excludes_retired_browser_records_without_deleting_them() {
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
        assert!(s.prompts.is_empty());
        assert_eq!(s.tokens, 0);
        let retained: i64 = store.connection.query_row("SELECT count(*) FROM browser_prompts", [], |r| r.get(0)).unwrap();
        assert_eq!(retained, 28);
    }
}
