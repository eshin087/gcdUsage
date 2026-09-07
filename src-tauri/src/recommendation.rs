//! Deterministic, local advice. Token measurements never stand in for provider quota percentages.
use crate::models::*;
use chrono::{NaiveDate, TimeZone, Utc};
use std::collections::HashMap;

const MIN_COMPLETED: u64 = 20;
const MIN_QUOTA_SAMPLES: u64 = 5;
const MAX_SNAPSHOT_AGE: i64 = 300;

fn effort_rank(effort: &str) -> u8 {
    match effort {
        "none" => 0,
        "minimal" => 1,
        "low" => 2,
        "medium" => 3,
        "high" => 4,
        "xhigh" => 5,
        "max" => 6,
        "ultra" => 7,
        _ => 0,
    }
}

fn task_tier(task: TaskClass) -> u8 {
    match task {
        TaskClass::Quick => 1,
        TaskClass::Everyday => 2,
        TaskClass::Complex => 3,
    }
}

fn choose_effort(model: &ModelCatalogEntry, task: TaskClass) -> Option<String> {
    let target = match task {
        TaskClass::Quick => 2,
        TaskClass::Everyday => 3,
        TaskClass::Complex => 4,
    };
    model
        .efforts
        .iter()
        .filter(|e| effort_rank(e) <= target)
        .max_by_key(|e| effort_rank(e))
        .or_else(|| model.efforts.iter().min_by_key(|e| effort_rank(e)))
        .cloned()
}

fn matches_model(
    stat: &ModelStat,
    model: &ModelCatalogEntry,
    effort: &Option<String>,
    account: &str,
) -> bool {
    // Dated aliases of exactly this model version are compatible; different model versions are not.
    let model_match = stat.model == model.id
        || stat.model.strip_prefix(&model.id).is_some_and(|tail| {
            tail.starts_with('-')
                && tail[1..].len() == 8
                && tail[1..].chars().all(|c| c.is_ascii_digit())
        });
    stat.provider == model.provider
        && stat.account_id.as_deref() == Some(account)
        && model_match
        && stat.effort == *effort
}

fn applies(window: &QuotaWindow, model: &ModelCatalogEntry) -> bool {
    let pool = window
        .id
        .rsplit_once(':')
        .map(|p| p.0)
        .unwrap_or(&window.id);
    if pool == model.provider.key() || pool == model.quota_pool {
        return true;
    }
    if model.provider == Provider::Claude {
        return ["sonnet", "opus", "haiku"]
            .iter()
            .any(|family| pool == format!("claude-{family}") && model.id.contains(family));
    }
    // Current app-server catalogs usually use the shared codex pool. Retain explicit per-model
    // buckets such as codex_spark even when model/list omitted their mapping.
    let compact = |s: &str| {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    };
    let pool = compact(pool);
    let model_id = compact(&model.id);
    pool.len() > 4 && (model_id.contains(&pool) || pool.contains(&model_id))
        || ["spark", "astra", "sol", "terra", "luna", "mini", "daybreak"]
            .iter()
            .any(|family| pool.contains(family) && model.id.contains(family))
}

fn known_mapping(window: &QuotaWindow, catalog: &[ModelCatalogEntry]) -> bool {
    catalog.iter().any(|model| applies(window, model))
}

fn matching_stats<'a>(
    stats: &'a DashboardStats,
    model: &ModelCatalogEntry,
    effort: &Option<String>,
    account: &str,
) -> Vec<&'a ModelStat> {
    if account.starts_with("unknown:") {
        return Vec::new();
    }
    stats
        .model_stats
        .iter()
        .filter(|s| matches_model(s, model, effort, account))
        .collect()
}

fn recent_daily_prompts(
    stats: &DashboardStats,
    provider: Provider,
    account: &str,
    now: i64,
) -> f64 {
    // One aggregate can be repeated for several quota windows. Count a model/effort only once.
    let mut groups = HashMap::<(String, Option<String>), u64>::new();
    for stat in &stats.model_stats {
        if stat.provider == provider && stat.account_id.as_deref() == Some(account) {
            let count = groups
                .entry((stat.model.clone(), stat.effort.clone()))
                .or_default();
            *count = (*count).max(stat.completed_prompts);
        }
    }
    let count = groups.values().sum::<u64>() as f64;
    let baseline = count / 30.0;
    let week_total: u64 = stats
        .daily
        .iter()
        .filter(|day| {
            NaiveDate::parse_from_str(&day.date, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|d| Utc.from_utc_datetime(&d).timestamp())
                .is_some_and(|date| date >= now - 7 * 86400 && date <= now)
        })
        .map(|day| day.prompts)
        .sum();
    // Cross-provider daily charts are apportioned by the provider's recent prompt share.
    let share = if stats.prompt_count > 0 {
        (count / stats.prompt_count as f64).min(1.0)
    } else {
        0.0
    };
    baseline.max(week_total as f64 / 7.0 * share)
}

fn evaluate(
    task: TaskClass,
    snapshot: &QuotaSnapshot,
    stats: &DashboardStats,
    model: &ModelCatalogEntry,
    catalog: &[ModelCatalogEntry],
    reserve: f64,
    now: i64,
) -> Recommendation {
    let effort = choose_effort(model, task);
    let samples = matching_stats(stats, model, &effort, &snapshot.account_id);
    let sample = samples.iter().max_by_key(|s| s.completed_prompts).copied();
    let count = sample.map_or(0, |s| s.completed_prompts);
    let personalized = count >= MIN_COMPLETED;
    let mut warnings = Vec::new();
    if model.source.contains("availability-unverified") {
        warnings.push("Model availability is based on the bundled catalog; confirm it in Claude Code's /model menu.".into());
    }
    let fresh = snapshot.fetched_at <= now + 30
        && now.saturating_sub(snapshot.fetched_at) <= MAX_SNAPSHOT_AGE;
    let connected = snapshot.status == ConnectionStatus::Connected;
    let windows: Vec<_> = snapshot
        .windows
        .iter()
        .filter(|window| applies(window, model))
        .collect();
    let general = model.provider.key();
    let has_weekly = windows.iter().any(|w| w.id == format!("{general}:10080"));
    let has_short =
        model.provider != Provider::Claude || windows.iter().any(|w| w.id == "claude:300");
    let missing_windows = !has_weekly || !has_short;
    let unknown_mapping = snapshot.windows.iter().any(|w| !known_mapping(w, catalog));
    let recent_rate = recent_daily_prompts(stats, model.provider, &snapshot.account_id, now);
    let mut fits = Some(true);
    let mut calibrated = 0;
    let mut limiting: Option<(&QuotaWindow, f64, f64)> = None;
    let mut estimated_quota = None;

    if !connected {
        fits = Some(false);
        warnings.push("Reconnect this provider before relying on its allowance.".into());
    } else if !fresh {
        fits = None;
        warnings.push("Usage is stale. Refresh before relying on the budget.".into());
    }
    if missing_windows {
        if fits != Some(false) {
            fits = None;
        }
        warnings.push("A required quota window is unavailable, so budget fit is unknown.".into());
    }
    if unknown_mapping {
        if fits != Some(false) {
            fits = None;
        }
        warnings.push("An additional quota pool has no known model mapping; its effect on this choice is unknown.".into());
    }

    for window in &windows {
        let Some(reset) = window.resets_at.filter(|r| *r > now) else {
            if fits != Some(false) {
                fits = None;
            }
            warnings.push(format!("{} reset needs a fresh reading.", window.label));
            continue;
        };
        if !window.used_percent.is_finite() || !(0.0..=100.0).contains(&window.used_percent) {
            if fits != Some(false) {
                fits = None;
            }
            continue;
        }
        let available = (100.0 - window.used_percent - reserve).max(0.0);
        if connected && fresh && available <= 0.0 {
            fits = Some(false);
        }
        let calibration = samples.iter().find(|s| {
            s.quota_window_id.as_deref() == Some(&window.id)
                && s.completed_prompts >= MIN_COMPLETED
                && s.quota_sample_count >= MIN_QUOTA_SAMPLES
                && s.estimated_quota_per_prompt
                    .is_some_and(|q| q.is_finite() && q > 0.0)
        });
        if let Some(stat) = calibration {
            calibrated += 1;
            let cost = stat.estimated_quota_per_prompt.unwrap();
            let remaining_days = (reset.saturating_sub(now) as f64 / 86400.0)
                .min(window.duration_minutes as f64 / 1440.0);
            let future_prompts = (recent_rate * remaining_days).ceil().max(1.0);
            let projected = cost * future_prompts;
            if connected && fresh && projected > available {
                fits = Some(false);
            }
            let headroom = available - projected;
            if limiting
                .as_ref()
                .is_none_or(|(_, _, old_headroom)| headroom < *old_headroom)
            {
                limiting = Some((window, future_prompts, headroom));
            }
            // Prefer the main weekly pool for the single headline estimate; never mix percentages.
            if window.id == format!("{general}:10080") {
                estimated_quota = Some(cost);
            }
        } else if fits != Some(false) {
            fits = None;
        }
    }
    if windows.is_empty() && fits != Some(false) {
        fits = None;
    }
    if !personalized {
        warnings.push(format!("{count} completed prompts for this exact model and effort; {MIN_COMPLETED} are needed for personalized consumption forecasts."));
    } else if calibrated < windows.len() {
        warnings.push(format!("At least {MIN_QUOTA_SAMPLES} isolated quota samples are needed for each applicable window. Token counts alone cannot establish quota cost."));
    }
    let confidence = if fits == Some(true) && calibrated > 0 {
        "calibrated"
    } else if personalized {
        "limited"
    } else {
        "provisional"
    };
    let effort_text = effort
        .as_ref()
        .map(|e| format!(" with {e} effort"))
        .unwrap_or_default();
    let task_text = match task {
        TaskClass::Quick => "quick work",
        TaskClass::Everyday => "everyday coding and writing",
        TaskClass::Complex => "complex analysis",
    };
    let reason = match fits {
        Some(true) => {
            let (window, future, _) = limiting.unwrap();
            format!("{}{} suits {}. Estimated consumption fits the applicable quotas with {:.0}% reserved. Projection assumes about {:.0} more prompts before {} resets, using your recent daily activity. Model suitability is a catalog judgment, not a measured quality score.", model.label, effort_text, task_text, reserve, future, window.label)
        }
        Some(false) if !connected => format!("{}{} suits {}, but this provider needs reconnection before its allowance can be used.", model.label, effort_text, task_text),
        Some(false) => format!("{}{} suits {}, but an applicable quota cannot support the projected work while preserving the {:.0}% reserve. Reduce planned work or wait for a reset.", model.label, effort_text, task_text, reserve),
        None => format!("{}{} is a provisional catalog choice for {}. There is not enough current quota evidence to confirm it fits the {:.0}% reserve. Measured tokens are separate from quota percentages.", model.label, effort_text, task_text, reserve),
    };
    Recommendation {
        provider: model.provider,
        model: model.id.clone(),
        effort,
        task,
        confidence: confidence.into(),
        sample_count: count,
        reason,
        estimated_tokens: if personalized {
            sample.map(|s| s.p75_tokens)
        } else {
            None
        },
        estimated_quota_percent: estimated_quota,
        fits_budget: fits,
        warnings,
    }
}

/// `stats` must be scoped to the last 30 days. Picks at most one advisory option per provider.
pub fn recommend(
    task: TaskClass,
    snapshots: &[QuotaSnapshot],
    stats: &DashboardStats,
    catalog: &[ModelCatalogEntry],
    reserve: f64,
    now: i64,
) -> RecommendationSet {
    let reserve = if reserve.is_finite() {
        reserve.clamp(0.0, 100.0)
    } else {
        10.0
    };
    let required_tier = task_tier(task);
    let mut recommendations = Vec::new();
    for provider in [Provider::Claude, Provider::Codex] {
        let Some(snapshot) = snapshots
            .iter()
            .filter(|s| s.provider == provider)
            .max_by_key(|s| s.fetched_at)
        else {
            continue;
        };
        // Authentication failures are explained in Overview, and never advertised as usable models.
        if snapshot.status != ConnectionStatus::Connected {
            continue;
        }
        let mut options: Vec<_> = catalog
            .iter()
            .filter(|m| m.available && m.provider == provider && m.quality_tier >= required_tier)
            .map(|model| {
                (
                    model,
                    evaluate(task, snapshot, stats, model, catalog, reserve, now),
                )
            })
            .collect();
        options.sort_by(|(ma, a), (mb, b)| {
            let fit_rank = |r: &Recommendation| match r.fits_budget {
                Some(true) => 0,
                None => 1,
                Some(false) => 2,
            };
            fit_rank(a)
                .cmp(&fit_rank(b))
                .then_with(|| {
                    // Favor capability for a supported fit and for complex-task provisional advice.
                    // For simpler provisional work, use the catalog's matching task tier.
                    if a.fits_budget == Some(true)
                        || (task == TaskClass::Complex && a.fits_budget.is_none())
                    {
                        mb.quality_tier.cmp(&ma.quality_tier)
                    } else {
                        ma.quality_tier.cmp(&mb.quality_tier)
                    }
                })
                .then_with(|| b.sample_count.cmp(&a.sample_count))
                .then_with(|| a.model.cmp(&b.model))
        });
        if let Some((_, choice)) = options.into_iter().next() {
            recommendations.push(choice);
        }
    }
    let summary = if recommendations.is_empty() {
        "Connect a provider and refresh its usage to get local model advice.".into()
    } else if recommendations.iter().all(|r| r.fits_budget == Some(false)) {
        "No suitable option fits the projected budget with your reserve. Wait for a reset or reduce the planned workload.".into()
    } else if recommendations.iter().any(|r| r.fits_budget == Some(true)) {
        "The strongest suitable choices with a supported budget estimate are shown. Advice stays local and never changes your model automatically.".into()
    } else {
        "Provisional choices based on task suitability. Budget fit remains unknown until fresh quotas and enough isolated usage samples are available.".into()
    };
    RecommendationSet {
        recommendations,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: i64 = 1_800_000_000;
    fn model(provider: Provider, id: &str, tier: u8) -> ModelCatalogEntry {
        ModelCatalogEntry {
            id: id.into(),
            provider,
            label: id.into(),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            quality_tier: tier,
            quota_pool: provider.key().into(),
            available: true,
            source: "test".into(),
        }
    }
    fn snapshot(provider: Provider, used: f64) -> QuotaSnapshot {
        QuotaSnapshot {
            id: "snapshot".into(),
            provider,
            account_id: "account-a".into(),
            device_id: "device".into(),
            fetched_at: NOW,
            status: ConnectionStatus::Connected,
            windows: vec![QuotaWindow {
                id: format!("{}:10080", provider.key()),
                label: "weekly".into(),
                duration_minutes: 10080,
                used_percent: used,
                resets_at: Some(NOW + 86400),
            }],
            message: None,
            retry_after_seconds: None,
        }
    }
    fn stat(id: &str, count: u64, quota: Option<f64>) -> ModelStat {
        ModelStat {
            provider: Provider::Codex,
            account_id: Some("account-a".into()),
            quota_window_id: Some("codex:10080".into()),
            model: id.into(),
            effort: Some("medium".into()),
            prompt_count: count,
            completed_prompts: count,
            request_count: count * 3,
            total_tokens: count * 500,
            median_tokens: 500,
            p75_tokens: 700,
            estimated_quota_per_prompt: quota,
            quota_sample_count: count,
        }
    }
    #[test]
    fn sparse_history_is_provisional_without_forecasts() {
        let catalog = vec![model(Provider::Codex, "regular", 2)];
        let stats = DashboardStats {
            model_stats: vec![stat("regular", 19, Some(0.1))],
            ..Default::default()
        };
        let result = recommend(
            TaskClass::Everyday,
            &[snapshot(Provider::Codex, 20.0)],
            &stats,
            &catalog,
            10.0,
            NOW,
        );
        let advice = &result.recommendations[0];
        assert_eq!(advice.confidence, "provisional");
        assert_eq!(advice.estimated_tokens, None);
        assert_eq!(advice.estimated_quota_percent, None);
        assert_eq!(advice.fits_budget, None);
    }
    #[test]
    fn calibrated_choice_prefers_strongest_fit() {
        let catalog = vec![
            model(Provider::Codex, "regular", 2),
            model(Provider::Codex, "strong", 3),
        ];
        let stats = DashboardStats {
            model_stats: vec![
                stat("regular", 30, Some(0.2)),
                stat("strong", 30, Some(0.5)),
            ],
            ..Default::default()
        };
        let result = recommend(
            TaskClass::Everyday,
            &[snapshot(Provider::Codex, 20.0)],
            &stats,
            &catalog,
            10.0,
            NOW,
        );
        assert_eq!(result.recommendations[0].model, "strong");
        assert_eq!(result.recommendations[0].fits_budget, Some(true));
    }
    #[test]
    fn exhausted_and_all_reserved_budgets_fail() {
        let catalog = vec![model(Provider::Codex, "regular", 2)];
        for (used, reserve) in [(100.0, 10.0), (90.0, 10.0), (0.0, 100.0)] {
            let result = recommend(
                TaskClass::Everyday,
                &[snapshot(Provider::Codex, used)],
                &DashboardStats::default(),
                &catalog,
                reserve,
                NOW,
            );
            assert_eq!(result.recommendations[0].fits_budget, Some(false));
        }
    }
    #[test]
    fn unknown_reset_and_stale_usage_cannot_fit() {
        let catalog = vec![model(Provider::Codex, "regular", 2)];
        let stats = DashboardStats {
            model_stats: vec![stat("regular", 30, Some(0.2))],
            ..Default::default()
        };
        let mut reading = snapshot(Provider::Codex, 20.0);
        reading.windows[0].resets_at = Some(NOW - 1);
        assert_eq!(
            recommend(
                TaskClass::Everyday,
                &[reading.clone()],
                &stats,
                &catalog,
                10.0,
                NOW
            )
            .recommendations[0]
                .fits_budget,
            None
        );
        reading.windows[0].resets_at = Some(NOW + 3600);
        reading.fetched_at = NOW - 600;
        assert_eq!(
            recommend(TaskClass::Everyday, &[reading], &stats, &catalog, 10.0, NOW).recommendations
                [0]
            .fits_budget,
            None
        );
    }
    #[test]
    fn account_and_window_must_match_calibration() {
        let catalog = vec![model(Provider::Codex, "regular", 2)];
        for (account, window) in [
            ("another-account", "codex:10080"),
            ("account-a", "codex:300"),
        ] {
            let mut sample = stat("regular", 30, Some(0.1));
            sample.account_id = Some(account.into());
            sample.quota_window_id = Some(window.into());
            let stats = DashboardStats {
                model_stats: vec![sample],
                ..Default::default()
            };
            assert_eq!(
                recommend(
                    TaskClass::Everyday,
                    &[snapshot(Provider::Codex, 20.0)],
                    &stats,
                    &catalog,
                    10.0,
                    NOW
                )
                .recommendations[0]
                    .fits_budget,
                None
            );
        }
    }
    #[test]
    fn short_claude_window_and_model_cap_are_respected() {
        let catalog = vec![model(Provider::Claude, "claude-opus-5", 3)];
        let mut reading = snapshot(Provider::Claude, 10.0);
        assert_eq!(
            recommend(
                TaskClass::Complex,
                &[reading.clone()],
                &DashboardStats::default(),
                &catalog,
                10.0,
                NOW
            )
            .recommendations[0]
                .fits_budget,
            None
        );
        reading.windows.push(QuotaWindow {
            id: "claude:300".into(),
            label: "five-hour".into(),
            duration_minutes: 300,
            used_percent: 20.0,
            resets_at: Some(NOW + 3600),
        });
        reading.windows.push(QuotaWindow {
            id: "claude-opus:10080".into(),
            label: "Opus weekly".into(),
            duration_minutes: 10080,
            used_percent: 98.0,
            resets_at: Some(NOW + 86400),
        });
        assert_eq!(
            recommend(
                TaskClass::Complex,
                &[reading],
                &DashboardStats::default(),
                &catalog,
                10.0,
                NOW
            )
            .recommendations[0]
                .fits_budget,
            Some(false)
        );
    }
    #[test]
    fn disconnected_accounts_are_not_suggested() {
        let mut reading = snapshot(Provider::Codex, 0.0);
        reading.status = ConnectionStatus::NeedsAuth;
        assert!(recommend(
            TaskClass::Quick,
            &[reading],
            &DashboardStats::default(),
            &[model(Provider::Codex, "quick", 1)],
            10.0,
            NOW
        )
        .recommendations
        .is_empty());
    }
    #[test]
    fn complex_provisional_choice_favors_capability_without_claiming_budget() {
        let catalog = vec![
            model(Provider::Codex, "legacy", 3),
            model(Provider::Codex, "frontier", 5),
        ];
        let result = recommend(
            TaskClass::Complex,
            &[snapshot(Provider::Codex, 30.0)],
            &DashboardStats::default(),
            &catalog,
            10.0,
            NOW,
        );
        assert_eq!(result.recommendations[0].model, "frontier");
        assert_eq!(result.recommendations[0].fits_budget, None);
    }

    #[test]
    fn opaque_live_pool_and_zero_only_samples_do_not_claim_fit() {
        let catalog = vec![model(Provider::Codex, "regular", 2)];
        let stats = DashboardStats {
            model_stats: vec![stat("regular", 30, Some(0.2))],
            ..Default::default()
        };
        let mut reading = snapshot(Provider::Codex, 20.0);
        reading.windows.push(QuotaWindow {
            id: "codex_bengalfox:300".into(),
            label: "Additional pool".into(),
            duration_minutes: 300,
            used_percent: 0.0,
            resets_at: Some(NOW + 3600),
        });
        assert_eq!(
            recommend(TaskClass::Everyday, &[reading], &stats, &catalog, 10.0, NOW).recommendations
                [0]
            .fits_budget,
            None
        );
        let zero_stats = DashboardStats {
            model_stats: vec![stat("regular", 30, Some(0.0))],
            ..Default::default()
        };
        assert_eq!(
            recommend(
                TaskClass::Everyday,
                &[snapshot(Provider::Codex, 20.0)],
                &zero_stats,
                &catalog,
                10.0,
                NOW
            )
            .recommendations[0]
                .fits_budget,
            None
        );
    }

    #[test]
    fn projected_daily_work_can_choose_a_lower_consumption_model() {
        let catalog = vec![
            model(Provider::Codex, "regular", 2),
            model(Provider::Codex, "strong", 5),
        ];
        let stats = DashboardStats {
            model_stats: vec![
                stat("regular", 300, Some(0.1)),
                stat("strong", 300, Some(10.0)),
            ],
            ..Default::default()
        };
        let result = recommend(
            TaskClass::Everyday,
            &[snapshot(Provider::Codex, 20.0)],
            &stats,
            &catalog,
            10.0,
            NOW,
        );
        assert_eq!(result.recommendations[0].model, "regular");
        assert_eq!(result.recommendations[0].fits_budget, Some(true));
    }
    #[test]
    fn model_switches_and_unknown_effort_do_not_share_samples() {
        let catalog = vec![model(Provider::Codex, "regular", 2)];
        let mut sample = stat("regular", 60, Some(0.1));
        sample.effort = None;
        let stats = DashboardStats {
            model_stats: vec![sample, stat("older", 60, Some(0.1))],
            ..Default::default()
        };
        let result = recommend(
            TaskClass::Everyday,
            &[snapshot(Provider::Codex, 10.0)],
            &stats,
            &catalog,
            10.0,
            NOW,
        );
        assert_eq!(result.recommendations[0].sample_count, 0);
    }
}
