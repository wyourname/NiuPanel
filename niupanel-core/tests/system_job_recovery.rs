use niupanel_common::config::{CONFIG, Config};
use niupanel_core::{event_bus::EventBus, task_manager::service::TaskManagerService};
use niupanel_entity::{settings, system_jobs, task_status::TaskStatus};
use sea_orm::{ConnectionTrait, Database, EntityTrait, Schema};
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn package_job_metadata_survives_reopening_database_and_completion() {
    let root = tempfile::tempdir().unwrap();
    // This integration test has its own process and never uses development paths.
    let config: Config = serde_json::from_value(json!({
        "jobs_dir": root.path().join("jobs"),
        "logs_dir": root.path().join("logs")
    }))
    .unwrap();
    CONFIG.set(config).unwrap();
    let url = format!(
        "sqlite://{}?mode=rwc",
        root.path().join("jobs.db").display()
    );
    let db = Database::connect(&url).await.unwrap();
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    for table in [
        schema.create_table_from_entity(settings::Entity),
        schema.create_table_from_entity(system_jobs::Entity),
    ] {
        db.execute(&table).await.unwrap();
    }
    let manager = TaskManagerService::new(db.clone(), EventBus::new())
        .await
        .unwrap();
    let metadata = json!({"kind":"environment-packages","env_type":"python","env_name":"3.12","operation":"install","packages":["requests"]});
    let (release, wait) = tokio::sync::oneshot::channel();
    let id = manager
        .submit_system_task_with_metadata(
            "install test".into(),
            Some(metadata.clone()),
            move |logs| async move {
                wait.await.unwrap();
                logs.send("安装完成".into()).unwrap();
                Ok(())
            },
        )
        .await
        .unwrap();
    let reopened = Database::connect(&url).await.unwrap();
    let running = system_jobs::Entity::find_by_id(id)
        .one(&reopened)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running.metadata, Some(metadata.clone()));
    assert_eq!(running.status, TaskStatus::Running);
    release.send(()).unwrap();
    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let job = system_jobs::Entity::find_by_id(id)
                .one(&reopened)
                .await
                .unwrap()
                .unwrap();
            if job.status == TaskStatus::Finished {
                break job;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(completed.metadata, Some(metadata));
}
