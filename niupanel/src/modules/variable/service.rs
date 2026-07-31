use super::models::{
    BatchSaveVariablesRequest, TaskSimpleResponse, VariableDto, VariableQueryParams,
    VariableRequest, VariableValueDto,
};
use super::scope::VariableScope;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::PaginatedData;
use niupanel_common::variable::{
    normalize_variable_key, normalize_variable_remarks, validate_variable_value,
};
use niupanel_entity::{tasks, variables, variables_tasks};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set, TransactionTrait,
    sea_query::{Condition, Expr},
};
use std::collections::{HashMap, HashSet};

const DEFAULT_PAGE_SIZE: u64 = 20;
const MAX_PAGE_SIZE: u64 = 1_000;

#[derive(Debug)]
struct NormalizedVariableInput {
    key: String,
    value: String,
    scope: VariableScope,
    task_ids: Vec<i32>,
    remarks: Option<String>,
    enabled: Option<bool>,
}

fn merge_scope_ids(scope_id: Option<i32>, scope_ids: Option<Vec<i32>>) -> Vec<i32> {
    let mut final_scope_ids = scope_ids.unwrap_or_default();
    if let Some(sid) = scope_id {
        final_scope_ids.push(sid);
    }
    dedupe_scope_ids(final_scope_ids)
}

fn dedupe_scope_ids(scope_ids: Vec<i32>) -> Vec<i32> {
    let mut seen = HashSet::new();
    scope_ids
        .into_iter()
        .filter(|scope_id| seen.insert(*scope_id))
        .collect()
}

fn dedupe_ids(ids: &[i32]) -> Vec<i32> {
    dedupe_scope_ids(ids.to_vec())
}

fn public_scope_condition() -> Condition {
    Condition::any()
        .add(variables::Column::Scope.eq(VariableScope::Global.as_str()))
        .add(variables::Column::Scope.eq(VariableScope::Script.as_str()))
}

async fn normalize_variable_input<C>(
    db: &C,
    payload: VariableRequest,
) -> Result<NormalizedVariableInput>
where
    C: ConnectionTrait,
{
    let key = normalize_variable_key(&payload.key)?;
    validate_variable_value(&payload.value)?;
    let remarks = normalize_variable_remarks(payload.remarks)?;

    let scope = VariableScope::parse(&payload.scope)?;
    let task_ids = merge_scope_ids(payload.scope_id, payload.scope_ids);
    match scope {
        VariableScope::Global if !task_ids.is_empty() => {
            return Err(AppError::ValidationError(
                "全局变量不能绑定任务".to_string(),
            ));
        }
        VariableScope::Script if task_ids.is_empty() => {
            return Err(AppError::ValidationError(
                "脚本变量必须至少绑定一个任务".to_string(),
            ));
        }
        VariableScope::Script => {
            if task_ids.iter().any(|id| *id <= 0) {
                return Err(AppError::ValidationError(
                    "任务 ID 必须为正整数".to_string(),
                ));
            }
            let count = tasks::Entity::find()
                .filter(tasks::Column::Id.is_in(task_ids.clone()))
                .count(db)
                .await?;
            if count != task_ids.len() as u64 {
                return Err(AppError::ValidationError(
                    "变量绑定包含不存在的任务".to_string(),
                ));
            }
        }
        VariableScope::Global => {}
    }

    Ok(NormalizedVariableInput {
        key,
        value: payload.value,
        scope,
        task_ids,
        remarks,
        enabled: payload.enabled,
    })
}

fn apply_key_filter(
    query: Select<variables::Entity>,
    key_filter: Option<&str>,
) -> Select<variables::Entity> {
    match key_filter.map(str::trim).filter(|key| !key.is_empty()) {
        Some(key) => query.filter(
            variables::Column::Key
                .contains(key.to_string())
                .or(variables::Column::Remarks.contains(key.to_string())),
        ),
        None => query,
    }
}

async fn next_variable_sort_order(db: &impl ConnectionTrait) -> Result<i32> {
    let last = variables::Entity::find()
        .order_by_desc(variables::Column::SortOrder)
        .one(db)
        .await?;
    last.map(|variable| {
        variable.sort_order.checked_add(1).ok_or_else(|| {
            AppError::ValidationError("变量排序值已达到上限，请先重新排序".to_string())
        })
    })
    .unwrap_or(Ok(1))
}

pub async fn list_variables(
    db: &DatabaseConnection,
    params: &VariableQueryParams,
) -> Result<PaginatedData<VariableDto>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = page.saturating_sub(1).saturating_mul(page_size);

    if let Some(task_id) = params.scope_id {
        if params.scope.as_deref() != Some(VariableScope::Script.as_str()) {
            return Err(AppError::ValidationError(
                "scope_id 只能与 Script 作用域一起使用".to_string(),
            ));
        }
        let rows = list_task_scoped_variables(db, task_id, params.key.as_deref()).await?;
        let total = rows.len() as u64;
        let items = rows
            .into_iter()
            .skip(offset as usize)
            .take(page_size as usize)
            .collect::<Vec<_>>();
        let dtos = build_variable_dtos(db, items).await?;
        return Ok(PaginatedData { items: dtos, total });
    }

    let mut query = variables::Entity::find().filter(public_scope_condition());
    if let Some(scope) = &params.scope {
        if !scope.is_empty() {
            let scope = VariableScope::parse(scope)?;
            query = query.filter(variables::Column::Scope.eq(scope.as_str()));
        }
    }
    query = apply_key_filter(query, params.key.as_deref());

    let total = query.clone().count(db).await?;
    let variables = query
        .order_by_asc(variables::Column::SortOrder)
        .order_by_asc(variables::Column::Id)
        .limit(page_size)
        .offset(offset)
        .all(db)
        .await?;
    let dtos = build_variable_dtos(db, variables).await?;

    Ok(PaginatedData { items: dtos, total })
}

pub async fn list_tasks_simple(db: &DatabaseConnection) -> Result<Vec<TaskSimpleResponse>> {
    tasks::Entity::find()
        .select_only()
        .column(tasks::Column::Id)
        .column(tasks::Column::Name)
        .order_by_desc(tasks::Column::Id)
        .into_model::<TaskSimpleResponse>()
        .all(db)
        .await
        .map_err(Into::into)
}

pub async fn create_variable(
    db: &DatabaseConnection,
    payload: VariableRequest,
) -> Result<VariableDto> {
    let txn = db.begin().await?;
    let created = create_variable_in(&txn, payload).await?;
    txn.commit().await?;
    Ok(created)
}

pub async fn update_variable(
    db: &DatabaseConnection,
    id: i32,
    payload: VariableRequest,
) -> Result<VariableDto> {
    let txn = db.begin().await?;
    let updated = update_variable_in(&txn, id, payload).await?;
    txn.commit().await?;
    Ok(updated)
}

async fn create_variable_in<C>(db: &C, payload: VariableRequest) -> Result<VariableDto>
where
    C: ConnectionTrait,
{
    let input = normalize_variable_input(db, payload).await?;
    let next_sort_order = next_variable_sort_order(db).await?;
    let primary_task_id = input.task_ids.first().copied();
    let created = variables::ActiveModel {
        key: Set(input.key),
        value: Set(input.value),
        scope: Set(input.scope.as_str().to_string()),
        scope_id: Set(primary_task_id),
        remarks: Set(input.remarks),
        enabled: Set(input.enabled.unwrap_or(true)),
        sort_order: Set(next_sort_order),
        ..Default::default()
    }
    .insert(db)
    .await?;

    if input.scope.is_script() {
        sync_variable_task_relations(db, created.id, &input.task_ids).await?;
    }

    let persisted = variables::Entity::find_by_id(created.id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::Internal("变量创建后无法重新读取".to_string()))?;
    Ok(VariableDto {
        inner: persisted,
        task_ids: input.task_ids,
    })
}

async fn update_variable_in<C>(db: &C, id: i32, payload: VariableRequest) -> Result<VariableDto>
where
    C: ConnectionTrait,
{
    let input = normalize_variable_input(db, payload).await?;
    let variable = variables::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound("Variable not found".to_string()))?;

    let mut active_model: variables::ActiveModel = variable.into();
    active_model.key = Set(input.key);
    active_model.value = Set(input.value);
    active_model.scope = Set(input.scope.as_str().to_string());
    active_model.scope_id = Set(input.task_ids.first().copied());
    active_model.remarks = Set(input.remarks);
    active_model.updated_at = Set(chrono::Utc::now());
    if let Some(enabled) = input.enabled {
        active_model.enabled = Set(enabled);
    }

    active_model.update(db).await?;
    if input.scope.is_script() {
        sync_variable_task_relations(db, id, &input.task_ids).await?;
    } else {
        variables_tasks::Entity::delete_many()
            .filter(variables_tasks::Column::VariableId.eq(id))
            .exec(db)
            .await?;
    }

    let persisted = variables::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::Internal("变量更新后无法重新读取".to_string()))?;
    Ok(VariableDto {
        inner: persisted,
        task_ids: input.task_ids,
    })
}

pub async fn batch_toggle_variables(
    db: &DatabaseConnection,
    ids: &[i32],
    enabled: bool,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let ids = dedupe_ids(ids);
    let existing = variables::Entity::find()
        .filter(variables::Column::Id.is_in(ids.clone()))
        .filter(public_scope_condition())
        .count(db)
        .await?;
    if existing != ids.len() as u64 {
        return Err(AppError::ValidationError(
            "批量操作包含不存在或不可管理的变量".to_string(),
        ));
    }

    variables::Entity::update_many()
        .col_expr(variables::Column::Enabled, Expr::value(enabled))
        .col_expr(
            variables::Column::UpdatedAt,
            Expr::value(chrono::Utc::now()),
        )
        .filter(variables::Column::Id.is_in(ids))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn batch_save_task_variables(
    db: &DatabaseConnection,
    payload: BatchSaveVariablesRequest,
) -> Result<Vec<VariableDto>> {
    if payload.task_id <= 0 {
        return Err(AppError::ValidationError(
            "任务 ID 必须为正整数".to_string(),
        ));
    }
    let task_exists = tasks::Entity::find_by_id(payload.task_id).one(db).await?;
    if task_exists.is_none() {
        return Err(AppError::NotFound("Task not found".to_string()));
    }

    let upsert_ids = payload
        .upserts
        .iter()
        .filter_map(|upsert| upsert.id)
        .collect::<Vec<_>>();
    if dedupe_ids(&upsert_ids).len() != upsert_ids.len() {
        return Err(AppError::ValidationError(
            "批量保存包含重复变量 ID".to_string(),
        ));
    }
    let delete_ids = dedupe_ids(&payload.delete_ids);
    if upsert_ids.iter().any(|id| delete_ids.contains(id)) {
        return Err(AppError::ValidationError(
            "同一个变量不能同时更新和删除".to_string(),
        ));
    }

    let txn = db.begin().await?;
    let mut saved = Vec::with_capacity(payload.upserts.len());
    for upsert in payload.upserts {
        let dto = match upsert.id {
            Some(id) => update_variable_in(&txn, id, upsert.variable).await?,
            None => create_variable_in(&txn, upsert.variable).await?,
        };
        if dto.inner.scope != VariableScope::Script.as_str()
            || !dto.task_ids.contains(&payload.task_id)
        {
            return Err(AppError::ValidationError(
                "批量保存的变量必须绑定到当前任务".to_string(),
            ));
        }
        saved.push(dto);
    }

    if !delete_ids.is_empty() {
        let related_ids = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::TaskId.eq(payload.task_id))
            .filter(variables_tasks::Column::VariableId.is_in(delete_ids.clone()))
            .all(&txn)
            .await?
            .into_iter()
            .map(|relation| relation.variable_id)
            .collect::<Vec<_>>();
        if related_ids.len() != delete_ids.len() {
            return Err(AppError::ValidationError(
                "批量保存包含不属于当前任务的待删除变量".to_string(),
            ));
        }
        variables_tasks::Entity::delete_many()
            .filter(variables_tasks::Column::TaskId.eq(payload.task_id))
            .filter(variables_tasks::Column::VariableId.is_in(delete_ids.clone()))
            .exec(&txn)
            .await?;
        cleanup_script_variable_links(&txn, &related_ids).await?;
    }

    txn.commit().await?;
    Ok(saved)
}

pub async fn batch_reorder_variables(
    db: &DatabaseConnection,
    task_id: Option<i32>,
    scope: Option<&str>,
    ids: &[i32],
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let requested_count = ids.len();
    let ids = dedupe_ids(ids);
    if ids.len() != requested_count {
        return Err(AppError::ValidationError(
            "排序列表不能包含重复 ID".to_string(),
        ));
    }

    let txn = db.begin().await?;
    if let Some(task_id) = task_id {
        if scope.is_some_and(|scope| scope != VariableScope::Script.as_str()) {
            return Err(AppError::ValidationError(
                "任务内排序只能用于 Script 变量".to_string(),
            ));
        }
        let expected = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::TaskId.eq(task_id))
            .count(&txn)
            .await?;
        let matched = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::TaskId.eq(task_id))
            .filter(variables_tasks::Column::VariableId.is_in(ids.clone()))
            .count(&txn)
            .await?;
        if expected != ids.len() as u64 || matched != expected {
            return Err(AppError::ValidationError(
                "请加载该任务的全部变量后再排序".to_string(),
            ));
        }
    } else {
        let scope = VariableScope::parse(
            scope.ok_or_else(|| AppError::ValidationError("全局排序必须指定 scope".to_string()))?,
        )?;
        let expected = variables::Entity::find()
            .filter(variables::Column::Scope.eq(scope.as_str()))
            .count(&txn)
            .await?;
        let matched = variables::Entity::find()
            .filter(variables::Column::Scope.eq(scope.as_str()))
            .filter(variables::Column::Id.is_in(ids.clone()))
            .count(&txn)
            .await?;
        if expected != ids.len() as u64 || matched != expected {
            return Err(AppError::ValidationError(
                "请加载当前作用域的全部变量后再排序".to_string(),
            ));
        }
    }

    for (index, var_id) in ids.iter().enumerate() {
        let sort_order = index as i32 + 1;
        if let Some(task_id) = task_id {
            variables_tasks::Entity::update_many()
                .col_expr(variables_tasks::Column::SortOrder, Expr::value(sort_order))
                .filter(variables_tasks::Column::TaskId.eq(task_id))
                .filter(variables_tasks::Column::VariableId.eq(*var_id))
                .exec(&txn)
                .await?;
        } else {
            variables::Entity::update_many()
                .col_expr(variables::Column::SortOrder, Expr::value(sort_order))
                .col_expr(
                    variables::Column::UpdatedAt,
                    Expr::value(chrono::Utc::now()),
                )
                .filter(variables::Column::Id.eq(*var_id))
                .exec(&txn)
                .await?;
        }
    }
    txn.commit().await?;
    Ok(())
}

pub async fn import_variables(
    db: &DatabaseConnection,
    payload: Vec<VariableRequest>,
) -> Result<usize> {
    let count = payload.len();
    let txn = db.begin().await?;
    for variable in payload {
        create_variable_in(&txn, variable).await?;
    }
    txn.commit().await?;
    Ok(count)
}

pub async fn get_variable_by_key(db: &DatabaseConnection, key: &str) -> Result<Vec<VariableDto>> {
    let variables = variables::Entity::find()
        .filter(variables::Column::Key.eq(key.to_string()))
        .filter(variables::Column::Enabled.eq(true))
        .filter(public_scope_condition())
        .order_by_asc(variables::Column::Id)
        .all(db)
        .await?;
    build_variable_dtos(db, variables).await
}

pub async fn get_variable_value(db: &DatabaseConnection, id: i32) -> Result<VariableValueDto> {
    let variable = variables::Entity::find_by_id(id)
        .filter(public_scope_condition())
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Variable not found".to_string()))?;
    Ok(VariableValueDto {
        id: variable.id,
        key: variable.key,
        value: variable.value,
        updated_at: variable.updated_at,
    })
}

pub async fn update_variable_by_key(
    db: &DatabaseConnection,
    key: &str,
    payload: VariableRequest,
) -> Result<Vec<VariableDto>> {
    let txn = db.begin().await?;
    let variables = variables::Entity::find()
        .filter(variables::Column::Key.eq(key.to_string()))
        .filter(public_scope_condition())
        .order_by_asc(variables::Column::Id)
        .all(&txn)
        .await?;

    if variables.is_empty() {
        return Err(AppError::NotFound(format!(
            "Variable with key '{}' not found",
            key
        )));
    }

    let mut updated_dtos = Vec::with_capacity(variables.len());
    for variable in variables {
        let id = variable.id;
        let dto = update_variable_in(&txn, id, payload.clone()).await?;
        updated_dtos.push(dto);
    }
    txn.commit().await?;
    Ok(updated_dtos)
}

pub async fn batch_delete_variables(
    db: &DatabaseConnection,
    ids: &[i32],
    task_id: Option<i32>,
) -> Result<String> {
    if ids.is_empty() {
        return Ok(String::new());
    }

    let txn = db.begin().await?;
    let action_message = if let Some(task_id) = task_id {
        let related_ids = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::TaskId.eq(task_id))
            .filter(variables_tasks::Column::VariableId.is_in(ids.to_vec()))
            .all(&txn)
            .await?
            .into_iter()
            .map(|relation| relation.variable_id)
            .collect::<Vec<_>>();
        let result = variables_tasks::Entity::delete_many()
            .filter(variables_tasks::Column::TaskId.eq(task_id))
            .filter(variables_tasks::Column::VariableId.is_in(ids.to_vec()))
            .exec(&txn)
            .await?;

        cleanup_script_variable_links(&txn, &related_ids).await?;
        format!(
            "从任务 {} 解除了 {} 个变量绑定",
            task_id, result.rows_affected
        )
    } else {
        let result = variables::Entity::delete_many()
            .filter(variables::Column::Id.is_in(ids.to_vec()))
            .filter(public_scope_condition())
            .exec(&txn)
            .await?;
        format!("删除了 {} 个变量", result.rows_affected)
    };

    txn.commit().await?;
    Ok(action_message)
}

async fn build_variable_dtos<C>(db: &C, vars: Vec<variables::Model>) -> Result<Vec<VariableDto>>
where
    C: ConnectionTrait,
{
    let variable_ids: Vec<i32> = vars.iter().map(|v| v.id).collect();
    let relations = if !variable_ids.is_empty() {
        variables_tasks::Entity::find()
            .filter(variables_tasks::Column::VariableId.is_in(variable_ids))
            .all(db)
            .await?
    } else {
        vec![]
    };

    let mut relation_map: HashMap<i32, Vec<i32>> = HashMap::new();
    for relation in relations {
        relation_map
            .entry(relation.variable_id)
            .or_default()
            .push(relation.task_id);
    }

    let mut dtos = Vec::with_capacity(vars.len());
    for var in vars {
        let mut task_ids = relation_map.remove(&var.id).unwrap_or_default();
        if var.scope == VariableScope::Script.as_str()
            && let Some(sid) = var.scope_id
        {
            if !task_ids.contains(&sid) {
                task_ids.push(sid);
            }
        }
        task_ids.sort();
        task_ids.dedup();
        dtos.push(VariableDto {
            inner: var,
            task_ids,
        });
    }
    Ok(dtos)
}

async fn list_task_scoped_variables(
    db: &impl ConnectionTrait,
    task_id: i32,
    key_filter: Option<&str>,
) -> Result<Vec<variables::Model>> {
    let relations = variables_tasks::Entity::find()
        .filter(variables_tasks::Column::TaskId.eq(task_id))
        .order_by_asc(variables_tasks::Column::SortOrder)
        .order_by_asc(variables_tasks::Column::VariableId)
        .all(db)
        .await?;

    let relation_ids: Vec<i32> = relations.iter().map(|r| r.variable_id).collect();
    let linked_vars = if relation_ids.is_empty() {
        vec![]
    } else {
        let query = apply_key_filter(
            variables::Entity::find()
                .filter(variables::Column::Id.is_in(relation_ids.clone()))
                .filter(variables::Column::Scope.eq(VariableScope::Script.as_str())),
            key_filter,
        );
        query.all(db).await?
    };

    let mut linked_map: HashMap<i32, variables::Model> =
        linked_vars.into_iter().map(|v| (v.id, v)).collect();
    let mut ordered = Vec::new();
    for variable_id in relation_ids {
        if let Some(var) = linked_map.remove(&variable_id) {
            ordered.push(var);
        }
    }

    let mut legacy_query = variables::Entity::find()
        .filter(variables::Column::Scope.eq(VariableScope::Script.as_str()))
        .filter(variables::Column::ScopeId.eq(task_id));
    if !linked_map.is_empty() {
        let extra_ids: Vec<i32> = linked_map.keys().copied().collect();
        legacy_query = legacy_query.filter(variables::Column::Id.is_not_in(extra_ids));
    } else if !ordered.is_empty() {
        let existing_ids: Vec<i32> = ordered.iter().map(|v| v.id).collect();
        legacy_query = legacy_query.filter(variables::Column::Id.is_not_in(existing_ids));
    }
    legacy_query = apply_key_filter(legacy_query, key_filter);

    let legacy_vars = legacy_query
        .order_by_asc(variables::Column::Id)
        .all(db)
        .await?;
    ordered.extend(legacy_vars);
    Ok(ordered)
}

async fn next_task_sort_order(db: &impl ConnectionTrait, task_id: i32) -> Result<i32> {
    let last = variables_tasks::Entity::find()
        .filter(variables_tasks::Column::TaskId.eq(task_id))
        .order_by_desc(variables_tasks::Column::SortOrder)
        .one(db)
        .await?;
    last.map(|row| {
        row.sort_order.checked_add(1).ok_or_else(|| {
            AppError::ValidationError("任务变量排序值已达到上限，请先重新排序".to_string())
        })
    })
    .unwrap_or(Ok(1))
}

async fn sync_variable_task_relations(
    db: &impl ConnectionTrait,
    variable_id: i32,
    task_ids: &[i32],
) -> Result<()> {
    let deduped = dedupe_scope_ids(task_ids.to_vec());

    let existing = variables_tasks::Entity::find()
        .filter(variables_tasks::Column::VariableId.eq(variable_id))
        .all(db)
        .await?;
    let existing_task_ids: HashSet<i32> = existing.iter().map(|r| r.task_id).collect();

    let to_remove: Vec<i32> = existing
        .iter()
        .filter(|r| !deduped.contains(&r.task_id))
        .map(|r| r.task_id)
        .collect();
    if !to_remove.is_empty() {
        variables_tasks::Entity::delete_many()
            .filter(variables_tasks::Column::VariableId.eq(variable_id))
            .filter(variables_tasks::Column::TaskId.is_in(to_remove))
            .exec(db)
            .await?;
    }

    for task_id in deduped {
        if !existing_task_ids.contains(&task_id) {
            let sort_order = next_task_sort_order(db, task_id).await?;
            variables_tasks::ActiveModel {
                variable_id: Set(variable_id),
                task_id: Set(task_id),
                sort_order: Set(sort_order),
            }
            .insert(db)
            .await?;
        }
    }

    cleanup_script_variable_links(db, &[variable_id]).await?;
    Ok(())
}

async fn cleanup_script_variable_links(
    db: &impl ConnectionTrait,
    variable_ids: &[i32],
) -> Result<()> {
    if variable_ids.is_empty() {
        return Ok(());
    }

    let vars = variables::Entity::find()
        .filter(variables::Column::Id.is_in(variable_ids.to_vec()))
        .filter(variables::Column::Scope.eq(VariableScope::Script.as_str()))
        .all(db)
        .await?;

    for var in vars {
        let relations = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::VariableId.eq(var.id))
            .order_by_asc(variables_tasks::Column::SortOrder)
            .order_by_asc(variables_tasks::Column::TaskId)
            .all(db)
            .await?;

        if relations.is_empty() {
            variables::Entity::delete_by_id(var.id).exec(db).await?;
            continue;
        }

        let preferred_scope_id = Some(relations[0].task_id);
        if var.scope_id != preferred_scope_id {
            let mut active: variables::ActiveModel = var.into();
            active.scope_id = Set(preferred_scope_id);
            active.updated_at = Set(chrono::Utc::now());
            active.update(db).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::tasks::models::{TaskVariable, UpdateTaskRequest};
    use crate::modules::tasks::service::TaskService;
    use crate::modules::variable::models::VariableUpsertRequest;
    use migration::{Migrator, MigratorTrait};
    use niupanel_core::task::{CreateTaskInput, TaskVariableInput, create_task_record};
    use sea_orm::{ConnectOptions, Database};

    async fn setup_db() -> DatabaseConnection {
        let mut options = ConnectOptions::new("sqlite::memory:");
        options.max_connections(1).min_connections(1);
        let db = Database::connect(options).await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO users (username, password_hash, role) VALUES ('tester', 'hash', 'Admin')",
        )
        .await
        .unwrap();
        db
    }

    async fn create_task(db: &DatabaseConnection, name: &str) -> i32 {
        db.execute_unprepared(&format!(
            "INSERT INTO tasks (name, path, env_type, user_id) VALUES ('{name}', '', 'shell', 1)"
        ))
        .await
        .unwrap();
        tasks::Entity::find()
            .order_by_desc(tasks::Column::Id)
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .id
    }

    fn request(key: &str, value: &str, scope: &str, task_ids: Vec<i32>) -> VariableRequest {
        VariableRequest {
            key: key.to_string(),
            value: value.to_string(),
            scope: scope.to_string(),
            scope_id: None,
            scope_ids: Some(task_ids),
            remarks: None,
            enabled: Some(true),
        }
    }

    #[tokio::test]
    async fn rejects_invalid_reserved_and_unbound_variables() {
        let db = setup_db().await;
        let task_id = create_task(&db, "task").await;

        let invalid_key =
            create_variable(&db, request("NOT-PORTABLE", "value", "Global", vec![])).await;
        assert!(
            invalid_key
                .err()
                .expect("invalid key must fail")
                .to_string()
                .contains("必须匹配")
        );

        let reserved =
            create_variable(&db, request("NIUPANEL_TASK_ID", "value", "Global", vec![])).await;
        assert!(
            reserved
                .err()
                .expect("reserved key must fail")
                .to_string()
                .contains("运行时保留")
        );

        let global_with_task =
            create_variable(&db, request("GLOBAL", "value", "Global", vec![task_id])).await;
        assert!(
            global_with_task
                .err()
                .expect("global variable with task must fail")
                .to_string()
                .contains("全局变量不能绑定任务")
        );

        let unbound = create_variable(&db, request("SCRIPT", "value", "Script", vec![])).await;
        assert!(
            unbound
                .err()
                .expect("unbound script variable must fail")
                .to_string()
                .contains("至少绑定一个任务")
        );
    }

    #[tokio::test]
    async fn creates_all_task_links_and_keeps_scope_id_as_projection() {
        let db = setup_db().await;
        let first_task = create_task(&db, "first").await;
        let second_task = create_task(&db, "second").await;

        let created = create_variable(
            &db,
            request(
                "SHARED_TOKEN",
                "secret",
                "Script",
                vec![first_task, second_task, first_task],
            ),
        )
        .await
        .unwrap();

        assert_eq!(created.inner.scope_id, Some(first_task));
        assert_eq!(created.task_ids, vec![first_task, second_task]);
        let links = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::VariableId.eq(created.inner.id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn import_and_task_batch_save_are_atomic() {
        let db = setup_db().await;
        let first_task = create_task(&db, "first").await;
        let second_task = create_task(&db, "second").await;

        let import_result = import_variables(
            &db,
            vec![
                request("WILL_ROLL_BACK", "one", "Global", vec![]),
                request("BROKEN", "two", "Script", vec![]),
            ],
        )
        .await;
        assert!(import_result.is_err());
        assert_eq!(variables::Entity::find().count(&db).await.unwrap(), 0);

        let created = create_variable(
            &db,
            request(
                "ATOMIC",
                "original",
                "Script",
                vec![first_task, second_task],
            ),
        )
        .await
        .unwrap();
        let batch_result = batch_save_task_variables(
            &db,
            BatchSaveVariablesRequest {
                task_id: first_task,
                upserts: vec![
                    VariableUpsertRequest {
                        id: Some(created.inner.id),
                        variable: request("ATOMIC", "changed", "Script", vec![first_task]),
                    },
                    VariableUpsertRequest {
                        id: None,
                        variable: request("INVALID-KEY", "new", "Script", vec![first_task]),
                    },
                ],
                delete_ids: vec![],
            },
        )
        .await;
        assert!(batch_result.is_err());

        let persisted = variables::Entity::find_by_id(created.inner.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted.value, "original");
        let links = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::VariableId.eq(created.inner.id))
            .count(&db)
            .await
            .unwrap();
        assert_eq!(links, 2);
    }

    #[tokio::test]
    async fn task_creation_rolls_back_when_a_nested_variable_is_invalid() {
        let db = setup_db().await;
        let result = create_task_record(
            &db,
            1,
            CreateTaskInput {
                name: "atomic task".to_string(),
                path: None,
                command: None,
                description: None,
                env_type: "shell".to_string(),
                env_version: None,
                tags: None,
                cron_schedule: None,
                requirements: None,
                variables: Some(vec![
                    TaskVariableInput {
                        key: "VALID".to_string(),
                        value: "one".to_string(),
                    },
                    TaskVariableInput {
                        key: "INVALID-KEY".to_string(),
                        value: "two".to_string(),
                    },
                ]),
                notify: None,
                cpu_limit: None,
                timeout_sec: None,
                memory_limit: None,
                trigger_next_tasks: None,
                random_config: None,
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(tasks::Entity::find().count(&db).await.unwrap(), 0);
        assert_eq!(variables::Entity::find().count(&db).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn task_update_keeps_existing_task_and_variables_when_replacement_fails() {
        let db = setup_db().await;
        let created = create_task_record(
            &db,
            1,
            CreateTaskInput {
                name: "original task".to_string(),
                path: None,
                command: None,
                description: None,
                env_type: "shell".to_string(),
                env_version: None,
                tags: None,
                cron_schedule: None,
                requirements: None,
                variables: Some(vec![TaskVariableInput {
                    key: "ORIGINAL".to_string(),
                    value: "original value".to_string(),
                }]),
                notify: None,
                cpu_limit: None,
                timeout_sec: None,
                memory_limit: None,
                trigger_next_tasks: None,
                random_config: None,
            },
        )
        .await
        .unwrap();

        let result = TaskService::update_task(
            &db,
            created.id,
            UpdateTaskRequest {
                name: Some("changed task".to_string()),
                variables: Some(vec![TaskVariable {
                    key: "INVALID-KEY".to_string(),
                    value: "new value".to_string(),
                }]),
                ..Default::default()
            },
        )
        .await;
        assert!(result.is_err());

        let persisted_task = tasks::Entity::find_by_id(created.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted_task.name, "original task");
        let persisted_variables = variables::Entity::find().all(&db).await.unwrap();
        assert_eq!(persisted_variables.len(), 1);
        assert_eq!(persisted_variables[0].key, "ORIGINAL");
        assert_eq!(persisted_variables[0].value, "original value");
    }

    #[tokio::test]
    async fn migration_reconciles_legacy_scope_ids_and_global_references() {
        let db = setup_db().await;
        Migrator::down(&db, Some(1)).await.unwrap();
        let task_id = create_task(&db, "legacy task").await;
        db.execute_unprepared(&format!(
            "INSERT INTO variables (key, value, scope, scope_id, sort_order) VALUES \
             ('LEGACY_SCRIPT', 'one', 'Script', {task_id}, 7), \
             ('STALE_GLOBAL', 'two', 'Global', {task_id}, 8)"
        ))
        .await
        .unwrap();

        Migrator::up(&db, Some(1)).await.unwrap();

        let script = variables::Entity::find()
            .filter(variables::Column::Key.eq("LEGACY_SCRIPT"))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let relation = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::VariableId.eq(script.id))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(relation.task_id, task_id);
        assert_eq!(relation.sort_order, 7);

        let global = variables::Entity::find()
            .filter(variables::Column::Key.eq("STALE_GLOBAL"))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(global.scope_id, None);
    }

    #[tokio::test]
    async fn list_clamps_pagination_searches_remarks_and_hides_internal_values() {
        let db = setup_db().await;
        let mut visible = request("PUBLIC_KEY", "public-secret", "Global", vec![]);
        visible.remarks = Some("production credential".to_string());
        let created = create_variable(&db, visible).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO variables (key, value, scope, sort_order) \
             VALUES ('INTERNAL_KEY', 'internal-secret', 'User', 2)",
        )
        .await
        .unwrap();

        let page = list_variables(
            &db,
            &VariableQueryParams {
                page: Some(0),
                page_size: Some(u64::MAX),
                scope: None,
                scope_id: None,
                key: Some("credential".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].inner.id, created.inner.id);

        let value = get_variable_value(&db, created.inner.id).await.unwrap();
        assert_eq!(value.value, "public-secret");
        let internal_id = variables::Entity::find()
            .filter(variables::Column::Key.eq("INTERNAL_KEY"))
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .id;
        assert!(get_variable_value(&db, internal_id).await.is_err());
    }
}
