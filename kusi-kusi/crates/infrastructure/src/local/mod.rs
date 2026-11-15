mod academic;
mod adaptive;
mod assets;
mod auth;
mod bureaucratic_cases;
mod connection_settings;
mod conversations;
mod publications;
mod resource_access;
mod worker;
pub use academic::{DemoStudent, GuardianStudentLink};
pub use auth::SessionUser;
use kusi_application::local::LocalError;
pub use resource_access::{AccessibleResource, ResourceGrantRow};
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;
pub use worker::run_worker;

#[derive(Clone)]
pub struct LocalStore {
    pub(crate) pool: PgPool,
    pub(crate) root: PathBuf,
    pub(crate) dummy_hash: String,
    pub(crate) connection_vault: std::sync::Arc<crate::oauth::TokenVault>,
}
impl LocalStore {
    pub async fn new(pool: PgPool, root: PathBuf) -> Result<Self, LocalError> {
        tokio::fs::create_dir_all(root.join("assets"))
            .await
            .map_err(|_| LocalError::Unavailable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
                .await
                .map_err(|_| LocalError::Unavailable)?;
        }
        let dummy_hash = auth::password_hash("not-a-real-user-password".into()).await?;
        let connection_vault = connection_settings::load_vault(&pool, &root).await?;
        Ok(Self {
            pool,
            root,
            dummy_hash,
            connection_vault,
        })
    }
    pub async fn rate_limit(&self, key: &str, limit: i32, window: i64) -> Result<(), LocalError> {
        let attempts:i32=sqlx::query_scalar("INSERT INTO local_rate_limits (key_hash) VALUES ($1) ON CONFLICT (key_hash) DO UPDATE SET attempts=CASE WHEN local_rate_limits.window_start<now()-$2::bigint*interval '1 second' THEN 1 ELSE local_rate_limits.attempts+1 END,window_start=CASE WHEN local_rate_limits.window_start<now()-$2::bigint*interval '1 second' THEN now() ELSE local_rate_limits.window_start END RETURNING attempts").bind(kusi_application::hash_api_key(key)).bind(window).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        if attempts > limit {
            Err(LocalError::RateLimited)
        } else {
            Ok(())
        }
    }
    pub async fn audit(
        &self,
        org: Uuid,
        user: Option<Uuid>,
        action: &str,
        target: &str,
    ) -> Result<(), LocalError> {
        sqlx::query(
            "INSERT INTO local_audit (organization_id,user_id,action,target) VALUES ($1,$2,$3,$4)",
        )
        .bind(org)
        .bind(user)
        .bind(action)
        .bind(target)
        .execute(&self.pool)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        Ok(())
    }
}
