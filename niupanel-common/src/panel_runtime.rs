use crate::version::{CoreReleaseDescriptor, PanelActivationReason, RELEASE_PROTOCOL_VERSION};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait, Value,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

pub const PANEL_RUNTIME_DATABASE_FILE: &str = "runtime.db";
pub const PANEL_RUNTIME_SCHEMA_VERSION: i64 = 1;
pub const PANEL_RUNTIME_NOT_INITIALIZED: &str = "Panel runtime is not initialized";
const DIRECTORY_DIGEST_DOMAIN: &[u8] = b"NIUPANEL-DIRECTORY-DIGEST-V1\0";
static ACTIVATION_QUEUE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PanelReleaseDescriptor {
    pub version: String,
    pub core: CoreReleaseDescriptor,
    pub core_digest: String,
    pub web_version: String,
    pub web_path: String,
    pub web_digest: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingPanelActivation {
    pub transaction_id: String,
    pub candidate: PanelReleaseDescriptor,
    pub reason: PanelActivationReason,
    pub requested_at: String,
    /// Snapshot of the database before this activation attempt. Launcher writes
    /// it to runtime.db before starting the candidate so an interrupted attempt
    /// can be retried without losing the original rollback point.
    pub activation_snapshot_path: Option<String>,
    pub restore_before_start: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PanelActivationFailure {
    pub transaction_id: String,
    pub version: String,
    pub failed_at: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PanelRuntimeState {
    pub protocol: u32,
    pub active: PanelReleaseDescriptor,
    pub previous: Option<PanelReleaseDescriptor>,
    pub pending: Option<PendingPanelActivation>,
    pub last_failure: Option<PanelActivationFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PanelActivationTransaction {
    pub transaction_id: String,
    pub from_version: String,
    pub to_version: String,
    pub reason: PanelActivationReason,
    pub snapshot_path: String,
    pub requested_at: String,
    pub completed_at: String,
    pub successful: bool,
    pub error: Option<String>,
}

pub fn runtime_database_path(system_root: impl AsRef<Path>) -> PathBuf {
    system_root.as_ref().join(PANEL_RUNTIME_DATABASE_FILE)
}

/// Hashes an unpacked component deterministically using each relative path and
/// its bytes. This digest is independent from directory iteration order.
pub fn directory_digest(root: impl AsRef<Path>) -> Result<String, String> {
    fn collect(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(current)
            .map_err(|error| format!("Failed to read {}: {error}", current.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_dir() {
                collect(root, &entry.path(), output)?;
            } else if file_type.is_file() {
                output.push(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|error| error.to_string())?
                        .to_path_buf(),
                );
            } else {
                return Err(format!(
                    "Release directory contains an unsupported entry: {}",
                    entry.path().display()
                ));
            }
        }
        Ok(())
    }

    let root = root.as_ref();
    if !root.is_dir() {
        return Err(format!(
            "Release directory does not exist: {}",
            root.display()
        ));
    }
    let mut paths = Vec::new();
    collect(root, root, &mut paths)?;
    paths.sort();
    let mut hasher = Sha256::new();
    hasher.update(DIRECTORY_DIGEST_DOMAIN);
    for relative in paths {
        let relative = relative.to_str().ok_or_else(|| {
            "Release directory contains a path that is not valid UTF-8".to_string()
        })?;
        let relative_bytes = relative.as_bytes();
        hasher.update((relative_bytes.len() as u64).to_le_bytes());
        hasher.update(relative_bytes);
        let file_path = root.join(relative);
        let mut file = fs::File::open(&file_path)
            .map_err(|error| format!("Failed to open {}: {error}", file_path.display()))?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("Failed to inspect {}: {error}", file_path.display()))?;
        let length = metadata.len();
        hasher.update(length.to_le_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            hasher.update((metadata.permissions().mode() & 0o7777).to_le_bytes());
        }
        #[cfg(not(unix))]
        hasher.update(0_u32.to_le_bytes());
        let mut buffer = [0_u8; 64 * 1024];
        let mut observed_length = 0_u64;
        loop {
            let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }
            observed_length += count as u64;
            hasher.update(&buffer[..count]);
        }
        if observed_length != length {
            return Err(format!(
                "Release file changed while it was being hashed: {}",
                file_path.display()
            ));
        }
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_panel_release(release: &PanelReleaseDescriptor) -> Result<(), String> {
    let core_root = Path::new(&release.core.path)
        .parent()
        .ok_or_else(|| format!("Panel {} Core path has no parent", release.version))?;
    let actual_core = directory_digest(core_root)?;
    if !actual_core.eq_ignore_ascii_case(&release.core_digest) {
        return Err(format!(
            "Panel {} Core digest does not match runtime.db",
            release.version
        ));
    }
    let actual_web = directory_digest(&release.web_path)?;
    if !actual_web.eq_ignore_ascii_case(&release.web_digest) {
        return Err(format!(
            "Panel {} Web digest does not match runtime.db",
            release.version
        ));
    }
    Ok(())
}

async fn open(system_root: impl AsRef<Path>) -> Result<DatabaseConnection, String> {
    let path = runtime_database_path(system_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    let url = format!("sqlite://{}?mode=rwc", path.to_string_lossy());
    let connection = Database::connect(&url)
        .await
        .map_err(|error| format!("Failed to open {}: {error}", path.display()))?;
    connection
        .execute_unprepared(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;",
        )
        .await
        .map_err(|error| format!("Failed to configure {}: {error}", path.display()))?;
    let schema_version = connection
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA user_version".to_string(),
        ))
        .await
        .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?
        .ok_or_else(|| format!("Failed to read schema version from {}", path.display()))?
        .try_get_by_index::<i64>(0)
        .map_err(|error| format!("Failed to decode schema version: {error}"))?;
    match schema_version {
        0 => {
            let existing_tables = connection
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'panel_%'"
                        .to_string(),
                ))
                .await
                .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?
                .ok_or_else(|| format!("Failed to inspect tables in {}", path.display()))?
                .try_get_by_index::<i64>(0)
                .map_err(|error| format!("Failed to decode table count: {error}"))?;
            if existing_tables != 0 {
                return Err(format!(
                    "Panel runtime database {} has unversioned tables and cannot be upgraded safely",
                    path.display()
                ));
            }
            initialize_schema(&connection, &path).await?;
        }
        PANEL_RUNTIME_SCHEMA_VERSION => {}
        version => {
            return Err(format!(
                "Unsupported panel runtime database schema {version}; expected {PANEL_RUNTIME_SCHEMA_VERSION}"
            ));
        }
    }
    Ok(connection)
}

async fn initialize_schema(connection: &DatabaseConnection, path: &Path) -> Result<(), String> {
    let transaction = connection.begin().await.map_err(|error| {
        format!(
            "Failed to begin initialization of {}: {error}",
            path.display()
        )
    })?;
    let schema = format!(
        "CREATE TABLE panel_releases (
           version TEXT PRIMARY KEY, core_version TEXT NOT NULL, core_path TEXT NOT NULL,
           core_digest TEXT NOT NULL,
           launcher_protocol INTEGER NOT NULL, api_contract INTEGER NOT NULL,
           schema_epoch INTEGER NOT NULL, schema_revision INTEGER NOT NULL,
           target TEXT NOT NULL, core_installed_at TEXT NOT NULL,
           web_version TEXT NOT NULL, web_path TEXT NOT NULL, web_digest TEXT NOT NULL,
           installed_at TEXT NOT NULL
         );
         CREATE TABLE panel_runtime_state (
           id INTEGER PRIMARY KEY CHECK (id = 1), protocol INTEGER NOT NULL,
           active_version TEXT NOT NULL REFERENCES panel_releases(version),
           previous_version TEXT REFERENCES panel_releases(version),
           pending_version TEXT REFERENCES panel_releases(version),
           pending_transaction_id TEXT, pending_reason TEXT,
           pending_requested_at TEXT, pending_activation_snapshot_path TEXT,
           pending_restore_path TEXT,
           failure_transaction_id TEXT, failure_version TEXT, failure_at TEXT,
           failure_message TEXT
         );
         CREATE TABLE panel_transactions (
           transaction_id TEXT PRIMARY KEY,
           from_version TEXT NOT NULL REFERENCES panel_releases(version),
           to_version TEXT NOT NULL REFERENCES panel_releases(version),
           reason TEXT NOT NULL, snapshot_path TEXT NOT NULL,
           requested_at TEXT NOT NULL, completed_at TEXT NOT NULL,
           successful INTEGER NOT NULL, error TEXT
         );
         PRAGMA user_version = {PANEL_RUNTIME_SCHEMA_VERSION};"
    );
    transaction
        .execute_unprepared(&schema)
        .await
        .map_err(|error| format!("Failed to initialize {}: {error}", path.display()))?;
    transaction.commit().await.map_err(|error| {
        format!(
            "Failed to commit initialization of {}: {error}",
            path.display()
        )
    })
}

fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Sqlite, sql, values)
}

fn text(value: &str) -> Value {
    value.to_owned().into()
}
fn optional_text(value: Option<&str>) -> Value {
    value.map(str::to_owned).into()
}
fn reason_name(reason: &PanelActivationReason) -> &'static str {
    match reason {
        PanelActivationReason::Update => "update",
        PanelActivationReason::Rollback => "rollback",
        PanelActivationReason::Recovery => "recovery",
    }
}
fn parse_reason(value: &str) -> Result<PanelActivationReason, sea_orm::DbErr> {
    match value {
        "update" => Ok(PanelActivationReason::Update),
        "rollback" => Ok(PanelActivationReason::Rollback),
        "recovery" => Ok(PanelActivationReason::Recovery),
        value => Err(sea_orm::DbErr::Custom(format!(
            "Unknown panel activation reason: {value}"
        ))),
    }
}

async fn upsert_release<C: ConnectionTrait>(
    connection: &C,
    release: &PanelReleaseDescriptor,
) -> Result<(), String> {
    validate_release_descriptor(release)?;
    connection
        .execute_raw(statement(
            "INSERT INTO panel_releases (
           version, core_version, core_path, core_digest, launcher_protocol, api_contract,
           schema_epoch, schema_revision, target, core_installed_at,
           web_version, web_path, web_digest, installed_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(version) DO NOTHING",
            vec![
                text(&release.version),
                text(&release.core.version),
                text(&release.core.path),
                text(&release.core_digest),
                (release.core.launcher_protocol as i64).into(),
                (release.core.api_contract as i64).into(),
                (release.core.schema_epoch as i64).into(),
                (release.core.schema_revision as i64).into(),
                text(&release.core.target),
                text(&release.core.installed_at),
                text(&release.web_version),
                text(&release.web_path),
                text(&release.web_digest),
                text(&release.installed_at),
            ],
        ))
        .await
        .map_err(|error| {
            format!(
                "Failed to persist panel release {}: {error}",
                release.version
            )
        })?;
    let persisted = release_by_version(connection, &release.version).await?;
    if !persisted
        .as_ref()
        .is_some_and(|persisted| same_release_identity(persisted, release))
    {
        return Err(format!(
            "Panel release {} already exists with different contents",
            release.version
        ));
    }
    Ok(())
}

fn same_release_identity(left: &PanelReleaseDescriptor, right: &PanelReleaseDescriptor) -> bool {
    left.version == right.version
        && left.core.version == right.core.version
        && left.core.path == right.core.path
        && left.core.launcher_protocol == right.core.launcher_protocol
        && left.core.api_contract == right.core.api_contract
        && left.core.schema_epoch == right.core.schema_epoch
        && left.core.schema_revision == right.core.schema_revision
        && left.core.target == right.core.target
        && left.core_digest == right.core_digest
        && left.web_version == right.web_version
        && left.web_path == right.web_path
        && left.web_digest == right.web_digest
}

async fn release_by_version<C: ConnectionTrait>(
    connection: &C,
    version: &str,
) -> Result<Option<PanelReleaseDescriptor>, String> {
    let row = connection
        .query_one_raw(statement(
            "SELECT version, core_version, core_path, core_digest, launcher_protocol, api_contract,
                schema_epoch, schema_revision, target, core_installed_at,
                web_version, web_path, web_digest, installed_at
         FROM panel_releases WHERE version = ?",
            vec![text(version)],
        ))
        .await
        .map_err(|error| format!("Failed to read panel release {version}: {error}"))?;
    row.map(|row| {
        Ok(PanelReleaseDescriptor {
            version: row.try_get_by_index(0)?,
            core: CoreReleaseDescriptor {
                version: row.try_get_by_index(1)?,
                path: row.try_get_by_index(2)?,
                launcher_protocol: row.try_get_by_index::<i64>(4)? as u32,
                api_contract: row.try_get_by_index::<i64>(5)? as u32,
                schema_epoch: row.try_get_by_index::<i64>(6)? as u32,
                schema_revision: row.try_get_by_index::<i64>(7)? as u32,
                target: row.try_get_by_index(8)?,
                installed_at: row.try_get_by_index(9)?,
            },
            core_digest: row.try_get_by_index(3)?,
            web_version: row.try_get_by_index(10)?,
            web_path: row.try_get_by_index(11)?,
            web_digest: row.try_get_by_index(12)?,
            installed_at: row.try_get_by_index(13)?,
        })
    })
    .transpose()
    .map_err(|error: sea_orm::DbErr| format!("Failed to decode panel release {version}: {error}"))
}

pub async fn initialize_runtime(
    system_root: impl AsRef<Path>,
    release: &PanelReleaseDescriptor,
) -> Result<PanelRuntimeState, String> {
    let state = PanelRuntimeState {
        protocol: RELEASE_PROTOCOL_VERSION,
        active: release.clone(),
        previous: None,
        pending: None,
        last_failure: None,
    };
    write_panel_runtime_state(system_root, &state).await?;
    Ok(state)
}

pub async fn read_panel_runtime_state(
    system_root: impl AsRef<Path>,
) -> Result<PanelRuntimeState, String> {
    let system_root = system_root.as_ref();
    let connection = open(system_root).await?;
    let row = connection
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT protocol, active_version, previous_version, pending_version,
                pending_transaction_id, pending_reason, pending_requested_at,
                pending_activation_snapshot_path, pending_restore_path,
                failure_transaction_id, failure_version, failure_at, failure_message
         FROM panel_runtime_state WHERE id = 1"
                .to_owned(),
        ))
        .await
        .map_err(|error| format!("Failed to read panel runtime state: {error}"))?
        .ok_or_else(|| PANEL_RUNTIME_NOT_INITIALIZED.to_string())?;
    let protocol = row
        .try_get_by_index::<i64>(0)
        .map_err(|error| error.to_string())? as u32;
    let active_version: String = row.try_get_by_index(1).map_err(|error| error.to_string())?;
    let previous_version: Option<String> =
        row.try_get_by_index(2).map_err(|error| error.to_string())?;
    let pending_version: Option<String> =
        row.try_get_by_index(3).map_err(|error| error.to_string())?;
    let pending_id: Option<String> = row.try_get_by_index(4).map_err(|error| error.to_string())?;
    let pending_reason: Option<String> =
        row.try_get_by_index(5).map_err(|error| error.to_string())?;
    let pending_at: Option<String> = row.try_get_by_index(6).map_err(|error| error.to_string())?;
    let activation_snapshot_path: Option<String> =
        row.try_get_by_index(7).map_err(|error| error.to_string())?;
    let restore_path: Option<String> =
        row.try_get_by_index(8).map_err(|error| error.to_string())?;
    let failure_id: Option<String> = row.try_get_by_index(9).map_err(|error| error.to_string())?;
    let failure_version: Option<String> = row
        .try_get_by_index(10)
        .map_err(|error| error.to_string())?;
    let failure_at: Option<String> = row
        .try_get_by_index(11)
        .map_err(|error| error.to_string())?;
    let failure_message: Option<String> = row
        .try_get_by_index(12)
        .map_err(|error| error.to_string())?;
    let active = release_by_version(&connection, &active_version)
        .await?
        .ok_or_else(|| format!("Active panel release {active_version} is missing"))?;
    let previous = match previous_version {
        Some(version) => Some(
            release_by_version(&connection, &version)
                .await?
                .ok_or_else(|| format!("Previous panel release {version} is missing"))?,
        ),
        None => None,
    };
    let pending = match (pending_version, pending_id, pending_reason, pending_at) {
        (Some(version), Some(transaction_id), Some(reason), Some(requested_at)) => {
            Some(PendingPanelActivation {
                candidate: release_by_version(&connection, &version)
                    .await?
                    .ok_or_else(|| format!("Pending panel release {version} is missing"))?,
                transaction_id,
                reason: parse_reason(&reason).map_err(|error| error.to_string())?,
                requested_at,
                activation_snapshot_path,
                restore_before_start: restore_path,
            })
        }
        (None, None, None, None)
            if activation_snapshot_path.is_none() && restore_path.is_none() =>
        {
            None
        }
        _ => return Err("Panel runtime contains incomplete pending activation state".into()),
    };
    let last_failure = match (failure_id, failure_version, failure_at, failure_message) {
        (Some(transaction_id), Some(version), Some(failed_at), Some(message)) => {
            Some(PanelActivationFailure {
                transaction_id,
                version,
                failed_at,
                message,
            })
        }
        (None, None, None, None) => None,
        _ => return Err("Panel runtime contains incomplete activation failure state".into()),
    };
    let state = PanelRuntimeState {
        protocol,
        active,
        previous,
        pending,
        last_failure,
    };
    validate_runtime_state(&state)?;
    validate_runtime_paths(system_root, &state)?;
    Ok(state)
}

fn validate_runtime_state(state: &PanelRuntimeState) -> Result<(), String> {
    if state.protocol != RELEASE_PROTOCOL_VERSION {
        return Err(format!(
            "Unsupported panel runtime protocol {}",
            state.protocol
        ));
    }
    if state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.candidate.version == state.active.version)
    {
        return Err("Pending panel release must differ from the active release".to_string());
    }
    if state
        .previous
        .as_ref()
        .is_some_and(|previous| previous.version == state.active.version)
    {
        return Err("Previous panel release must differ from the active release".to_string());
    }
    validate_release_descriptor(&state.active)?;
    if let Some(previous) = &state.previous {
        validate_release_descriptor(previous)?;
    }
    if let Some(pending) = &state.pending {
        validate_release_descriptor(&pending.candidate)?;
        if pending.transaction_id.trim().is_empty() || pending.requested_at.trim().is_empty() {
            return Err("Pending panel activation metadata is incomplete".to_string());
        }
    }
    Ok(())
}

fn validate_release_descriptor(release: &PanelReleaseDescriptor) -> Result<(), String> {
    let core_path = Path::new(&release.core.path);
    let core_root = core_path.parent();
    let panel_root = core_root.and_then(Path::parent);
    let expected_web = panel_root.map(|root| root.join("web"));
    let valid_digest = |digest: &str| {
        digest.len() == 64
            && digest
                .chars()
                .all(|character| character.is_ascii_hexdigit())
    };
    if semver::Version::parse(&release.version).is_err()
        || semver::Version::parse(&release.core.version).is_err()
        || semver::Version::parse(&release.web_version).is_err()
        || release.core.launcher_protocol != RELEASE_PROTOCOL_VERSION
        || release.core.api_contract == 0
        || release.core.schema_epoch == 0
        || release.core.schema_revision == 0
        || release.installed_at.trim().is_empty()
        || release.core.installed_at.trim().is_empty()
        || release.core.target.trim().is_empty()
        || !core_path.is_absolute()
        || core_path.file_name().and_then(|name| name.to_str()) != Some("niupanel")
        || core_root
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            != Some("core")
        || expected_web.as_deref() != Some(Path::new(&release.web_path))
        || !valid_digest(&release.core_digest)
        || !valid_digest(&release.web_digest)
    {
        return Err(format!(
            "Panel release {} has an invalid descriptor",
            release.version
        ));
    }
    Ok(())
}

fn valid_transaction_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate_runtime_paths(system_root: &Path, state: &PanelRuntimeState) -> Result<(), String> {
    let release_root = system_root.join("releases/panel");
    let snapshot_root = system_root.join("snapshots");
    let validate_release_path = |release: &PanelReleaseDescriptor| {
        let expected = release_root.join(&release.version);
        let actual = Path::new(&release.core.path)
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| format!("Panel {} release root is invalid", release.version))?;
        if actual != expected {
            return Err(format!(
                "Panel {} is outside the managed release directory",
                release.version
            ));
        }
        Ok(())
    };
    validate_release_path(&state.active)?;
    if let Some(previous) = &state.previous {
        validate_release_path(previous)?;
    }
    if let Some(pending) = &state.pending {
        validate_release_path(&pending.candidate)?;
        if !valid_transaction_id(&pending.transaction_id) {
            return Err("Pending panel transaction id is invalid".to_string());
        }
        if let Some(snapshot) = pending.activation_snapshot_path.as_deref() {
            if Path::new(snapshot) != snapshot_root.join(&pending.transaction_id) {
                return Err("Pending activation snapshot path is invalid".to_string());
            }
        }
        if let Some(snapshot) = pending.restore_before_start.as_deref() {
            if Path::new(snapshot).parent() != Some(snapshot_root.as_path()) {
                return Err("Pending restore snapshot path is invalid".to_string());
            }
        }
    }
    if let Some(failure) = &state.last_failure {
        if !valid_transaction_id(&failure.transaction_id) {
            return Err("Panel activation failure transaction id is invalid".to_string());
        }
    }
    Ok(())
}

async fn persist_runtime_state<C: ConnectionTrait>(
    connection: &C,
    state: &PanelRuntimeState,
) -> Result<(), String> {
    upsert_release(connection, &state.active).await?;
    if let Some(previous) = &state.previous {
        upsert_release(connection, previous).await?;
    }
    if let Some(pending) = &state.pending {
        upsert_release(connection, &pending.candidate).await?;
    }
    let pending = state.pending.as_ref();
    let failure = state.last_failure.as_ref();
    connection.execute_raw(statement(
        "INSERT INTO panel_runtime_state (
           id, protocol, active_version, previous_version, pending_version,
           pending_transaction_id, pending_reason, pending_requested_at,
           pending_activation_snapshot_path, pending_restore_path,
           failure_transaction_id, failure_version, failure_at, failure_message
         ) VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET protocol=excluded.protocol,
           active_version=excluded.active_version, previous_version=excluded.previous_version,
           pending_version=excluded.pending_version,
           pending_transaction_id=excluded.pending_transaction_id,
           pending_reason=excluded.pending_reason, pending_requested_at=excluded.pending_requested_at,
           pending_activation_snapshot_path=excluded.pending_activation_snapshot_path,
           pending_restore_path=excluded.pending_restore_path,
           failure_transaction_id=excluded.failure_transaction_id,
           failure_version=excluded.failure_version, failure_at=excluded.failure_at,
           failure_message=excluded.failure_message",
        vec![
            (state.protocol as i64).into(), text(&state.active.version),
            optional_text(state.previous.as_ref().map(|value| value.version.as_str())),
            optional_text(pending.map(|value| value.candidate.version.as_str())),
            optional_text(pending.map(|value| value.transaction_id.as_str())),
            optional_text(pending.map(|value| reason_name(&value.reason))),
            optional_text(pending.map(|value| value.requested_at.as_str())),
            optional_text(pending.and_then(|value| value.activation_snapshot_path.as_deref())),
            optional_text(pending.and_then(|value| value.restore_before_start.as_deref())),
            optional_text(failure.map(|value| value.transaction_id.as_str())),
            optional_text(failure.map(|value| value.version.as_str())),
            optional_text(failure.map(|value| value.failed_at.as_str())),
            optional_text(failure.map(|value| value.message.as_str())),
        ],
    )).await.map_err(|error| format!("Failed to persist panel runtime state: {error}"))?;
    Ok(())
}

async fn persist_activation_transaction<C: ConnectionTrait>(
    connection: &C,
    item: &PanelActivationTransaction,
) -> Result<(), String> {
    connection
        .execute_raw(statement(
            "INSERT INTO panel_transactions (
           transaction_id, from_version, to_version, reason, snapshot_path,
           requested_at, completed_at, successful, error
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(transaction_id) DO UPDATE SET
           from_version=excluded.from_version, to_version=excluded.to_version,
           reason=excluded.reason, snapshot_path=excluded.snapshot_path,
           requested_at=excluded.requested_at, completed_at=excluded.completed_at,
           successful=excluded.successful, error=excluded.error",
            vec![
                text(&item.transaction_id),
                text(&item.from_version),
                text(&item.to_version),
                text(reason_name(&item.reason)),
                text(&item.snapshot_path),
                text(&item.requested_at),
                text(&item.completed_at),
                item.successful.into(),
                optional_text(item.error.as_deref()),
            ],
        ))
        .await
        .map_err(|error| format!("Failed to persist panel transaction: {error}"))?;
    Ok(())
}

pub async fn write_panel_runtime_state(
    system_root: impl AsRef<Path>,
    state: &PanelRuntimeState,
) -> Result<(), String> {
    let system_root = system_root.as_ref();
    validate_runtime_state(state)?;
    validate_runtime_paths(system_root, state)?;
    let connection = open(system_root).await?;
    let transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to begin panel runtime transaction: {error}"))?;
    persist_runtime_state(&transaction, state).await?;
    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit panel runtime state: {error}"))
}

/// Queues one activation with an in-process compare-and-set guard. Core update
/// and rollback requests share this boundary, so concurrent requests cannot
/// silently replace each other's pending transaction.
pub async fn enqueue_panel_activation(
    system_root: impl AsRef<Path>,
    expected_active_version: &str,
    pending: PendingPanelActivation,
) -> Result<(), String> {
    let system_root = system_root.as_ref();
    let _guard = ACTIVATION_QUEUE_LOCK.lock().await;
    let mut state = read_panel_runtime_state(system_root).await?;
    if state.active.version != expected_active_version {
        return Err(format!(
            "Active Panel changed from {expected_active_version} to {} while queuing activation",
            state.active.version
        ));
    }
    if let Some(existing) = &state.pending {
        return Err(format!(
            "Panel {} is already waiting for activation",
            existing.candidate.version
        ));
    }
    if pending.candidate.version == state.active.version {
        return Err(format!(
            "Panel {} is already active",
            pending.candidate.version
        ));
    }
    state.pending = Some(pending);
    write_panel_runtime_state(system_root, &state).await
}

/// Commits the final runtime pointer and its activation journal entry in the
/// same SQLite transaction. A release is never visible as active without the
/// snapshot metadata required to roll it back.
pub async fn commit_panel_activation(
    system_root: impl AsRef<Path>,
    state: &PanelRuntimeState,
    item: &PanelActivationTransaction,
) -> Result<(), String> {
    let system_root = system_root.as_ref();
    validate_runtime_state(state)?;
    validate_runtime_paths(system_root, state)?;
    validate_activation_commit(system_root, state, item)?;
    let connection = open(system_root).await?;
    let transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to begin panel activation transaction: {error}"))?;
    persist_runtime_state(&transaction, state).await?;
    persist_activation_transaction(&transaction, item).await?;
    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit panel activation transaction: {error}"))
}

fn validate_activation_commit(
    system_root: &Path,
    state: &PanelRuntimeState,
    item: &PanelActivationTransaction,
) -> Result<(), String> {
    if !valid_transaction_id(&item.transaction_id)
        || item.from_version == item.to_version
        || item.requested_at.trim().is_empty()
        || item.completed_at.trim().is_empty()
        || Path::new(&item.snapshot_path)
            != system_root.join("snapshots").join(&item.transaction_id)
        || !Path::new(&item.snapshot_path).is_dir()
        || state.pending.is_some()
    {
        return Err("Panel activation transaction metadata is invalid".to_string());
    }
    if item.successful {
        if state.active.version != item.to_version {
            return Err("Successful activation does not match the active Panel".to_string());
        }
    } else if state.active.version != item.from_version
        || state
            .last_failure
            .as_ref()
            .map(|failure| failure.transaction_id.as_str())
            != Some(item.transaction_id.as_str())
    {
        return Err("Failed activation does not match the runtime failure state".to_string());
    }
    Ok(())
}

pub async fn list_panel_releases(
    system_root: impl AsRef<Path>,
) -> Result<Vec<PanelReleaseDescriptor>, String> {
    let connection = open(system_root).await?;
    let rows = connection
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT version FROM panel_releases ORDER BY installed_at DESC".to_owned(),
        ))
        .await
        .map_err(|error| format!("Failed to list panel releases: {error}"))?;
    let mut releases = Vec::with_capacity(rows.len());
    for row in rows {
        let version: String = row.try_get_by_index(0).map_err(|error| error.to_string())?;
        if let Some(release) = release_by_version(&connection, &version).await? {
            releases.push(release);
        }
    }
    Ok(releases)
}

pub async fn list_panel_transactions(
    system_root: impl AsRef<Path>,
) -> Result<Vec<PanelActivationTransaction>, String> {
    let system_root = system_root.as_ref();
    let connection = open(system_root).await?;
    let rows = connection
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT transaction_id, from_version, to_version, reason, snapshot_path,
                requested_at, completed_at, successful, error
         FROM panel_transactions ORDER BY completed_at DESC"
                .to_owned(),
        ))
        .await
        .map_err(|error| format!("Failed to list panel transactions: {error}"))?;
    let transactions = rows
        .into_iter()
        .map(|row| {
            let reason: String = row.try_get_by_index(3)?;
            Ok(PanelActivationTransaction {
                transaction_id: row.try_get_by_index(0)?,
                from_version: row.try_get_by_index(1)?,
                to_version: row.try_get_by_index(2)?,
                reason: parse_reason(&reason)?,
                snapshot_path: row.try_get_by_index(4)?,
                requested_at: row.try_get_by_index(5)?,
                completed_at: row.try_get_by_index(6)?,
                successful: row.try_get_by_index(7)?,
                error: row.try_get_by_index(8)?,
            })
        })
        .collect::<Result<Vec<_>, sea_orm::DbErr>>()
        .map_err(|error| format!("Failed to decode panel transactions: {error}"))?;
    for item in &transactions {
        if !valid_transaction_id(&item.transaction_id)
            || Path::new(&item.snapshot_path)
                != system_root.join("snapshots").join(&item.transaction_id)
        {
            return Err(format!(
                "Panel transaction {} contains invalid snapshot metadata",
                item.transaction_id
            ));
        }
    }
    Ok(transactions)
}

pub async fn latest_snapshot_for_release(
    system_root: impl AsRef<Path>,
    version: &str,
) -> Result<Option<PathBuf>, String> {
    Ok(list_panel_transactions(system_root)
        .await?
        .into_iter()
        .filter(|transaction| transaction.successful && transaction.from_version == version)
        .filter_map(|transaction| {
            let path = PathBuf::from(transaction.snapshot_path);
            path.is_dir().then_some((transaction.completed_at, path))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, path)| path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(root: &Path, version: &str) -> PanelReleaseDescriptor {
        let release_root = root.join("releases/panel").join(version);
        PanelReleaseDescriptor {
            version: version.into(),
            core: CoreReleaseDescriptor {
                version: version.into(),
                path: release_root
                    .join("core/niupanel")
                    .to_string_lossy()
                    .into_owned(),
                launcher_protocol: 1,
                api_contract: 1,
                schema_epoch: 1,
                schema_revision: 30,
                target: "linux-test".into(),
                installed_at: "2026-08-01T00:00:00Z".into(),
            },
            core_digest: "d".repeat(64),
            web_version: "2.0.1".into(),
            web_path: release_root.join("web").to_string_lossy().into_owned(),
            web_digest: "a".repeat(64),
            installed_at: "2026-08-01T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn runtime_state_round_trips_without_json_files() {
        let temp = tempfile::tempdir().unwrap();
        initialize_runtime(temp.path(), &release(temp.path(), "0.8.1"))
            .await
            .unwrap();
        let restored = read_panel_runtime_state(temp.path()).await.unwrap();
        assert_eq!(restored.active.version, "0.8.1");
        assert!(temp.path().join(PANEL_RUNTIME_DATABASE_FILE).is_file());
        assert!(!temp.path().join("core/state.json").exists());
    }

    #[tokio::test]
    async fn activation_commit_is_atomic_with_its_journal() {
        let temp = tempfile::tempdir().unwrap();
        let active = release(temp.path(), "0.8.1");
        initialize_runtime(temp.path(), &active).await.unwrap();
        let candidate = release(temp.path(), "0.8.2");
        let state = PanelRuntimeState {
            protocol: RELEASE_PROTOCOL_VERSION,
            active: candidate.clone(),
            previous: Some(active.clone()),
            pending: None,
            last_failure: None,
        };
        let item = PanelActivationTransaction {
            transaction_id: "tx-atomic".into(),
            from_version: active.version,
            to_version: candidate.version,
            reason: PanelActivationReason::Update,
            snapshot_path: temp
                .path()
                .join("snapshots/tx-atomic")
                .to_string_lossy()
                .into_owned(),
            requested_at: "2026-08-01T00:00:00Z".into(),
            completed_at: "2026-08-01T00:01:00Z".into(),
            successful: true,
            error: None,
        };
        fs::create_dir_all(&item.snapshot_path).unwrap();

        commit_panel_activation(temp.path(), &state, &item)
            .await
            .unwrap();

        let restored = read_panel_runtime_state(temp.path()).await.unwrap();
        let transactions = list_panel_transactions(temp.path()).await.unwrap();
        assert_eq!(restored.active.version, "0.8.2");
        assert_eq!(restored.previous.unwrap().version, "0.8.1");
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].transaction_id, "tx-atomic");
    }

    #[tokio::test]
    async fn activation_queue_uses_compare_and_set_semantics() {
        let temp = tempfile::tempdir().unwrap();
        initialize_runtime(temp.path(), &release(temp.path(), "0.8.1"))
            .await
            .unwrap();
        let pending = PendingPanelActivation {
            transaction_id: "tx-queue".into(),
            candidate: release(temp.path(), "0.8.2"),
            reason: PanelActivationReason::Update,
            requested_at: "2026-08-01T00:00:00Z".into(),
            activation_snapshot_path: None,
            restore_before_start: None,
        };

        let stale = enqueue_panel_activation(temp.path(), "0.8.0", pending.clone())
            .await
            .unwrap_err();
        assert!(stale.contains("changed from 0.8.0 to 0.8.1"));
        enqueue_panel_activation(temp.path(), "0.8.1", pending)
            .await
            .unwrap();

        let duplicate = enqueue_panel_activation(
            temp.path(),
            "0.8.1",
            PendingPanelActivation {
                transaction_id: "tx-duplicate".into(),
                candidate: release(temp.path(), "0.8.3"),
                reason: PanelActivationReason::Rollback,
                requested_at: "2026-08-01T00:01:00Z".into(),
                activation_snapshot_path: None,
                restore_before_start: None,
            },
        )
        .await
        .unwrap_err();
        assert!(duplicate.contains("already waiting for activation"));

        let state = read_panel_runtime_state(temp.path()).await.unwrap();
        assert_eq!(state.pending.unwrap().candidate.version, "0.8.2");
    }

    #[tokio::test]
    async fn release_versions_are_immutable() {
        let temp = tempfile::tempdir().unwrap();
        let original = release(temp.path(), "0.8.1");
        initialize_runtime(temp.path(), &original).await.unwrap();
        let mut changed = original.clone();
        changed.web_digest = "b".repeat(64);
        let state = PanelRuntimeState {
            protocol: RELEASE_PROTOCOL_VERSION,
            active: changed,
            previous: None,
            pending: None,
            last_failure: None,
        };

        let error = write_panel_runtime_state(temp.path(), &state)
            .await
            .unwrap_err();
        assert!(error.contains("already exists with different contents"));
    }

    #[tokio::test]
    async fn release_versions_cannot_escape_the_managed_directory() {
        let temp = tempfile::tempdir().unwrap();
        let error = initialize_runtime(temp.path(), &release(temp.path(), "../outside"))
            .await
            .unwrap_err();

        assert!(error.contains("invalid descriptor"));
        assert!(!temp.path().join("outside").exists());
    }

    #[tokio::test]
    async fn registration_timestamps_do_not_change_release_identity() {
        let temp = tempfile::tempdir().unwrap();
        let original = release(temp.path(), "0.8.1");
        initialize_runtime(temp.path(), &original).await.unwrap();
        let mut repeated = original.clone();
        repeated.installed_at = "2026-08-02T00:00:00Z".into();
        repeated.core.installed_at = "2026-08-02T00:00:00Z".into();
        write_panel_runtime_state(
            temp.path(),
            &PanelRuntimeState {
                protocol: RELEASE_PROTOCOL_VERSION,
                active: repeated,
                previous: None,
                pending: None,
                last_failure: None,
            },
        )
        .await
        .unwrap();

        let restored = read_panel_runtime_state(temp.path()).await.unwrap();
        assert_eq!(restored.active.installed_at, original.installed_at);
        assert_eq!(
            restored.active.core.installed_at,
            original.core.installed_at
        );
    }

    #[tokio::test]
    async fn unsupported_runtime_database_schema_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = runtime_database_path(temp.path());
        let connection = Database::connect(format!("sqlite://{}?mode=rwc", path.display()))
            .await
            .unwrap();
        connection
            .execute_unprepared("PRAGMA user_version = 999")
            .await
            .unwrap();
        connection.close().await.unwrap();

        let error = read_panel_runtime_state(temp.path()).await.unwrap_err();
        assert!(error.contains("Unsupported panel runtime database schema 999"));
    }

    #[tokio::test]
    async fn unversioned_runtime_tables_are_not_silently_adopted() {
        let temp = tempfile::tempdir().unwrap();
        let path = runtime_database_path(temp.path());
        let connection = Database::connect(format!("sqlite://{}?mode=rwc", path.display()))
            .await
            .unwrap();
        connection
            .execute_unprepared("CREATE TABLE panel_legacy_state (value TEXT)")
            .await
            .unwrap();
        connection.close().await.unwrap();

        let error = read_panel_runtime_state(temp.path()).await.unwrap_err();
        assert!(error.contains("unversioned tables"));
    }

    #[test]
    fn directory_digest_is_order_independent_and_content_sensitive() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        fs::create_dir_all(first.path().join("nested")).unwrap();
        fs::write(first.path().join("b"), b"second").unwrap();
        fs::write(first.path().join("nested/a"), b"first").unwrap();
        fs::create_dir_all(second.path().join("nested")).unwrap();
        fs::write(second.path().join("nested/a"), b"first").unwrap();
        fs::write(second.path().join("b"), b"second").unwrap();

        let expected = directory_digest(first.path()).unwrap();
        assert_eq!(expected, directory_digest(second.path()).unwrap());
        fs::write(second.path().join("b"), b"changed").unwrap();
        assert_ne!(expected, directory_digest(second.path()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn directory_digest_rejects_symbolic_links() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("target"), b"content").unwrap();
        symlink("target", temp.path().join("link")).unwrap();
        let error = directory_digest(temp.path()).unwrap_err();
        assert!(error.contains("unsupported entry"));
    }
}
