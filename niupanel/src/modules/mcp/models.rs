use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

mod audit;
mod catalog;
mod environments;
mod files;
mod git;
mod jobs;
mod notifications;
mod share;
mod system;
mod tasks;
mod variables;

pub use audit::*;
pub use catalog::*;
pub use environments::*;
pub use files::*;
pub use git::*;
pub use jobs::*;
pub use notifications::*;
pub use share::*;
pub use system::*;
pub use tasks::*;
pub use variables::*;

#[cfg(test)]
mod tests {
    use super::McpInfoResponse;
    use std::collections::HashSet;

    #[test]
    fn advertised_tools_are_unique_and_permission_scoped() {
        let info = McpInfoResponse::current();
        let names = info
            .tools
            .iter()
            .map(|tool| tool.name)
            .collect::<HashSet<_>>();

        assert_eq!(names.len(), info.tools.len());
        assert_eq!(info.tools.len(), 49);
        assert!(info.tools.iter().all(|tool| !tool.permission.is_empty()));
        assert!(names.contains("tasks_history"));
        assert!(names.contains("tasks_get_run_log"));
        assert!(names.contains("environments_list"));
        assert!(names.contains("environments_create"));
        assert!(names.contains("environments_install_packages"));
        assert!(names.contains("environments_delete"));
        assert!(names.contains("tasks_delete"));
        assert!(names.contains("variables_get"));
        assert!(names.contains("audit_list"));
        assert!(names.contains("files_read"));
        assert!(names.contains("jobs_cancel"));
        assert!(names.contains("webhook_push"));
        assert!(names.contains("share_station_stats"));
        assert!(names.contains("git_repo_sync"));
        assert!(names.contains("system_releases"));
        assert!(names.contains("system_update_check"));
    }
}
