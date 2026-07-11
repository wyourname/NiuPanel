use chrono::Utc;
use niupanel_common::auth::permissions::AuthenticatedUser;
use niupanel_common::error;
use niupanel_entity::audit_logs;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

pub struct AuditService;

impl AuditService {
    /// 通用的底层日志记录方法
    pub async fn log(
        db: &DatabaseConnection,
        user_id: Option<i32>,
        actor_type: &str,
        action: &str,
        resource: &str,
        resource_id: Option<String>,
        details: Option<String>,
        ip_address: Option<String>,
    ) {
        let log = audit_logs::ActiveModel {
            user_id: Set(user_id),
            actor_type: Set(actor_type.to_string()),
            action: Set(action.to_string()),
            resource: Set(resource.to_string()),
            resource_id: Set(resource_id),
            details: Set(details),
            ip_address: Set(ip_address),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        };

        if let Err(e) = log.insert(db).await {
            error!("panic!插入审计日志失败: {}", e);
        }
    }

    /// 简化的用户行为日志记录方法
    pub async fn log_user(
        db: &DatabaseConnection,
        user: &AuthenticatedUser,
        action: &str,
        resource: &str,
        resource_id: Option<String>,
        details: Option<String>,
    ) {
        use niupanel_common::auth::permissions::UserRole;
        let actor_type = match user.role {
            UserRole::System => "Key",
            UserRole::ApiClient => "APIKey",
            _ => "User",
        };

        Self::log(
            db,
            Some(user.id),
            actor_type,
            action,
            resource,
            resource_id,
            details,
            Some(user.ip.clone()),
        )
        .await;
    }
}
