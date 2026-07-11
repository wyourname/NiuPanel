use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use niupanel_common::error::Result;
use niupanel_common::response::PaginatedData;
use niupanel_entity::{task_runs, tasks, variables};

use super::TaskService;
use crate::modules::tasks::models::QueryParams;

impl TaskService {
    pub async fn list_tasks(
        db: &DatabaseConnection,
        params: QueryParams,
    ) -> Result<PaginatedData<tasks::Model>> {
        let mut query = tasks::Entity::find();

        if let Some(q) = params.q {
            if !q.is_empty() {
                query = query.filter(
                    tasks::Column::Name
                        .contains(&q)
                        .or(tasks::Column::Path.contains(&q))
                        .or(tasks::Column::Command.contains(&q)),
                );
            }
        }

        if let Some(status) = params.status {
            if !status.is_empty() && status != "all" {
                query = query.filter(tasks::Column::Status.eq(status));
            }
        }

        let total = query.clone().count(db).await?;
        let mut query = query
            .order_by_desc(tasks::Column::IsPinned)
            .order_by_desc(tasks::Column::Id);

        if let Some(page_size) = params.page_size {
            let page = params.page.unwrap_or(1);
            let offset = (page - 1) * page_size;
            query = query.limit(page_size).offset(offset);
        }

        let items = query.all(db).await?;
        Ok(PaginatedData { items, total })
    }

    pub async fn save_pagination_setting(
        db: &DatabaseConnection,
        user_id: i32,
        page_size: u64,
    ) -> Result<()> {
        let key = "task_page_size";
        let value = page_size.to_string();

        let existing = variables::Entity::find()
            .filter(variables::Column::Key.eq(key))
            .filter(variables::Column::Scope.eq("User"))
            .filter(variables::Column::ScopeId.eq(user_id))
            .one(db)
            .await?;

        if let Some(existing) = existing {
            let mut active: variables::ActiveModel = existing.into();
            active.value = Set(value);
            active.update(db).await?;
        } else {
            variables::ActiveModel {
                key: Set(key.to_string()),
                value: Set(value),
                scope: Set("User".to_string()),
                scope_id: Set(Some(user_id)),
                enabled: Set(true),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }

        Ok(())
    }

    pub async fn get_pagination_setting(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Option<u64>> {
        let key = "task_page_size";
        let existing = variables::Entity::find()
            .filter(variables::Column::Key.eq(key))
            .filter(variables::Column::Scope.eq("User"))
            .filter(variables::Column::ScopeId.eq(user_id))
            .one(db)
            .await?;

        Ok(existing.and_then(|value| value.value.parse::<u64>().ok()))
    }

    pub async fn batch_set_pinned(
        db: &DatabaseConnection,
        ids: &[i32],
        pinned: bool,
    ) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        tasks::Entity::update_many()
            .col_expr(
                tasks::Column::IsPinned,
                sea_orm::sea_query::Expr::value(pinned),
            )
            .filter(tasks::Column::Id.is_in(ids.iter().copied()))
            .exec(db)
            .await?;
        Ok(())
    }

    pub async fn list_task_history(
        db: &DatabaseConnection,
        id: i32,
        params: QueryParams,
    ) -> Result<PaginatedData<task_runs::Model>> {
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(10);
        let offset = (page - 1) * page_size;

        let total = task_runs::Entity::find()
            .filter(task_runs::Column::TaskId.eq(id))
            .count(db)
            .await?;

        let items = task_runs::Entity::find()
            .filter(task_runs::Column::TaskId.eq(id))
            .order_by_desc(task_runs::Column::StartedAt)
            .limit(page_size)
            .offset(offset)
            .all(db)
            .await?;

        Ok(PaginatedData { items, total })
    }
}
