use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = variable_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "List variable metadata without returning variable values")]
    async fn variables_list(
        &self,
        Parameters(params): Parameters<VariableListParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<VariableListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::VarList)?;
        let result = variable_service::list_variables(
            &self.state.db,
            &VariableQueryParams {
                page: Some(1),
                page_size: Some(params.limit.unwrap_or(50).clamp(1, 100)),
                scope: params.scope,
                scope_id: params.task_id,
                key: params.query,
            },
        )
        .await
        .map_err(tool_error)?;
        self.audit(&user, "variables_list", None).await;
        Ok(Json(VariableListOutput {
            total: result.total,
            items: result
                .items
                .into_iter()
                .map(|item| variable_output(item.inner, item.task_ids, false))
                .collect(),
        }))
    }

    #[tool(description = "Get one variable including its sensitive value")]
    async fn variables_get(
        &self,
        Parameters(VariableIdParams { variable_id }): Parameters<VariableIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<VariableOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::VarRead)?;
        let variable = variables::Entity::find_by_id(variable_id)
            .one(&self.state.db)
            .await
            .map_err(tool_error)?
            .ok_or_else(|| ErrorData::invalid_params("Variable not found", None))?;
        let mut task_ids = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::VariableId.eq(variable_id))
            .all(&self.state.db)
            .await
            .map_err(tool_error)?
            .into_iter()
            .map(|relation| relation.task_id)
            .collect::<Vec<_>>();
        if let Some(scope_id) = variable.scope_id {
            task_ids.push(scope_id);
        }
        task_ids.sort_unstable();
        task_ids.dedup();
        self.audit(&user, "variables_get", Some(variable_id.to_string()))
            .await;
        Ok(Json(variable_output(variable, task_ids, true)))
    }

    #[tool(description = "Create a NiuPanel variable")]
    async fn variables_create(
        &self,
        Parameters(params): Parameters<VariableCreateParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<VariableOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::VarCreate)?;
        let created = variable_service::create_variable(
            &self.state.db,
            VariableRequest {
                key: params.key,
                value: params.value,
                scope: params.scope,
                scope_id: None,
                scope_ids: params.task_ids,
                remarks: params.remarks,
                enabled: params.enabled,
            },
        )
        .await
        .map_err(tool_error)?;
        AuditService::log_user(
            &self.state.db,
            &user,
            "创建变量",
            "variable",
            Some(created.inner.id.to_string()),
            Some(format!("通过 MCP 创建变量: {}", created.inner.key)),
        )
        .await;
        self.audit(
            &user,
            "variables_create",
            Some(created.inner.id.to_string()),
        )
        .await;
        Ok(Json(variable_output(created.inner, created.task_ids, true)))
    }

    #[tool(description = "Update a NiuPanel variable")]
    async fn variables_update(
        &self,
        Parameters(params): Parameters<VariableUpdateParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<VariableOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::VarUpdate)?;
        let variable_id = params.variable_id;
        let updated = variable_service::update_variable(
            &self.state.db,
            variable_id,
            VariableRequest {
                key: params.key,
                value: params.value,
                scope: params.scope,
                scope_id: None,
                scope_ids: params.task_ids,
                remarks: params.remarks,
                enabled: params.enabled,
            },
        )
        .await
        .map_err(tool_error)?;
        AuditService::log_user(
            &self.state.db,
            &user,
            "更新变量",
            "variable",
            Some(variable_id.to_string()),
            Some(format!("通过 MCP 更新变量: {}", updated.inner.key)),
        )
        .await;
        self.audit(&user, "variables_update", Some(variable_id.to_string()))
            .await;
        Ok(Json(variable_output(updated.inner, updated.task_ids, true)))
    }

    #[tool(description = "Delete a NiuPanel variable. This is a destructive operation")]
    async fn variables_delete(
        &self,
        Parameters(VariableIdParams { variable_id }): Parameters<VariableIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<VariableActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::VarDelete)?;
        let details =
            variable_service::batch_delete_variables(&self.state.db, &[variable_id], None)
                .await
                .map_err(tool_error)?;
        AuditService::log_user(
            &self.state.db,
            &user,
            "删除变量",
            "variable",
            Some(variable_id.to_string()),
            Some(format!("通过 MCP {details}")),
        )
        .await;
        self.audit(&user, "variables_delete", Some(variable_id.to_string()))
            .await;
        Ok(Json(VariableActionOutput {
            variable_id,
            accepted: true,
            message: "Variable deleted".to_string(),
        }))
    }
}
