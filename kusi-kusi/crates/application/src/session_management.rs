use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionManagementError {
    #[error("session not found in permitted scope")]
    NotFound,
    #[error("current session cannot be revoked here")]
    CurrentSession,
    #[error("session store unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInventoryItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub username: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub current: bool,
}

#[async_trait]
pub trait SessionManagementRepository: Send + Sync {
    async fn list_active(
        &self,
        organization: Uuid,
        user: Uuid,
        owner_scope: bool,
    ) -> Result<Vec<SessionInventoryItem>, SessionManagementError>;
    async fn revoke_active(
        &self,
        organization: Uuid,
        actor: Uuid,
        owner_scope: bool,
        session_id: Uuid,
    ) -> Result<(), SessionManagementError>;
}

pub async fn list_sessions<R: SessionManagementRepository>(
    repository: &R,
    organization: Uuid,
    user: Uuid,
    owner_scope: bool,
    current_session: Uuid,
) -> Result<Vec<SessionInventoryItem>, SessionManagementError> {
    let mut sessions = repository
        .list_active(organization, user, owner_scope)
        .await?;
    for session in &mut sessions {
        session.current = session.id == current_session;
    }
    Ok(sessions)
}

pub async fn revoke_session<R: SessionManagementRepository>(
    repository: &R,
    organization: Uuid,
    actor: Uuid,
    owner_scope: bool,
    current_session: Uuid,
    target_session: Uuid,
) -> Result<(), SessionManagementError> {
    if current_session == target_session {
        return Err(SessionManagementError::CurrentSession);
    }
    repository
        .revoke_active(organization, actor, owner_scope, target_session)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Repository(Arc<Mutex<Vec<Uuid>>>);
    #[async_trait]
    impl SessionManagementRepository for Repository {
        async fn list_active(
            &self,
            _: Uuid,
            _: Uuid,
            _: bool,
        ) -> Result<Vec<SessionInventoryItem>, SessionManagementError> {
            Ok(Vec::new())
        }
        async fn revoke_active(
            &self,
            _: Uuid,
            _: Uuid,
            _: bool,
            id: Uuid,
        ) -> Result<(), SessionManagementError> {
            self.0
                .lock()
                .map_err(|_| SessionManagementError::Unavailable)?
                .push(id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn refuses_to_revoke_current_session_before_repository_call() {
        let repository = Repository::default();
        let current = Uuid::new_v4();
        assert_eq!(
            revoke_session(
                &repository,
                Uuid::new_v4(),
                Uuid::new_v4(),
                true,
                current,
                current
            )
            .await,
            Err(SessionManagementError::CurrentSession)
        );
        assert!(repository.0.lock().expect("test lock").is_empty());
    }

    #[tokio::test]
    async fn revokes_other_session_via_scoped_repository() {
        let repository = Repository::default();
        let target = Uuid::new_v4();
        revoke_session(
            &repository,
            Uuid::new_v4(),
            Uuid::new_v4(),
            false,
            Uuid::new_v4(),
            target,
        )
        .await
        .expect("repository accepted in-scope session");
        assert_eq!(*repository.0.lock().expect("test lock"), [target]);
    }
}
