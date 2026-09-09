//! Local, versioned storage. Provider logs and credentials are never written here.
#[path = "dock_storage.rs"]
mod dock_storage;
#[path = "metrics.rs"]
mod metrics;
use crate::models::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

pub fn stable_id(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "record", rename_all = "snake_case")]
pub enum SyncEvent {
    BrowserPrompt(crate::browser_history::BrowserPrompt),
    Prompt(PromptRecord),
    Request(RequestUsage),
    Snapshot(QuotaSnapshot),
    Link {
        request_id: String,
        provider: Provider,
        account_id: String,
        turn_id: String,
    },
}

pub(crate) fn validate_measurements(event: &SyncEvent) -> Result<(), String> {
    let time = |value: i64| (0..=253_402_300_799).contains(&value);
    let valid = match event {
        SyncEvent::BrowserPrompt(p) => return p.validate(),
        SyncEvent::Prompt(p) => time(p.timestamp) && p.completed_at.is_none_or(time),
        SyncEvent::Request(r) => {
            time(r.timestamp)
                && [
                    r.tokens.input,
                    r.tokens.cache_read,
                    r.tokens.cache_write,
                    r.tokens.output,
                    r.tokens.reasoning,
                ]
                .into_iter()
                .flatten()
                .all(|n| n <= 1_000_000_000_000)
        }
        SyncEvent::Snapshot(s) => {
            time(s.fetched_at)
                && s.retry_after_seconds.is_none_or(|n| n <= 86400)
                && s.windows.iter().all(|w| {
                    w.resets_at.is_none_or(time) && (1..=525600).contains(&w.duration_minutes)
                })
        }
        SyncEvent::Link { .. } => true,
    };
    if valid {
        Ok(())
    } else {
        Err("Invalid history measurement".into())
    }
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn browser_history(
        &self,
        filter: &crate::browser_history::BrowserFilter,
    ) -> Result<crate::browser_history::BrowserPage, String> {
        use crate::browser_history::{BrowserPage, BrowserPrompt};
        if !["", "claude", "chatgpt"].contains(&filter.provider.as_str())
            || filter.query.len() > 1024
            || filter.from.zip(filter.to).is_some_and(|(a, b)| a >= b)
        {
            return Err("Choose valid browser history filters".into());
        }
        let clause = "(?1='' OR provider=?1) AND (?2='' OR instr(lower(preview),lower(?2))>0) AND (?3 IS NULL OR timestamp>=?3) AND (?4 IS NULL OR timestamp<?4)";
        let args = params![filter.provider, filter.query, filter.from, filter.to];
        let total: u64 = self
            .connection
            .query_row(
                &format!("SELECT COUNT(*) FROM browser_prompts WHERE {clause}"),
                args,
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let conversations: u64 = self.connection.query_row(&format!("SELECT COUNT(*) FROM (SELECT DISTINCT provider,account_id,conversation_id FROM browser_prompts WHERE {clause})"), args, |r|r.get(0)).map_err(|e|e.to_string())?;
        let replies: u64 = self.connection.query_row(&format!("SELECT COALESCE(SUM(json_extract(data,'$.replies')),0) FROM browser_prompts WHERE {clause}"), args, |r|r.get(0)).map_err(|e|e.to_string())?;
        let items: Vec<BrowserPrompt> = self.query_json(&format!("SELECT data FROM browser_prompts WHERE {clause} ORDER BY timestamp DESC,id LIMIT 50 OFFSET ?5"),params![filter.provider,filter.query,filter.from,filter.to,filter.offset])?;
        Ok(BrowserPage {
            items,
            total,
            conversations,
            replies,
        })
    }
    pub fn sync_inventory(&self) -> Result<(Vec<String>, u64), String> {
        let mut query = self
            .connection
            .prepare("SELECT hash FROM batches ORDER BY hash LIMIT 10001")
            .map_err(|e| e.to_string())?;
        let hashes = query
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())?;
        let pending = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM events WHERE outbound=1 AND exported=0",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok((hashes, pending))
    }
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;
            CREATE TABLE IF NOT EXISTS prompts(id TEXT PRIMARY KEY, provider TEXT NOT NULL, account_id TEXT NOT NULL, device_id TEXT NOT NULL, session_id TEXT NOT NULL, turn_id TEXT NOT NULL, timestamp INTEGER NOT NULL, preview TEXT NOT NULL, status TEXT NOT NULL, kind TEXT NOT NULL, data TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS prompts_time ON prompts(timestamp DESC);
            CREATE INDEX IF NOT EXISTS prompts_turn ON prompts(provider,account_id,turn_id);
            CREATE TABLE IF NOT EXISTS requests(id TEXT PRIMARY KEY, prompt_id TEXT, provider TEXT NOT NULL, account_id TEXT NOT NULL, device_id TEXT NOT NULL, session_id TEXT NOT NULL, timestamp INTEGER NOT NULL, model TEXT NOT NULL, effort TEXT, kind TEXT NOT NULL, data TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS requests_prompt ON requests(prompt_id);
            CREATE INDEX IF NOT EXISTS requests_time ON requests(timestamp);
            CREATE TABLE IF NOT EXISTS snapshots(id TEXT PRIMARY KEY, provider TEXT NOT NULL, account_id TEXT NOT NULL, device_id TEXT NOT NULL, timestamp INTEGER NOT NULL, data TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS snapshots_time ON snapshots(provider,account_id,timestamp);
            CREATE TABLE IF NOT EXISTS checkpoints(path TEXT PRIMARY KEY, offset INTEGER NOT NULL, identity TEXT NOT NULL, state TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS events(hash TEXT PRIMARY KEY, data TEXT NOT NULL, outbound INTEGER NOT NULL, exported INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS batches(hash TEXT PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS request_links(request_id TEXT PRIMARY KEY, provider TEXT NOT NULL, account_id TEXT NOT NULL, turn_id TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS browser_prompts(id TEXT PRIMARY KEY, provider TEXT NOT NULL, account_id TEXT NOT NULL, conversation_id TEXT NOT NULL, timestamp INTEGER, preview TEXT NOT NULL, data TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS browser_time ON browser_prompts(timestamp DESC);
").map_err(|e|e.to_string())?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version < 2 {
            // Earlier releases updated JSON while retaining the indexed timestamp.
            // Restore consistency without changing record identity or token data.
            connection.execute_batch("BEGIN; UPDATE prompts SET data=json_set(data,'$.timestamp',timestamp,'$.sessionId',session_id) WHERE json_extract(data,'$.timestamp')!=timestamp OR json_extract(data,'$.sessionId')!=session_id; UPDATE requests SET data=json_set(data,'$.timestamp',timestamp,'$.sessionId',session_id) WHERE json_extract(data,'$.timestamp')!=timestamp OR json_extract(data,'$.sessionId')!=session_id; PRAGMA user_version=2; COMMIT;").map_err(|e|e.to_string())?;
        }
        Ok(Self { connection })
    }

    pub fn checkpoint(&self, path: &str) -> Result<Option<(u64, String, String)>, String> {
        self.connection
            .query_row(
                "SELECT offset,identity,state FROM checkpoints WHERE path=?1",
                [path],
                |r| Ok((r.get::<_, i64>(0)? as u64, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())
    }
    pub fn save_checkpoint(
        &self,
        path: &str,
        offset: u64,
        identity: &str,
        state: &str,
    ) -> Result<(), String> {
        self.connection.execute("INSERT INTO checkpoints VALUES(?1,?2,?3,?4) ON CONFLICT(path) DO UPDATE SET offset=excluded.offset,identity=excluded.identity,state=excluded.state",params![path,offset as i64,identity,state]).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn begin(&self) -> Result<(), String> {
        self.connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())
    }
    pub fn commit(&self) -> Result<(), String> {
        self.connection
            .execute_batch("COMMIT")
            .map_err(|e| e.to_string())
    }
    pub fn rollback(&self) {
        let _ = self.connection.execute_batch("ROLLBACK");
    }

    pub fn save_prompt(&mut self, prompt: &PromptRecord) -> Result<bool, String> {
        self.apply_event(&SyncEvent::Prompt(prompt.clone()), true)
    }
    pub fn save_request(&mut self, request: &RequestUsage) -> Result<bool, String> {
        self.apply_event(&SyncEvent::Request(request.clone()), true)
    }
    pub fn save_snapshot(&mut self, snapshot: &QuotaSnapshot) -> Result<(), String> {
        self.apply_event(&SyncEvent::Snapshot(snapshot.clone()), true)
            .map(|_| ())
    }
    pub fn link_to_turn(
        &mut self,
        request_id: &str,
        provider: Provider,
        account_id: &str,
        turn_id: &str,
    ) -> Result<(), String> {
        self.apply_event(
            &SyncEvent::Link {
                request_id: request_id.into(),
                provider,
                account_id: account_id.into(),
                turn_id: turn_id.into(),
            },
            true,
        )
        .map(|_| ())
    }
    pub fn prompt(&self, id: &str) -> Result<Option<PromptRecord>, String> {
        self.read_one("SELECT data FROM prompts WHERE id=?1", id)
    }
    pub fn request(&self, id: &str) -> Result<Option<RequestUsage>, String> {
        self.read_one("SELECT data FROM requests WHERE id=?1", id)
    }
    fn read_one<T: serde::de::DeserializeOwned>(
        &self,
        sql: &str,
        id: &str,
    ) -> Result<Option<T>, String> {
        let data: Option<String> = self
            .connection
            .query_row(sql, [id], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        data.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
            .transpose()
    }

    pub fn apply_event(&mut self, event: &SyncEvent, outbound: bool) -> Result<bool, String> {
        validate_measurements(event)?;
        let mut normalized = event.clone();
        match &mut normalized {
            SyncEvent::BrowserPrompt(p) => {
                if let Some(prior) = self.read_one::<crate::browser_history::BrowserPrompt>(
                    "SELECT data FROM browser_prompts WHERE id=?1",
                    &p.id,
                )? {
                    p.timestamp = prior.timestamp.or(p.timestamp);
                }
            }
            SyncEvent::Prompt(p) => {
                p.preview = p.preview.chars().take(160).collect();
                if let Some(prior) = self.prompt(&p.id)? {
                    p.timestamp = prior.timestamp;
                    p.session_id = prior.session_id;
                }
            }
            SyncEvent::Request(r) => {
                if let Some(prior) = self.request(&r.id)? {
                    r.timestamp = prior.timestamp;
                    r.session_id = prior.session_id;
                }
            }
            _ => {}
        }
        // Provider messages can contain local installation details; never replicate them.
        let mut exported = normalized.clone();
        if let SyncEvent::Snapshot(s) = &mut exported {
            s.message = None;
        }
        let data = serde_json::to_string(&exported).map_err(|e| e.to_string())?;
        let hash = stable_id(&[&data]);
        if self
            .connection
            .query_row("SELECT 1 FROM events WHERE hash=?1", [&hash], |_| Ok(()))
            .optional()
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Ok(false);
        }
        let inserted = match normalized {
            SyncEvent::BrowserPrompt(p) => {
                let exists: bool = self
                    .connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM browser_prompts WHERE id=?1)",
                        [&p.id],
                        |r| r.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                self.connection.execute("INSERT INTO browser_prompts VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET timestamp=COALESCE(browser_prompts.timestamp,excluded.timestamp),preview=excluded.preview,data=excluded.data", params![p.id,p.provider,p.account_id,p.conversation_id,p.timestamp,p.preview,json(&p)?]).map_err(|e|e.to_string())?;
                !exists
            }
            SyncEvent::Prompt(mut p) => {
                let old = self.prompt(&p.id)?;
                if let Some(ref prior) = old {
                    if prior.provider != p.provider
                        || prior.account_id != p.account_id
                        || prior.timestamp != p.timestamp
                        || prior.session_id != p.session_id
                    {
                        return Err("Conflicting prompt identity".into());
                    }
                    p.device_id = prior.device_id.clone().min(p.device_id);
                    p.completed_at = prior.completed_at.max(p.completed_at);
                    if status_rank(&prior.status) > status_rank(&p.status) {
                        p.status = prior.status.clone();
                    }
                    if p.preview.is_empty() {
                        p.preview = prior.preview.clone();
                    }
                }
                self.connection.execute("INSERT INTO prompts VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(id) DO UPDATE SET device_id=excluded.device_id,turn_id=excluded.turn_id,preview=excluded.preview,status=excluded.status,kind=excluded.kind,data=excluded.data",params![p.id,p.provider.key(),p.account_id,p.device_id,p.session_id,p.turn_id,p.timestamp,p.preview,p.status,kind_key(p.kind),json(&p)?]).map_err(|e|e.to_string())?;
                old.is_none()
            }
            SyncEvent::Request(mut r) => {
                let old = self.request(&r.id)?;
                if let Some(ref prior) = old {
                    if prior.provider != r.provider
                        || prior.account_id != r.account_id
                        || prior.timestamp != r.timestamp
                        || prior.session_id != r.session_id
                    {
                        return Err("Conflicting request identity".into());
                    }
                    r.device_id = prior.device_id.clone().min(r.device_id);
                    if r.prompt_id.is_none() {
                        r.prompt_id = prior.prompt_id.clone();
                    }
                    if r.model == "unknown" {
                        r.model = prior.model.clone();
                    }
                    if r.effort.is_none() {
                        r.effort = prior.effort.clone();
                    }
                    r.tokens = maximum_tokens(&r.tokens, &prior.tokens);
                }
                self.connection.execute("INSERT INTO requests VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(id) DO UPDATE SET prompt_id=excluded.prompt_id,device_id=excluded.device_id,model=excluded.model,effort=excluded.effort,kind=excluded.kind,data=excluded.data",params![r.id,r.prompt_id,r.provider.key(),r.account_id,r.device_id,r.session_id,r.timestamp,r.model,r.effort,kind_key(r.kind),json(&r)?]).map_err(|e|e.to_string())?;
                old.is_none()
            }
            SyncEvent::Snapshot(s) => {
                if s.windows.iter().any(|w| {
                    !w.used_percent.is_finite() || !(0.0..=100.0).contains(&w.used_percent)
                }) {
                    return Err("Invalid quota percentage".into());
                }
                self.connection
                    .execute(
                        "INSERT OR IGNORE INTO snapshots VALUES(?1,?2,?3,?4,?5,?6)",
                        params![
                            s.id,
                            s.provider.key(),
                            s.account_id,
                            s.device_id,
                            s.fetched_at,
                            json(&s)?
                        ],
                    )
                    .map_err(|e| e.to_string())?
                    > 0
            }
            SyncEvent::Link {
                request_id,
                provider,
                account_id,
                turn_id,
            } => {
                if self
                    .request(&request_id)?
                    .is_some_and(|r| r.provider != provider || r.account_id != account_id)
                {
                    return Err("Conflicting request link identity".into());
                }
                self.connection
                    .execute(
                        "INSERT OR IGNORE INTO request_links VALUES(?1,?2,?3,?4)",
                        params![request_id, provider.key(), account_id, turn_id],
                    )
                    .map_err(|e| e.to_string())?
                    > 0
            }
        };
        self.connection
            .execute(
                "INSERT OR IGNORE INTO events(hash,data,outbound) VALUES(?1,?2,?3)",
                params![hash, data, outbound],
            )
            .map_err(|e| e.to_string())?;
        Ok(inserted)
    }

    pub fn resolve_links(&mut self) -> Result<(), String> {
        self.connection.execute("DELETE FROM request_links WHERE EXISTS(SELECT 1 FROM requests r WHERE r.id=request_links.request_id AND (r.provider!=request_links.provider OR r.account_id!=request_links.account_id))", []).map_err(|e|e.to_string())?;
        let links: Vec<(String, String)> = {
            let mut q=self.connection.prepare("SELECT l.request_id,p.id FROM request_links l JOIN requests r ON r.id=l.request_id AND r.provider=l.provider AND r.account_id=l.account_id JOIN prompts p ON p.provider=l.provider AND p.account_id=l.account_id AND p.turn_id=l.turn_id AND p.timestamp<=r.timestamp WHERE p.kind='user' ORDER BY p.timestamp DESC").map_err(|e|e.to_string())?;
            let rows = q
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
        };
        let mut seen = HashSet::new();
        for (request_id, prompt_id) in links {
            if !seen.insert(request_id.clone()) {
                continue;
            }
            if let Some(mut r) = self.request(&request_id)? {
                if r.prompt_id.as_deref() != Some(&prompt_id) {
                    r.prompt_id = Some(prompt_id);
                    self.save_request(&r)?;
                }
            }
        }
        Ok(())
    }
    pub fn latest_snapshots(&self) -> Result<Vec<QuotaSnapshot>, String> {
        self.query_json("SELECT data FROM snapshots s WHERE timestamp=(SELECT MAX(timestamp) FROM snapshots WHERE provider=s.provider AND device_id=s.device_id) ORDER BY timestamp DESC",[])
    }
    pub fn all_prompts(&self) -> Result<Vec<PromptRecord>, String> {
        self.query_json("SELECT data FROM prompts ORDER BY timestamp", [])
    }
    pub fn all_requests(&self) -> Result<Vec<RequestUsage>, String> {
        self.query_json("SELECT data FROM requests ORDER BY timestamp", [])
    }
    fn query_json<T: serde::de::DeserializeOwned, P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<T>, String> {
        let mut q = self.connection.prepare(sql).map_err(|e| e.to_string())?;
        let rows = q
            .query_map(params, |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.map(|r| {
            r.map_err(|e| e.to_string())
                .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        })
        .collect()
    }
    pub fn history(&self, filter: &HistoryFilter) -> Result<HistoryPage, String> {
        let mut terms = vec!["p.kind='user'".to_string()];
        let mut args: Vec<SqlValue> = vec![];
        macro_rules! value {
            ($opt:expr,$sql:expr) => {
                if let Some(v) = $opt {
                    terms.push($sql.into());
                    args.push(v.into());
                }
            };
        }
        value!(filter.provider.map(|p| p.key().to_string()), "p.provider=?");
        value!(filter.device_id.clone(), "p.device_id=?");
        value!(filter.from, "p.timestamp>=?");
        value!(filter.to, "p.timestamp<=?");
        value!(
            filter.query.as_ref().map(|q| format!(
                "%{}%",
                q.replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )),
            "p.preview LIKE ? ESCAPE '\\'"
        );
        let mut request_terms = vec![
            "r.prompt_id=p.id AND r.provider=p.provider AND r.account_id=p.account_id".to_string(),
        ];
        if let Some(model) = &filter.model {
            request_terms.push("r.model=?".into());
            args.push(model.clone().into());
        }
        if let Some(effort) = &filter.effort {
            if effort == "unknown" {
                request_terms.push("r.effort IS NULL".into());
            } else {
                request_terms.push("r.effort=?".into());
                args.push(effort.clone().into());
            }
        }
        if request_terms.len() > 1 {
            terms.push(format!(
                "EXISTS(SELECT 1 FROM requests r WHERE {})",
                request_terms.join(" AND ")
            ));
        }
        let clause = terms.join(" AND ");
        let total: i64 = self
            .connection
            .query_row(
                &format!("SELECT COUNT(*) FROM prompts p WHERE {clause}"),
                params_from_iter(args.iter()),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        args.push(filter.limit.unwrap_or(100).clamp(1, 1000).into());
        args.push(filter.offset.unwrap_or(0).into());
        let prompts:Vec<PromptRecord>=self.query_json(&format!("SELECT p.data FROM prompts p WHERE {clause} ORDER BY p.timestamp DESC,p.id LIMIT ? OFFSET ?"),params_from_iter(args.iter()))?;
        let mut items = Vec::with_capacity(prompts.len());
        for prompt in prompts {
            let requests = self.query_json::<RequestUsage, _>(
                "SELECT data FROM requests WHERE prompt_id=?1 AND provider=?2 AND account_id=?3 ORDER BY timestamp",
                params![prompt.id, prompt.provider.key(), prompt.account_id],
            )?;
            let mut tokens = TokenUsage::default();
            for r in &requests {
                tokens.add(&r.tokens);
            }
            let quota_estimate = self.quota_estimate(&prompt)?;
            items.push(HistoryItem {
                prompt,
                requests,
                tokens,
                quota_estimate,
            });
        }
        Ok(HistoryPage {
            items,
            total: total as u64,
        })
    }

    /// Attribution is deliberately conservative: one fully bounded completed prompt,
    /// one device, no unattributed work, unchanged reset, and <=5 minute sampling gap.
    pub fn quota_estimate(&self, prompt: &PromptRecord) -> Result<Option<QuotaEstimate>, String> {
        Ok(self.quota_estimates(prompt)?.into_iter().next())
    }

    fn quota_estimates(&self, prompt: &PromptRecord) -> Result<Vec<QuotaEstimate>, String> {
        if prompt.status != "completed"
            || prompt.kind != ActivityKind::User
            || prompt.account_id.starts_with("local-unverified")
            || prompt.account_id.starts_with("unknown:")
        {
            return Ok(vec![]);
        }
        let before: Vec<QuotaSnapshot> = self.query_json("SELECT data FROM snapshots WHERE provider=?1 AND account_id=?2 AND device_id=?3 AND timestamp<=?4 ORDER BY timestamp DESC LIMIT 1", params![prompt.provider.key(),prompt.account_id,prompt.device_id,prompt.timestamp])?;
        let Some(first) = before.first() else {
            return Ok(vec![]);
        };
        let after: Vec<QuotaSnapshot> = self.query_json("SELECT data FROM snapshots WHERE provider=?1 AND account_id=?2 AND device_id=?3 AND timestamp>?4 AND timestamp<=?5 ORDER BY timestamp LIMIT 1", params![prompt.provider.key(),prompt.account_id,prompt.device_id,prompt.timestamp,first.fetched_at+300])?;
        let Some(last) = after.first() else {
            return Ok(vec![]);
        };
        if prompt
            .completed_at
            .is_none_or(|t| t > last.fetched_at || t < prompt.timestamp)
            || first.status != ConnectionStatus::Connected
            || last.status != ConnectionStatus::Connected
        {
            return Ok(vec![]);
        }
        let requests:Vec<RequestUsage> = self.query_json("SELECT data FROM requests WHERE provider=?1 AND account_id=?2 AND timestamp>?3 AND timestamp<=?4",params![prompt.provider.key(),prompt.account_id,first.fetched_at,last.fetched_at])?;
        if requests.is_empty()
            || requests.iter().any(|r| {
                r.prompt_id.as_deref() != Some(&prompt.id)
                    || r.device_id != prompt.device_id
                    || matches!(r.kind, ActivityKind::Background | ActivityKind::Review)
            })
        {
            return Ok(vec![]);
        }
        let outside:i64=self.connection.query_row("SELECT COUNT(*) FROM requests WHERE prompt_id=?1 AND (timestamp<=?2 OR timestamp>?3) AND provider=?4 AND account_id=?5",params![prompt.id,first.fetched_at,last.fetched_at,prompt.provider.key(),prompt.account_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        // Earlier prompts still in progress can consume allowance even before their next request record arrives.
        let competing:i64=self.connection.query_row("SELECT COUNT(*) FROM prompts WHERE provider=?1 AND account_id=?2 AND id!=?3 AND timestamp<=?5 AND (json_extract(data,'$.completedAt') IS NULL OR json_extract(data,'$.completedAt')>?4)",params![prompt.provider.key(),prompt.account_id,prompt.id,first.fetched_at,last.fetched_at],|r|r.get(0)).map_err(|e|e.to_string())?;
        if outside > 0 || competing > 0 {
            return Ok(vec![]);
        }
        let mut windows: Vec<_> = first.windows.iter().collect();
        windows.sort_by_key(|w| std::cmp::Reverse(w.duration_minutes));
        let mut estimates = vec![];
        for a in windows {
            let Some(b) = last
                .windows
                .iter()
                .find(|w| w.id == a.id && w.duration_minutes == a.duration_minutes)
            else {
                continue;
            };
            if a.resets_at.is_none()
                || a.resets_at != b.resets_at
                || a.resets_at.is_some_and(|t| t <= last.fetched_at)
                || b.used_percent < a.used_percent
            {
                continue;
            }
            let percent = b.used_percent - a.used_percent;
            estimates.push(QuotaEstimate {percent,window_id:a.id.clone(),confidence:"estimated".into(),explanation:if percent==0.0 {"No visible quota change in an isolated completed interval. Reporting can round small consumption to zero; this does not mean the prompt was free.".into()}else{"Quota change in a short interval containing only this completed prompt on this computer. Rounding and unobserved activity can affect the estimate.".into()}});
        }
        Ok(estimates)
    }

    pub fn stats(&self, from: Option<i64>, to: Option<i64>) -> Result<DashboardStats, String> {
        let prompts: Vec<PromptRecord> = self.query_json(
            "SELECT data FROM prompts WHERE timestamp>=?1 AND timestamp<=?2",
            params![from.unwrap_or(i64::MIN), to.unwrap_or(i64::MAX)],
        )?;
        let users: HashMap<_, _> = prompts
            .iter()
            .filter(|p| p.kind == ActivityKind::User)
            .map(|p| (p.id.clone(), p))
            .collect();
        let mut result = DashboardStats::default();
        result.prompt_count = users.len() as u64;
        result.conversation_count = users
            .values()
            .map(|p| format!("{}:{}:{}", p.provider.key(), p.account_id, p.session_id))
            .collect::<HashSet<_>>()
            .len() as u64;
        let mut daily: BTreeMap<String, DailyStat> = BTreeMap::new();
        let mut devices = HashSet::new();
        let mut per_prompt: HashMap<String, u64> = users.keys().map(|id| (id.clone(), 0)).collect();
        type GroupKey = (Provider, String, String, Option<String>);
        #[derive(Default)]
        struct Group {
            requests: u64,
            total: u64,
            prompts: HashMap<String, u64>,
        }
        let mut groups: HashMap<GroupKey, Group> = HashMap::new();
        let mut prompt_models: HashMap<String, HashSet<(String, Option<String>)>> = HashMap::new();
        for p in users.values() {
            let date = day(p.timestamp);
            daily
                .entry(date.clone())
                .or_insert_with(|| DailyStat {
                    date,
                    ..Default::default()
                })
                .prompts += 1;
            devices.insert(p.device_id.clone());
        }
        // Stream request rows: dashboard aggregation does not retain every request record.
        let mut query = self
            .connection
            .prepare("SELECT data FROM requests WHERE timestamp>=?1 AND timestamp<=?2")
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map(
                params![from.unwrap_or(i64::MIN), to.unwrap_or(i64::MAX)],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| e.to_string())?;
        for row in rows {
            let mut r: RequestUsage = serde_json::from_str(&row.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            r.prompt_id = r.prompt_id.filter(|id| {
                users
                    .get(id)
                    .is_some_and(|p| p.provider == r.provider && p.account_id == r.account_id)
            });
            result.request_count += 1;
            result.token_totals.add(&r.tokens);
            devices.insert(r.device_id.clone());
            if matches!(r.kind, ActivityKind::Review | ActivityKind::Background)
                || r.prompt_id
                    .as_ref()
                    .is_none_or(|id| !users.contains_key(id))
            {
                result.background_requests += 1;
            }
            let total = r.tokens.total();
            if let Some(id) = &r.prompt_id {
                if let Some(value) = per_prompt.get_mut(id) {
                    *value = value.saturating_add(total);
                }
                prompt_models
                    .entry(id.clone())
                    .or_default()
                    .insert((r.model.clone(), r.effort.clone()));
            }
            let date = day(r.timestamp);
            let daily_tokens = &mut daily
                .entry(date.clone())
                .or_insert_with(|| DailyStat {
                    date,
                    ..Default::default()
                })
                .tokens;
            *daily_tokens = daily_tokens.saturating_add(total);
            let group = groups
                .entry((r.provider, r.account_id, r.model, r.effort))
                .or_default();
            group.requests += 1;
            group.total = group.total.saturating_add(total);
            if let Some(id) = r.prompt_id.filter(|id| users.contains_key(id)) {
                let value = group.prompts.entry(id).or_default();
                *value = value.saturating_add(total);
            }
        }
        result.total_tokens = result.token_totals.total();
        let mut samples: Vec<_> = per_prompt.values().copied().collect();
        samples.sort_unstable();
        result.median_tokens = percentile(&samples, 0.5);
        result.p75_tokens = percentile(&samples, 0.75);
        let mut quota_cache: HashMap<String, Vec<QuotaEstimate>> = HashMap::new();
        for ((provider, account, model, effort), group) in groups {
            let mut consumption = vec![];
            let mut window_quotas: BTreeMap<String, Vec<f64>> = BTreeMap::new();
            let mut completed = 0;
            for (id, tokens) in &group.prompts {
                let p = users[id];
                if p.status == "completed" {
                    completed += 1;
                    consumption.push(*tokens);
                }
                if prompt_models
                    .get(id)
                    .is_some_and(|models| models.len() == 1)
                {
                    if !quota_cache.contains_key(id) {
                        quota_cache.insert(id.clone(), self.quota_estimates(p)?);
                    }
                    for q in &quota_cache[id] {
                        window_quotas
                            .entry(q.window_id.clone())
                            .or_default()
                            .push(q.percent);
                    }
                }
            }
            consumption.sort_unstable();
            let base = ModelStat {
                provider,
                account_id: Some(account),
                quota_window_id: None,
                model,
                effort,
                prompt_count: group.prompts.len() as u64,
                completed_prompts: completed,
                request_count: group.requests,
                total_tokens: group.total,
                median_tokens: percentile(&consumption, 0.5),
                p75_tokens: percentile(&consumption, 0.75),
                estimated_quota_per_prompt: None,
                quota_sample_count: 0,
            };
            if window_quotas.is_empty() {
                result.model_stats.push(base);
            } else {
                for (window, quotas) in window_quotas {
                    let mut row = base.clone();
                    row.quota_window_id = Some(window);
                    row.quota_sample_count = quotas.len() as u64;
                    let average = quotas.iter().sum::<f64>() / quotas.len() as f64;
                    // All-zero samples mean below reporting resolution, never free usage.
                    row.estimated_quota_per_prompt = (average > 0.0).then_some(average);
                    result.model_stats.push(row);
                }
            }
        }
        result.model_stats.sort_by(|a, b| {
            b.total_tokens
                .cmp(&a.total_tokens)
                .then_with(|| a.model.cmp(&b.model))
                .then_with(|| a.quota_window_id.cmp(&b.quota_window_id))
        });
        for (id, prompt) in &users {
            if !quota_cache.contains_key(id) {
                quota_cache.insert(id.clone(), self.quota_estimates(prompt)?);
            }
        }
        result.quota_allocations = self.quota_allocations(from, to, &users, &quota_cache)?;
        result.daily = daily.into_values().collect();
        result.computers = devices.into_iter().collect();
        result.computers.sort();
        Ok(result)
    }
    /// Changes across consecutive snapshots are disjoint, even when different computers
    /// contribute samples. Reset boundaries and missing coverage are retained as gaps.
    fn quota_allocations(
        &self,
        from: Option<i64>,
        to: Option<i64>,
        prompts: &HashMap<String, &PromptRecord>,
        estimates: &HashMap<String, Vec<QuotaEstimate>>,
    ) -> Result<Vec<QuotaAllocation>, String> {
        type Key = (Provider, String, String);
        struct Aggregate {
            row: QuotaAllocation,
            previous_time: i64,
            previous_used: f64,
            previous_reset: Option<i64>,
            observed: f64,
        }
        let mut groups: HashMap<Key, Aggregate> = HashMap::new();
        let mut query=self.connection.prepare("SELECT data FROM snapshots WHERE timestamp>=?1 AND timestamp<=?2 ORDER BY timestamp,id").map_err(|e|e.to_string())?;
        let rows = query
            .query_map(
                params![from.unwrap_or(i64::MIN), to.unwrap_or(i64::MAX)],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| e.to_string())?;
        for row in rows {
            let snapshot: QuotaSnapshot = serde_json::from_str(&row.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if snapshot.status != ConnectionStatus::Connected {
                continue;
            }
            for window in snapshot.windows {
                let key = (
                    snapshot.provider,
                    snapshot.account_id.clone(),
                    window.id.clone(),
                );
                if let Some(group) = groups.get_mut(&key) {
                    if snapshot.fetched_at == group.previous_time {
                        group.previous_used = group.previous_used.max(window.used_percent);
                        continue;
                    }
                    let same_window = window.resets_at.is_some()
                        && window.resets_at == group.previous_reset
                        && window
                            .resets_at
                            .is_some_and(|reset| reset > snapshot.fetched_at);
                    if same_window && window.used_percent >= group.previous_used {
                        group.observed += window.used_percent - group.previous_used;
                        group.row.interval_count += 1;
                        if snapshot.fetched_at - group.previous_time > 300 {
                            group.row.gap_count += 1;
                        }
                    } else {
                        group.row.gap_count += 1;
                    }
                    group.previous_time = snapshot.fetched_at;
                    group.previous_used = window.used_percent;
                    group.previous_reset = window.resets_at;
                } else {
                    groups.insert(
                        key,
                        Aggregate {
                            row: QuotaAllocation {
                                provider: snapshot.provider,
                                account_id: snapshot.account_id.clone(),
                                window_id: window.id,
                                observed_percent: None,
                                allocated_percent: None,
                                unallocated_percent: None,
                                interval_count: 0,
                                gap_count: 0,
                            },
                            previous_time: snapshot.fetched_at,
                            previous_used: window.used_percent,
                            previous_reset: window.resets_at,
                            observed: 0.,
                        },
                    );
                }
            }
        }
        let mut allocated: HashMap<Key, f64> = HashMap::new();
        for (id, values) in estimates {
            let Some(prompt) = prompts.get(id) else {
                continue;
            };
            for value in values {
                *allocated
                    .entry((
                        prompt.provider,
                        prompt.account_id.clone(),
                        value.window_id.clone(),
                    ))
                    .or_default() += value.percent;
            }
        }
        let mut result = vec![];
        for (key, mut group) in groups {
            if group.row.interval_count > 0 {
                group.row.observed_percent = Some(group.observed);
                let amount = allocated.get(&key).copied().unwrap_or(0.);
                // Conflicting reporting timestamps can make isolated estimates inconsistent.
                // Preserve that uncertainty instead of clamping it into a false allocation.
                if amount <= group.observed + 1e-9 {
                    group.row.allocated_percent = Some(amount);
                    group.row.unallocated_percent = Some((group.observed - amount).max(0.));
                }
            }
            result.push(group.row);
        }
        result.sort_by(|a, b| {
            a.provider
                .key()
                .cmp(b.provider.key())
                .then_with(|| a.account_id.cmp(&b.account_id))
                .then_with(|| a.window_id.cmp(&b.window_id))
        });
        Ok(result)
    }
    pub fn pending_events(&self, limit: usize) -> Result<Vec<(String, SyncEvent)>, String> {
        let mut q=self.connection.prepare("SELECT hash,data FROM events WHERE outbound=1 AND exported=0 ORDER BY rowid LIMIT ?1").map_err(|e|e.to_string())?;
        let rows = q
            .query_map([limit as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.map(|r| {
            r.map_err(|e| e.to_string()).and_then(|(h, s)| {
                serde_json::from_str(&s)
                    .map(|e| (h, e))
                    .map_err(|e| e.to_string())
            })
        })
        .collect()
    }
    pub fn mark_exported(&self, hashes: &[String]) -> Result<(), String> {
        for hash in hashes {
            self.connection
                .execute("UPDATE events SET exported=1 WHERE hash=?1", [hash])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn has_batch(&self, hash: &str) -> Result<bool, String> {
        self.connection
            .query_row("SELECT 1 FROM batches WHERE hash=?1", [hash], |_| Ok(()))
            .optional()
            .map(|v| v.is_some())
            .map_err(|e| e.to_string())
    }
    pub fn mark_batch(&self, hash: &str) -> Result<(), String> {
        self.connection
            .execute("INSERT OR IGNORE INTO batches VALUES(?1)", [hash])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn clear_history(&mut self) -> Result<(), String> {
        self.connection.execute_batch("DELETE FROM requests; DELETE FROM prompts; DELETE FROM checkpoints; DELETE FROM request_links; DELETE FROM events WHERE json_extract(data,'$.type') IN ('prompt','request','link'); DELETE FROM batches;").map_err(|e|e.to_string())
    }
}

fn json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}
fn kind_key(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::User => "user",
        ActivityKind::Subagent => "subagent",
        ActivityKind::Review => "review",
        ActivityKind::Background => "background",
    }
}
fn status_rank(s: &str) -> u8 {
    match s {
        "completed" => 3,
        "interrupted" => 2,
        _ => 1,
    }
}
fn day(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".into())
}
pub fn percentile(sorted: &[u64], percent: f64) -> u64 {
    if sorted.is_empty() {
        0
    } else {
        sorted[((sorted.len() as f64 * percent).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1)]
    }
}
pub fn maximum_tokens(a: &TokenUsage, b: &TokenUsage) -> TokenUsage {
    fn max(a: Option<u64>, b: Option<u64>) -> Option<u64> {
        match (a, b) {
            (None, None) => None,
            _ => Some(a.unwrap_or(0).max(b.unwrap_or(0))),
        }
    }
    TokenUsage {
        input: max(a.input, b.input),
        cache_read: max(a.cache_read, b.cache_read),
        cache_write: max(a.cache_write, b.cache_write),
        output: max(a.output, b.output),
        reasoning: max(a.reasoning, b.reasoning),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn prompt() -> PromptRecord {
        PromptRecord {
            id: "p1".into(),
            provider: Provider::Codex,
            account_id: "account".into(),
            device_id: "device".into(),
            session_id: "s".into(),
            turn_id: "t".into(),
            timestamp: 110,
            preview: "hello".into(),
            status: "completed".into(),
            completed_at: Some(125),
            kind: ActivityKind::User,
        }
    }
    pub fn request() -> RequestUsage {
        RequestUsage {
            id: "r1".into(),
            prompt_id: Some("p1".into()),
            provider: Provider::Codex,
            account_id: "account".into(),
            device_id: "device".into(),
            session_id: "s".into(),
            timestamp: 120,
            model: "gpt".into(),
            effort: None,
            tokens: TokenUsage {
                input: Some(10),
                cache_read: Some(5),
                cache_write: None,
                output: Some(3),
                reasoning: Some(2),
            },
            kind: ActivityKind::User,
            source: "test".into(),
            source_event_id: "r1".into(),
        }
    }
    fn snapshot(id: &str, time: i64, used: f64, reset: i64) -> QuotaSnapshot {
        QuotaSnapshot {
            id: id.into(),
            provider: Provider::Codex,
            account_id: "account".into(),
            device_id: "device".into(),
            fetched_at: time,
            status: ConnectionStatus::Connected,
            windows: vec![QuotaWindow {
                id: "weekly".into(),
                label: "Weekly".into(),
                duration_minutes: 10080,
                used_percent: used,
                resets_at: Some(reset),
            }],
            message: None,
            retry_after_seconds: None,
        }
    }
    #[test]
    fn deduplicates_and_preserves_nulls() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        s.save_prompt(&prompt()).unwrap();
        let mut r = request();
        s.save_request(&r).unwrap();
        r.tokens.output = Some(4);
        s.save_request(&r).unwrap();
        s.save_request(&request()).unwrap();
        let stats = s.stats(None, None).unwrap();
        assert_eq!(stats.request_count, 1);
        assert_eq!(stats.total_tokens, 19);
        assert_eq!(stats.token_totals.cache_write, None);
        assert_eq!(stats.prompt_count, 1);
        assert_eq!(
            s.history(&HistoryFilter::default()).unwrap().items[0].requests[0]
                .tokens
                .output,
            Some(4)
        );
    }
    #[test]
    fn estimates_only_isolated_unchanged_windows() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        s.save_prompt(&prompt()).unwrap();
        s.save_request(&request()).unwrap();
        s.save_snapshot(&snapshot("a", 100, 5., 1000)).unwrap();
        s.save_snapshot(&snapshot("b", 140, 6., 1000)).unwrap();
        assert_eq!(s.quota_estimate(&prompt()).unwrap().unwrap().percent, 1.);
        let mut other = request();
        other.id = "other".into();
        other.prompt_id = None;
        s.save_request(&other).unwrap();
        assert!(s.quota_estimate(&prompt()).unwrap().is_none());
    }
    #[test]
    fn excludes_reset_and_account_contamination() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        s.save_prompt(&prompt()).unwrap();
        s.save_request(&request()).unwrap();
        s.save_snapshot(&snapshot("a", 100, 5., 130)).unwrap();
        s.save_snapshot(&snapshot("b", 140, 6., 1000)).unwrap();
        assert!(s.quota_estimate(&prompt()).unwrap().is_none());
    }
    #[test]
    fn preview_limit_and_filters() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let mut p = prompt();
        p.preview = "💡".repeat(170);
        s.save_prompt(&p).unwrap();
        s.save_request(&request()).unwrap();
        let page = s
            .history(&HistoryFilter {
                effort: Some("unknown".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].prompt.preview.chars().count(), 160);
        assert_eq!(
            s.history(&HistoryFilter {
                model: Some("absent".into()),
                ..Default::default()
            })
            .unwrap()
            .total,
            0
        );
    }
    #[test]
    fn late_completion_and_preexisting_work_block_calibration() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let mut p = prompt();
        p.completed_at = Some(150);
        s.save_prompt(&p).unwrap();
        s.save_request(&request()).unwrap();
        s.save_snapshot(&snapshot("a", 100, 5., 1000)).unwrap();
        s.save_snapshot(&snapshot("b", 140, 6., 1000)).unwrap();
        assert!(s.quota_estimate(&p).unwrap().is_none());
        let mut earlier = prompt();
        earlier.id = "prior".into();
        earlier.timestamp = 90;
        earlier.completed_at = None;
        earlier.status = "in_progress".into();
        s.save_prompt(&earlier).unwrap();
        assert!(s.quota_estimate(&prompt()).unwrap().is_none());
    }
    #[test]
    fn zero_deltas_and_every_window_are_retained_without_free_forecasts() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        s.save_prompt(&prompt()).unwrap();
        s.save_request(&request()).unwrap();
        let mut a = snapshot("a", 100, 5., 1000);
        a.windows.push(QuotaWindow {
            id: "short".into(),
            label: "5h".into(),
            duration_minutes: 300,
            used_percent: 10.,
            resets_at: Some(500),
        });
        let mut b = snapshot("b", 140, 5., 1000);
        b.windows.push(QuotaWindow {
            id: "short".into(),
            label: "5h".into(),
            duration_minutes: 300,
            used_percent: 11.,
            resets_at: Some(500),
        });
        s.save_snapshot(&a).unwrap();
        s.save_snapshot(&b).unwrap();
        let rows = s.stats(None, None).unwrap().model_stats;
        assert_eq!(rows.len(), 2);
        let weekly = rows
            .iter()
            .find(|r| r.quota_window_id.as_deref() == Some("weekly"))
            .unwrap();
        assert_eq!(weekly.quota_sample_count, 1);
        assert_eq!(weekly.estimated_quota_per_prompt, None);
        assert_eq!(
            rows.iter()
                .find(|r| r.quota_window_id.as_deref() == Some("short"))
                .unwrap()
                .estimated_quota_per_prompt,
            Some(1.)
        );
        assert_eq!(s.quota_estimate(&prompt()).unwrap().unwrap().percent, 0.);
    }
    #[test]
    fn delayed_prompt_events_cannot_revert_completion() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let done = prompt();
        s.apply_event(&SyncEvent::Prompt(done.clone()), false)
            .unwrap();
        let mut early = done;
        early.status = "in_progress".into();
        early.completed_at = None;
        early.device_id = "other".into();
        s.apply_event(&SyncEvent::Prompt(early), false).unwrap();
        let saved = s.prompt("p1").unwrap().unwrap();
        assert_eq!(saved.status, "completed");
        assert_eq!(saved.completed_at, Some(125));
    }
    #[test]
    fn accounting_retains_unallocated_deltas_and_unknown_reset_gaps() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        s.save_snapshot(&snapshot("a", 100, 5., 1000)).unwrap();
        s.save_snapshot(&snapshot("b", 140, 7., 1000)).unwrap();
        let row = s.stats(None, None).unwrap().quota_allocations.remove(0);
        assert_eq!(row.observed_percent, Some(2.));
        assert_eq!(row.allocated_percent, Some(0.));
        assert_eq!(row.unallocated_percent, Some(2.));
        assert_eq!(row.interval_count, 1);
        let mut duplicate = snapshot("other-device", 140, 7., 1000);
        duplicate.device_id = "other".into();
        s.save_snapshot(&duplicate).unwrap();
        s.save_snapshot(&snapshot("c", 1100, 1., 2000)).unwrap();
        let row = s.stats(None, None).unwrap().quota_allocations.remove(0);
        assert_eq!(row.observed_percent, Some(2.));
        assert_eq!(row.gap_count, 1);
        let only_gap = s
            .stats(Some(140), None)
            .unwrap()
            .quota_allocations
            .remove(0);
        assert_eq!(only_gap.observed_percent, None);
        assert_eq!(only_gap.unallocated_percent, None);
    }
    #[test]
    fn unknown_account_never_calibrates_and_combined_filter_matches_one_request() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let mut p = prompt();
        p.account_id = "unknown:claude".into();
        assert!(s.quota_estimate(&p).unwrap().is_none());
        s.save_prompt(&prompt()).unwrap();
        let mut first = request();
        first.effort = Some("high".into());
        s.save_request(&first).unwrap();
        let mut second = request();
        second.id = "r2".into();
        second.model = "other".into();
        second.effort = Some("low".into());
        s.save_request(&second).unwrap();
        assert_eq!(
            s.history(&HistoryFilter {
                model: Some("gpt".into()),
                effort: Some("low".into()),
                ..Default::default()
            })
            .unwrap()
            .total,
            0
        );
    }
    #[test]
    fn security_direct_prompt_references_never_cross_account_or_provider() {
        for different_provider in [false, true] {
            for prompt_first in [false, true] {
                let mut store = Store::open(Path::new(":memory:")).unwrap();
                let mut r = request();
                if different_provider {
                    r.provider = Provider::Claude;
                } else {
                    r.account_id = "other".into();
                }
                if prompt_first {
                    store.save_prompt(&prompt()).unwrap();
                }
                store.save_request(&r).unwrap();
                if !prompt_first {
                    store.save_prompt(&prompt()).unwrap();
                }
                let history = store.history(&HistoryFilter::default()).unwrap();
                assert!(history.items[0].requests.is_empty());
                assert_eq!(
                    store
                        .history(&HistoryFilter {
                            model: Some("gpt".into()),
                            ..Default::default()
                        })
                        .unwrap()
                        .total,
                    0
                );
                let stats = store.stats(None, None).unwrap();
                assert_eq!(stats.background_requests, 1);
                assert_eq!(stats.median_tokens, 0);
                assert_eq!(stats.model_stats[0].prompt_count, 0);
            }
        }
    }
    #[test]
    fn security_migration_repairs_legacy_json_without_changing_usage() {
        let path =
            std::env::temp_dir().join(format!("gcd-migration-{}.sqlite3", uuid::Uuid::new_v4()));
        {
            let mut store = Store::open(&path).unwrap();
            store.save_prompt(&prompt()).unwrap();
            store.save_request(&request()).unwrap();
            store.connection.execute_batch("UPDATE prompts SET data=json_set(data,'$.timestamp',999999,'$.sessionId','changed'); UPDATE requests SET data=json_set(data,'$.timestamp',999999,'$.sessionId','changed'); PRAGMA user_version=1;").unwrap();
        }
        {
            let store = Store::open(&path).unwrap();
            assert_eq!(
                store.prompt("p1").unwrap().unwrap().timestamp,
                prompt().timestamp
            );
            let saved = store.request("r1").unwrap().unwrap();
            assert_eq!(saved.timestamp, request().timestamp);
            assert_eq!(saved.session_id, request().session_id);
            assert_eq!(saved.tokens.output, request().tokens.output);
            assert_eq!(store.stats(None, None).unwrap().request_count, 1);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn security_preserves_request_timestamps_and_rejects_cross_account_links() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        store.save_prompt(&prompt()).unwrap();
        store.save_request(&request()).unwrap();
        let mut conflicting = request();
        conflicting.timestamp += 86400;
        store
            .apply_event(&SyncEvent::Request(conflicting), false)
            .unwrap();
        assert_eq!(store.request("r1").unwrap().unwrap().timestamp, 120);
        assert!(store
            .apply_event(
                &SyncEvent::Link {
                    request_id: "r1".into(),
                    provider: Provider::Codex,
                    account_id: "different".into(),
                    turn_id: "t".into()
                },
                false
            )
            .is_err());
    }
}
