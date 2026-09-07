//! Sync immutable normalized batches through a user-selected existing cloud folder.
//! SQLite, source paths, provider credentials, and raw conversations stay local.
use crate::storage::{stable_id, Store, SyncEvent};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

const VERSION: u32 = 1;
const MAX_BATCH_BYTES: u64 = 8 * 1024 * 1024;
const EVENTS_PER_BATCH: usize = 500;

#[derive(Debug, Serialize, Deserialize)]
struct Batch {
    version: u32,
    device_id: String,
    events: Vec<SyncEvent>,
}

pub fn synchronize(store: &mut Store, folder: &Path, device_id: &str) -> Result<(), String> {
    if !folder.is_dir() {
        return Err("The shared folder is unavailable. Local history remains available; synchronization will retry.".into());
    }
    let directory = folder.join("gcd-usage-v1");
    fs::create_dir_all(&directory)
        .map_err(|_| "Cannot create the GCD Usage folder in the selected shared folder.")?;
    let entries = fs::read_dir(&directory).map_err(|_| "Cannot read the shared folder.")?;
    let mut problems = 0;
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => {
                problems += 1;
                continue;
            }
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("batch-") || path.extension().and_then(|s| s.to_str()) != Some("json")
        {
            continue;
        }
        // Canonical batches are immutable; skip known names before reading their contents.
        if let Some(digest) = name
            .strip_prefix("batch-")
            .and_then(|n| n.strip_suffix(".json"))
        {
            if digest.len() == 64
                && digest.bytes().all(|b| b.is_ascii_hexdigit())
                && store.has_batch(digest)?
            {
                continue;
            }
        }
        // Do not follow symlinks out of the selected folder.
        let metadata = match entry.file_type() {
            Ok(kind) if kind.is_file() => entry.metadata(),
            _ => continue,
        }
        .map_err(|_| "Cannot inspect a shared history batch.")?;
        if metadata.len() > MAX_BATCH_BYTES {
            problems += 1;
            continue;
        }
        let data = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                problems += 1;
                continue;
            }
        };
        if data.len() as u64 > MAX_BATCH_BYTES {
            problems += 1;
            continue;
        }
        let digest = stable_id(&[&String::from_utf8_lossy(&data)]);
        if store.has_batch(&digest)? {
            continue;
        }
        let batch = match serde_json::from_slice::<Batch>(&data) {
            Ok(batch) if batch.version == VERSION && batch.events.len() <= EVENTS_PER_BATCH => {
                batch
            }
            _ => {
                problems += 1;
                continue;
            }
        };
        store.begin()?;
        let imported = (|| -> Result<(), String> {
            for event in &batch.events {
                validate_event(event)?;
                store.apply_event(event, false)?;
            }
            store.mark_batch(&digest)?;
            store.commit()
        })();
        if imported.is_err() {
            store.rollback();
            problems += 1;
        }
    }
    store.resolve_links()?;
    // Bounded batches avoid holding the full event history in memory.
    loop {
        let pending = store.pending_events(EVENTS_PER_BATCH)?;
        if pending.is_empty() {
            break;
        }
        let hashes: Vec<_> = pending.iter().map(|(id, _)| id.clone()).collect();
        let batch = Batch {
            version: VERSION,
            device_id: device_id.into(),
            events: pending.into_iter().map(|(_, event)| event).collect(),
        };
        let bytes = serde_json::to_vec(&batch).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BATCH_BYTES {
            return Err(
                "A normalized history batch is unexpectedly large; local data was preserved."
                    .into(),
            );
        }
        let digest = stable_id(&[&String::from_utf8_lossy(&bytes)]);
        let final_path = directory.join(format!("batch-{digest}.json"));
        if !final_path.exists() {
            let temporary = directory.join(format!(".pending-{}", uuid::Uuid::new_v4()));
            let written = (|| -> Result<(), String> {
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)
                    .map_err(|_| "Cannot write to the shared folder.")?;
                file.write_all(&bytes)
                    .map_err(|_| "Cannot write to the shared folder.")?;
                file.sync_all()
                    .map_err(|_| "Cannot finish writing to the shared folder.")?;
                drop(file);
                fs::rename(&temporary, &final_path)
                    .map_err(|_| "Cannot publish the history batch in the shared folder.")?;
                Ok(())
            })();
            if written.is_err() {
                let _ = fs::remove_file(&temporary);
                written?;
            }
        }
        store.begin()?;
        let marked = (|| -> Result<(), String> {
            store.mark_exported(&hashes)?;
            store.mark_batch(&digest)?;
            store.commit()
        })();
        if marked.is_err() {
            store.rollback();
            marked?;
        }
    }
    if problems > 0 {
        Err(format!("Skipped {problems} unreadable or unsupported shared batches. Other history synchronized successfully."))
    } else {
        Ok(())
    }
}

fn validate_event(event: &SyncEvent) -> Result<(), String> {
    fn identifier(value: &str) -> bool {
        !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
    }
    let valid = match event {
        SyncEvent::Prompt(p) => {
            identifier(&p.id)
                && identifier(&p.account_id)
                && identifier(&p.device_id)
                && p.preview.chars().count() <= 160
                && p.session_id.len() <= 512
                && p.turn_id.len() <= 512
                && p.status.len() <= 64
        }
        SyncEvent::Request(r) => {
            identifier(&r.id)
                && identifier(&r.account_id)
                && identifier(&r.device_id)
                && r.prompt_id.as_ref().is_none_or(|p| identifier(p))
                && r.session_id.len() <= 512
                && r.model.len() <= 256
                && r.effort.as_ref().is_none_or(|s| s.len() <= 128)
                && r.source.len() <= 256
                && r.source_event_id.len() <= 512
        }
        SyncEvent::Snapshot(s) => {
            identifier(&s.id)
                && identifier(&s.account_id)
                && identifier(&s.device_id)
                && s.windows.len() <= 32
                && s.windows.iter().all(|w| {
                    identifier(&w.id)
                        && w.label.len() <= 256
                        && w.used_percent.is_finite()
                        && (0.0..=100.0).contains(&w.used_percent)
                })
        }
        SyncEvent::Link {
            request_id,
            account_id,
            turn_id,
            ..
        } => identifier(request_id) && identifier(account_id) && identifier(turn_id),
    };
    if valid {
        Ok(())
    } else {
        Err("Invalid shared history event".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    fn directory() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("gcd-sync-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn offline_duplicate_delivery_and_account_separation() {
        let folder = directory();
        let mut a = Store::open(Path::new(":memory:")).unwrap();
        let mut b = Store::open(Path::new(":memory:")).unwrap();
        let prompt = PromptRecord {
            id: "p".into(),
            provider: Provider::Claude,
            account_id: "account-a".into(),
            device_id: "a".into(),
            session_id: "s".into(),
            turn_id: "t".into(),
            timestamp: 100,
            preview: "Preview".into(),
            status: "completed".into(),
            completed_at: Some(125),
            kind: ActivityKind::User,
        };
        a.save_prompt(&prompt).unwrap();
        assert!(synchronize(&mut a, &folder.join("offline"), "a").is_err());
        synchronize(&mut a, &folder, "a").unwrap();
        synchronize(&mut b, &folder, "b").unwrap();
        let batch = fs::read_dir(folder.join("gcd-usage-v1"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::copy(&batch, folder.join("gcd-usage-v1").join("batch-copy.json")).unwrap();
        synchronize(&mut b, &folder, "b").unwrap();
        assert_eq!(b.stats(None, None).unwrap().prompt_count, 1);
        let mut other = prompt;
        other.id = "different-account".into();
        other.account_id = "account-b".into();
        b.save_prompt(&other).unwrap();
        synchronize(&mut b, &folder, "b").unwrap();
        synchronize(&mut a, &folder, "a").unwrap();
        assert_eq!(a.stats(None, None).unwrap().prompt_count, 2);
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn snapshot_messages_are_not_replicated() {
        let folder = directory();
        let mut a = Store::open(Path::new(":memory:")).unwrap();
        let mut b = Store::open(Path::new(":memory:")).unwrap();
        a.save_snapshot(&QuotaSnapshot {
            id: "s".into(),
            provider: Provider::Codex,
            account_id: "account".into(),
            device_id: "a".into(),
            fetched_at: 100,
            status: ConnectionStatus::NeedsAuth,
            windows: vec![],
            message: Some("local path omitted".into()),
            retry_after_seconds: None,
        })
        .unwrap();
        synchronize(&mut a, &folder, "a").unwrap();
        synchronize(&mut b, &folder, "b").unwrap();
        assert_eq!(b.latest_snapshots().unwrap()[0].message, None);
        fs::remove_dir_all(folder).unwrap();
    }
}
