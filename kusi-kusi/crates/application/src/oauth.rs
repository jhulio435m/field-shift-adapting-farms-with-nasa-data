//! Social account linking, separate from administrator OIDC authentication.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("invalid OAuth request")]
    Invalid,
    #[error("OAuth authorization state unavailable or invalid")]
    InvalidState,
    #[error("social connection not found")]
    NotFound,
    #[error("social connection busy or requires reauthorization")]
    Conflict,
    #[error("OAuth provider unavailable")]
    Provider,
    #[error("OAuth storage or encryption unavailable")]
    Storage,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OAuthProfile {
    pub provider: String,
    pub client_id: String,
    pub client_secret_env: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub graph_version: Option<String>,
    #[serde(default)]
    pub pkce: bool,
}
// No Debug/Serialize on public responses: token sets must never reach an HTTP response.
#[derive(Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub refresh_expires_in: Option<i64>,
    pub external_id: String,
    pub scopes: Vec<String>,
}
#[derive(Serialize)]
pub struct SocialConnection {
    pub id: Uuid,
    pub provider: String,
    pub external_id: String,
    pub scopes: Vec<String>,
    pub status: String,
    pub expires_at: i64,
    pub refresh_expires_at: Option<i64>,
}
pub struct PendingOAuth {
    pub organization_id: Uuid,
    pub principal_hash: String,
    pub profile: OAuthProfile,
    pub verifier: Option<String>,
}
pub struct ConnectionSecret {
    pub profile: OAuthProfile,
    pub tokens: OAuthTokens,
    pub operation_id: Uuid,
}
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn authorize_url(
        &self,
        profile: &OAuthProfile,
        state: &str,
        challenge: Option<&str>,
    ) -> Result<String, OAuthError>;
    async fn exchange(
        &self,
        profile: &OAuthProfile,
        code: &str,
        verifier: Option<&str>,
    ) -> Result<OAuthTokens, OAuthError>;
    async fn refresh(
        &self,
        profile: &OAuthProfile,
        tokens: &OAuthTokens,
    ) -> Result<OAuthTokens, OAuthError>;
    async fn revoke(&self, profile: &OAuthProfile, tokens: &OAuthTokens) -> Result<(), OAuthError>;
}
#[async_trait]
pub trait OAuthRepository: Send + Sync {
    async fn authorization_failed(
        &self,
        pending: &PendingOAuth,
        outcome: &str,
    ) -> Result<(), OAuthError>;
    async fn begin(
        &self,
        organization: Uuid,
        principal: &str,
        state_hash: &str,
        browser_hash: &str,
        profile: &OAuthProfile,
        verifier: Option<&str>,
    ) -> Result<(), OAuthError>;
    async fn consume(
        &self,
        provider: &str,
        state_hash: &str,
        browser_hash: &str,
    ) -> Result<PendingOAuth, OAuthError>;
    async fn save(
        &self,
        pending: PendingOAuth,
        tokens: OAuthTokens,
    ) -> Result<SocialConnection, OAuthError>;
    async fn list(&self, organization: Uuid) -> Result<Vec<SocialConnection>, OAuthError>;
    async fn claim(
        &self,
        organization: Uuid,
        id: Uuid,
        principal: &str,
        action: &str,
    ) -> Result<ConnectionSecret, OAuthError>;
    async fn refreshed(
        &self,
        organization: Uuid,
        id: Uuid,
        operation: Uuid,
        tokens: OAuthTokens,
    ) -> Result<(), OAuthError>;
    async fn finish_action(
        &self,
        organization: Uuid,
        id: Uuid,
        operation: Uuid,
        action: &str,
        success: bool,
        principal: &str,
    ) -> Result<(), OAuthError>;
}
