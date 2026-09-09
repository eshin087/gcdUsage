//! Request-time statistics for interactive intervals. Recommendation calibration
//! keeps its separate, complete-prompt population in storage::stats.
use super::{percentile, Store};
use crate::models::*;
use rusqlite::params;
use std::collections::{BTreeMap, HashMap, HashSet};

impl Store {
    pub fn usage_metrics(&self, range: MetricsRange) -> Result<UsageMetrics, String> {
        range.validate()?;
        let start = match range.from {
            Some(from) => from,
            None => self.connection.query_row(
                "SELECT MIN(timestamp) FROM (SELECT MIN(timestamp) timestamp FROM prompts WHERE timestamp<?1 UNION ALL SELECT MIN(timestamp) timestamp FROM requests WHERE timestamp<?1)",
                [range.to], |r| r.get::<_, Option<i64>>(0),
            ).map_err(|e| e.to_string())?.unwrap_or(range.to - 1).max(0),
        };
        let span = (range.to - start).max(1);
        let bucket_seconds = match span {
            0..=3600 => 60,
            3601..=21600 => 300,
            21601..=172800 => 3600,
            172801..=864000 => 21600,
            864001..=5184000 => 86400,
            _ => ((span + 80 * 86400 - 1) / (80 * 86400)) * 86400,
        };
        let mut activity: Vec<ActivityBucket> = (0..(span + bucket_seconds - 1) / bucket_seconds)
            .map(|index| ActivityBucket {
                timestamp: start + index * bucket_seconds,
                ..Default::default()
            })
            .collect();
        let mut stats = DashboardStats::default();
        let mut devices = HashSet::new();
        let mut conversations = HashSet::new();
        let mut prompt_query = self
            .connection
            .prepare(
                "SELECT data FROM prompts WHERE timestamp>=?1 AND timestamp<?2 AND kind='user'",
            )
            .map_err(|e| e.to_string())?;
        let prompts = prompt_query
            .query_map(params![start, range.to], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        for row in prompts {
            let prompt: PromptRecord = serde_json::from_str(&row.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if prompt.timestamp < start || prompt.timestamp >= range.to {
                return Err("Inconsistent history timestamp; statistics unavailable".into());
            }
            stats.prompt_count += 1;
            devices.insert(prompt.device_id.clone());
            conversations.insert((prompt.provider, prompt.account_id, prompt.session_id));
            activity[((prompt.timestamp - start) / bucket_seconds) as usize].prompts += 1;
        }
        #[derive(Default)]
        struct Group {
            requests: u64,
            tokens: u64,
            prompts: HashMap<String, (u64, bool)>,
        }
        let mut groups: BTreeMap<(String, String, String, Option<String>), Group> = BTreeMap::new();
        let mut per_prompt: HashMap<String, u64> = HashMap::new();
        // Join the originating prompt regardless of its start time. A request
        // from an earlier user prompt is active user work, not background work.
        let mut query = self.connection.prepare(
            "SELECT r.data,p.data FROM requests r LEFT JOIN prompts p ON p.id=r.prompt_id AND p.provider=r.provider AND p.account_id=r.account_id WHERE r.timestamp>=?1 AND r.timestamp<?2"
        ).map_err(|e|e.to_string())?;
        let rows = query
            .query_map(params![start, range.to], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (request, prompt) = row.map_err(|e| e.to_string())?;
            let request: RequestUsage =
                serde_json::from_str(&request).map_err(|e| e.to_string())?;
            let prompt: Option<PromptRecord> = prompt
                .map(|p| serde_json::from_str(&p))
                .transpose()
                .map_err(|e| e.to_string())?;
            let user = prompt.filter(|p| {
                p.kind == ActivityKind::User
                    && !matches!(
                        request.kind,
                        ActivityKind::Review | ActivityKind::Background
                    )
            });
            if request.timestamp < start || request.timestamp >= range.to {
                return Err("Inconsistent history timestamp; statistics unavailable".into());
            }
            let tokens = request.tokens.total();
            stats.request_count += 1;
            stats.token_totals.add(&request.tokens);
            devices.insert(request.device_id.clone());
            let bucket = &mut activity[((request.timestamp - start) / bucket_seconds) as usize];
            bucket.tokens = bucket.tokens.saturating_add(tokens);
            bucket.requests += 1;
            let group = groups
                .entry((
                    request.provider.key().into(),
                    request.account_id,
                    request.model,
                    request.effort,
                ))
                .or_default();
            group.requests += 1;
            group.tokens = group.tokens.saturating_add(tokens);
            if let Some(p) = user {
                conversations.insert((p.provider, p.account_id, p.session_id));
                let value = per_prompt.entry(p.id.clone()).or_default();
                *value = value.saturating_add(tokens);
                let entry = group.prompts.entry(p.id).or_default();
                entry.0 = entry.0.saturating_add(tokens);
                entry.1 = p.status == "completed" && p.completed_at.is_some_and(|at| at < range.to);
            } else {
                stats.background_requests += 1;
            }
        }
        stats.total_tokens = stats.token_totals.total();
        stats.conversation_count = conversations.len() as u64;
        stats.computers = devices.into_iter().collect();
        stats.computers.sort();
        let active_prompt_count = per_prompt.len() as u64;
        let mut samples: Vec<_> = per_prompt.into_values().collect();
        samples.sort_unstable();
        stats.median_tokens = percentile(&samples, 0.5);
        stats.p75_tokens = percentile(&samples, 0.75);
        for ((provider, account, model, effort), group) in groups {
            let mut samples: Vec<_> = group.prompts.values().map(|(tokens, _)| *tokens).collect();
            samples.sort_unstable();
            stats.model_stats.push(ModelStat {
                provider: if provider == "claude" {
                    Provider::Claude
                } else {
                    Provider::Codex
                },
                account_id: Some(account),
                quota_window_id: None,
                model,
                effort,
                prompt_count: samples.len() as u64,
                completed_prompts: group.prompts.values().filter(|(_, done)| *done).count() as u64,
                request_count: group.requests,
                total_tokens: group.tokens,
                median_tokens: percentile(&samples, 0.5),
                p75_tokens: percentile(&samples, 0.75),
                estimated_quota_per_prompt: None,
                quota_sample_count: 0,
            });
        }
        stats.model_stats.sort_by(|a, b| {
            b.total_tokens
                .cmp(&a.total_tokens)
                .then_with(|| a.model.cmp(&b.model))
                .then_with(|| a.effort.cmp(&b.effort))
        });
        Ok(UsageMetrics {
            range,
            bucket_seconds,
            activity,
            active_prompt_count,
            stats,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::{prompt, request};
    use std::path::Path;

    #[test]
    fn adjacent_intervals_preserve_carry_in_prompts_models_and_token_arithmetic() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        store.save_prompt(&prompt()).unwrap();
        store.save_request(&request()).unwrap();
        let mut second = request();
        second.id = "r2".into();
        second.source_event_id = "r2".into();
        second.timestamp = 130;
        second.model = "second-model".into();
        second.effort = Some("high".into());
        second.kind = ActivityKind::Subagent;
        store.save_request(&second).unwrap();
        store.save_request(&second).unwrap();
        let first = store
            .usage_metrics(MetricsRange {
                from: Some(110),
                to: 130,
            })
            .unwrap();
        let next = store
            .usage_metrics(MetricsRange {
                from: Some(130),
                to: 140,
            })
            .unwrap();
        assert_eq!(
            (
                first.stats.prompt_count,
                first.stats.request_count,
                first.stats.total_tokens
            ),
            (1, 1, 18)
        );
        assert_eq!(
            (
                next.stats.prompt_count,
                next.active_prompt_count,
                next.stats.background_requests
            ),
            (0, 1, 0)
        );
        assert_eq!(
            (next.stats.total_tokens, next.stats.median_tokens),
            (18, 18)
        );
        assert_eq!(next.stats.model_stats[0].effort.as_deref(), Some("high"));
        assert_eq!(first.stats.model_stats[0].effort, None);
        assert_eq!(next.stats.token_totals.reasoning, Some(2));
        assert_eq!(next.stats.token_totals.cache_write, None);
        let whole = store
            .usage_metrics(MetricsRange {
                from: Some(110),
                to: 140,
            })
            .unwrap();
        assert_eq!(
            whole.stats.request_count,
            first.stats.request_count + next.stats.request_count
        );
        assert_eq!(
            whole.stats.total_tokens,
            first.stats.total_tokens + next.stats.total_tokens
        );
        assert_eq!(whole.active_prompt_count, 1);
        assert_eq!(whole.stats.model_stats.len(), 2);
        assert_eq!(
            whole.activity.iter().map(|b| b.tokens).sum::<u64>(),
            whole.stats.total_tokens
        );
    }
    #[test]
    fn review_and_unmatched_accounts_are_background_and_empty_intervals_are_bounded() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        store.save_prompt(&prompt()).unwrap();
        let mut review = request();
        review.kind = ActivityKind::Review;
        store.save_request(&review).unwrap();
        let mut other = request();
        other.id = "different-account".into();
        other.account_id = "other".into();
        store.save_request(&other).unwrap();
        let metrics = store
            .usage_metrics(MetricsRange {
                from: None,
                to: 150,
            })
            .unwrap();
        assert_eq!(
            (
                metrics.stats.background_requests,
                metrics.active_prompt_count
            ),
            (2, 0)
        );
        assert_eq!(metrics.stats.model_stats.len(), 2);
        let empty = store
            .usage_metrics(MetricsRange {
                from: Some(200),
                to: 220,
            })
            .unwrap();
        assert_eq!(empty.stats.total_tokens, 0);
        assert_eq!(empty.stats.token_totals.input, None);
        assert_eq!(empty.activity.len(), 1);
        assert!(store
            .usage_metrics(MetricsRange {
                from: Some(10),
                to: 10
            })
            .is_err());
        let long = store
            .usage_metrics(MetricsRange {
                from: Some(0),
                to: 253_402_300_799,
            })
            .unwrap();
        assert!(long.activity.len() <= 80);
    }
}
