use async_trait::async_trait;
use kusi_domain::admin_identity::AdminIdentity;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdminIdentityError {
    #[error("identity is not provisioned")]
    Forbidden,
    #[error("authentication storage failed")]
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct AdminLoginAttempt {
    pub state_hash: String,
    pub browser_hash: String,
    pub nonce: String,
    pub code_verifier: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone)]
pub struct AdminSession {
    pub identity: AdminIdentity,
    pub expires_at: i64,
}

#[async_trait]
pub trait AdminIdentityRepository: Send + Sync {
    async fn provisioned_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<AdminIdentity>, AdminIdentityError>;
    async fn create_attempt(&self, attempt: &AdminLoginAttempt) -> Result<(), AdminIdentityError>;
    async fn consume_attempt(
        &self,
        state_hash: &str,
        browser_hash: &str,
    ) -> Result<Option<AdminLoginAttempt>, AdminIdentityError>;
    async fn create_session(
        &self,
        token_hash: &str,
        issuer: &str,
        subject: &str,
        expires_at: i64,
    ) -> Result<(), AdminIdentityError>;
    async fn active_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminSession>, AdminIdentityError>;
    async fn revoke_session(&self, token_hash: &str) -> Result<(), AdminIdentityError>;
}

pub async fn resolve_identity<R: AdminIdentityRepository>(
    repository: &R,
    issuer: &str,
    subject: &str,
) -> Result<AdminIdentity, AdminIdentityError> {
    repository
        .provisioned_identity(issuer, subject)
        .await?
        .filter(|identity| !identity.memberships.is_empty())
        .ok_or(AdminIdentityError::Forbidden)
}

pub async fn current_session<R: AdminIdentityRepository>(
    repository: &R,
    token_hash: &str,
) -> Result<AdminSession, AdminIdentityError> {
    repository
        .active_session(token_hash)
        .await?
        .filter(|session| !session.identity.memberships.is_empty())
        .ok_or(AdminIdentityError::Forbidden)
}
