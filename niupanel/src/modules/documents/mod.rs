use crate::modules::audit;
use crate::modules::auth;
use crate::modules::compiler;
use crate::modules::environment;
use crate::modules::file_manager;
use crate::modules::git_sync;
use crate::modules::jobs;
use crate::modules::mcp;
use crate::modules::overview;
use crate::modules::plugins;
use crate::modules::settings;
use crate::modules::share;
use crate::modules::system;
use crate::modules::system_key;
use crate::modules::tasks;
use crate::modules::telegram;
use crate::modules::terminal;
use crate::modules::user;
use crate::modules::variable;
use crate::modules::webhook;
use utoipa::OpenApi;

pub mod routes;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "NiuPanel API Documentation",
        version = env!("CARGO_PKG_VERSION"),
        description = r#"
# NiuPanel API

Complete REST API documentation for NiuPanel - A task management panel.

## Authentication

All internal APIs require session-based authentication via cookies.
Open APIs and MCP use API Key authentication via the `X-API-Key` header.

## Base URL

- Internal API: `/api/v1`
- Open API: `/open/api`

## Response Format

All responses follow the standard format:
```json
{
  "code": 0,
  "message": "Success",
  "data": { ... }
}
```

## Error Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 400 | Validation Error |
| 5000 | Internal Error |
| 5001 | Database Error |
| 5015 | Concurrency Limit |
"#
    ),
    tags(
        (name = "Auth", description = "Authentication and session management"),
        (name = "User", description = "User profile and preferences"),
        (name = "Tasks", description = "Task management and scheduling"),
        (name = "Variables", description = "Environment variables management"),
        (name = "Audit", description = "Audit logs"),
        (name = "Jobs", description = "System background jobs"),
        (name = "Environments", description = "Runtimes and environments"),
        (name = "Settings", description = "System settings, maintenance, updates and notifications"),
        (name = "Overview", description = "System overview and metrics"),
        (name = "API Keys", description = "API key management"),
        (name = "Files", description = "File manager operations"),
        (name = "Share", description = "Task sharing and import/export"),
        (name = "Compiler", description = "Code compilation and encryption"),
        (name = "Webhook", description = "Webhook push notifications"),
        (name = "Git Sync", description = "Git repository synchronization"),
        (name = "Telegram", description = "Telegram bot management, workflows and commands"),
        (name = "Terminal", description = "WebSocket terminal session"),
        (name = "Plugins", description = "Plugin installation, lifecycle and extension management"),
        (name = "MCP", description = "NiuPanel Model Context Protocol server"),
        (name = "System", description = "Core, Web and schema version contract")
    ),
    paths(
        auth::handlers::login,
        auth::handlers::logout,
        auth::handlers::identify_reset,
        auth::handlers::forgot_password,
        auth::handlers::verify_reset_code,
        auth::handlers::reset_password,
        auth::handlers::verify_login_2fa,
        auth::handlers::get_setup_status,
        auth::handlers::register_admin,
        user::handlers::get_my_profile,
        user::handlers::update_profile,
        user::handlers::change_password,
        user::handlers::update_preferences,
        user::handlers::request_email_change,
        user::handlers::confirm_email_change
    ),
    components(
        schemas(
            auth::handlers::LoginRequest,
            auth::handlers::LoginResponse,
            auth::handlers::UserInfo,
            auth::handlers::IdentifyRequest,
            auth::handlers::ResetInfoResponse,
            auth::handlers::ForgotPasswordRequest,
            auth::handlers::VerifyCodeRequest,
            auth::handlers::ResetPasswordRequest,
            auth::handlers::VerifyLogin2faRequest,
            auth::handlers::SetupStatus,
            auth::handlers::RegisterRequest,
            user::handlers::UserProfileDto,
            user::handlers::UpdateProfileRequest,
            user::handlers::ChangePasswordRequest,
            user::handlers::UpdatePreferencesRequest,
            user::handlers::EmailChangeRequest,
            user::handlers::ConfirmEmailChangeRequest,
            niupanel_entity::users::Model
        )
    )
)]
pub struct ApiDocAuth;

#[derive(OpenApi)]
#[openapi(
    paths(
        tasks::handlers::list_tasks,
        tasks::handlers::create_task,
        tasks::handlers::update_task,
        tasks::handlers::stream_logs,
        tasks::handlers::stream_task_run_logs,
        tasks::handlers::get_latest_log,
        tasks::handlers::list_task_history,
        tasks::handlers::batch_delete_tasks,
        tasks::handlers::batch_run_tasks,
        tasks::handlers::batch_stop_tasks,
        tasks::handlers::batch_enable_tasks,
        tasks::handlers::batch_disable_tasks,
        tasks::handlers::save_pagination_setting,
        tasks::handlers::get_pagination_setting,
        tasks::handlers::batch_pin_tasks,
        tasks::handlers::batch_unpin_tasks,
        tasks::handlers::batch_pause_tasks,
        tasks::handlers::batch_resume_tasks,
        tasks::handlers::get_task_run_log,
        tasks::handlers::delete_task_run,
        tasks::handlers::stream_status,
        tasks::handlers::quick_create_task_from_url,
        variable::handlers::list_variables,
        variable::handlers::create_variable,
        variable::handlers::update_variable,
        variable::handlers::batch_delete_variables,
        variable::handlers::list_tasks_simple,
        variable::handlers::batch_toggle_variables,
        variable::handlers::import_variables,
        variable::handlers::get_variable_by_key,
        variable::handlers::update_variable_by_key,
        audit::handlers::list_audit_logs,
        jobs::handlers::list_jobs,
        jobs::handlers::get_job,
        jobs::handlers::cancel_job,
        jobs::handlers::stream_job_logs,
        jobs::handlers::get_latest_job_log
    ),
    components(schemas(
        niupanel_entity::tasks::Model,
        niupanel_entity::variables::Model,
        niupanel_entity::audit_logs::Model,
        niupanel_entity::system_jobs::Model,
        tasks::models::CreateTaskRequest,
        tasks::models::UpdateTaskRequest,
        tasks::models::TaskVariable,
        tasks::models::QueryParams,
        tasks::models::BatchIdRequest,
        tasks::models::LogResponse,
        tasks::models::PaginationSettingRequest,
        tasks::models::QuickCreateUrlRequest,
        variable::handlers::VariableRequest,
        variable::handlers::VariableDto,
        variable::handlers::VariableQueryParams,
        variable::handlers::BatchIdRequest,
        variable::handlers::BatchToggleRequest,
        variable::handlers::KeyQuery,
        audit::handlers::AuditQueryParams
    ))
)]
pub struct ApiDocTasks;

#[derive(OpenApi)]
#[openapi(
    paths(
        settings::handlers::basic::get_settings,
        settings::handlers::basic::get_active_sessions,
        settings::handlers::basic::delete_session,
        settings::handlers::basic::update_general_settings,
        settings::handlers::basic::update_security_settings,
        settings::handlers::basic::get_onboarding_status,
        settings::handlers::basic::complete_onboarding,
        settings::handlers::maintenance::get_version,
        settings::handlers::maintenance::backup_system,
        settings::handlers::maintenance::get_maintenance_status,
        settings::handlers::maintenance::download_backup,
        settings::handlers::maintenance::list_backups,
        settings::handlers::maintenance::delete_backup,
        settings::handlers::maintenance::restore_backup,
        settings::handlers::maintenance::restore_from_local,
        settings::handlers::maintenance::cleanup_logs,
        settings::handlers::update::check_update,
        settings::handlers::update::get_update_status,
        settings::handlers::update::cancel_update,
        settings::handlers::update::execute_update,
        settings::handlers::update::upload_update,
        settings::handlers::notification::update_notification_settings,
        settings::handlers::notification::test_notification,
        environment::handlers::list_environments,
        environment::handlers::create_python_env,
        environment::handlers::delete_python_env,
        environment::handlers::install_python_packages,
        environment::handlers::uninstall_python_package,
        environment::handlers::create_node_env,
        environment::handlers::delete_node_env,
        environment::handlers::list_available_versions,
        environment::handlers::list_python_packages,
        environment::handlers::list_node_packages,
        environment::handlers::install_node_packages,
        environment::handlers::uninstall_node_package,
        environment::handlers::set_node_default,
        environment::handlers::list_shell_packages,
        environment::handlers::install_shell_packages,
        environment::handlers::uninstall_shell_package,
        environment::handlers::set_mirror_source,
        overview::handlers::events_stream,
        overview::handlers::get_overview,
        system_key::handlers::list_api_keys,
        system_key::handlers::create_api_key,
        system_key::handlers::update_api_key,
        system_key::handlers::delete_api_key,
        plugins::handlers::list_plugins,
        plugins::handlers::preview_install_plugin,
        plugins::handlers::install_plugin,
        plugins::handlers::preview_upload_install_plugin,
        plugins::handlers::upload_install_plugin,
        plugins::handlers::list_plugin_versions,
        plugins::handlers::preview_update_plugin,
        plugins::handlers::update_plugin,
        plugins::handlers::preview_upload_update_plugin,
        plugins::handlers::upload_update_plugin,
        plugins::handlers::rollback_plugin,
        plugins::handlers::enable_plugin,
        plugins::handlers::disable_plugin,
        plugins::handlers::uninstall_plugin,
        plugins::handlers::list_plugin_apps,
        plugins::handlers::list_plugin_themes,
        plugins::handlers::list_plugin_health,
        plugins::handlers::proxy_plugin_api_request,
        plugins::handlers::get_plugin_market,
        plugins::handlers::list_plugin_market_sources,
        plugins::handlers::update_plugin_market_sources,
        plugins::handlers::check_plugin_market_updates,
        plugins::handlers::preview_plugin_from_market,
        plugins::handlers::install_plugin_from_market,
        plugins::handlers::invoke_agent_plugin
    ),
    components(schemas(
        niupanel_entity::environments::Model,
        settings::models::SettingDto,
        settings::models::GeneralSettingsRequest,
        settings::models::SecuritySettingsRequest,
        settings::models::SessionDto,
        settings::models::BackupOptions,
        settings::models::TestNotifyRequest,
        settings::models::CleanupLogsRequest,
        settings::models::NotificationSettingsRequest,
        environment::handlers::EnvironmentInfo,
        environment::handlers::CreateEnvRequest,
        environment::handlers::InstallPackageRequest,
        environment::handlers::SetMirrorSourceRequest,
        overview::handlers::SystemOverview,
        overview::handlers::TaskStats,
        overview::handlers::ActivityItem,
        overview::handlers::ChartData,
        system_key::handlers::ApiKeyDto,
        system_key::handlers::CreateApiKeyRequest,
        system_key::handlers::UpdateApiKeyRequest,
        system_key::handlers::CreateApiKeyResponse,
        plugins::models::PluginInstallRequest,
        plugins::models::PluginUpdateRequest,
        plugins::models::PluginMarketQuery,
        plugins::models::PluginMarketInstallRequest,
        plugins::models::PluginMarketSourcesUpdateRequest,
        plugins::models::PluginMarketSource,
        plugins::models::PluginMarketUpdateRecord,
        plugins::models::PluginMarketIndex,
        plugins::models::PluginMarketEntry,
        plugins::models::PluginApiProxyRequest,
        plugins::models::PluginAppRecord,
        plugins::models::PluginAppUi,
        plugins::models::PluginThemeRecord,
        plugins::models::PluginImpactPreview,
        plugins::models::PluginImpactRoute,
        plugins::models::PluginRouteConflict,
        plugins::models::PluginHealthReport,
        plugins::models::PluginHealthCheck,
        niupanel_plugin::PluginManifest,
        niupanel_plugin::PluginRuntime,
        niupanel_plugin::PluginRuntimePermission,
        niupanel_plugin::PluginProcessProtocol,
        niupanel_plugin::PluginCompatibilityManifest,
        niupanel_plugin::PluginPanelCompatibility,
        niupanel_plugin::PluginDependency,
        niupanel_plugin::PluginUiManifest,
        niupanel_plugin::PluginUiDisplayManifest,
        niupanel_plugin::PluginUiLayout,
        niupanel_plugin::PluginUiApiManifest,
        niupanel_plugin::PluginUiApiRule,
        niupanel_plugin::PluginUiMode,
        niupanel_plugin::PluginUiRoute,
        niupanel_plugin::PluginThemeManifest,
        niupanel_plugin::PluginThemePalette,
        niupanel_plugin::PluginWorkerConfig,
        niupanel_plugin::PluginState,
        niupanel_plugin::PluginVersionRecord,
        niupanel_plugin::PluginRecord,
        niupanel_plugin::PluginSource,
        niupanel_plugin::PluginStatus,
        niupanel_plugin::PluginInvokeRequest,
        niupanel_plugin::PluginInvokeResponse,
        niupanel_plugin::ProcessPluginRequest,
        niupanel_plugin::ProcessPluginToolCall,
        niupanel_plugin::ProcessPluginToolResult,
        niupanel_plugin::ProcessPluginResponse,
        niupanel_plugin::ProcessPluginError
    ))
)]
pub struct ApiDocSettings;

#[derive(OpenApi)]
#[openapi(
    paths(mcp::handlers::get_info),
    components(schemas(mcp::models::McpInfoResponse, mcp::models::McpToolInfo))
)]
pub struct ApiDocMcp;

#[derive(OpenApi)]
#[openapi(
    paths(
        system::handlers::get_system_meta,
        system::handlers::list_core_releases,
        system::handlers::activate_core_release,
        system::handlers::list_web_releases,
        system::handlers::upload_web_release,
        system::handlers::activate_web_release,
        system::handlers::rollback_web_release,
        system::handlers::check_web_update,
        system::handlers::install_web_update
    ),
    components(schemas(
        system::models::SystemVersionInfo,
        system::web_releases::WebReleaseList,
        system::web_releases::WebReleaseRecord,
        system::web_releases::WebReleaseMutation,
        system::core_releases::CoreReleaseList,
        system::core_releases::CoreReleaseRecord,
        system::core_releases::CoreReleaseMutation,
        system::core_releases::ActivateCoreReleaseRequest,
        niupanel_common::version::CoreActivationFailure,
        niupanel_common::version::WebReleaseManifest,
        niupanel_common::version::ComponentCompatibility
    ))
)]
pub struct ApiDocSystem;

#[derive(OpenApi)]
#[openapi(
    paths(
        file_manager::handlers::list_directory_root,
        file_manager::handlers::list_directory_contents,
        file_manager::handlers::read_file_content,
        file_manager::handlers::write_file_content,
        file_manager::handlers::create_file,
        file_manager::handlers::create_directory,
        file_manager::handlers::delete_item,
        file_manager::handlers::rename_item,
        file_manager::handlers::copy_item,
        file_manager::handlers::extract_archive,
        file_manager::handlers::delete_root_item,
        file_manager::handlers::download_file,
        file_manager::handlers::upload_file,
        file_manager::handlers::upload_file_root,
        file_manager::handlers::download_from_url,
        file_manager::handlers::download_batch,
        compiler::handlers::get_supported_versions,
        compiler::handlers::encrypt_code,
        webhook::handlers::send_notification
    ),
    components(schemas(
        file_manager::handlers::FileSystemItem,
        file_manager::handlers::CreateFileRequest,
        file_manager::handlers::CreateDirectoryRequest,
        file_manager::handlers::WriteFileRequest,
        file_manager::handlers::RenameItemRequest,
        file_manager::handlers::CopyItemRequest,
        file_manager::handlers::ExtractArchiveRequest,
        file_manager::handlers::DownloadUrlRequest,
        file_manager::handlers::DownloadBatchRequest,
        webhook::handlers::WebhookPushRequest,
        compiler::models::EncryptCodeRequest
    ))
)]
pub struct ApiDocFiles;

#[derive(OpenApi)]
#[openapi(
    paths(
        share::handlers::get_task_file_tree,
        share::handlers::create_share,
        share::handlers::submit_import,
        share::handlers::retry_import,
        share::handlers::get_import_status
    ),
    components(schemas(
        share::models::FileNode,
        share::models::TaskFileTreeData,
        share::models::TaskFileMapping,
        share::models::CreateShareRequest,
        share::models::TaskExportSpec,
        share::models::FileAssociations,
        share::models::CreateShareResponse,
        share::models::SubmitImportRequest,
        share::models::SubmitImportResponse,
        share::models::ImportStatusResponse,
        share::models::ImportState
    ))
)]
pub struct ApiDocShareExport;

#[derive(OpenApi)]
#[openapi(
    paths(
        share::handlers::get_import_preview,
        share::handlers::confirm_import,
        share::handlers::delete_imported_tasks,
        share::handlers::list_imported_sources,
        share::handlers::list_station_files,
        share::handlers::get_station_stats,
        share::handlers::save_station_config,
        share::handlers::update_station_file,
        share::handlers::update_station_content,
        share::handlers::delete_station_file
    ),
    components(schemas(
        share::models::ConfirmImportRequest,
        share::models::ImportSummary,
        share::models::ImportFailure,
        share::models::ImportSourceGroup,
        share::models::ImportHistoryItem,
        share::models::DeleteImportQuery,
        share::models::ShareVariable,
        share::models::StationFile,
        share::models::StationStats,
        share::models::StationConfigPayload,
        share::models::UpdateStationFileRequest
    ))
)]
pub struct ApiDocShareImport;

#[derive(OpenApi)]
#[openapi(
    paths(
        git_sync::handlers::list_repos,
        git_sync::handlers::list_repo_files,
        git_sync::handlers::scan_repo_tasks,
        git_sync::handlers::import_tasks,
        git_sync::handlers::create_repo,
        git_sync::handlers::update_repo,
        git_sync::handlers::delete_repo,
        git_sync::handlers::sync_repo,
        telegram::handlers::get_telegram_config,
        telegram::handlers::update_telegram_config,
        telegram::handlers::get_latency,
        telegram::handlers::get_bot_metrics,
        telegram::handlers::send_message,
        telegram::handlers::send_server_file,
        telegram::handlers::send_file,
        telegram::handlers::test_telegram,
        telegram::workflows_api::list_workflows,
        telegram::workflows_api::create_workflow,
        telegram::workflows_api::update_workflow,
        telegram::workflows_api::delete_workflow,
        telegram::commands_api::list_commands,
        telegram::commands_api::create_command,
        telegram::commands_api::update_command,
        telegram::commands_api::delete_command,
        terminal::handlers::terminal_ws_handler
    ),
    components(schemas(
        git_sync::models::GitRepoRequest,
        git_sync::models::SyncResult,
        git_sync::models::FileEntry,
        git_sync::models::DiscoveredTask,
        git_sync::models::ImportTasksRequest,
        telegram::handlers::SendMessageRequest,
        telegram::handlers::SendServerFileRequest,
        telegram::handlers::TelegramConfigSchema,
        telegram::workflows_api::QueryParams,
        telegram::workflows_api::CreateWorkflowReq,
        telegram::workflows_api::WorkflowDto,
        telegram::commands_api::CreateCmdReq,
        telegram::commands_api::CommandDto,
        terminal::handlers::TerminalMessage
    ))
)]
pub struct ApiDocPlugins;
