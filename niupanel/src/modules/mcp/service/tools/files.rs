use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = file_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "List files and directories under the configured scripts directory")]
    async fn files_list(
        &self,
        Parameters(params): Parameters<FileListParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<FileListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::FileList)?;
        let path = params.path.unwrap_or_default();
        let items = FileManagerService::list_directory(path.clone(), params.query)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "files_list", Some(path)).await;
        Ok(Json(FileListOutput {
            items: items
                .into_iter()
                .map(|item| FileItemOutput {
                    name: item.name,
                    path: item.path,
                    size: item.size,
                    is_directory: item.is_dir,
                    modified_at_unix: item.mtime,
                })
                .collect(),
        }))
    }

    #[tool(description = "Read a UTF-8 script file up to 2 MiB")]
    async fn files_read(
        &self,
        Parameters(FilePathParams { path }): Parameters<FilePathParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<FileContentOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::FileRead)?;
        let content = FileManagerService::read_file_content(path.clone())
            .await
            .map_err(tool_error)?;
        self.audit(&user, "files_read", Some(path.clone())).await;
        Ok(Json(FileContentOutput { path, content }))
    }

    #[tool(description = "Write a UTF-8 script file up to 2 MiB")]
    async fn files_write(
        &self,
        Parameters(params): Parameters<FileWriteParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<FileActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::FileWrite)?;
        if params.content.len() > 2 * 1024 * 1024 {
            return Err(ErrorData::invalid_params(
                "File content exceeds the 2 MiB MCP limit",
                None,
            ));
        }
        let path = params.path;
        FileManagerService::write_file_content(WriteFileRequest {
            path: path.clone(),
            content: params.content,
        })
        .await
        .map_err(tool_error)?;
        self.audit(&user, "files_write", Some(path.clone())).await;
        Ok(Json(FileActionOutput {
            path,
            accepted: true,
            message: "File written".to_string(),
        }))
    }

    #[tool(description = "Create a directory under the configured scripts directory")]
    async fn files_create_directory(
        &self,
        Parameters(FilePathParams { path }): Parameters<FilePathParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<FileActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::FileWrite)?;
        FileManagerService::create_directory(CreateDirectoryRequest { path: path.clone() })
            .await
            .map_err(tool_error)?;
        self.audit(&user, "files_create_directory", Some(path.clone()))
            .await;
        Ok(Json(FileActionOutput {
            path,
            accepted: true,
            message: "Directory created".to_string(),
        }))
    }

    #[tool(description = "Delete a file or directory. This is a destructive operation")]
    async fn files_delete(
        &self,
        Parameters(FilePathParams { path }): Parameters<FilePathParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<FileActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::FileDelete)?;
        FileManagerService::delete_item(path.clone())
            .await
            .map_err(tool_error)?;
        self.audit(&user, "files_delete", Some(path.clone())).await;
        Ok(Json(FileActionOutput {
            path,
            accepted: true,
            message: "File or directory deleted".to_string(),
        }))
    }
}
