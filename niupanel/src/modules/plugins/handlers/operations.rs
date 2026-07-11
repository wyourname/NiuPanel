use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/plugins",
    responses(
        (status = 200, description = "List all installed plugins")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn list_plugins() -> Result<ApiResponse<Vec<PluginRecord>>> {
    Ok(ApiResponse::success(installed_plugins()?))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/preview",
    request_body = PluginInstallRequest,
    responses(
        (status = 200, description = "Preview plugin installation")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn preview_install_plugin(
    Json(payload): Json<PluginInstallRequest>,
) -> Result<ApiResponse<PluginImpactPreview>> {
    let source_path = PathBuf::from(payload.source_path);
    Ok(ApiResponse::success(preview_plugin_impact(
        "install",
        PLUGIN_CONTEXT,
        &source_path,
        None,
    )?))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/install",
    request_body = PluginInstallRequest,
    responses(
        (status = 200, description = "Install a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn install_plugin(
    Json(payload): Json<PluginInstallRequest>,
) -> Result<ApiResponse<PluginRecord>> {
    let source_path = PathBuf::from(payload.source_path);
    ensure_plugin_impact_allowed("install", PLUGIN_CONTEXT, &source_path, None)?;
    let record =
        unified_plugin_service().install_from_dir(source_path, payload.enable.unwrap_or(false))?;
    Ok(ApiResponse::success(record))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/preview-upload",
    responses(
        (status = 200, description = "Preview uploaded plugin installation")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn preview_upload_install_plugin(
    multipart: Multipart,
) -> Result<ApiResponse<PluginImpactPreview>> {
    lifecycle::preview_upload(
        multipart,
        ManifestCompatibility::PluginJsonOnly,
        "Plugin",
        |source_path| preview_plugin_impact("install", PLUGIN_CONTEXT, source_path, None),
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/upload",
    responses(
        (status = 200, description = "Upload and install a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn upload_install_plugin(multipart: Multipart) -> Result<ApiResponse<PluginRecord>> {
    lifecycle::upload_install_with_preflight(
        multipart,
        ManifestCompatibility::PluginJsonOnly,
        "Plugin",
        |source_path| ensure_plugin_impact_allowed("install", PLUGIN_CONTEXT, source_path, None),
        |source_path, enable| {
            let record = unified_plugin_service().install_from_dir(source_path, enable)?;
            Ok(record)
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/plugins/{id}/versions",
    responses(
        (status = 200, description = "List archived versions for a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn list_plugin_versions(
    AxumPath(id): AxumPath<String>,
) -> Result<ApiResponse<Vec<PluginVersionRecord>>> {
    Ok(ApiResponse::success(
        unified_plugin_service().list_versions(&id)?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/preview-update",
    request_body = PluginUpdateRequest,
    responses(
        (status = 200, description = "Preview plugin update impact")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn preview_update_plugin(
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<PluginUpdateRequest>,
) -> Result<ApiResponse<PluginImpactPreview>> {
    Ok(ApiResponse::success(preview_plugin_impact(
        "update",
        PLUGIN_CONTEXT,
        Path::new(&payload.source_path),
        Some(&id),
    )?))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/update",
    request_body = PluginUpdateRequest,
    responses(
        (status = 200, description = "Update a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn update_plugin(
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<PluginUpdateRequest>,
) -> Result<ApiResponse<PluginRecord>> {
    let source_path = PathBuf::from(payload.source_path);
    ensure_plugin_impact_allowed("update", PLUGIN_CONTEXT, &source_path, Some(&id))?;
    let service = unified_plugin_service();
    Ok(ApiResponse::success(
        service.update_from_dir_async(&id, source_path).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/preview-upload-update",
    responses(
        (status = 200, description = "Preview uploaded plugin update impact")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn preview_upload_update_plugin(
    AxumPath(id): AxumPath<String>,
    multipart: Multipart,
) -> Result<ApiResponse<PluginImpactPreview>> {
    lifecycle::preview_upload(
        multipart,
        ManifestCompatibility::PluginJsonOnly,
        "Plugin",
        |source_path| preview_plugin_impact("update", PLUGIN_CONTEXT, source_path, Some(&id)),
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/upload-update",
    responses(
        (status = 200, description = "Upload and update a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn upload_update_plugin(
    AxumPath(id): AxumPath<String>,
    multipart: Multipart,
) -> Result<ApiResponse<PluginRecord>> {
    let preview_id = id.clone();
    let service = unified_plugin_service();
    lifecycle::upload_update_with_preflight(
        id,
        multipart,
        ManifestCompatibility::PluginJsonOnly,
        "Plugin",
        move |source_path| {
            ensure_plugin_impact_allowed("update", PLUGIN_CONTEXT, source_path, Some(&preview_id))
        },
        move |id, source_path| async move { service.update_from_dir_async(&id, source_path).await },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/rollback/{version_id}",
    responses(
        (status = 200, description = "Roll back a plugin to an archived version")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn rollback_plugin(
    AxumPath((id, version_id)): AxumPath<(String, String)>,
) -> Result<ApiResponse<PluginRecord>> {
    Ok(ApiResponse::success(
        unified_plugin_service()
            .rollback_async(&id, &version_id)
            .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/enable",
    responses(
        (status = 200, description = "Enable a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn enable_plugin(AxumPath(id): AxumPath<String>) -> Result<ApiResponse<PluginRecord>> {
    ensure_plugin_enable_allowed(&id)?;
    Ok(ApiResponse::success(
        unified_plugin_service()
            .set_enabled_async(&id, true)
            .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/disable",
    responses(
        (status = 200, description = "Disable a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn disable_plugin(AxumPath(id): AxumPath<String>) -> Result<ApiResponse<PluginRecord>> {
    Ok(ApiResponse::success(
        unified_plugin_service()
            .set_enabled_async(&id, false)
            .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/plugins/{id}",
    responses(
        (status = 200, description = "Uninstall a plugin")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn uninstall_plugin(AxumPath(id): AxumPath<String>) -> Result<ApiResponse<()>> {
    unified_plugin_service().uninstall_async(&id).await?;
    Ok(ApiResponse::success(()))
}
