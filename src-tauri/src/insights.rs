//! Local usage insights and conservative quota forecasts from observed provider readings.
use super::Store;
use crate::models::*;
use chrono::{Datelike, Local, TimeZone, Timelike};
use rusqlite::params;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ForecastPoint { pub timestamp:i64, pub remaining:f64 }
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct AllowanceForecast {
    pub provider:Provider, pub label:String, pub state:String,
    pub remaining:Option<f64>, pub resets_at:Option<i64>,
    pub rate_per_hour:Option<f64>, pub exhausts_at:Option<i64>,
    pub remaining_at_reset:Option<f64>, pub observed_seconds:i64,
    pub sample_count:usize, pub points:Vec<ForecastPoint>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct UsageInsights {
    pub days:u32, pub current:UsageMetrics, pub previous:UsageMetrics,
    pub active_days:usize, pub longest_streak:usize,
    pub hourly_prompts:Vec<u64>, pub unknown_requests:u64,
    pub forecasts:Vec<AllowanceForecast>,
}

fn forecast(current:&QuotaSnapshot, window:&QuotaWindow, history:&[QuotaSnapshot], now:i64) -> AllowanceForecast {
    let mut result=AllowanceForecast {
        provider:current.provider,label:window.label.clone(),state:"learning".into(),
        remaining:None,resets_at:window.resets_at,rate_per_hour:None,exhausts_at:None,
        remaining_at_reset:None,observed_seconds:0,sample_count:0,points:vec![],
    };
    if current.status!=ConnectionStatus::Connected || current.fetched_at>now
        || now-current.fetched_at>300 || current.account_id.starts_with("unknown:")
        || !window.used_percent.is_finite() || !(0.0..=100.0).contains(&window.used_percent) {
        result.state="stale".into();return result;
    }
    result.remaining=Some(100.0-window.used_percent);
    let Some(reset)=window.resets_at.filter(|r|*r>now) else {return result;};
    let mut readings:Vec<(i64,f64)>=history.iter().filter(|s|
        s.provider==current.provider && s.account_id==current.account_id
        && s.device_id==current.device_id && s.status==ConnectionStatus::Connected
        && s.fetched_at>=now-86400 && s.fetched_at<=current.fetched_at)
        .filter_map(|s|s.windows.iter().find(|w|w.id==window.id && w.duration_minutes==window.duration_minutes
            && w.resets_at.is_some_and(|r|(r-reset).abs()<=60)
            && w.used_percent.is_finite() && (0.0..=100.0).contains(&w.used_percent))
            .map(|w|(s.fetched_at,w.used_percent))).collect();
    readings.push((current.fetched_at,window.used_percent));
    readings.sort_by_key(|r|r.0);readings.dedup_by_key(|r|r.0);
    // Use only the latest continuous monotonic interval. Never infer consumption
    // across offline gaps, account switches or resets.
    let mut start=readings.len()-1;
    while start>0 {
        let (a,b)=(readings[start-1],readings[start]);
        if b.0-a.0>900 || b.1+0.01<a.1 {break;}
        start-=1;
    }
    let readings=&readings[start..];
    result.sample_count=readings.len();
    result.observed_seconds=readings.last().unwrap().0-readings[0].0;
    let stride=readings.len().div_ceil(96).max(1);
    result.points=readings.iter().step_by(stride).map(|&(timestamp,used)|ForecastPoint{timestamp,remaining:100.0-used}).collect();
    if result.points.last().is_none_or(|p|p.timestamp!=current.fetched_at) {
        result.points.push(ForecastPoint{timestamp:current.fetched_at,remaining:100.0-window.used_percent});
    }
    if window.used_percent>=100.0 {result.state="exhausted".into();result.exhausts_at=Some(now);return result;}
    if readings.len()<4 || result.observed_seconds<1800 {return result;}
    let consumed=(window.used_percent-readings[0].1).max(0.0);
    let rate=consumed/(result.observed_seconds as f64/3600.0);
    result.rate_per_hour=Some(rate);
    if consumed<0.1 {
        result.state="steady".into();
        result.remaining_at_reset=result.remaining;
        return result;
    }
    let remaining=100.0-window.used_percent;
    let seconds=remaining/rate*3600.0;
    let exhausts=now.saturating_add(seconds.ceil().min(i64::MAX as f64) as i64);
    result.remaining_at_reset=Some((remaining-rate*(reset-now) as f64/3600.0).clamp(0.0,100.0));
    if exhausts<reset {result.exhausts_at=Some(exhausts);result.state="before_reset".into();}
    else {result.state="after_reset".into();}
    result
}

impl Store {
    pub fn usage_insights(&self,days:u32,current:&[QuotaSnapshot],now:i64)->Result<UsageInsights,String> {
        if ![7,30,90].contains(&days) {return Err("Choose 7, 30 or 90 days".into());}
        let span=days as i64*86400;
        let from=(now-span).max(1);
        let metrics=self.usage_metrics(MetricsRange{from:Some(from),to:now})?;
        let previous=self.usage_metrics(MetricsRange{from:Some((from-span).max(0)),to:from})?;
        let mut active=BTreeSet::new();let mut hourly=vec![0u64;168];
        let mut query=self.connection.prepare("SELECT timestamp FROM prompts WHERE kind='user' AND timestamp>=?1 AND timestamp<?2").map_err(|e|e.to_string())?;
        let rows=query.query_map(params![from,now],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?;
        for row in rows {
            let t=row.map_err(|e|e.to_string())?;
            if let Some(local)=Local.timestamp_opt(t,0).single() {
                active.insert(local.date_naive());
                hourly[local.weekday().num_days_from_monday() as usize*24+local.hour() as usize]+=1;
            }
        }
        let mut longest=0;let mut streak=0;let mut last=None;
        for day in &active {
            streak=if last.is_some_and(|d:chrono::NaiveDate|d.succ_opt()==Some(*day)){streak+1}else{1};
            longest=longest.max(streak);last=Some(*day);
        }
        let unknown_requests=self.connection.query_row(
            "SELECT COUNT(*) FROM requests WHERE timestamp>=?1 AND timestamp<?2 AND json_extract(data,'$.tokens.input') IS NULL AND json_extract(data,'$.tokens.cacheRead') IS NULL AND json_extract(data,'$.tokens.cacheWrite') IS NULL AND json_extract(data,'$.tokens.output') IS NULL",
            params![from,now],|r|r.get(0)).map_err(|e|e.to_string())?;
        let mut forecasts=vec![];
        for snapshot in current {
            let history:Vec<QuotaSnapshot>=self.query_json(
                "SELECT data FROM snapshots WHERE provider=?1 AND account_id=?2 AND device_id=?3 AND timestamp>=?4 AND timestamp<=?5 ORDER BY timestamp DESC LIMIT 1500",
                params![snapshot.provider.key(),snapshot.account_id,snapshot.device_id,now-86400,now])?;
            for window in &snapshot.windows {forecasts.push(forecast(snapshot,window,&history,now));}
        }
        Ok(UsageInsights{days,current:metrics,previous,active_days:active.len(),
            longest_streak:longest,hourly_prompts:hourly,unknown_requests,forecasts})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn reading(at:i64,used:f64)->QuotaSnapshot {
        QuotaSnapshot {id:at.to_string(),provider:Provider::Claude,account_id:"account-a".into(),device_id:"device-a".into(),
            fetched_at:at,status:ConnectionStatus::Connected,message:None,retry_after_seconds:None,
            windows:vec![QuotaWindow{id:"claude:300".into(),label:"Claude 5h".into(),duration_minutes:300,used_percent:used,resets_at:Some(12000)}]}
    }
    #[test]
    fn forecasts_use_observed_percent_not_tokens_and_stop_at_reset() {
        let history:Vec<_>=(0..=6).map(|i|reading(6000+i*300,20.0+i as f64*10.0)).collect();
        let current=history.last().unwrap();
        let f=forecast(current,&current.windows[0],&history,7800);
        assert_eq!(f.state,"before_reset");assert_eq!(f.rate_per_hour,Some(120.0));
        assert_eq!(f.exhausts_at,Some(8400));assert_eq!(f.remaining_at_reset,Some(0.0));
        let steady:Vec<_>=(0..=6).map(|i|reading(6000+i*300,20.0)).collect();
        assert_eq!(forecast(&steady[6],&steady[6].windows[0],&steady,7800).state,"steady");
    }
    #[test]
    fn forecast_rejects_stale_accounts_gaps_resets_and_sparse_data() {
        let history:Vec<_>=(0..=6).map(|i|reading(6000+i*300,20.0+i as f64)).collect();
        let current=history[6].clone();
        assert_eq!(forecast(&current,&current.windows[0],&history,8200).state,"stale");
        let mut other=history.clone();for s in &mut other {s.account_id="other".into();}
        assert_eq!(forecast(&current,&current.windows[0],&other,7800).state,"learning");
        let gap=vec![history[0].clone(),history[6].clone()];
        assert_eq!(forecast(&current,&current.windows[0],&gap,7800).state,"learning");
        let mut reset=history.clone();for s in &mut reset[..6] {s.windows[0].resets_at=Some(9999);}
        assert_eq!(forecast(&current,&current.windows[0],&reset,7800).state,"learning");
        let mut unknown=current.clone();unknown.account_id="unknown:claude:device".into();
        assert_eq!(forecast(&unknown,&unknown.windows[0],&history,7800).remaining,None);
    }
    #[test]
    fn insights_preserve_unknown_requests_and_exclude_background_prompts() {
        let mut store=Store::open(std::path::Path::new(":memory:")).unwrap();
        let mut p=crate::storage::tests::prompt();p.timestamp=1_000_000;store.save_prompt(&p).unwrap();
        let mut r=crate::storage::tests::request();r.timestamp=p.timestamp;r.tokens=TokenUsage::default();store.save_request(&r).unwrap();
        let result=store.usage_insights(7,&[],1_000_100).unwrap();
        assert_eq!(result.active_days,1);assert_eq!(result.hourly_prompts.iter().sum::<u64>(),1);
        assert_eq!(result.unknown_requests,1);assert_eq!(result.current.stats.token_totals.input,None);
        assert!(store.usage_insights(10,&[],1_000_100).is_err());
    }
}
