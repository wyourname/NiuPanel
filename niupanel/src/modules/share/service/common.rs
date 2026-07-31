use crate::modules::share::models::{
    FileAssociations, FileEntry, NiuPackage, ShareVariable, StationConfigPayload, TaskData,
    TaskExportSpec, TaskMeta,
};
use niupanel_common::error::{AppError, Result};
use niupanel_common::filesystem::resolve_path;
use niupanel_common::info;
use niupanel_core::settings::SettingsManager;
use niupanel_entity::{tasks, variables};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, BufReader};

const MAX_DECOMPRESSED_PACKAGE_SIZE: u64 = 256 * 1024 * 1024;
const MAX_PACKAGE_TASKS: usize = 100;
const MAX_FILES_PER_TASK: usize = 1_000;
const MAX_PACKAGE_FILE_SIZE: usize = 10 * 1024 * 1024;

pub(super) async fn read_package(path: &Path) -> Result<NiuPackage> {
    let file = File::open(path).await.map_err(AppError::Io)?;
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(file));
    let mut limited = decoder.take(MAX_DECOMPRESSED_PACKAGE_SIZE + 1);
    let mut decompressed_bytes = Vec::new();
    limited
        .read_to_end(&mut decompressed_bytes)
        .await
        .map_err(AppError::Io)?;
    if decompressed_bytes.len() as u64 > MAX_DECOMPRESSED_PACKAGE_SIZE {
        return Err(AppError::FileSizeLimitExceeded(
            "分享包解压后超过 256MB 限制".to_string(),
        ));
    }

    let package: NiuPackage = rmp_serde::decode::from_slice(&decompressed_bytes)
        .map_err(|e| AppError::Serialization(format!("MsgPack 反序列化失败: {}", e)))?;
    validate_package_limits(&package)?;
    Ok(package)
}

fn validate_package_limits(package: &NiuPackage) -> Result<()> {
    if package.version != 1 {
        return Err(AppError::ValidationError(format!(
            "不支持的分享包版本: {}",
            package.version
        )));
    }
    if package.tasks.len() > MAX_PACKAGE_TASKS {
        return Err(AppError::FileSizeLimitExceeded(format!(
            "分享包任务数超过 {} 个限制",
            MAX_PACKAGE_TASKS
        )));
    }

    for task in &package.tasks {
        if task.files.len() > MAX_FILES_PER_TASK {
            return Err(AppError::FileSizeLimitExceeded(format!(
                "任务 '{}' 的文件数超过 {} 个限制",
                task.meta.name, MAX_FILES_PER_TASK
            )));
        }
        for file in &task.files {
            if file.content.len() > MAX_PACKAGE_FILE_SIZE {
                return Err(AppError::FileSizeLimitExceeded(format!(
                    "文件 '{}' 超过 10MB 限制",
                    file.path
                )));
            }
        }
    }
    Ok(())
}

pub(super) async fn get_station_config(db: &DatabaseConnection) -> (String, String) {
    let url = SettingsManager::get(db, "system.station_url")
        .await
        .unwrap_or_default();
    let token = SettingsManager::get(db, "system.station_admin_token")
        .await
        .unwrap_or_default();
    (url, token)
}

pub async fn save_station_config(
    db: &DatabaseConnection,
    payload: StationConfigPayload,
) -> Result<()> {
    SettingsManager::set(db, "system.station_url", &payload.url, Some("System")).await?;
    SettingsManager::set(
        db,
        "system.station_admin_token",
        &payload.token,
        Some("System"),
    )
    .await?;
    info!("中转站配置已更新: {}", payload.url);
    Ok(())
}

pub(super) async fn collect_task_files(files_spec: &[FileAssociations]) -> Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    let mut paths = HashSet::<String>::new();

    for assoc in files_spec {
        paths.insert(assoc.main_file.clone());
        for dep in &assoc.dependencies {
            paths.insert(dep.clone());
        }
    }

    for rel_path in paths {
        let full_path = resolve_path(&rel_path)?;
        if !full_path.is_file() {
            continue;
        }

        let metadata = fs::metadata(&full_path).await.map_err(AppError::Io)?;
        const MAX_SIZE: u64 = 10 * 1024 * 1024;
        if metadata.len() > MAX_SIZE {
            return Err(AppError::FileSizeLimitExceeded(format!(
                "文件 '{}' 超过 10MB 限制",
                rel_path
            )));
        }

        let content = fs::read(&full_path).await.map_err(AppError::Io)?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::MetadataExt;
            metadata.mode()
        };
        #[cfg(not(unix))]
        let mode = 0o644;

        files.push(FileEntry {
            path: rel_path,
            mode,
            content,
        });
    }

    Ok(files)
}

pub(super) async fn collect_task_variables(
    db: &DatabaseConnection,
    task_id: i32,
) -> Result<Option<Vec<ShareVariable>>> {
    let relations = niupanel_entity::variables_tasks::Entity::find()
        .filter(niupanel_entity::variables_tasks::Column::TaskId.eq(task_id))
        .order_by_asc(niupanel_entity::variables_tasks::Column::SortOrder)
        .order_by_asc(niupanel_entity::variables_tasks::Column::VariableId)
        .all(db)
        .await?;
    let relation_ids: Vec<i32> = relations.iter().map(|row| row.variable_id).collect();
    let linked_vars = if relation_ids.is_empty() {
        vec![]
    } else {
        variables::Entity::find()
            .filter(variables::Column::Id.is_in(relation_ids.clone()))
            .all(db)
            .await?
    };
    let mut linked_map: HashMap<i32, variables::Model> =
        linked_vars.into_iter().map(|v| (v.id, v)).collect();
    let mut db_vars = Vec::new();
    for variable_id in relation_ids {
        if let Some(var) = linked_map.remove(&variable_id) {
            db_vars.push(var);
        }
    }

    let mut legacy_query = variables::Entity::find()
        .filter(variables::Column::Scope.eq("Script"))
        .filter(variables::Column::ScopeId.eq(task_id));
    if !db_vars.is_empty() {
        let existing_ids: Vec<i32> = db_vars.iter().map(|v| v.id).collect();
        legacy_query = legacy_query.filter(variables::Column::Id.is_not_in(existing_ids));
    }
    db_vars.extend(
        legacy_query
            .order_by_asc(variables::Column::Id)
            .all(db)
            .await?,
    );

    if db_vars.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            db_vars
                .into_iter()
                .map(|v| ShareVariable {
                    key: v.key,
                    value: v.value,
                })
                .collect(),
        ))
    }
}

pub(super) async fn generate_package(
    db: &DatabaseConnection,
    tasks_spec: &[TaskExportSpec],
) -> Result<NiuPackage> {
    let mut task_data_list = Vec::new();

    for spec in tasks_spec {
        let task = tasks::Entity::find_by_id(spec.task_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("任务 {} 不存在", spec.task_id)))?;

        let files = collect_task_files(&spec.files).await?;
        let variables = if spec.include_envs {
            collect_task_variables(db, task.id).await?
        } else {
            None
        };

        task_data_list.push(TaskData {
            meta: TaskMeta {
                name: task.name,
                description: task.description,
                env_type: task.env_type,
                env_version: task.env_version,
                cron_schedule: task.cron_schedule,
                command: task.command,
                requirements: task.requirements,
                notify: task.notify,
                variables,
                remote_task_id: Some(task.id.to_string()),
            },
            main_file: spec.files.first().map(|f| f.main_file.clone()),
            files,
        });
    }

    Ok(NiuPackage {
        version: 1,
        created_at: chrono::Utc::now().timestamp(),
        tasks: task_data_list,
    })
}
