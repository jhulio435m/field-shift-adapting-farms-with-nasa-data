use super::LocalStore;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use kusi_application::local::LocalError;
use kusi_application::session_management::{
    SessionInventoryItem, SessionManagementError, SessionManagementRepository,
};
use kusi_domain::local::{LocalRole, LocalUser};
use sqlx::FromRow;
use uuid::Uuid;

pub async fn password_hash(password: String) -> Result<String, LocalError> {
    if !(12..=128).contains(&password.len()) {
        return Err(LocalError::Invalid);
    }
    tokio::task::spawn_blocking(move || {
        let hash: argon2::PasswordHash = Argon2::default()
            .hash_password(password.as_bytes())
            .map_err(|_| LocalError::Unavailable)?;
        Ok(hash.to_string())
    })
    .await
    .map_err(|_| LocalError::Unavailable)?
}
async fn verify(password: String, encoded: String) -> Result<bool, LocalError> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .verify_password(password.as_bytes(), encoded.as_str())
            .is_ok()
    })
    .await
    .map_err(|_| LocalError::Unavailable)
}
#[derive(FromRow)]
struct UserRow {
    id: Uuid,
    organization_id: Uuid,
    organization: String,
    username: String,
    display_name: String,
    role: String,
    platform_admin: bool,
    disabled: bool,
}
impl UserRow {
    fn decode(self) -> Result<LocalUser, LocalError> {
        Ok(LocalUser {
            id: self.id,
            organization_id: self.organization_id,
            organization: self.organization,
            username: self.username,
            display_name: self.display_name,
            role: serde_json::from_value(serde_json::Value::String(self.role))
                .map_err(|_| LocalError::Unavailable)?,
            platform_admin: self.platform_admin,
            disabled: self.disabled,
        })
    }
}
pub enum SessionUser {
    Missing,
    Invalid,
    Active(LocalUser),
}
fn valid_username(username: &str) -> bool {
    (3..=64).contains(&username.len())
        && username
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c))
}
fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 80 && !name.chars().any(char::is_control)
}
async fn revoke_user_sessions(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization: Uuid,
    actor: Uuid,
    user: Uuid,
) -> Result<(), LocalError> {
    let sessions = sqlx::query_scalar::<_, Uuid>(
        "UPDATE local_sessions SET revoked_at=now() \
         WHERE user_id=$1 AND revoked_at IS NULL RETURNING id",
    )
    .bind(user)
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| LocalError::Unavailable)?;
    for session_id in sessions {
        sqlx::query(
            "INSERT INTO local_audit(organization_id,user_id,action,target) \
             VALUES($1,$2,'session_revoked',$3)",
        )
        .bind(organization)
        .bind(actor)
        .bind(session_id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
    }
    sqlx::query("UPDATE organization_api_keys SET revoked_at=now() WHERE key_hash IN (SELECT key_hash FROM local_sessions WHERE user_id=$1)")
        .bind(user)
        .execute(&mut **tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
    Ok(())
}
impl LocalStore {
    async fn current_local_session_id(
        &self,
        user_id: Uuid,
        token: &str,
    ) -> Result<Uuid, LocalError> {
        let hash = kusi_application::hash_api_key(token);
        sqlx::query_scalar("SELECT id FROM local_sessions WHERE key_hash=$1 AND user_id=$2")
            .bind(hash)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| LocalError::Unavailable)?
            .ok_or(LocalError::Unauthorized)
    }

    pub async fn active_sessions(
        &self,
        actor: &LocalUser,
        token: &str,
    ) -> Result<Vec<SessionInventoryItem>, LocalError> {
        let current = self.current_local_session_id(actor.id, token).await?;
        kusi_application::session_management::list_sessions(
            &PgSessionManagement(self.pool.clone()),
            actor.organization_id,
            actor.id,
            actor.role.is_owner(),
            current,
        )
        .await
        .map_err(map_session_error)
    }

    pub async fn revoke_local_session(
        &self,
        actor: &LocalUser,
        token: &str,
        target: Uuid,
    ) -> Result<(), LocalError> {
        let current = self.current_local_session_id(actor.id, token).await?;
        kusi_application::session_management::revoke_session(
            &PgSessionManagement(self.pool.clone()),
            actor.organization_id,
            actor.id,
            actor.role.is_owner(),
            current,
            target,
        )
        .await
        .map_err(map_session_error)
    }

    pub async fn login(
        &self,
        org: &str,
        username: &str,
        password: String,
    ) -> Result<(LocalUser, String), LocalError> {
        self.rate_limit("local-login-global", 60, 60).await?;
        self.rate_limit(&format!("login:{org}:{username}"), 8, 300)
            .await?;
        if org.len() > 63 || !valid_username(username) || password.len() > 128 {
            return Err(LocalError::Unauthorized);
        }
        let row=sqlx::query_as::<_,(Uuid,String)>("SELECT u.id,u.password_hash FROM local_users u JOIN organizations o ON o.id=u.organization_id WHERE o.slug=$1 AND u.username=$2 AND NOT u.disabled").bind(org).bind(username).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        let encoded = row
            .as_ref()
            .map(|r| r.1.clone())
            .unwrap_or_else(|| self.dummy_hash.clone());
        let correct = verify(password, encoded).await?;
        if !correct || row.is_none() {
            return Err(LocalError::Unauthorized);
        }
        let user = self.user(row.ok_or(LocalError::Unauthorized)?.0).await?;
        let token = crate::oauth::random_secret().map_err(|_| LocalError::Unavailable)?;
        let hash = kusi_application::hash_api_key(&token);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("INSERT INTO organization_api_keys (organization_id,key_hash,expires_at) VALUES ($1,$2,now()+interval '8 hours')").bind(user.organization_id).bind(&hash).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("INSERT INTO local_sessions (key_hash,user_id,expires_at) VALUES ($1,$2,now()+interval '8 hours')").bind(&hash).bind(user.id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("INSERT INTO local_audit (organization_id,user_id,action,target) VALUES ($1,$2,'login',$3)").bind(user.organization_id).bind(user.id).bind(user.username.clone()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        Ok((user, token))
    }
    pub async fn session(&self, token: &str) -> Result<SessionUser, LocalError> {
        let hash = kusi_application::hash_api_key(token);
        let row=sqlx::query_as::<_,(Uuid,bool)>("SELECT s.user_id, s.revoked_at IS NULL AND s.expires_at>now() AND k.revoked_at IS NULL AND k.expires_at>now() AND NOT u.disabled FROM local_sessions s JOIN organization_api_keys k ON k.key_hash=s.key_hash JOIN local_users u ON u.id=s.user_id WHERE s.key_hash=$1").bind(hash).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        match row {
            None => Ok(SessionUser::Missing),
            Some((_, false)) => Ok(SessionUser::Invalid),
            Some((id, true)) => Ok(SessionUser::Active(self.user(id).await?)),
        }
    }
    pub async fn logout(&self, user: &LocalUser, token: &str) -> Result<(), LocalError> {
        let hash = kusi_application::hash_api_key(token);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let session_id = sqlx::query_scalar::<_, Uuid>(
            "UPDATE local_sessions SET revoked_at=now() \
             WHERE key_hash=$1 AND user_id=$2 AND revoked_at IS NULL RETURNING id",
        )
        .bind(&hash)
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?
        .ok_or(LocalError::Unauthorized)?;
        sqlx::query("UPDATE organization_api_keys SET revoked_at=now() WHERE key_hash=$1 AND organization_id=$2").bind(hash).bind(user.organization_id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query(
            "INSERT INTO local_audit(organization_id,user_id,action,target) \
             VALUES($1,$2,'session_revoked',$3)",
        )
        .bind(user.organization_id)
        .bind(user.id)
        .bind(session_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        Ok(())
    }
    pub async fn user(&self, id: Uuid) -> Result<LocalUser, LocalError> {
        sqlx::query_as::<_,UserRow>("SELECT u.id,u.organization_id,o.slug as organization,u.username,u.display_name,u.role,u.platform_admin,u.disabled FROM local_users u JOIN organizations o ON o.id=u.organization_id WHERE u.id=$1").bind(id).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)?.decode()
    }
    pub async fn users(&self, actor: &LocalUser) -> Result<Vec<LocalUser>, LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        sqlx::query_as::<_,UserRow>("SELECT u.id,u.organization_id,o.slug as organization,u.username,u.display_name,u.role,u.platform_admin,u.disabled FROM local_users u JOIN organizations o ON o.id=u.organization_id WHERE u.organization_id=$1 ORDER BY u.created_at").bind(actor.organization_id).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?.into_iter().map(UserRow::decode).collect()
    }
    pub async fn bootstrap(
        &self,
        org: Uuid,
        username: &str,
        name: &str,
        password: String,
        platform: bool,
    ) -> Result<LocalUser, LocalError> {
        if !valid_username(username) || !valid_name(name) {
            return Err(LocalError::Invalid);
        }
        let hash = password_hash(password).await?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("local-bootstrap:{org}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_users WHERE organization_id=$1)")
                .bind(org)
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| LocalError::Unavailable)?;
        if exists {
            return Err(LocalError::Conflict);
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO local_users (id,organization_id,username,display_name,password_hash,role,platform_admin) VALUES ($1,$2,$3,$4,$5,'owner',$6)").bind(id).bind(org).bind(username).bind(name.trim()).bind(hash).bind(platform).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.audit(org, Some(id), "owner_bootstrapped", username)
            .await?;
        self.user(id).await
    }
    pub async fn create_user(
        &self,
        actor: &LocalUser,
        username: &str,
        name: &str,
        password: String,
        role: LocalRole,
    ) -> Result<LocalUser, LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        if !valid_username(username) || !valid_name(name) {
            return Err(LocalError::Invalid);
        }
        let hash = password_hash(password).await?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO local_users (id,organization_id,username,display_name,password_hash,role) VALUES ($1,$2,$3,$4,$5,$6)").bind(id).bind(actor.organization_id).bind(username).bind(name.trim()).bind(hash).bind(role.name()).execute(&self.pool).await.map_err(|e|if e.as_database_error().is_some_and(|e|e.is_unique_violation()){LocalError::Conflict}else{LocalError::Unavailable})?;
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "user_created",
            &id.to_string(),
        )
        .await?;
        self.user(id).await
    }
    pub async fn update_user(
        &self,
        actor: &LocalUser,
        id: Uuid,
        role: LocalRole,
        disabled: bool,
    ) -> Result<LocalUser, LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('local-user-management',0))")
            .execute(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let previous=sqlx::query_as::<_,(String,bool,bool)>("SELECT role,disabled,platform_admin FROM local_users WHERE organization_id=$1 AND id=$2 FOR UPDATE").bind(actor.organization_id).bind(id).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)?;
        if previous.2 && !actor.platform_admin {
            return Err(LocalError::Forbidden);
        }
        if previous.0 == "owner" && !previous.1 && (disabled || role != LocalRole::Owner) {
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM local_users WHERE organization_id=$1 AND role='owner' AND NOT disabled").bind(actor.organization_id).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            if count <= 1 {
                return Err(LocalError::Conflict);
            }
        }
        if previous.2 && role != LocalRole::Owner {
            return Err(LocalError::Conflict);
        }
        if previous.2 && disabled {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM local_users WHERE platform_admin AND NOT disabled",
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
            if count <= 1 {
                return Err(LocalError::Conflict);
            }
        }
        sqlx::query(
            "UPDATE local_users SET role=$3,disabled=$4 WHERE organization_id=$1 AND id=$2",
        )
        .bind(actor.organization_id)
        .bind(id)
        .bind(role.name())
        .bind(disabled)
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        if disabled {
            revoke_user_sessions(&mut tx, actor.organization_id, actor.id, id).await?;
        }
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "user_updated",
            &id.to_string(),
        )
        .await?;
        self.user(id).await
    }
    pub async fn reset_password(
        &self,
        actor: &LocalUser,
        id: Uuid,
        password: String,
    ) -> Result<(), LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        let target = self.user(id).await?;
        if target.organization_id != actor.organization_id {
            return Err(LocalError::NotFound);
        }
        if target.platform_admin && !actor.platform_admin {
            return Err(LocalError::Forbidden);
        }
        let hash = password_hash(password).await?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("UPDATE local_users SET password_hash=$3 WHERE organization_id=$1 AND id=$2")
            .bind(actor.organization_id)
            .bind(id)
            .bind(hash)
            .execute(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
        revoke_user_sessions(&mut tx, actor.organization_id, actor.id, id).await?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "password_reset",
            &id.to_string(),
        )
        .await
    }
}

#[derive(FromRow)]
struct SessionRow {
    id: Uuid,
    user_id: Uuid,
    display_name: String,
    username: String,
    created_at: i64,
    expires_at: i64,
}

struct PgSessionManagement(sqlx::PgPool);

fn map_session_error(error: SessionManagementError) -> LocalError {
    match error {
        SessionManagementError::NotFound => LocalError::NotFound,
        SessionManagementError::CurrentSession => LocalError::Conflict,
        SessionManagementError::Unavailable => LocalError::Unavailable,
    }
}

#[async_trait::async_trait]
impl SessionManagementRepository for PgSessionManagement {
    async fn list_active(
        &self,
        organization: Uuid,
        user: Uuid,
        owner_scope: bool,
    ) -> Result<Vec<SessionInventoryItem>, SessionManagementError> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT s.id,s.user_id,u.display_name,u.username,\
             extract(epoch FROM s.created_at)::bigint AS created_at,\
             extract(epoch FROM s.expires_at)::bigint AS expires_at \
             FROM local_sessions s \
             JOIN local_users u ON u.id=s.user_id \
             JOIN organization_api_keys k ON k.key_hash=s.key_hash AND k.organization_id=u.organization_id \
             WHERE u.organization_id=$1 AND ($3 OR u.id=$2) \
             AND NOT u.disabled AND s.revoked_at IS NULL AND s.expires_at>now() \
             AND k.revoked_at IS NULL AND k.expires_at>now() \
             ORDER BY s.created_at DESC",
        )
        .bind(organization)
        .bind(user)
        .bind(owner_scope)
        .fetch_all(&self.0)
        .await
        .map_err(|_| SessionManagementError::Unavailable)?;
        Ok(rows
            .into_iter()
            .map(|row| SessionInventoryItem {
                id: row.id,
                user_id: row.user_id,
                display_name: row.display_name,
                username: row.username,
                created_at: row.created_at,
                expires_at: row.expires_at,
                current: false,
            })
            .collect())
    }

    async fn revoke_active(
        &self,
        organization: Uuid,
        actor: Uuid,
        owner_scope: bool,
        session_id: Uuid,
    ) -> Result<(), SessionManagementError> {
        let mut tx = self
            .0
            .begin()
            .await
            .map_err(|_| SessionManagementError::Unavailable)?;
        let target = sqlx::query_as::<_, (String, Uuid)>(
            "SELECT s.key_hash,s.user_id FROM local_sessions s \
             JOIN local_users u ON u.id=s.user_id \
             JOIN organization_api_keys k ON k.key_hash=s.key_hash AND k.organization_id=u.organization_id \
             WHERE s.id=$1 AND u.organization_id=$2 AND ($4 OR s.user_id=$3) \
             AND NOT u.disabled AND s.revoked_at IS NULL AND s.expires_at>now() \
             AND k.revoked_at IS NULL AND k.expires_at>now() \
             FOR UPDATE OF s,k",
        )
        .bind(session_id)
        .bind(organization)
        .bind(actor)
        .bind(owner_scope)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| SessionManagementError::Unavailable)?
        .ok_or(SessionManagementError::NotFound)?;
        let session_update = sqlx::query(
            "UPDATE local_sessions SET revoked_at=now() WHERE id=$1 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| SessionManagementError::Unavailable)?;
        if session_update.rows_affected() != 1 {
            return Err(SessionManagementError::NotFound);
        }
        let credential_update = sqlx::query(
            "UPDATE organization_api_keys SET revoked_at=now() WHERE key_hash=$1 AND organization_id=$2 AND revoked_at IS NULL",
        )
        .bind(target.0)
        .bind(organization)
        .execute(&mut *tx)
        .await
        .map_err(|_| SessionManagementError::Unavailable)?;
        if credential_update.rows_affected() != 1 {
            return Err(SessionManagementError::Unavailable);
        }
        sqlx::query(
            "INSERT INTO local_audit(organization_id,user_id,action,target) \
             VALUES($1,$2,'session_revoked',$3)",
        )
        .bind(organization)
        .bind(actor)
        .bind(session_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|_| SessionManagementError::Unavailable)?;
        tx.commit()
            .await
            .map_err(|_| SessionManagementError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn passwords_are_salted_and_verified_by_argon2() {
        let a = password_hash("fixture-long-password".into())
            .await
            .expect("hash");
        let b = password_hash("fixture-long-password".into())
            .await
            .expect("hash");
        assert_ne!(a, b);
        assert!(
            verify("fixture-long-password".into(), a.clone())
                .await
                .expect("verify")
        );
        assert!(!verify("wrong-password".into(), a).await.expect("verify"));
    }
}
