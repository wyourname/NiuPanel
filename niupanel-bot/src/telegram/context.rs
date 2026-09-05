use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const DEFAULT_CONTEXT_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const DEFAULT_MAX_ENTRIES: usize = 2_048;
const MAX_CONTEXT_BYTES: usize = 8 * 1024;

#[derive(Clone)]
pub struct NotificationContextStore {
    state: Arc<Mutex<ContextState>>,
    ttl: Duration,
    max_entries: usize,
}

#[derive(Default)]
struct ContextState {
    entries: HashMap<(String, i32), ContextEntry>,
    next_sequence: u64,
}

struct ContextEntry {
    value: Value,
    sequence: u64,
    expires_at: Instant,
}

impl Default for NotificationContextStore {
    fn default() -> Self {
        Self::new(DEFAULT_CONTEXT_TTL, DEFAULT_MAX_ENTRIES)
    }
}

impl NotificationContextStore {
    fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(ContextState::default())),
            ttl,
            max_entries: max_entries.max(1),
        }
    }

    pub async fn remember(&self, chat_id: String, message_id: i32, value: Value) {
        if serde_json::to_vec(&value).is_ok_and(|encoded| encoded.len() > MAX_CONTEXT_BYTES) {
            return;
        }

        let mut state = self.state.lock().await;
        cleanup(&mut state);
        let key = (chat_id, message_id);
        if !state.entries.contains_key(&key) && state.entries.len() >= self.max_entries {
            let oldest = state
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.sequence)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                state.entries.remove(&oldest);
            }
        }
        let now = Instant::now();
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.wrapping_add(1);
        state.entries.insert(
            key,
            ContextEntry {
                value,
                sequence,
                expires_at: now + self.ttl,
            },
        );
    }

    pub async fn get(&self, chat_id: &str, message_id: i32) -> Option<Value> {
        let mut state = self.state.lock().await;
        cleanup(&mut state);
        state
            .entries
            .get(&(chat_id.to_string(), message_id))
            .map(|entry| entry.value.clone())
    }
}

fn cleanup(state: &mut ContextState) {
    let now = Instant::now();
    state.entries.retain(|_, entry| entry.expires_at > now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn contexts_are_isolated_by_chat_and_message() {
        let store = NotificationContextStore::new(Duration::from_secs(60), 8);
        store
            .remember("100".to_string(), 7, serde_json::json!({"task_id": 9}))
            .await;

        assert_eq!(store.get("100", 7).await.unwrap()["task_id"], 9);
        assert!(store.get("200", 7).await.is_none());
        assert!(store.get("100", 8).await.is_none());
    }

    #[tokio::test]
    async fn expired_and_oldest_contexts_are_removed() {
        let expired = NotificationContextStore::new(Duration::ZERO, 2);
        expired
            .remember("100".to_string(), 1, serde_json::json!({"id": 1}))
            .await;
        assert!(expired.get("100", 1).await.is_none());

        let bounded = NotificationContextStore::new(Duration::from_secs(60), 2);
        bounded
            .remember("100".to_string(), 1, serde_json::json!({"id": 1}))
            .await;
        bounded
            .remember("100".to_string(), 2, serde_json::json!({"id": 2}))
            .await;
        bounded
            .remember("100".to_string(), 3, serde_json::json!({"id": 3}))
            .await;
        assert!(bounded.get("100", 1).await.is_none());
        assert!(bounded.get("100", 2).await.is_some());
        assert!(bounded.get("100", 3).await.is_some());
    }
}
