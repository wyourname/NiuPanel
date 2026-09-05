use niupanel_common::error::{AppError, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex, OnceLock, Weak};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedMutexGuard};

const RUNNING_TTL: Duration = Duration::from_secs(3 * 60);
const TERMINAL_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_IDEMPOTENCY_ENTRIES: usize = 2_048;

#[derive(Debug, Clone)]
pub struct InvocationToken {
    key: String,
    fingerprint: String,
}

pub enum IdempotencyDecision {
    Execute(Option<InvocationToken>),
    Replay(Value),
}

#[derive(Debug)]
enum InvocationOutcome {
    Running,
    Completed(Value),
    Failed,
}

#[derive(Debug)]
struct InvocationRecord {
    fingerprint: String,
    outcome: InvocationOutcome,
    created_at: Instant,
    expires_at: Instant,
}

#[derive(Default)]
struct InvocationStore {
    records: HashMap<String, InvocationRecord>,
}

impl InvocationStore {
    fn cleanup(&mut self) {
        let now = Instant::now();
        for record in self.records.values_mut() {
            if matches!(record.outcome, InvocationOutcome::Running) && record.expires_at <= now {
                record.outcome = InvocationOutcome::Failed;
                record.expires_at = now + TERMINAL_TTL;
            }
        }
        self.records.retain(|_, record| record.expires_at > now);
        if self.records.len() <= MAX_IDEMPOTENCY_ENTRIES {
            return;
        }
        let mut oldest = self
            .records
            .iter()
            .map(|(key, record)| (key.clone(), record.created_at))
            .collect::<Vec<_>>();
        oldest.sort_by_key(|(_, created_at)| *created_at);
        for (key, _) in oldest
            .into_iter()
            .take(self.records.len() - MAX_IDEMPOTENCY_ENTRIES)
        {
            self.records.remove(&key);
        }
    }
}

fn invocation_store() -> &'static Mutex<InvocationStore> {
    static STORE: OnceLock<Mutex<InvocationStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(InvocationStore::default()))
}

fn session_locks() -> &'static StdMutex<HashMap<String, Weak<Mutex<()>>>> {
    static LOCKS: OnceLock<StdMutex<HashMap<String, Weak<Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| StdMutex::new(HashMap::new()))
}

pub async fn acquire_session(scope: &str) -> OwnedMutexGuard<()> {
    let lock = {
        let mut locks = session_locks()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(scope).and_then(Weak::upgrade) {
            lock
        } else {
            let lock = Arc::new(Mutex::new(()));
            locks.insert(scope.to_string(), Arc::downgrade(&lock));
            lock
        }
    };
    lock.lock_owned().await
}

pub async fn begin(
    scope: &str,
    action: &str,
    client_request_id: Option<&str>,
    input: &Value,
) -> Result<IdempotencyDecision> {
    let Some(client_request_id) = normalize_client_request_id(client_request_id)? else {
        return Ok(IdempotencyDecision::Execute(None));
    };
    let key = format!("{scope}:{action}:{client_request_id}");
    let fingerprint = request_fingerprint(action, input)?;
    let mut store = invocation_store().lock().await;
    store.cleanup();
    if let Some(record) = store.records.get(&key) {
        if record.fingerprint != fingerprint {
            return Err(AppError::ValidationError(
                "client_request_id was already used with a different plugin request".to_string(),
            ));
        }
        return match &record.outcome {
            InvocationOutcome::Completed(output) => Ok(IdempotencyDecision::Replay(output.clone())),
            InvocationOutcome::Running => Err(AppError::ConcurrencyLimitExceeded(
                "The same plugin request is still running".to_string(),
            )),
            InvocationOutcome::Failed => Err(AppError::ValidationError(
                "The previous plugin request failed; use a new client_request_id to retry"
                    .to_string(),
            )),
        };
    }
    let now = Instant::now();
    store.records.insert(
        key.clone(),
        InvocationRecord {
            fingerprint: fingerprint.clone(),
            outcome: InvocationOutcome::Running,
            created_at: now,
            expires_at: now + RUNNING_TTL,
        },
    );
    Ok(IdempotencyDecision::Execute(Some(InvocationToken {
        key,
        fingerprint,
    })))
}

pub async fn complete(token: Option<InvocationToken>, output: &Value) {
    finish(token, InvocationOutcome::Completed(output.clone())).await;
}

pub async fn fail(token: Option<InvocationToken>) {
    finish(token, InvocationOutcome::Failed).await;
}

async fn finish(token: Option<InvocationToken>, outcome: InvocationOutcome) {
    let Some(token) = token else {
        return;
    };
    let mut store = invocation_store().lock().await;
    store.cleanup();
    if let Some(record) = store.records.get_mut(&token.key)
        && record.fingerprint == token.fingerprint
    {
        record.outcome = outcome;
        record.expires_at = Instant::now() + TERMINAL_TTL;
    }
}

fn normalize_client_request_id(value: Option<&str>) -> Result<Option<&str>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(AppError::ValidationError(
            "client_request_id contains unsupported characters or is too long".to_string(),
        ));
    }
    Ok(Some(value))
}

fn request_fingerprint(action: &str, input: &Value) -> Result<String> {
    let payload = serde_json::to_vec(&(action, input))?;
    let mut hasher = Sha256::new();
    hasher.update(payload);
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn completed_request_is_replayed_but_changed_input_is_rejected() {
        let request_id = nanoid::nanoid!(12);
        let first = begin(
            "plugin:user:1:session",
            "chat",
            Some(&request_id),
            &serde_json::json!({"message": "hello"}),
        )
        .await
        .expect("begin");
        let IdempotencyDecision::Execute(token) = first else {
            panic!("first request must execute");
        };
        complete(token, &serde_json::json!({"message": "done"})).await;
        let replay = begin(
            "plugin:user:1:session",
            "chat",
            Some(&request_id),
            &serde_json::json!({"message": "hello"}),
        )
        .await
        .expect("replay");
        assert!(matches!(replay, IdempotencyDecision::Replay(_)));
        assert!(
            begin(
                "plugin:user:1:session",
                "chat",
                Some(&request_id),
                &serde_json::json!({"message": "different"}),
            )
            .await
            .is_err()
        );
    }
}
