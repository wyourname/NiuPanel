use super::*;
use chrono::{Duration as ChronoDuration, Utc};
use moka::future::Cache;
use nanoid::nanoid;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use utoipa::ToSchema;

const UPLOAD_SESSION_TTL: Duration = Duration::from_secs(15 * 60);
const UPLOAD_SESSION_CAPACITY: u64 = 64;
const MAX_PENDING_EXTRACTED_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_CONCURRENT_UPLOADS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluginUploadOperation {
    Install,
    Update,
}

impl PluginUploadOperation {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "install" => Ok(Self::Install),
            "update" => Ok(Self::Update),
            _ => Err(AppError::ValidationError(
                "operation must be 'install' or 'update'".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginUploadSessionResponse {
    pub token: String,
    pub expires_at: String,
    pub file_name: String,
    pub compressed_size: u64,
    pub extracted_size: u64,
    pub sha256: String,
    pub preview: PluginImpactPreview,
}

struct PluginUploadSession {
    owner_id: i32,
    operation: PluginUploadOperation,
    target_plugin_id: Option<String>,
    package: lifecycle::PreparedPluginPackage,
    _storage_reservation: StorageReservation,
}

struct StorageBudget {
    used: AtomicU64,
    limit: u64,
}

impl StorageBudget {
    fn reserve(self: &Arc<Self>, bytes: u64) -> Result<StorageReservation> {
        let mut current = self.used.load(AtomicOrdering::Acquire);
        loop {
            let next = current.checked_add(bytes).ok_or_else(|| {
                AppError::ConcurrencyLimitExceeded(
                    "Plugin upload storage budget is exhausted".to_string(),
                )
            })?;
            if next > self.limit {
                return Err(AppError::ConcurrencyLimitExceeded(
                    "Plugin upload storage budget is exhausted; commit or cancel a pending upload"
                        .to_string(),
                ));
            }
            match self.used.compare_exchange_weak(
                current,
                next,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            ) {
                Ok(_) => {
                    return Ok(StorageReservation {
                        budget: Arc::clone(self),
                        bytes,
                    });
                }
                Err(actual) => current = actual,
            }
        }
    }
}

struct StorageReservation {
    budget: Arc<StorageBudget>,
    bytes: u64,
}

impl StorageReservation {
    fn shrink_to(&mut self, bytes: u64) {
        debug_assert!(bytes <= self.bytes);
        let released = self.bytes.saturating_sub(bytes);
        if released > 0 {
            self.budget.used.fetch_sub(released, AtomicOrdering::AcqRel);
            self.bytes = bytes;
        }
    }
}

impl Drop for StorageReservation {
    fn drop(&mut self) {
        self.budget
            .used
            .fetch_sub(self.bytes, AtomicOrdering::AcqRel);
    }
}

pub struct PluginUploadSessionStore {
    sessions: Cache<String, Arc<PluginUploadSession>>,
    upload_root: OnceLock<PathBuf>,
    storage_budget: Arc<StorageBudget>,
    upload_slots: Arc<Semaphore>,
    commit_lock: Mutex<()>,
}

impl PluginUploadSessionStore {
    fn new() -> Self {
        Self {
            sessions: Cache::builder()
                .max_capacity(UPLOAD_SESSION_CAPACITY)
                .time_to_live(UPLOAD_SESSION_TTL)
                .build(),
            upload_root: OnceLock::new(),
            storage_budget: Arc::new(StorageBudget {
                used: AtomicU64::new(0),
                limit: MAX_PENDING_EXTRACTED_BYTES,
            }),
            upload_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_UPLOADS)),
            commit_lock: Mutex::new(()),
        }
    }

    fn upload_root(&self) -> PathBuf {
        self.upload_root
            .get_or_init(|| {
                let upload_root = Config::global().plugins_dir.join(".uploads");
                if let Err(error) = fs::remove_dir_all(&upload_root)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    tracing::warn!(
                        path = %upload_root.display(),
                        %error,
                        "failed to remove stale plugin upload sessions"
                    );
                }
                if let Err(error) = fs::create_dir_all(&upload_root) {
                    tracing::warn!(
                        path = %upload_root.display(),
                        %error,
                        "failed to initialize plugin upload directory"
                    );
                }
                upload_root
            })
            .clone()
    }

    fn reserve_extraction(&self) -> Result<StorageReservation> {
        self.storage_budget
            .reserve(lifecycle::MAX_PLUGIN_EXTRACTED_BYTES)
    }

    fn acquire_upload_slot(&self) -> Result<OwnedSemaphorePermit> {
        Arc::clone(&self.upload_slots)
            .try_acquire_owned()
            .map_err(|_| {
                AppError::ConcurrencyLimitExceeded(
                    "Too many plugin uploads are being processed".to_string(),
                )
            })
    }

    async fn insert(&self, token: String, session: PluginUploadSession) {
        self.sessions.insert(token.clone(), Arc::new(session)).await;
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            tokio::time::sleep(UPLOAD_SESSION_TTL).await;
            sessions.remove(&token).await;
        });
    }

    async fn take_owned(&self, token: &str, owner_id: i32) -> Result<Arc<PluginUploadSession>> {
        let session = self.owned(token, owner_id).await?;
        let removed = self
            .sessions
            .remove(token)
            .await
            .ok_or_else(session_not_found)?;
        if !Arc::ptr_eq(&session, &removed) {
            return Err(session_not_found());
        }
        Ok(removed)
    }

    async fn owned(&self, token: &str, owner_id: i32) -> Result<Arc<PluginUploadSession>> {
        let session = self
            .sessions
            .get(token)
            .await
            .ok_or_else(session_not_found)?;
        if session.owner_id != owner_id {
            return Err(session_not_found());
        }
        Ok(session)
    }
}

fn session_not_found() -> AppError {
    AppError::NotFound("Plugin upload session not found or expired".to_string())
}

static PLUGIN_UPLOAD_SESSION_STORE: OnceLock<Arc<PluginUploadSessionStore>> = OnceLock::new();

pub fn plugin_upload_session_store() -> Arc<PluginUploadSessionStore> {
    Arc::clone(
        PLUGIN_UPLOAD_SESSION_STORE.get_or_init(|| Arc::new(PluginUploadSessionStore::new())),
    )
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/upload-sessions",
    responses((status = 201, body = PluginUploadSessionResponse)),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn create_plugin_upload_session(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(store): Extension<Arc<PluginUploadSessionStore>>,
    multipart: Multipart,
) -> Result<(StatusCode, ApiResponse<PluginUploadSessionResponse>)> {
    let upload_slot = store.acquire_upload_slot()?;
    let reservation = store.reserve_extraction()?;
    let upload_root = store.upload_root();
    let (package, reservation) = tokio::spawn(async move {
        let _upload_slot = upload_slot;
        let mut reservation = reservation;
        let package = lifecycle::prepare_plugin_package_upload(
            multipart,
            upload_root,
            ManifestCompatibility::PluginJsonOnly,
            "Plugin",
        )
        .await?;
        reservation.shrink_to(package.extracted_size);
        Ok::<_, AppError>((package, reservation))
    })
    .await
    .map_err(|err| AppError::Internal(format!("Plugin upload task failed: {err}")))??;

    let operation = PluginUploadOperation::parse(&package.operation)?;
    match operation {
        PluginUploadOperation::Install if package.target_plugin_id.is_some() => {
            return Err(AppError::ValidationError(
                "target_plugin_id is only valid for update uploads".to_string(),
            ));
        }
        PluginUploadOperation::Update if package.target_plugin_id.is_none() => {
            return Err(AppError::ValidationError(
                "target_plugin_id is required for update uploads".to_string(),
            ));
        }
        _ => {}
    }

    unified_plugin_service().validate_package_dir(package.package_root())?;
    let preview = match operation {
        PluginUploadOperation::Install => {
            preview_plugin_impact("install", PLUGIN_CONTEXT, package.package_root(), None)?
        }
        PluginUploadOperation::Update => preview_plugin_impact(
            "update",
            PLUGIN_CONTEXT,
            package.package_root(),
            package.target_plugin_id.as_deref(),
        )?,
    };

    let token = nanoid!(32);
    let expires_at = Utc::now() + ChronoDuration::from_std(UPLOAD_SESSION_TTL).unwrap();
    let response = PluginUploadSessionResponse {
        token: token.clone(),
        expires_at: expires_at.to_rfc3339(),
        file_name: package.file_name.clone(),
        compressed_size: package.compressed_size,
        extracted_size: package.extracted_size,
        sha256: package.sha256.clone(),
        preview,
    };
    store
        .insert(
            token,
            PluginUploadSession {
                owner_id: user.id,
                operation,
                target_plugin_id: package.target_plugin_id.clone(),
                package,
                _storage_reservation: reservation,
            },
        )
        .await;

    Ok((StatusCode::CREATED, ApiResponse::success(response)))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/upload-sessions/{token}/commit",
    responses((status = 200, body = PluginRecord)),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn commit_plugin_upload_session(
    AxumPath(token): AxumPath<String>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(store): Extension<Arc<PluginUploadSessionStore>>,
) -> Result<ApiResponse<PluginRecord>> {
    let session = store.take_owned(&token, user.id).await?;
    let _commit_guard = store.commit_lock.lock().await;
    let service = unified_plugin_service();
    let record = match session.operation {
        PluginUploadOperation::Install => {
            ensure_plugin_impact_allowed(
                "install",
                PLUGIN_CONTEXT,
                session.package.package_root(),
                None,
            )?;
            service.install_prepared_dir(
                session.package.package_root(),
                session.package.enable,
                session.package.sha256.clone(),
            )?
        }
        PluginUploadOperation::Update => {
            let plugin_id = session
                .target_plugin_id
                .as_deref()
                .ok_or_else(session_not_found)?;
            ensure_plugin_impact_allowed(
                "update",
                PLUGIN_CONTEXT,
                session.package.package_root(),
                Some(plugin_id),
            )?;
            service
                .update_prepared_dir_async(
                    plugin_id,
                    session.package.package_root(),
                    session.package.sha256.clone(),
                )
                .await?
        }
    };
    Ok(ApiResponse::success(record))
}

#[utoipa::path(
    delete,
    path = "/api/v1/plugins/upload-sessions/{token}",
    responses((status = 204)),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn delete_plugin_upload_session(
    AxumPath(token): AxumPath<String>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(store): Extension<Arc<PluginUploadSessionStore>>,
) -> Result<StatusCode> {
    store.take_owned(&token, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_budget_tracks_actual_retained_bytes() {
        let budget = Arc::new(StorageBudget {
            used: AtomicU64::new(0),
            limit: lifecycle::MAX_PLUGIN_EXTRACTED_BYTES,
        });
        let mut reservation = budget
            .reserve(lifecycle::MAX_PLUGIN_EXTRACTED_BYTES)
            .expect("reserve extraction maximum");
        assert!(budget.reserve(1).is_err());

        reservation.shrink_to(128);
        let second = budget
            .reserve(lifecycle::MAX_PLUGIN_EXTRACTED_BYTES - 128)
            .expect("released capacity can be reused");
        assert_eq!(
            budget.used.load(AtomicOrdering::Acquire),
            lifecycle::MAX_PLUGIN_EXTRACTED_BYTES
        );
        drop(second);
        drop(reservation);
        assert_eq!(budget.used.load(AtomicOrdering::Acquire), 0);
    }

    #[test]
    fn upload_operation_is_strict() {
        assert_eq!(
            PluginUploadOperation::parse("install").unwrap(),
            PluginUploadOperation::Install
        );
        assert_eq!(
            PluginUploadOperation::parse("update").unwrap(),
            PluginUploadOperation::Update
        );
        assert!(PluginUploadOperation::parse("preview").is_err());
    }
}
