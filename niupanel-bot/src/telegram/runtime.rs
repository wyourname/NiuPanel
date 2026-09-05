use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex, Weak};

use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

use super::agent::{
    TelegramAgentError, TelegramAgentHandler, TelegramAgentProgress, TelegramAgentRequest,
    TelegramAgentResponse,
};

#[derive(Clone)]
pub struct TelegramAgentRuntime {
    handler: TelegramAgentHandler,
    sessions: Arc<StdMutex<HashMap<String, Weak<Mutex<()>>>>>,
    invocations: Arc<StdMutex<HashMap<String, HashMap<u64, CancellationToken>>>>,
    slots: Arc<Semaphore>,
    next_invocation_id: Arc<AtomicU64>,
}

impl TelegramAgentRuntime {
    pub fn new(handler: TelegramAgentHandler, max_concurrency: usize) -> Self {
        Self {
            handler,
            sessions: Arc::new(StdMutex::new(HashMap::new())),
            invocations: Arc::new(StdMutex::new(HashMap::new())),
            slots: Arc::new(Semaphore::new(max_concurrency.max(1))),
            next_invocation_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn invoke(
        &self,
        request: TelegramAgentRequest,
    ) -> Result<TelegramAgentResponse, TelegramAgentError> {
        let session_key = request.session_key();
        let cancellation = CancellationToken::new();
        let registration = self.register(&session_key, cancellation.clone());
        let session = self.session_lock(&session_key);

        request.report_progress(TelegramAgentProgress::WaitingForSession);
        let _session_guard = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(TelegramAgentError::Cancelled),
            guard = session.lock() => guard,
        };

        request.report_progress(TelegramAgentProgress::WaitingForCapacity);
        let _slot = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(TelegramAgentError::Cancelled),
            permit = self.slots.clone().acquire_owned() => permit.map_err(|_| {
                TelegramAgentError::Failed("Telegram Agent runtime is shutting down".to_string())
            })?,
        };

        request.report_progress(TelegramAgentProgress::Running);
        let result = (self.handler)(request, cancellation.clone()).await;
        drop(registration);
        result
    }

    pub fn cancel_session(&self, session_key: &str) -> usize {
        let tokens = self
            .invocations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(session_key)
            .map(|invocations| invocations.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for token in &tokens {
            token.cancel();
        }
        tokens.len()
    }

    fn session_lock(&self, session_key: &str) -> Arc<Mutex<()>> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        sessions.retain(|_, session| session.strong_count() > 0);
        if let Some(session) = sessions.get(session_key).and_then(Weak::upgrade) {
            return session;
        }
        let session = Arc::new(Mutex::new(()));
        sessions.insert(session_key.to_string(), Arc::downgrade(&session));
        session
    }

    fn register(
        &self,
        session_key: &str,
        cancellation: CancellationToken,
    ) -> InvocationRegistration {
        let id = self.next_invocation_id.fetch_add(1, Ordering::Relaxed);
        self.invocations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(session_key.to_string())
            .or_default()
            .insert(id, cancellation.clone());
        InvocationRegistration {
            id,
            session_key: session_key.to_string(),
            cancellation,
            invocations: self.invocations.clone(),
        }
    }
}

struct InvocationRegistration {
    id: u64,
    session_key: String,
    cancellation: CancellationToken,
    invocations: Arc<StdMutex<HashMap<String, HashMap<u64, CancellationToken>>>>,
}

impl Drop for InvocationRegistration {
    fn drop(&mut self) {
        self.cancellation.cancel();
        let mut invocations = self
            .invocations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(session) = invocations.get_mut(&self.session_key) {
            session.remove(&self.id);
            if session.is_empty() {
                invocations.remove(&self.session_key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::Value;
    use tokio::sync::{Notify, watch};

    use super::*;
    use crate::telegram::agent::{TelegramAgentAction, TelegramAgentProgress};

    fn request(telegram_user_id: &str) -> TelegramAgentRequest {
        let (progress, _) = watch::channel(TelegramAgentProgress::Queued);
        TelegramAgentRequest {
            client_request_id: format!("test-{telegram_user_id}"),
            chat_id: "-1001".to_string(),
            thread_id: Some(7),
            telegram_user_id: telegram_user_id.to_string(),
            user_id: 1,
            text: "diagnose".to_string(),
            attachments: Vec::new(),
            action: TelegramAgentAction::Chat,
            panel_context: Value::Null,
            progress,
        }
    }

    #[tokio::test]
    async fn same_session_is_sequential_but_different_sessions_can_run_in_parallel() {
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let handler: TelegramAgentHandler = Arc::new({
            let active = active.clone();
            let peak = peak.clone();
            move |_, _| {
                let active = active.clone();
                let peak = peak.clone();
                Box::pin(async move {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(TelegramAgentResponse {
                        text: "ok".to_string(),
                    })
                })
            }
        });
        let runtime = TelegramAgentRuntime::new(handler, 2);

        let same_a = tokio::spawn({
            let runtime = runtime.clone();
            async move { runtime.invoke(request("10")).await }
        });
        let same_b = tokio::spawn({
            let runtime = runtime.clone();
            async move { runtime.invoke(request("10")).await }
        });
        same_a.await.unwrap().unwrap();
        same_b.await.unwrap().unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), 1);

        peak.store(0, Ordering::SeqCst);
        let different_a = tokio::spawn({
            let runtime = runtime.clone();
            async move { runtime.invoke(request("10")).await }
        });
        let different_b = tokio::spawn({
            let runtime = runtime.clone();
            async move { runtime.invoke(request("11")).await }
        });
        different_a.await.unwrap().unwrap();
        different_b.await.unwrap().unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cancellation_reaches_the_running_handler() {
        let started = Arc::new(Notify::new());
        let handler: TelegramAgentHandler = Arc::new({
            let started = started.clone();
            move |_, cancellation| {
                let started = started.clone();
                Box::pin(async move {
                    started.notify_one();
                    cancellation.cancelled().await;
                    Err(TelegramAgentError::Cancelled)
                })
            }
        });
        let runtime = TelegramAgentRuntime::new(handler, 1);
        let request = request("10");
        let session_key = request.session_key();
        let invocation = tokio::spawn({
            let runtime = runtime.clone();
            async move { runtime.invoke(request).await }
        });
        started.notified().await;

        assert_eq!(runtime.cancel_session(&session_key), 1);
        assert_eq!(
            invocation.await.unwrap().unwrap_err(),
            TelegramAgentError::Cancelled
        );
        assert_eq!(runtime.cancel_session(&session_key), 0);
    }
}
