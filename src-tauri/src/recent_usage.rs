//! Observed allowance changes inside a selected rolling interval.
//! Never turn token totals, reset boundaries, or missing reads into quota usage.
use super::Store;
use crate::models::*;
use rusqlite::params;
use serde::{Deserialize, Serialize};

const MAX_GAP_SECONDS: i64 = 900;
const MAX_SNAPSHOTS: usize = 50_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAllowanceWindow {
    pub provider: Provider,
    pub window_id: String,
    pub label: String,
    pub consumed_percent: Option<f64>,
    pub state: String,
    pub observed_seconds: i64,
    pub first_reading_at: Option<i64>,
    pub last_reading_at: Option<i64>,
    pub sample_count: usize,
    pub gap_count: usize,
    pub reset_count: usize,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAllowance {
    pub minutes: u32,
    pub from: i64,
    pub to: i64,
    pub windows: Vec<RecentAllowanceWindow>,
}

struct Accumulator {
    row: RecentAllowanceWindow,
    duration_minutes: u32,
    previous: Option<(i64, f64, i64)>,
    total: f64,
    intervals: usize,
}
impl Accumulator {
    fn new(snapshot: &QuotaSnapshot, window: &QuotaWindow) -> Self {
        Self {
            row: RecentAllowanceWindow {
                provider: snapshot.provider, window_id: window.id.clone(), label: window.label.clone(),
                consumed_percent: None, state: "learning".into(), observed_seconds: 0,
                first_reading_at: None, last_reading_at: None, sample_count: 0, gap_count: 0, reset_count: 0,
            },
            duration_minutes: window.duration_minutes,
            previous: None, total: 0.0, intervals: 0,
        }
    }
    fn push(&mut self, snapshot: &QuotaSnapshot, from: i64, to: i64) {
        let timestamp = snapshot.fetched_at;
        if timestamp < from || timestamp > to { return; }
        // Repeated cached snapshots never become new observations.
        if self.row.last_reading_at.is_some_and(|last| timestamp <= last) { return; }
        let reading = snapshot.windows.iter().find(|w| w.id == self.row.window_id
            && w.duration_minutes == self.duration_minutes
            && w.used_percent.is_finite() && (0.0..=100.0).contains(&w.used_percent));
        let valid = snapshot.status == ConnectionStatus::Connected;
        let reading = reading.filter(|_| valid)
            .and_then(|w| w.resets_at.filter(|reset| *reset > timestamp).map(|reset| (w.used_percent, reset)));
        let Some((used, reset)) = reading else {
            if self.previous.take().is_some() { self.row.gap_count += 1; }
            return;
        };
        self.row.sample_count += 1;
        if self.row.first_reading_at.is_none() { self.row.first_reading_at = Some(timestamp); }
        self.row.last_reading_at = Some(timestamp);
        if let Some((previous_time, previous_used, previous_reset)) = self.previous {
            let elapsed = timestamp - previous_time;
            if reset.abs_diff(previous_reset) > 60 || used < previous_used {
                self.row.reset_count += 1;
            } else if elapsed > MAX_GAP_SECONDS {
                self.row.gap_count += 1;
            } else if elapsed > 0 {
                self.total += used - previous_used;
                self.row.observed_seconds += elapsed;
                self.intervals += 1;
            }
        }
        self.previous = Some((timestamp, used, reset));
    }
    fn finish(mut self, from: i64, to: i64, fresh: bool, truncated: bool) -> RecentAllowanceWindow {
        if self.intervals > 0 {
            self.row.consumed_percent = Some(self.total);
            // Normal polling leaves short unobserved edges; expose actual coverage
            // separately rather than interpolating or extrapolating consumption.
            let edge_tolerance = 300.min((to - from) / 4);
            let edges_covered = self.row.first_reading_at.is_some_and(|t| t - from <= edge_tolerance)
                && self.row.last_reading_at.is_some_and(|t| to - t <= edge_tolerance);
            self.row.state = if fresh && edges_covered && !truncated
                && self.row.gap_count == 0 && self.row.reset_count == 0 { "observed" } else { "partial" }.into();
        } else if !fresh {
            self.row.state = "stale".into();
        }
        self.row
    }
}

impl Store {
    pub fn recent_allowance(&self, minutes: u32, current: &[QuotaSnapshot], now: i64) -> Result<RecentAllowance, String> {
        if !(1..=43200).contains(&minutes) { return Err("Choose 1 minute through 30 days".into()); }
        let from = now.saturating_sub(i64::from(minutes) * 60);
        let mut windows = Vec::new();
        for snapshot in current {
            let mut rows: Vec<_> = snapshot.windows.iter().map(|w| Accumulator::new(snapshot, w)).collect();
            let known_account = !snapshot.account_id.starts_with("unknown:") && !snapshot.account_id.is_empty();
            let fresh = known_account && snapshot.status == ConnectionStatus::Connected
                && snapshot.fetched_at <= now && now - snapshot.fetched_at <= 300;
            let mut count = 0;
            if known_account {
                // Bound memory and work to the most recent samples, then stream in
                // chronological order. The response never exposes account or device IDs.
                let mut query = self.connection.prepare(
                    "SELECT data,timestamp FROM (SELECT data,timestamp,id FROM snapshots WHERE provider=?1 AND account_id=?2 AND device_id=?3 AND timestamp>=?4 AND timestamp<=?5 ORDER BY timestamp DESC,id DESC LIMIT ?6) ORDER BY timestamp,id"
                ).map_err(|e| e.to_string())?;
                let saved = query.query_map(params![snapshot.provider.key(), snapshot.account_id, snapshot.device_id, from, now, MAX_SNAPSHOTS as i64],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))).map_err(|e| e.to_string())?;
                for saved in saved {
                    let (payload, stored_time) = saved.map_err(|e|e.to_string())?;
                    let saved: QuotaSnapshot = serde_json::from_str(&payload).map_err(|e|e.to_string())?;
                    // Validate payload scope too; a damaged row must not cross accounts.
                    if saved.fetched_at != stored_time || saved.provider != snapshot.provider || saved.account_id != snapshot.account_id || saved.device_id != snapshot.device_id {
                        return Err("Inconsistent allowance history; recent usage unavailable".into());
                    }
                    count += 1;
                    for row in &mut rows { row.push(&saved, from, now); }
                }
                for row in &mut rows { row.push(snapshot, from, now); }
            }
            windows.extend(rows.into_iter().map(|row| row.finish(from, now, fresh, count >= MAX_SNAPSHOTS)));
        }
        Ok(RecentAllowance { minutes, from, to: now, windows })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(at: i64, used: f64, reset: i64) -> QuotaSnapshot {
        QuotaSnapshot {
            id: at.to_string(), provider: Provider::Claude, account_id: "account".into(), device_id: "device".into(),
            fetched_at: at, status: ConnectionStatus::Connected, message: None, retry_after_seconds: None,
            windows: vec![QuotaWindow { id:"claude:300".into(), label:"Claude 5h".into(), duration_minutes:300, used_percent:used, resets_at:Some(reset) }],
        }
    }
    #[test]
    fn recent_usage_counts_observed_changes_without_estimated_edges_or_duplicate_reads() {
        let mut store = Store::open(std::path::Path::new(":memory:")).unwrap();
        // 60-point change just outside the requested interval must not leak in.
        store.save_snapshot(&snapshot(4199, 1.0, 20_000)).unwrap();
        let samples: Vec<_> = (0..=6).map(|i| snapshot(4200 + i*300, 61.0 + i as f64, 20_000)).collect();
        for row in &samples { store.save_snapshot(row).unwrap(); }
        let result = store.recent_allowance(30, &[samples[6].clone()], 6000).unwrap();
        assert_eq!(result.windows[0].consumed_percent, Some(6.0));
        assert_eq!(result.windows[0].state, "observed");
        assert_eq!(result.windows[0].sample_count, 7);
        assert_eq!(result.windows[0].observed_seconds, 1800);
        let shorter = store.recent_allowance(10, &[samples[6].clone()], 6000).unwrap();
        assert_eq!(shorter.windows[0].consumed_percent, Some(2.0));
    }
    #[test]
    fn resets_and_missing_readings_are_partial_not_negative_or_fabricated_usage() {
        let first = snapshot(4200, 90.0, 5100);
        let mut row = Accumulator::new(&first, &first.windows[0]);
        for sample in [first, snapshot(4500, 92.0, 5100), snapshot(5100, 1.0, 23000), snapshot(5400, 4.0, 23000)] {
            row.push(&sample, 4200, 6000);
        }
        let result = row.finish(4200, 6000, true, false);
        assert_eq!(result.consumed_percent, Some(5.0));
        assert_eq!(result.reset_count, 1);
        assert_eq!(result.observed_seconds, 600);
        assert_eq!(result.state, "partial");
        let s = snapshot(4200, 5.0, 20000);
        let mut gap = Accumulator::new(&s, &s.windows[0]);
        gap.push(&s, 4200, 6000);
        gap.push(&snapshot(6000, 50.0, 20000), 4200, 6000);
        assert_eq!(gap.finish(4200, 6000, true, false).consumed_percent, None);
    }
    #[test]
    fn account_device_unknown_and_zero_are_distinct() {
        let mut store = Store::open(std::path::Path::new(":memory:")).unwrap();
        for at in [4200,4500,4800,5100,5400,5700,6000] { store.save_snapshot(&snapshot(at, 12.0, 20_000)).unwrap(); }
        let current = snapshot(6000, 12.0, 20_000);
        assert_eq!(store.recent_allowance(30, &[current.clone()], 6000).unwrap().windows[0].consumed_percent, Some(0.0));
        for (account, device) in [("other","device"),("account","other"),("unknown:claude:device","device")] {
            let mut other = current.clone(); other.account_id=account.into();other.device_id=device.into();
            assert_eq!(store.recent_allowance(30, &[other], 6000).unwrap().windows[0].consumed_percent, None);
        }
        assert!(store.recent_allowance(0, &[], 6000).is_err());
        assert!(store.recent_allowance(43201, &[], 6000).is_err());
    }
}
