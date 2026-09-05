use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const LEDGER_TTL_SECONDS: u64 = 24 * 60 * 60;
const MAX_LEDGER_ENTRIES: usize = 2_048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerBegin {
    Started,
    Duplicate,
    Interrupted,
}

#[derive(Clone)]
pub struct TelegramMessageLedger {
    path: Option<Arc<PathBuf>>,
    state: Arc<Mutex<LedgerState>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct LedgerState {
    entries: BTreeMap<String, LedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LedgerEntry {
    state: LedgerEntryState,
    updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LedgerEntryState {
    Processing,
    Completed,
    Interrupted,
}

impl TelegramMessageLedger {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            state: Arc::new(Mutex::new(LedgerState::default())),
        }
    }

    pub async fn persistent(path: PathBuf) -> Result<Self, String> {
        let mut state = if path.is_file() {
            let bytes = tokio::fs::read(&path)
                .await
                .map_err(|error| format!("read Telegram message ledger: {error}"))?;
            serde_json::from_slice::<LedgerState>(&bytes)
                .map_err(|error| format!("parse Telegram message ledger: {error}"))?
        } else {
            LedgerState::default()
        };
        let now = unix_timestamp();
        cleanup(&mut state, now);
        for entry in state.entries.values_mut() {
            if entry.state == LedgerEntryState::Processing {
                entry.state = LedgerEntryState::Interrupted;
                entry.updated_at = now;
            }
        }
        let ledger = Self {
            path: Some(Arc::new(path)),
            state: Arc::new(Mutex::new(state)),
        };
        {
            let state = ledger.state.lock().await;
            ledger.persist(&state).await?;
        }
        Ok(ledger)
    }

    pub async fn begin(&self, key: &str) -> Result<LedgerBegin, String> {
        let mut state = self.state.lock().await;
        let now = unix_timestamp();
        cleanup(&mut state, now);
        let outcome = match state.entries.get(key).map(|entry| entry.state) {
            Some(LedgerEntryState::Processing | LedgerEntryState::Completed) => {
                LedgerBegin::Duplicate
            }
            Some(LedgerEntryState::Interrupted) => {
                if let Some(entry) = state.entries.get_mut(key) {
                    entry.state = LedgerEntryState::Completed;
                    entry.updated_at = now;
                }
                LedgerBegin::Interrupted
            }
            None => {
                state.entries.insert(
                    key.to_string(),
                    LedgerEntry {
                        state: LedgerEntryState::Processing,
                        updated_at: now,
                    },
                );
                LedgerBegin::Started
            }
        };
        self.persist(&state).await?;
        Ok(outcome)
    }

    pub async fn complete(&self, key: &str) -> Result<(), String> {
        let mut state = self.state.lock().await;
        state.entries.insert(
            key.to_string(),
            LedgerEntry {
                state: LedgerEntryState::Completed,
                updated_at: unix_timestamp(),
            },
        );
        cleanup(&mut state, unix_timestamp());
        self.persist(&state).await
    }

    async fn persist(&self, state: &LedgerState) -> Result<(), String> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| format!("create Telegram ledger directory: {error}"))?;
        }
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| format!("serialize Telegram message ledger: {error}"))?;
        tokio::fs::write(&temporary, bytes)
            .await
            .map_err(|error| format!("write Telegram message ledger: {error}"))?;
        set_private_file_permissions(&temporary)?;
        tokio::fs::rename(&temporary, path)
            .await
            .map_err(|error| format!("replace Telegram message ledger: {error}"))?;
        set_private_file_permissions(path)
    }
}

fn cleanup(state: &mut LedgerState, now: u64) {
    state
        .entries
        .retain(|_, entry| now.saturating_sub(entry.updated_at) <= LEDGER_TTL_SECONDS);
    while state.entries.len() > MAX_LEDGER_ENTRIES {
        let Some(oldest) = state
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.updated_at)
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        state.entries.remove(&oldest);
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("protect Telegram message ledger: {error}"))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn duplicate_messages_are_not_started_twice() {
        let ledger = TelegramMessageLedger::in_memory();
        assert_eq!(
            ledger.begin("bot:chat:1").await.unwrap(),
            LedgerBegin::Started
        );
        assert_eq!(
            ledger.begin("bot:chat:1").await.unwrap(),
            LedgerBegin::Duplicate
        );
        ledger.complete("bot:chat:1").await.unwrap();
        assert_eq!(
            ledger.begin("bot:chat:1").await.unwrap(),
            LedgerBegin::Duplicate
        );
    }

    #[tokio::test]
    async fn restart_marks_inflight_message_interrupted_instead_of_replaying_it() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("ledger.json");
        let ledger = TelegramMessageLedger::persistent(path.clone())
            .await
            .expect("ledger");
        assert_eq!(
            ledger.begin("bot:chat:2").await.unwrap(),
            LedgerBegin::Started
        );
        drop(ledger);

        let restarted = TelegramMessageLedger::persistent(path)
            .await
            .expect("restarted ledger");
        assert_eq!(
            restarted.begin("bot:chat:2").await.unwrap(),
            LedgerBegin::Interrupted
        );
        assert_eq!(
            restarted.begin("bot:chat:2").await.unwrap(),
            LedgerBegin::Duplicate
        );
    }
}
