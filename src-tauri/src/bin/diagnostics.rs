//! Opt-in local diagnostics. Never prints prompt previews, credentials, or raw provider failures.
use gcd_usage_lib::{history, models::*, providers, recommendation, storage::Store};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf, time::Instant};

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), &'static str> {
    let mut read_providers = false;
    let mut import = false;
    let mut data_dir = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--providers" => read_providers = true,
            "--import" => import = true,
            "--data-dir" => {
                data_dir = Some(PathBuf::from(
                    args.next().ok_or("--data-dir requires an absolute path")?,
                ))
            }
            "--help" | "-h" => {
                println!("GCD Usage diagnostics\n  --providers                    Read sanitized live provider snapshots and available models\n  --import --data-dir ABSOLUTE    Import existing local history into an explicit diagnostic directory, then repeat to check deduplication\nCombine both options to print local advice for the imported history. No prompt previews are printed.");
                return Ok(());
            }
            _ => return Err("Unknown argument. Run diagnostics --help for supported options."),
        }
    }
    if !read_providers && !import {
        return Err("Choose --providers or --import. Run diagnostics --help for details.");
    }
    if import && data_dir.is_none() {
        return Err("--import requires --data-dir with an explicit absolute diagnostic directory.");
    }
    let mut settings = AppSettings {
        launch_at_login: false,
        device_name: "Diagnostics".into(),
        ..Default::default()
    };
    if let Some(directory) = &data_dir {
        if !directory.is_absolute() {
            return Err("--data-dir must be an absolute path.");
        }
        std::fs::create_dir_all(directory)
            .map_err(|_| "Could not create the diagnostic directory.")?;
        let path = directory.join("diagnostic-settings.json");
        if path.exists() {
            settings = serde_json::from_slice(
                &std::fs::read(path).map_err(|_| "Could not read diagnostic settings.")?,
            )
            .map_err(|_| "Invalid diagnostic settings.")?;
        } else {
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&settings)
                    .map_err(|_| "Could not encode diagnostic settings.")?,
            )
            .map_err(|_| "Could not save diagnostic settings.")?;
        }
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| "Could not start the diagnostic runtime.")?;
    let mut snapshots = Vec::new();
    let mut catalog = Vec::new();
    if read_providers {
        for provider in [Provider::Claude, Provider::Codex] {
            let started = Instant::now();
            let snapshot = runtime.block_on(providers::poll(provider, &settings));
            println!(
                "{}",
                json!({"check":"provider","elapsedMs":started.elapsed().as_millis(),"snapshot":snapshot})
            );
            snapshots.push(snapshot);
        }
        catalog = runtime
            .block_on(providers::discover_models(&settings))
            .map_err(|_| "Could not discover the local model catalog.")?;
        println!(
            "{}",
            json!({"check":"catalog","version":providers::CATALOG_VERSION,"models":catalog})
        );
    }
    if import {
        let directory = data_dir.ok_or("Missing diagnostic directory.")?;
        let mut store = Store::open(&directory.join("diagnostics.sqlite3"))
            .map_err(|_| "Could not open the diagnostic database.")?;
        let mut accounts: HashMap<_, _> = [Provider::Claude, Provider::Codex]
            .into_iter()
            .map(|provider| (provider, providers::current_account_id(&settings, provider)))
            .collect();
        for snapshot in &snapshots {
            accounts.insert(snapshot.provider, snapshot.account_id.clone());
            store
                .save_snapshot(snapshot)
                .map_err(|_| "Could not save a sanitized snapshot in the diagnostic database.")?;
        }
        for pass in 1..=2 {
            let started = Instant::now();
            let report = history::import_all(&mut store, &settings, &accounts)
                .map_err(|_| "History import failed; source logs were not changed.")?;
            let import_ms = started.elapsed().as_millis();
            let stats = store
                .stats(None, None)
                .map_err(|_| "Could not read diagnostic statistics.")?;
            println!(
                "{}",
                json!({"check":"import","pass":pass,"importMs":import_ms,"totalElapsedMs":started.elapsed().as_millis(),"filesRead":report.files,"addedPrompts":report.prompts,"addedRequests":report.requests,"warnings":report.warnings,"promptCount":stats.prompt_count,"conversationCount":stats.conversation_count,"requestCount":stats.request_count,"totalTokens":stats.total_tokens,"tokenTotals":stats.token_totals,"medianTokens":stats.median_tokens,"p75Tokens":stats.p75_tokens,"backgroundRequests":stats.background_requests})
            );
        }
        if read_providers {
            let now = chrono::Utc::now().timestamp();
            let stats = store
                .stats(Some(now - 30 * 86400), Some(now))
                .map_err(|_| "Could not read recent diagnostic statistics.")?;
            for task in [TaskClass::Quick, TaskClass::Everyday, TaskClass::Complex] {
                println!(
                    "{}",
                    json!({"check":"recommendation","task":task,"advice":recommendation::recommend(task,&snapshots,&stats,&catalog,settings.reserve_percent,now)})
                );
            }
        }
    }
    Ok(())
}
