//! Streaming, resumable import of local provider JSONL. Only prompt previews survive parsing.
use crate::{
    models::*,
    storage::{stable_id, Store},
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

const MAX_LINE: usize = 16 * 1024 * 1024;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct ParserState {
    project: Option<String>,
    chat_title: Option<String>,
    session: String,
    turn: String,
    root_turn: Option<String>,
    current_prompt: Option<String>,
    model: String,
    effort: Option<String>,
    kind: Option<ActivityKind>,
    last_user_hash: Option<String>,
    cumulative: Option<TokenUsage>,
    explicit_since_legacy: bool,
    account: String,
    available_account: String,
    claude_parents: HashMap<String, String>,
}

pub fn import_all(
    store: &mut Store,
    settings: &AppSettings,
    accounts: &HashMap<Provider, String>,
) -> Result<ImportReport, String> {
    let home = dirs::home_dir().ok_or("Cannot locate home directory")?;
    let codex = settings
        .codex_home
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex"))
        });
    let claude = crate::providers::claude_home(settings);
    let roots = [
        (Provider::Codex, codex.join("sessions")),
        (Provider::Codex, codex.join("archived_sessions")),
        (Provider::Claude, claude.join("projects")),
    ];
    let mut report = ImportReport::default();
    for (provider, root) in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root).follow_links(false).sort_by_file_name() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    warn(&mut report, "Some history folders could not be read.");
                    continue;
                }
            };
            if !entry.file_type().is_file()
                || entry.path().extension().and_then(|s| s.to_str()) != Some("jsonl")
            {
                continue;
            }
            let account = accounts
                .get(&provider)
                .cloned()
                .unwrap_or_else(|| format!("local-unverified:{}", settings.device_id));
            match import_file(
                store,
                entry.path(),
                provider,
                &account,
                &settings.device_id,
                &mut report,
            ) {
                Ok(()) => {}
                Err(_) => warn(
                    &mut report,
                    "A history file could not be imported; it will be retried.",
                ),
            }
        }
    }
    store.resolve_links()?;
    Ok(report)
}

fn warn(report: &mut ImportReport, message: &str) {
    if report.warnings.len() < 20 && !report.warnings.iter().any(|s| s == message) {
        report.warnings.push(message.into());
    }
}

fn import_file(
    store: &mut Store,
    path: &Path,
    provider: Provider,
    account: &str,
    device: &str,
    report: &mut ImportReport,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let mut reader = BufReader::new(file);
    let mut first = Vec::new();
    reader
        .by_ref()
        .take((MAX_LINE + 1) as u64)
        .read_until(b'\n', &mut first)
        .map_err(|e| e.to_string())?;
    if first.last() != Some(&b'\n') {
        return Ok(());
    } // Initial record is still being written.
    let identity = stable_id(&[provider.key(), &String::from_utf8_lossy(&first)]);
    let key = path.to_string_lossy();
    let checkpoint = store.checkpoint(&key)?;
    // Enrich previously imported files without changing their token/account history.
    if checkpoint.is_some() {
        store.enrich_file(path, provider, &identity)?;
    }
    let (mut offset, mut state) = match checkpoint {
        Some((offset, saved_identity, state)) if saved_identity == identity && offset <= length => {
            (
                offset,
                serde_json::from_str::<ParserState>(&state).unwrap_or_default(),
            )
        }
        _ => (0, ParserState::default()),
    };
    if offset == length {
        return Ok(());
    }
    if state.account.is_empty() {
        state.account = account.into();
    }
    state.available_account = if account.starts_with("local-unverified")
        && !state.account.starts_with("local-unverified")
    {
        state.account.clone()
    } else {
        account.into()
    };
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(|e| e.to_string())?;
    let mut lines = 0u32;
    let mut buffer = Vec::new();
    store.begin()?;
    let imported = (|| -> Result<(), String> {
        loop {
            buffer.clear();
            let count = reader
                .by_ref()
                .take((MAX_LINE + 1) as u64)
                .read_until(b'\n', &mut buffer)
                .map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            if count > MAX_LINE {
                // Finish skipping this oversized record without retaining its contents.
                let mut ended = buffer.last() == Some(&b'\n');
                let mut skipped = count as u64;
                while !ended {
                    buffer.clear();
                    let read = reader
                        .by_ref()
                        .take((MAX_LINE + 1) as u64)
                        .read_until(b'\n', &mut buffer)
                        .map_err(|e| e.to_string())?;
                    if read == 0 {
                        break;
                    }
                    skipped += read as u64;
                    ended = buffer.last() == Some(&b'\n');
                }
                if !ended {
                    break;
                }
                offset += skipped;
                warn(report, "An oversized history record was skipped.");
                continue;
            }
            if buffer.last() != Some(&b'\n') {
                break;
            } // Do not checkpoint a partial trailing record.
            let line_offset = offset;
            offset += count as u64;
            lines += 1;
            match serde_json::from_slice::<Value>(&buffer) {
                Ok(value) => {
                    let parsed = match provider {
                        Provider::Codex => {
                            parse_codex(store, &value, &mut state, device, line_offset, report)
                        }
                        Provider::Claude => {
                            parse_claude(store, &value, &mut state, device, line_offset, report)
                        }
                    };
                    parsed?;
                }
                Err(_) => warn(
                    report,
                    "A malformed history record was skipped; later complete records were imported.",
                ),
            }
            if lines % 300 == 0 {
                store.save_checkpoint(
                    &key,
                    offset,
                    &identity,
                    &serde_json::to_string(&state).map_err(|e| e.to_string())?,
                )?;
                store.commit()?;
                store.begin()?;
            }
        }
        store.save_checkpoint(
            &key,
            offset,
            &identity,
            &serde_json::to_string(&state).map_err(|e| e.to_string())?,
        )?;
        store.commit()?;
        Ok(())
    })();
    if imported.is_err() {
        store.rollback();
    } else {
        report.files += 1;
    }
    imported
}

fn text(value: &Value) -> Option<String> {
    value.as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}
fn timestamp(value: &Value) -> i64 {
    value
        .as_i64()
        .map(|n| if n > 10_000_000_000 { n / 1000 } else { n })
        .or_else(|| {
            value
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.timestamp())
        })
        .unwrap_or(0)
}
pub(crate) fn event_time(v: &Value) -> i64 {
    timestamp(&v["timestamp"])
}
pub(crate) fn content_text(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        return s.into();
    }
    value
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|v| matches!(v["type"].as_str(), Some("text" | "input_text")))
                .filter_map(|v| v["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
pub(crate) fn is_context_only(s: &str) -> bool {
    crate::presentation::user_text(s).trim().is_empty()
}
fn classify(value: &Value) -> ActivityKind {
    let source = value.to_string().to_ascii_lowercase();
    if source.contains("guardian") || source.contains("review") {
        ActivityKind::Review
    } else if source.contains("subagent") || source.contains("sub_agent") {
        ActivityKind::Subagent
    } else if source.contains("automation") || source.contains("heartbeat") {
        ActivityKind::Background
    } else {
        ActivityKind::User
    }
}

fn create_prompt(
    store: &mut Store,
    state: &mut ParserState,
    provider: Provider,
    device: &str,
    body: &str,
    time: i64,
    explicit_id: Option<String>,
    report: &mut ImportReport,
) -> Result<(), String> {
    if body.trim().is_empty() || is_context_only(body) {
        return Ok(());
    }
    let content_hash = stable_id(&[body]);
    if state.last_user_hash.as_ref() == Some(&content_hash) {
        return Ok(());
    }
    // Keep past unknown records unchanged; switch identity only for newly starting prompts.
    if !state.available_account.is_empty() {
        state.account = state.available_account.clone();
    }
    let turn = if state.turn.is_empty() {
        format!("event:{time}:{content_hash}")
    } else {
        state.turn.clone()
    };
    let origin = explicit_id.unwrap_or_else(|| format!("{turn}:{content_hash}"));
    let id = stable_id(&[provider.key(), &state.account, "prompt", &origin]);
    let prompt = PromptRecord {
        project: state.project.clone(),
        chat_title: state.chat_title.clone(),
        id: id.clone(),
        provider,
        account_id: state.account.clone(),
        device_id: device.into(),
        session_id: state.session.clone(),
        turn_id: turn,
        timestamp: time,
        preview: crate::presentation::clean_preview(body),
        status: "in_progress".into(),
        completed_at: None,
        kind: state.kind.unwrap_or(ActivityKind::User),
    };
    if store.save_prompt(&prompt)? {
        report.prompts += 1;
    }
    state.current_prompt = Some(id);
    state.last_user_hash = Some(content_hash);
    Ok(())
}
fn complete(
    store: &mut Store,
    state: &ParserState,
    status: &str,
    completed_at: i64,
) -> Result<(), String> {
    if let Some(id) = &state.current_prompt {
        if let Some(mut p) = store.prompt(id)? {
            p.status = status.into();
            p.completed_at = Some(completed_at);
            store.save_prompt(&p)?;
        }
    }
    Ok(())
}

fn parse_codex(
    store: &mut Store,
    v: &Value,
    state: &mut ParserState,
    device: &str,
    offset: u64,
    report: &mut ImportReport,
) -> Result<(), String> {
    let p = &v["payload"];
    let time = event_time(v);
    match v["type"].as_str().unwrap_or("") {
        "session_meta" => {
            state.project = p["cwd"]
                .as_str()
                .and_then(crate::presentation::project_name);
            state.chat_title = p["title"]
                .as_str()
                .map(|s| crate::presentation::plain(s, 120));
            state.session = text(&p["id"])
                .or_else(|| text(&p["session_id"]))
                .unwrap_or_else(|| stable_id(&[&time.to_string(), "codex-session"]));
            state.kind = Some(classify(&serde_json::json!([
                p["source"],
                p["thread_source"]
            ])));
        }
        "turn_context" => {
            if let Some(project) = p["cwd"]
                .as_str()
                .and_then(crate::presentation::project_name)
            {
                state.project = Some(project);
            }
            if let Some(turn) = text(&p["turn_id"]) {
                if state.turn != turn {
                    state.current_prompt = None;
                    state.last_user_hash = None;
                }
                state.turn = turn;
            }
            state.root_turn = text(&p["root_turn_id"]).filter(|t| *t != state.turn);
            state.model = text(&p["model"]).unwrap_or_else(|| "unknown".into());
            state.effort = text(&p["effort"]).or_else(|| text(&p["reasoning_effort"]));
        }
        "event_msg" => match p["type"].as_str().unwrap_or("") {
            "task_started" => {
                state.turn =
                    text(&p["turn_id"]).unwrap_or_else(|| format!("start:{time}:{offset}"));
                state.current_prompt = None;
                state.last_user_hash = None;
            }
            "user_message" => {
                let body = content_text(&p["message"]);
                create_prompt(
                    store,
                    state,
                    Provider::Codex,
                    device,
                    &body,
                    time,
                    None,
                    report,
                )?;
            }
            "task_complete" | "task_completed" => complete(store, state, "completed", time)?,
            "turn_aborted" => complete(store, state, "interrupted", time)?,
            "token_count" => {
                let total = &p["info"]["total_token_usage"];
                if total.is_object() {
                    let cumulative = codex_raw_tokens(total);
                    // These totals can lag the modern stream after compaction. Keep
                    // a separate legacy baseline and never compare the two streams.
                    let last = p["info"]["last_token_usage"]
                        .is_object()
                        .then(|| codex_raw_tokens(&p["info"]["last_token_usage"]));
                    let mut delta = state
                        .cumulative
                        .as_ref()
                        .map(|old| {
                            let regressed = [
                                (cumulative.input, old.input),
                                (cumulative.cache_read, old.cache_read),
                                (cumulative.cache_write, old.cache_write),
                                (cumulative.output, old.output),
                            ]
                            .iter()
                            .any(|(new, old)| matches!((new, old), (Some(n), Some(o)) if n < o));
                            if regressed {
                                last.clone().unwrap_or_default()
                            } else {
                                delta_tokens(&cumulative, old)
                            }
                        })
                        .unwrap_or_else(|| last.unwrap_or_else(|| cumulative.clone()));
                    state.cumulative = Some(cumulative);
                    if std::mem::take(&mut state.explicit_since_legacy) {
                        return Ok(());
                    }
                    separate_codex_cache(&mut delta);
                    if delta.total() > 0 || delta.reasoning.is_some_and(|n| n > 0) {
                        let event = stable_id(&[&time.to_string(), &total.to_string()]);
                        save_usage(
                            store,
                            state,
                            Provider::Codex,
                            device,
                            time,
                            &event,
                            "codex.cumulative_delta",
                            delta,
                            report,
                        )?;
                    }
                }
            }
            _ => {}
        },
        "response_item" => {
            if p["type"] == "message" && p["role"] == "user" {
                let body = content_text(&p["content"]);
                create_prompt(
                    store,
                    state,
                    Provider::Codex,
                    device,
                    &body,
                    time,
                    None,
                    report,
                )?;
            }
        }
        "token_usage_record" => {
            if let Some(turn) = text(&p["turn_id"]) {
                state.turn = turn;
            }
            if let Some(root) = text(&p["root_turn_id"]).filter(|t| *t != state.turn) {
                state.root_turn = Some(root);
            }
            if let Some(model) = text(&p["model"]) {
                state.model = model;
            }
            let event = text(&p["response_id"])
                .or_else(|| text(&p["request_id"]))
                .or_else(|| text(&p["id"]))
                .unwrap_or_else(|| {
                    stable_id(&[&state.turn, &time.to_string(), &p["usage"].to_string()])
                });
            if p["usage"].is_object() {
                state.explicit_since_legacy = true;
                save_usage(
                    store,
                    state,
                    Provider::Codex,
                    device,
                    time,
                    &event,
                    "codex.token_usage_record",
                    codex_tokens(&p["usage"]),
                    report,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_claude(
    store: &mut Store,
    v: &Value,
    state: &mut ParserState,
    device: &str,
    offset: u64,
    report: &mut ImportReport,
) -> Result<(), String> {
    if let Some(project) = v["cwd"]
        .as_str()
        .and_then(crate::presentation::project_name)
    {
        state.project = Some(project);
    }
    if let Some(title) = v["customTitle"].as_str() {
        state.chat_title = Some(crate::presentation::plain(title, 120));
    }
    if let Some(session) = text(&v["sessionId"]) {
        state.session = session;
    }
    if state.session.is_empty() {
        return Ok(());
    }
    let is_side = v["isSidechain"].as_bool().unwrap_or(false) || v["agentId"].is_string();
    state.kind = Some(if is_side {
        ActivityKind::Subagent
    } else {
        ActivityKind::User
    });
    if is_side {
        state.root_turn = text(&v["promptId"]).or_else(|| state.root_turn.clone());
    }
    let time = event_time(v);
    let parent = text(&v["parentUuid"]);
    if let Some(parent) = parent.as_ref().and_then(|id| state.claude_parents.get(id)) {
        state.current_prompt = Some(parent.clone());
    }
    match v["type"].as_str().unwrap_or("") {
        "user" => {
            let content = &v["message"]["content"];
            let has_tool_result = content
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v["type"] == "tool_result"));
            let body = content_text(content);
            if !has_tool_result && !v["isMeta"].as_bool().unwrap_or(false) {
                state.turn = text(&v["promptId"])
                    .or_else(|| text(&v["uuid"]))
                    .unwrap_or_else(|| format!("{time}:{offset}"));
                state.last_user_hash = None;
                create_prompt(
                    store,
                    state,
                    Provider::Claude,
                    device,
                    &body,
                    time,
                    text(&v["uuid"]).or_else(|| text(&v["promptId"])),
                    report,
                )?;
            }
        }
        "assistant" => {
            let message = &v["message"];
            state.model = text(&message["model"]).unwrap_or_else(|| "unknown".into());
            state.effort = text(&v["effort"]).or_else(|| text(&message["effort"]));
            let event = text(&v["requestId"])
                .or_else(|| text(&message["id"]))
                .or_else(|| text(&v["uuid"]))
                .unwrap_or_else(|| {
                    stable_id(&[&state.session, &time.to_string(), &offset.to_string()])
                });
            if message["usage"].is_object() {
                save_usage(
                    store,
                    state,
                    Provider::Claude,
                    device,
                    time,
                    &event,
                    "claude.message_usage",
                    claude_tokens(&message["usage"]),
                    report,
                )?;
            }
            if matches!(
                message["stop_reason"].as_str(),
                Some("end_turn" | "stop_sequence")
            ) {
                complete(store, state, "completed", time)?;
            }
        }
        "system" => {
            if matches!(v["subtype"].as_str(), Some("turn_duration")) {
                complete(store, state, "completed", time)?;
            }
        }
        _ => {}
    }
    if let (Some(id), Some(prompt)) = (text(&v["uuid"]), state.current_prompt.as_ref()) {
        state.claude_parents.insert(id, prompt.clone());
    }
    // Parent maps contain IDs only. Prune very old ancestry while retaining recent chains.
    if state.claude_parents.len() > 4096 {
        let current = state.current_prompt.clone();
        state
            .claude_parents
            .retain(|_, prompt| Some(&*prompt) == current.as_ref());
    }
    Ok(())
}

fn save_usage(
    store: &mut Store,
    state: &ParserState,
    provider: Provider,
    device: &str,
    time: i64,
    event: &str,
    source: &str,
    tokens: TokenUsage,
    report: &mut ImportReport,
) -> Result<(), String> {
    let id = stable_id(&[provider.key(), &state.account, "request", event]);
    let kind = state.kind.unwrap_or(ActivityKind::Background);
    let request = RequestUsage {
        id: id.clone(),
        prompt_id: state.current_prompt.clone(),
        provider,
        account_id: state.account.clone(),
        device_id: device.into(),
        session_id: state.session.clone(),
        timestamp: time,
        model: if state.model.is_empty() {
            "unknown".into()
        } else {
            state.model.clone()
        },
        effort: state.effort.clone(),
        tokens,
        kind,
        source: source.into(),
        source_event_id: event.into(),
    };
    if store.save_request(&request)? {
        report.requests += 1;
    }
    if let Some(root) = &state.root_turn {
        if kind == ActivityKind::Subagent {
            store.link_to_turn(&id, provider, &state.account, root)?;
        }
    }
    Ok(())
}
fn number(v: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| v[*key].as_u64())
}
fn codex_raw_tokens(v: &Value) -> TokenUsage {
    TokenUsage {
        input: number(v, &["input_tokens"]),
        cache_read: number(v, &["cached_input_tokens", "cache_read_input_tokens"]),
        cache_write: number(v, &["cache_write_input_tokens"]),
        output: number(v, &["output_tokens"]),
        reasoning: number(v, &["reasoning_output_tokens", "reasoning_tokens"]),
    }
}
fn separate_codex_cache(tokens: &mut TokenUsage) {
    if let Some(total) = tokens.input {
        tokens.cache_read = tokens.cache_read.map(|n| n.min(total));
        tokens.cache_write = tokens
            .cache_write
            .map(|n| n.min(total.saturating_sub(tokens.cache_read.unwrap_or(0))));
        tokens.input = Some(
            total
                .saturating_sub(tokens.cache_read.unwrap_or(0))
                .saturating_sub(tokens.cache_write.unwrap_or(0)),
        );
    }
}
fn codex_tokens(v: &Value) -> TokenUsage {
    let mut usage = codex_raw_tokens(v);
    separate_codex_cache(&mut usage);
    usage
}
fn claude_tokens(v: &Value) -> TokenUsage {
    TokenUsage {
        input: number(v, &["input_tokens"]),
        cache_read: number(v, &["cache_read_input_tokens"]),
        cache_write: number(v, &["cache_creation_input_tokens"]),
        output: number(v, &["output_tokens"]),
        reasoning: number(v, &["reasoning_tokens"]),
    }
}
fn delta_tokens(new: &TokenUsage, old: &TokenUsage) -> TokenUsage {
    fn diff(a: Option<u64>, b: Option<u64>) -> Option<u64> {
        a.map(|n| {
            if n < b.unwrap_or(0) {
                n
            } else {
                n - b.unwrap_or(0)
            }
        })
    }
    TokenUsage {
        input: diff(new.input, old.input),
        cache_read: diff(new.cache_read, old.cache_read),
        cache_write: diff(new.cache_write, old.cache_write),
        output: diff(new.output, old.output),
        reasoning: diff(new.reasoning, old.reasoning),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn fixture() -> (PathBuf, Store, ParserState, ImportReport) {
        let p = std::env::temp_dir().join(format!("gcd-history-{}.jsonl", uuid::Uuid::new_v4()));
        (
            p,
            Store::open(Path::new(":memory:")).unwrap(),
            ParserState {
                account: "account".into(),
                ..Default::default()
            },
            ImportReport::default(),
        )
    }
    #[test]
    fn modern_codex_deduplicates_mirrored_events_and_forks() {
        let (_, mut store, mut state, mut report) = fixture();
        let rows = [
            serde_json::json!({"type":"session_meta","payload":{"id":"session"}}),
            serde_json::json!({"type":"event_msg","timestamp":100,"payload":{"type":"task_started","turn_id":"turn"}}),
            serde_json::json!({"type":"response_item","timestamp":101,"payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Please fix this"}]}}),
            serde_json::json!({"type":"turn_context","payload":{"turn_id":"turn","model":"model","effort":"high"}}),
            serde_json::json!({"type":"token_usage_record","timestamp":102,"payload":{"response_id":"response","usage":{"input_tokens":20,"cached_input_tokens":5,"output_tokens":6,"reasoning_output_tokens":4}}}),
            serde_json::json!({"type":"event_msg","timestamp":103,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":20,"cached_input_tokens":5,"output_tokens":6,"reasoning_output_tokens":4}}}}),
            serde_json::json!({"type":"event_msg","payload":{"type":"task_complete"}}),
        ];
        for _ in 0..2 {
            for (i, row) in rows.iter().enumerate() {
                parse_codex(&mut store, row, &mut state, "device", i as u64, &mut report).unwrap();
            }
        }
        let stats = store.stats(None, None).unwrap();
        assert_eq!(stats.request_count, 1);
        assert_eq!(stats.prompt_count, 1);
        assert_eq!(stats.total_tokens, 26);
        assert_eq!(stats.model_stats[0].completed_prompts, 1);
    }
    #[test]
    fn claude_blocks_and_tool_results_are_not_extra_prompts() {
        let (_, mut store, mut state, mut report) = fixture();
        for row in [
            serde_json::json!({"type":"user","uuid":"u","sessionId":"s","timestamp":100,"message":{"content":"Hello"}}),
            serde_json::json!({"type":"assistant","uuid":"a1","parentUuid":"u","requestId":"r","sessionId":"s","timestamp":101,"message":{"id":"m","model":"claude","usage":{"input_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":4,"output_tokens":5}}}),
            serde_json::json!({"type":"assistant","uuid":"a2","parentUuid":"u","requestId":"r","sessionId":"s","timestamp":101,"message":{"id":"m","model":"claude","usage":{"input_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":4,"output_tokens":7},"stop_reason":"end_turn"}}),
            serde_json::json!({"type":"user","uuid":"tool","parentUuid":"a2","sessionId":"s","timestamp":102,"message":{"content":[{"type":"tool_result","content":"result"}]}}),
        ] {
            parse_claude(&mut store, &row, &mut state, "d", 0, &mut report).unwrap();
        }
        let stats = store.stats(None, None).unwrap();
        assert_eq!(stats.prompt_count, 1);
        assert_eq!(stats.request_count, 1);
        assert_eq!(stats.total_tokens, 16);
        assert_eq!(stats.token_totals.reasoning, None);
    }
    #[test]
    fn resumes_partial_tail_and_recovers_malformed_lines() {
        let (path, mut store, _, mut report) = fixture();
        let mut f = File::create(&path).unwrap();
        writeln!(
            f,
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"s\"}}}}"
        )
        .unwrap();
        writeln!(f, "not json").unwrap();
        write!(f,"{{\"type\":\"event_msg\",\"timestamp\":100,\"payload\":{{\"type\":\"user_message\",\"message\":\"hello").unwrap();
        f.flush().unwrap();
        import_file(&mut store, &path, Provider::Codex, "a", "d", &mut report).unwrap();
        assert_eq!(store.stats(None, None).unwrap().prompt_count, 0);
        write!(f, "\"}}}}\n").unwrap();
        f.flush().unwrap();
        import_file(&mut store, &path, Provider::Codex, "a", "d", &mut report).unwrap();
        import_file(&mut store, &path, Provider::Codex, "a", "d", &mut report).unwrap();
        assert_eq!(store.stats(None, None).unwrap().prompt_count, 1);
        assert!(!report.warnings.is_empty());
        drop(f);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn cumulative_delta_handles_cache_and_resets() {
        let a = codex_tokens(
            &serde_json::json!({"input_tokens":100,"cached_input_tokens":80,"output_tokens":10,"reasoning_output_tokens":5}),
        );
        let b = codex_tokens(
            &serde_json::json!({"input_tokens":140,"cached_input_tokens":100,"output_tokens":15,"reasoning_output_tokens":6}),
        );
        let delta = delta_tokens(&b, &a);
        assert_eq!(delta.total(), 45);
        assert_eq!(delta.reasoning, Some(1));
        assert_eq!(codex_tokens(&Value::Null), TokenUsage::default());
    }
    #[test]
    fn sidechains_use_explicit_root_prompt_ids() {
        let (_, mut store, mut root, mut report) = fixture();
        parse_claude(&mut store,&serde_json::json!({"type":"user","uuid":"parent-user","promptId":"root-turn","sessionId":"s","timestamp":100,"message":{"content":"Work"}}),&mut root,"d",0,&mut report).unwrap();
        let mut side = ParserState {
            account: "account".into(),
            ..Default::default()
        };
        parse_claude(&mut store,&serde_json::json!({"type":"user","uuid":"child-user","promptId":"root-turn","sessionId":"s","agentId":"child","isSidechain":true,"timestamp":101,"message":{"content":"Subtask"}}),&mut side,"d",0,&mut report).unwrap();
        parse_claude(&mut store,&serde_json::json!({"type":"assistant","uuid":"a","sessionId":"s","agentId":"child","isSidechain":true,"timestamp":102,"message":{"id":"r","model":"claude","usage":{"input_tokens":5,"output_tokens":6},"stop_reason":"end_turn"}}),&mut side,"d",0,&mut report).unwrap();
        store.resolve_links().unwrap();
        let history = store.history(&HistoryFilter::default()).unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].requests.len(), 1);
        assert_eq!(history.items[0].requests[0].kind, ActivityKind::Subagent);
        assert_eq!(history.items[0].tokens.total(), 11);
    }
    #[test]
    fn new_prompts_after_reconnect_use_current_account_without_retagging_history() {
        let (path, mut store, _, mut report) = fixture();
        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            "{}",
            serde_json::json!({"type":"session_meta","payload":{"id":"s"}})
        )
        .unwrap();
        writeln!(file,"{}",serde_json::json!({"type":"event_msg","timestamp":100,"payload":{"type":"task_started","turn_id":"t1"}})).unwrap();
        writeln!(file,"{}",serde_json::json!({"type":"event_msg","timestamp":101,"payload":{"type":"user_message","message":"First"}})).unwrap();
        file.flush().unwrap();
        import_file(
            &mut store,
            &path,
            Provider::Codex,
            "local-unverified:d",
            "d",
            &mut report,
        )
        .unwrap();
        writeln!(file,"{}",serde_json::json!({"type":"event_msg","timestamp":200,"payload":{"type":"task_started","turn_id":"t2"}})).unwrap();
        writeln!(file,"{}",serde_json::json!({"type":"event_msg","timestamp":201,"payload":{"type":"user_message","message":"Second"}})).unwrap();
        file.flush().unwrap();
        import_file(
            &mut store,
            &path,
            Provider::Codex,
            "signed-account",
            "d",
            &mut report,
        )
        .unwrap();
        let prompts = store.all_prompts().unwrap();
        assert_eq!(prompts[0].account_id, "local-unverified:d");
        assert_eq!(prompts[1].account_id, "signed-account");
        drop(file);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn cumulative_cache_growth_does_not_inflate_uncached_input() {
        let (_, mut store, mut state, mut report) = fixture();
        for (i, (input, cache)) in [(100, 80), (140, 130)].iter().enumerate() {
            parse_codex(&mut store,&serde_json::json!({"type":"event_msg","timestamp":100+i,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":input,"cached_input_tokens":cache,"output_tokens":0}}}}),&mut state,"d",i as u64,&mut report).unwrap();
        }
        // An impossible cache delta larger than incremental input is clamped; no negative input is invented.
        let requests = store.all_requests().unwrap();
        assert_eq!(requests[1].tokens.input, Some(0));
        assert_eq!(requests[1].tokens.cache_read, Some(40));
        assert_eq!(store.stats(None, None).unwrap().total_tokens, 140);
    }
    #[test]
    fn a_file_can_mix_legacy_requests_and_modern_records() {
        let (path, mut store, _, mut report) = fixture();
        let mut file = File::create(&path).unwrap();
        for value in [
            serde_json::json!({"type":"session_meta","payload":{"id":"s"}}),
            serde_json::json!({"type":"event_msg","timestamp":100,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":10}}}}),
            serde_json::json!({"type":"token_usage_record","timestamp":200,"payload":{"response_id":"modern","usage":{"input_tokens":20,"output_tokens":5},"thread_token_usage":{"input_tokens":120,"output_tokens":15}}}),
            serde_json::json!({"type":"event_msg","timestamp":200,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":120,"output_tokens":15}}}}),
        ] {
            writeln!(file, "{value}").unwrap();
        }
        file.flush().unwrap();
        import_file(&mut store, &path, Provider::Codex, "a", "d", &mut report).unwrap();
        let stats = store.stats(None, None).unwrap();
        assert_eq!(stats.request_count, 2);
        assert_eq!(stats.total_tokens, 135);
        drop(file);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn modern_records_do_not_share_rebased_legacy_counters() {
        let (_, mut store, mut state, mut report) = fixture();
        let records = [
            serde_json::json!({"type":"token_usage_record","timestamp":100,"payload":{"response_id":"a","usage":{"input_tokens":100,"cached_input_tokens":80,"output_tokens":10},"thread_token_usage":{"input_tokens":100,"cached_input_tokens":80,"output_tokens":10}}}),
            serde_json::json!({"type":"event_msg","timestamp":100,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":80,"output_tokens":10}}}}),
            serde_json::json!({"type":"token_usage_record","timestamp":101,"payload":{"response_id":"b","usage":{"input_tokens":20,"cached_input_tokens":10,"output_tokens":4},"thread_token_usage":{"input_tokens":120,"cached_input_tokens":90,"output_tokens":14}}}),
            serde_json::json!({"type":"event_msg","timestamp":101,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":80,"output_tokens":10},"last_token_usage":{"input_tokens":0,"cached_input_tokens":0,"output_tokens":0}}}}),
            serde_json::json!({"type":"token_usage_record","timestamp":102,"payload":{"response_id":"c","usage":{"input_tokens":30,"cached_input_tokens":20,"output_tokens":5},"thread_token_usage":{"input_tokens":150,"cached_input_tokens":110,"output_tokens":19}}}),
            serde_json::json!({"type":"event_msg","timestamp":102,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":130,"cached_input_tokens":100,"output_tokens":15}}}}),
            serde_json::json!({"type":"event_msg","timestamp":103,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":140,"cached_input_tokens":105,"output_tokens":18}}}}),
        ];
        for (i, record) in records.iter().enumerate() {
            // Exercise checkpoint serialization between every appended record.
            state = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
            parse_codex(&mut store, record, &mut state, "d", i as u64, &mut report).unwrap();
        }
        let stats = store.stats(None, None).unwrap();
        assert_eq!(stats.request_count, 4);
        assert_eq!(stats.total_tokens, 182);
        assert_eq!(stats.token_totals.cache_read, Some(115));
    }

    #[test]
    fn initial_legacy_snapshot_uses_last_request_not_inherited_total() {
        let (_, mut store, mut state, mut report) = fixture();
        let record = serde_json::json!({"type":"event_msg","timestamp":100,"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000000,"cached_input_tokens":900000,"output_tokens":10000},"last_token_usage":{"input_tokens":100,"cached_input_tokens":80,"output_tokens":5}}}});
        parse_codex(&mut store, &record, &mut state, "d", 0, &mut report).unwrap();
        assert_eq!(store.stats(None, None).unwrap().total_tokens, 105);
    }
    #[test]
    fn identical_prompts_in_distinct_legacy_turns_are_counted_separately() {
        let (_, mut store, mut state, mut report) = fixture();
        for (time, turn) in [(100, "one"), (200, "two")] {
            parse_codex(&mut store, &serde_json::json!({"type":"turn_context","timestamp":time,"payload":{"turn_id":turn,"model":"gpt"}}), &mut state, "d", time, &mut report).unwrap();
            parse_codex(&mut store, &serde_json::json!({"type":"event_msg","timestamp":time,"payload":{"type":"user_message","message":"continue"}}), &mut state, "d", time, &mut report).unwrap();
        }
        assert_eq!(store.stats(None, None).unwrap().prompt_count, 2);
    }
}
