pub use sea_orm_migration::prelude::*;

mod m;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m::m20251207_000001_init::Migration),
            Box::new(m::m20251209_000001_add_shares::Migration),
            Box::new(m::m20251210_000001_create_settings::Migration),
            Box::new(m::m20251211_000001_add_resource_limits::Migration),
            Box::new(m::m20251217_000001_add_is_pinned_to_tasks::Migration),
            Box::new(m::m20251221_000002_auth_refactor::Migration),
            Box::new(m::m20251221_000003_add_audit_and_tracking::Migration),
            Box::new(m::m20251222_000001_role_migration::Migration),
            Box::new(m::m20251222_000002_add_email_and_resets::Migration),
            Box::new(m::m20251223_000001_auth_enhancements::Migration),
            Box::new(m::m20251223_000002_fix_user_columns::Migration),
            Box::new(m::m20251227_000001_multi_task_variables::Migration),
            Box::new(m::m20251230_000001_add_trigger_next_tasks::Migration),
            Box::new(m::m20251230_000002_create_git_repositories::Migration),
            Box::new(m::m20260102_000001_create_station_files::Migration),
            Box::new(m::m20260103_000001_share_update_features::Migration),
            Box::new(m::m20260213_000001_create_environments::Migration),
            Box::new(m::m20260228_000001_create_tg_tables::Migration),
            Box::new(m::m20260303_000001_clean_legacy_system_node::Migration),
            Box::new(m::m20260306_000001_clean_legacy_system_environments::Migration),
            Box::new(m::m20260317_000001_add_remote_task_id::Migration),
            Box::new(m::m20260418_000001_add_task_random_config::Migration),
            Box::new(m::m20260418_000002_create_market_sources::Migration),
            Box::new(m::m20260425_000001_add_sort_order_to_variables::Migration),
            Box::new(m::m20260425_000002_add_sort_order_to_task_variables::Migration),
            Box::new(m::m20260501_000001_add_timeout_sec_to_tasks::Migration),
            Box::new(m::m20260618_000001_create_agent_sessions::Migration),
            Box::new(m::m20260619_000001_create_agent_memories::Migration),
            Box::new(m::m20260619_000002_create_agent_memory_events::Migration),
            Box::new(m::m20260728_000001_reconcile_variable_bindings::Migration),
        ]
    }
}
