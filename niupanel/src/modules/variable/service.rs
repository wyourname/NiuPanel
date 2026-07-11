use super::models::{TaskSimpleResponse, VariableDto, VariableQueryParams, VariableRequest};
use super::scope::VariableScope;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::PaginatedData;
use niupanel_entity::{tasks, variables, variables_tasks};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set, TransactionTrait,
    sea_query::Expr,
};
use std::collections::{HashMap, HashSet};

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

fn apply_key_filter(
    query: Select<variables::Entity>,
    key_filter: Option<&str>,
) -> Select<variables::Entity> {
    match key_filter.filter(|key| !key.is_empty()) {
        Some(key) => query.filter(variables::Column::Key.contains(key.to_string())),
        None => query,
    }
}

async fn next_variable_sort_order(db: &impl ConnectionTrait) -> Result<i32> {
    let last = variables::Entity::find()
        .order_by_desc(variables::Column::SortOrder)
        .one(db)
        .await?;
    Ok(last.map(|v| v.sort_order + 1).unwrap_or(1))
}

pub async fn list_variables(
    db: &DatabaseConnection,
    params: &VariableQueryParams,
) -> Result<PaginatedData<VariableDto>> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    if let Some(task_id) = params.scope_id {
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

    let mut query = variables::Entity::find();
    if let Some(scope) = &params.scope {
        if !scope.is_empty() {
            query = query.filter(variables::Column::Scope.eq(scope.clone()));
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
    let scope = VariableScope::parse(&payload.scope)?;

    let final_scope_ids = merge_scope_ids(payload.scope_id, payload.scope_ids.clone());

    let next_sort_order = next_variable_sort_order(db).await?;

    let new_variable = variables::ActiveModel {
        key: Set(payload.key),
        value: Set(payload.value),
        scope: Set(scope.as_str().to_string()),
        scope_id: Set(payload.scope_id),
        remarks: Set(payload.remarks),
        enabled: Set(payload.enabled.unwrap_or(true)),
        sort_order: Set(next_sort_order),
        ..Default::default()
    };
    let res = new_variable.insert(db).await?;

    if scope.is_script() && !final_scope_ids.is_empty() {
        sync_variable_task_relations(db, res.id, &final_scope_ids).await?;
    }

    Ok(VariableDto {
        inner: res,
        task_ids: final_scope_ids,
    })
}

pub async fn update_variable(
    db: &DatabaseConnection,
    id: i32,
    payload: VariableRequest,
) -> Result<VariableDto> {
    let scope = VariableScope::parse(&payload.scope)?;

    let variable = variables::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound("Variable not found".to_string()))?;

    let mut active_model: variables::ActiveModel = variable.into();
    active_model.key = Set(payload.key);
    active_model.value = Set(payload.value);
    active_model.scope = Set(scope.as_str().to_string());
    active_model.scope_id = Set(payload.scope_id);
    active_model.remarks = Set(payload.remarks);
    active_model.updated_at = Set(chrono::Utc::now());
    if let Some(enabled) = payload.enabled {
        active_model.enabled = Set(enabled);
    }

    let updated = active_model.update(db).await?;
    let mut final_scope_ids = merge_scope_ids(payload.scope_id, payload.scope_ids);
    if scope.is_script() {
        sync_variable_task_relations(db, id, &final_scope_ids).await?;
    } else {
        variables_tasks::Entity::delete_many()
            .filter(variables_tasks::Column::VariableId.eq(id))
            .exec(db)
            .await?;
        final_scope_ids.clear();
    }

    Ok(VariableDto {
        inner: updated,
        task_ids: final_scope_ids,
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

    variables::Entity::update_many()
        .col_expr(variables::Column::Enabled, Expr::value(enabled))
        .filter(variables::Column::Id.is_in(ids.to_vec()))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn batch_reorder_variables(
    db: &DatabaseConnection,
    task_id: Option<i32>,
    ids: &[i32],
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    let txn = db.begin().await?;
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
    let mut active_models = Vec::new();

    // Get current max sort order
    let mut current_max = variables::Entity::find()
        .order_by_desc(variables::Column::SortOrder)
        .one(db)
        .await?
        .map(|v| v.sort_order)
        .unwrap_or(0);

    for var in payload {
        let scope = VariableScope::parse(&var.scope)?;
        current_max += 1;
        active_models.push(variables::ActiveModel {
            key: Set(var.key),
            value: Set(var.value),
            scope: Set(scope.as_str().to_string()),
            scope_id: Set(var.scope_id),
            remarks: Set(var.remarks),
            enabled: Set(var.enabled.unwrap_or(true)),
            sort_order: Set(current_max),
            ..Default::default()
        });
    }

    let count = active_models.len();
    if !active_models.is_empty() {
        variables::Entity::insert_many(active_models)
            .exec(db)
            .await?;
    }
    Ok(count)
}

pub async fn get_variable_by_key(db: &DatabaseConnection, key: &str) -> Result<Vec<VariableDto>> {
    let variables = variables::Entity::find()
        .filter(variables::Column::Key.eq(key.to_string()))
        .filter(variables::Column::Enabled.eq(true))
        .order_by_asc(variables::Column::Id)
        .all(db)
        .await?;
    build_variable_dtos(db, variables).await
}

pub async fn update_variable_by_key(
    db: &DatabaseConnection,
    key: &str,
    payload: VariableRequest,
) -> Result<Vec<VariableDto>> {
    VariableScope::parse(&payload.scope)?;

    let variables = variables::Entity::find()
        .filter(variables::Column::Key.eq(key.to_string()))
        .order_by_asc(variables::Column::Id)
        .all(db)
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
        let dto = update_variable(db, id, payload.clone()).await?;
        updated_dtos.push(dto);
    }
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
        variables_tasks::Entity::delete_many()
            .filter(variables_tasks::Column::TaskId.eq(task_id))
            .filter(variables_tasks::Column::VariableId.is_in(ids.to_vec()))
            .exec(&txn)
            .await?;

        cleanup_script_variable_links(&txn, ids).await?;
        format!("从任务 {} 解除了 {} 个变量绑定", task_id, ids.len())
    } else {
        variables::Entity::delete_many()
            .filter(variables::Column::Id.is_in(ids.to_vec()))
            .exec(&txn)
            .await?;
        format!("删除了 {} 个变量", ids.len())
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
        if let Some(sid) = var.scope_id {
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
            variables::Entity::find().filter(variables::Column::Id.is_in(relation_ids.clone())),
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
    Ok(last.map(|row| row.sort_order + 1).unwrap_or(1))
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
            active.update(db).await?;
        }
    }
    Ok(())
}
