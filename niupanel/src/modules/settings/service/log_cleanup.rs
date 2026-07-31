use super::*;

#[derive(Debug)]
enum CandidatePathState {
    Safe,
    Planned(PathBuf),
    Invalid,
}

impl CandidatePathState {
    fn can_delete_record(&self, failed_paths: &HashSet<PathBuf>) -> bool {
        match self {
            Self::Safe => true,
            Self::Planned(path) => !failed_paths.contains(path),
            Self::Invalid => false,
        }
    }
}

pub(super) struct LogCleanupCandidates {
    pub(super) task_runs: Vec<task_runs::Model>,
    pub(super) system_jobs: Vec<system_jobs::Model>,
    pub(super) audit_logs: u64,
    pub(super) protected_paths: Vec<String>,
}

struct PreparedLogCleanup {
    files: BTreeMap<PathBuf, u64>,
    task_run_paths: HashMap<i32, CandidatePathState>,
    system_job_paths: HashMap<i32, CandidatePathState>,
    protected_files: u64,
    warnings: Vec<String>,
    roots: Vec<PathBuf>,
}

fn terminal_task_statuses() -> Vec<TaskStatus> {
    vec![
        TaskStatus::Finished,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
        TaskStatus::Stopped,
    ]
}

pub(super) fn validate_log_retention_days(days: i64) -> Result<()> {
    if !(defaults::LOG_RETENTION_MIN_DAYS..=defaults::LOG_RETENTION_MAX_DAYS).contains(&days) {
        return Err(AppError::ValidationError(format!(
            "日志保留天数必须在 {} 到 {} 之间",
            defaults::LOG_RETENTION_MIN_DAYS,
            defaults::LOG_RETENTION_MAX_DAYS
        )));
    }
    Ok(())
}

fn push_cleanup_warning(warnings: &mut Vec<String>, message: String) {
    const MAX_WARNINGS: usize = 20;
    if warnings.len() < MAX_WARNINGS {
        warnings.push(message);
    } else if warnings.len() == MAX_WARNINGS {
        warnings.push("还有更多清理警告未显示，请检查服务端日志".to_string());
    }
}

pub(super) async fn load_log_cleanup_candidates(
    db: &DatabaseConnection,
    threshold: chrono::DateTime<chrono::FixedOffset>,
) -> Result<LogCleanupCandidates> {
    let terminal = terminal_task_statuses();
    let latest_task_run_ids = task_runs::Entity::find()
        .select_only()
        .column_as(task_runs::Column::Id.max(), "latest_id")
        .group_by(task_runs::Column::TaskId)
        .into_tuple::<Option<i32>>()
        .all(db)
        .await?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let latest_task_run_id_set = latest_task_run_ids.iter().copied().collect::<HashSet<_>>();
    let protected_task_condition = Condition::any()
        .add(task_runs::Column::EndedAt.is_null())
        .add(task_runs::Column::EndedAt.gte(threshold))
        .add(task_runs::Column::Status.is_not_in(terminal.clone()));
    let protected_job_condition = Condition::any()
        .add(system_jobs::Column::EndedAt.is_null())
        .add(system_jobs::Column::EndedAt.gte(threshold))
        .add(system_jobs::Column::Status.is_not_in(terminal.clone()));

    let (
        task_run_candidates,
        system_job_candidates,
        protected_task_runs,
        protected_system_jobs,
        latest_task_runs,
        audit_count,
    ) = tokio::try_join!(
        task_runs::Entity::find()
            .filter(task_runs::Column::EndedAt.lt(threshold))
            .filter(task_runs::Column::Status.is_in(terminal.clone()))
            .all(db),
        system_jobs::Entity::find()
            .filter(system_jobs::Column::EndedAt.lt(threshold))
            .filter(system_jobs::Column::Status.is_in(terminal))
            .all(db),
        task_runs::Entity::find()
            .filter(protected_task_condition)
            .all(db),
        system_jobs::Entity::find()
            .filter(protected_job_condition)
            .all(db),
        task_runs::Entity::find()
            .filter(task_runs::Column::Id.is_in(latest_task_run_ids))
            .all(db),
        audit_logs::Entity::find()
            .filter(audit_logs::Column::CreatedAt.lt(threshold))
            .count(db),
    )?;

    let task_run_candidates = task_run_candidates
        .into_iter()
        .filter(|run| !latest_task_run_id_set.contains(&run.id))
        .collect();
    let protected_paths = protected_task_runs
        .into_iter()
        .filter_map(|run| run.log_path)
        .chain(latest_task_runs.into_iter().filter_map(|run| run.log_path))
        .chain(
            protected_system_jobs
                .into_iter()
                .filter_map(|job| job.log_path),
        )
        .collect();

    Ok(LogCleanupCandidates {
        task_runs: task_run_candidates,
        system_jobs: system_job_candidates,
        audit_logs: audit_count,
        protected_paths,
    })
}

fn canonical_root(path: &Path, warnings: &mut Vec<String>) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    match path.canonicalize() {
        Ok(path) => Some(path),
        Err(err) => {
            push_cleanup_warning(
                warnings,
                format!("无法解析日志目录 {}: {}", path.display(), err),
            );
            None
        }
    }
}

fn canonical_log_file(
    path: &Path,
    root: Option<&PathBuf>,
    warnings: &mut Vec<String>,
) -> Option<Option<(PathBuf, u64)>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Some(None),
        Err(err) => {
            push_cleanup_warning(
                warnings,
                format!("无法读取日志文件 {}: {}", path.display(), err),
            );
            return None;
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        push_cleanup_warning(warnings, format!("已跳过非普通日志文件 {}", path.display()));
        return None;
    }
    let Some(root) = root else {
        push_cleanup_warning(
            warnings,
            format!("日志目录不存在，无法验证 {}", path.display()),
        );
        return None;
    };
    let canonical = match path.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            push_cleanup_warning(
                warnings,
                format!("无法解析日志文件 {}: {}", path.display(), err),
            );
            return None;
        }
    };
    if !canonical.starts_with(root) {
        push_cleanup_warning(
            warnings,
            format!("已跳过日志目录外的路径 {}", path.display()),
        );
        return None;
    }
    Some(Some((canonical, metadata.len())))
}

fn register_candidate_path(
    path: Option<&str>,
    root: Option<&PathBuf>,
    protected_paths: &HashSet<PathBuf>,
    files: &mut BTreeMap<PathBuf, u64>,
    forced_paths: &mut HashSet<PathBuf>,
    warnings: &mut Vec<String>,
) -> CandidatePathState {
    let Some(path) = path.filter(|path| !path.trim().is_empty()) else {
        return CandidatePathState::Safe;
    };
    match canonical_log_file(Path::new(path), root, warnings) {
        Some(None) => CandidatePathState::Safe,
        Some(Some((canonical, size))) => {
            if protected_paths.contains(&canonical) {
                return CandidatePathState::Safe;
            }
            files.entry(canonical.clone()).or_insert(size);
            forced_paths.insert(canonical.clone());
            CandidatePathState::Planned(canonical)
        }
        None => CandidatePathState::Invalid,
    }
}

pub(super) fn scan_expired_log_files(
    root: Option<&PathBuf>,
    threshold: chrono::DateTime<Utc>,
    preserve_newest_root_log: bool,
    protected_paths: &HashSet<PathBuf>,
    forced_paths: &HashSet<PathBuf>,
    files: &mut BTreeMap<PathBuf, u64>,
    warnings: &mut Vec<String>,
) {
    let Some(root) = root else {
        return;
    };
    let threshold: std::time::SystemTime = threshold.into();
    let mut expired = Vec::new();
    let mut newest_root_log: Option<(PathBuf, std::time::SystemTime)> = None;

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                push_cleanup_warning(warnings, format!("遍历日志目录失败: {}", err));
                continue;
            }
        };
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("log")
        {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                push_cleanup_warning(
                    warnings,
                    format!("无法读取日志文件 {}: {}", entry.path().display(), err),
                );
                continue;
            }
        };
        let modified = match metadata.modified() {
            Ok(modified) => modified,
            Err(err) => {
                push_cleanup_warning(
                    warnings,
                    format!("无法读取日志时间 {}: {}", entry.path().display(), err),
                );
                continue;
            }
        };
        let canonical = match entry.path().canonicalize() {
            Ok(path) if path.starts_with(root) => path,
            Ok(_) => {
                push_cleanup_warning(
                    warnings,
                    format!("已跳过日志目录外的路径 {}", entry.path().display()),
                );
                continue;
            }
            Err(err) => {
                push_cleanup_warning(
                    warnings,
                    format!("无法解析日志文件 {}: {}", entry.path().display(), err),
                );
                continue;
            }
        };

        if preserve_newest_root_log && entry.depth() == 1 {
            let replace = newest_root_log
                .as_ref()
                .is_none_or(|(_, newest)| modified > *newest);
            if replace {
                newest_root_log = Some((canonical.clone(), modified));
            }
        }
        if modified < threshold && !protected_paths.contains(&canonical) {
            expired.push((canonical, metadata.len()));
        }
    }

    let preserved = newest_root_log.map(|(path, _)| path);
    for (path, size) in expired {
        if preserved.as_ref() == Some(&path) && !forced_paths.contains(&path) {
            continue;
        }
        files.entry(path).or_insert(size);
    }
}

fn prepare_log_cleanup_files(
    candidates: &LogCleanupCandidates,
    logs_dir: PathBuf,
    jobs_dir: PathBuf,
    threshold: chrono::DateTime<Utc>,
) -> PreparedLogCleanup {
    let mut warnings = Vec::new();
    let logs_root = canonical_root(&logs_dir, &mut warnings);
    let jobs_root = canonical_root(&jobs_dir, &mut warnings);
    let roots = [logs_root.clone(), jobs_root.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let protected_paths = candidates
        .protected_paths
        .iter()
        .filter_map(|path| {
            let canonical = Path::new(path).canonicalize().ok()?;
            roots
                .iter()
                .any(|root| canonical.starts_with(root))
                .then_some(canonical)
        })
        .collect::<HashSet<_>>();

    let mut files = BTreeMap::new();
    let mut forced_paths = HashSet::new();
    let task_run_paths = candidates
        .task_runs
        .iter()
        .map(|run| {
            (
                run.id,
                register_candidate_path(
                    run.log_path.as_deref(),
                    logs_root.as_ref(),
                    &protected_paths,
                    &mut files,
                    &mut forced_paths,
                    &mut warnings,
                ),
            )
        })
        .collect();
    let system_job_paths = candidates
        .system_jobs
        .iter()
        .map(|job| {
            (
                job.id,
                register_candidate_path(
                    job.log_path.as_deref(),
                    jobs_root.as_ref(),
                    &protected_paths,
                    &mut files,
                    &mut forced_paths,
                    &mut warnings,
                ),
            )
        })
        .collect();

    scan_expired_log_files(
        logs_root.as_ref(),
        threshold,
        true,
        &protected_paths,
        &forced_paths,
        &mut files,
        &mut warnings,
    );
    scan_expired_log_files(
        jobs_root.as_ref(),
        threshold,
        false,
        &protected_paths,
        &forced_paths,
        &mut files,
        &mut warnings,
    );

    PreparedLogCleanup {
        files,
        task_run_paths,
        system_job_paths,
        protected_files: protected_paths.len() as u64,
        warnings,
        roots,
    }
}

fn remove_empty_log_directories(roots: &[PathBuf], warnings: &mut Vec<String>) -> u64 {
    let mut removed = 0;
    for root in roots {
        for entry in WalkDir::new(root)
            .min_depth(1)
            .contents_first(true)
            .follow_links(false)
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    push_cleanup_warning(warnings, format!("遍历空日志目录失败: {}", err));
                    continue;
                }
            };
            if !entry.file_type().is_dir() {
                continue;
            }
            match std::fs::remove_dir(entry.path()) {
                Ok(()) => removed += 1,
                Err(err)
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::NotFound
                    ) => {}
                Err(err) => push_cleanup_warning(
                    warnings,
                    format!("无法删除空目录 {}: {}", entry.path().display(), err),
                ),
            }
        }
    }
    removed
}

async fn run_log_cleanup(
    db: &DatabaseConnection,
    days: i64,
    dry_run: bool,
) -> Result<LogCleanupReport> {
    validate_log_retention_days(days)?;
    let _maintenance_guard = acquire_maintenance_lock()?;
    let threshold = Utc::now() - chrono::Duration::days(days);
    let threshold_db: chrono::DateTime<chrono::FixedOffset> = threshold.into();
    let candidates = load_log_cleanup_candidates(db, threshold_db).await?;
    let audit_log_candidates = candidates.audit_logs;
    let config = Config::global();
    let logs_dir = config.logs_dir.clone();
    let jobs_dir = config.jobs_dir.clone();
    let mut prepared = tokio::task::spawn_blocking(move || {
        prepare_log_cleanup_files(&candidates, logs_dir, jobs_dir, threshold)
    })
    .await
    .map_err(|e| AppError::Generic(format!("日志清理任务失败: {}", e)))?;

    let preview_task_runs = prepared
        .task_run_paths
        .values()
        .filter(|state| !matches!(state, CandidatePathState::Invalid))
        .count() as u64;
    let preview_system_jobs = prepared
        .system_job_paths
        .values()
        .filter(|state| !matches!(state, CandidatePathState::Invalid))
        .count() as u64;
    let preview_bytes = prepared.files.values().copied().sum();
    if dry_run {
        return Ok(LogCleanupReport {
            dry_run: true,
            cutoff_at: threshold.to_rfc3339(),
            files: prepared.files.len() as u64,
            task_runs: preview_task_runs,
            system_jobs: preview_system_jobs,
            audit_logs: audit_log_candidates,
            bytes: preview_bytes,
            empty_directories: 0,
            protected_files: prepared.protected_files,
            warnings: prepared.warnings,
        });
    }

    let files = std::mem::take(&mut prepared.files);
    let (deleted_files, deleted_bytes, failed_paths, delete_warnings) =
        tokio::task::spawn_blocking(move || {
            let mut deleted_files = 0;
            let mut deleted_bytes = 0;
            let mut failed_paths = HashSet::new();
            let mut warnings = Vec::new();
            for (path, size) in files {
                match std::fs::remove_file(&path) {
                    Ok(()) => {
                        deleted_files += 1;
                        deleted_bytes += size;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => {
                        failed_paths.insert(path.clone());
                        push_cleanup_warning(
                            &mut warnings,
                            format!("删除日志文件 {} 失败: {}", path.display(), err),
                        );
                    }
                }
            }
            (deleted_files, deleted_bytes, failed_paths, warnings)
        })
        .await
        .map_err(|e| AppError::Generic(format!("删除日志文件失败: {}", e)))?;
    for warning in delete_warnings {
        push_cleanup_warning(&mut prepared.warnings, warning);
    }

    let task_run_ids = prepared
        .task_run_paths
        .iter()
        .filter(|(_, state)| state.can_delete_record(&failed_paths))
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let system_job_ids = prepared
        .system_job_paths
        .iter()
        .filter(|(_, state)| state.can_delete_record(&failed_paths))
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();

    let txn = db.begin().await?;
    let deleted_task_runs = if task_run_ids.is_empty() {
        0
    } else {
        task_runs::Entity::delete_many()
            .filter(task_runs::Column::Id.is_in(task_run_ids))
            .exec(&txn)
            .await?
            .rows_affected
    };
    let deleted_system_jobs = if system_job_ids.is_empty() {
        0
    } else {
        system_jobs::Entity::delete_many()
            .filter(system_jobs::Column::Id.is_in(system_job_ids))
            .exec(&txn)
            .await?
            .rows_affected
    };
    let deleted_audit_logs = audit_logs::Entity::delete_many()
        .filter(audit_logs::Column::CreatedAt.lt(threshold_db))
        .exec(&txn)
        .await?
        .rows_affected;
    txn.commit().await?;

    let protected_files = prepared.protected_files;
    let roots = prepared.roots;
    let mut directory_warnings = prepared.warnings;
    let empty_directories = tokio::task::spawn_blocking(move || {
        let count = remove_empty_log_directories(&roots, &mut directory_warnings);
        (count, directory_warnings)
    })
    .await
    .map_err(|e| AppError::Generic(format!("清理空日志目录失败: {}", e)))?;

    let report = LogCleanupReport {
        dry_run: false,
        cutoff_at: threshold.to_rfc3339(),
        files: deleted_files,
        task_runs: deleted_task_runs,
        system_jobs: deleted_system_jobs,
        audit_logs: deleted_audit_logs,
        bytes: deleted_bytes,
        empty_directories: empty_directories.0,
        protected_files,
        warnings: empty_directories.1,
    };
    info!(
        "日志清理完成: 文件 {}, 任务运行记录 {}, 系统任务 {}, 审计记录 {}, 释放 {} 字节",
        report.files, report.task_runs, report.system_jobs, report.audit_logs, report.bytes
    );
    Ok(report)
}

pub async fn preview_log_cleanup(db: &DatabaseConnection, days: i64) -> Result<LogCleanupReport> {
    run_log_cleanup(db, days, true).await
}

pub async fn cleanup_logs(db: &DatabaseConnection, days: i64) -> Result<LogCleanupReport> {
    run_log_cleanup(db, days, false).await
}

pub async fn start_log_cleanup_scheduler(db: DatabaseConnection) {
    tokio::spawn(async move {
        let period = std::time::Duration::from_secs(24 * 3600);
        let first_run = tokio::time::Instant::now() + std::time::Duration::from_secs(5 * 60);
        let mut interval = tokio::time::interval_at(first_run, period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let days_str = SettingsManager::get(&db, SYSTEM_LOG_RETENTION)
                .await
                .unwrap_or_default();
            let days = days_str
                .parse::<i64>()
                .unwrap_or(defaults::LOG_RETENTION_DAYS);

            if let Err(e) = cleanup_logs(&db, days).await {
                error!("定时清理日志失败: {}", e);
            }
        }
    });
}
