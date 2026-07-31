use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

static DEPENDENCY_LOCKS: LazyLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct DependencyInstallLock {
    _guard: OwnedMutexGuard<()>,
}

impl DependencyInstallLock {
    pub async fn acquire(key: impl Into<String>) -> Self {
        let key = key.into();
        let lock = {
            let mut locks = DEPENDENCY_LOCKS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            locks
                .entry(key)
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };

        Self {
            _guard: lock.lock_owned().await,
        }
    }
}
