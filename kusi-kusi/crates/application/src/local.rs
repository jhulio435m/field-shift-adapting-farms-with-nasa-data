use crate::creative::CreativeRecipe;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;
#[derive(Debug, Error)]
pub enum LocalError {
    #[error("invalid local request")]
    Invalid,
    #[error("authentication required")]
    Unauthorized,
    #[error("operation forbidden")]
    Forbidden,
    #[error("resource not found")]
    NotFound,
    #[error("operation conflicts with current state")]
    Conflict,
    #[error("request limit exceeded")]
    RateLimited,
    #[error("local operation unavailable")]
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAsset {
    pub id: Uuid,
    pub label: String,
    pub filename: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub simulated: bool,
    pub creative_job_id: Option<Uuid>,
    pub created_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPublication {
    pub idempotency_key: Uuid,
    pub id: Uuid,
    pub asset_id: Uuid,
    pub platform: String,
    pub caption: String,
    pub privacy: String,
    pub scheduled_at: Option<i64>,
    pub revision: i32,
    pub approval_digest: String,
    pub state: String,
    pub receipt: Option<String>,
    pub created_at: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationInput {
    pub asset_id: Uuid,
    pub platform: String,
    pub caption: String,
    pub privacy: String,
    pub scheduled_at: Option<i64>,
    pub idempotency_key: Uuid,
}
pub fn publication_digest(
    org: Uuid,
    id: Uuid,
    revision: i32,
    input: &PublicationInput,
) -> Result<String, LocalError> {
    Ok(crate::hash_api_key(
        &serde_json::to_string(&(org, id, revision, input)).map_err(|_| LocalError::Invalid)?,
    ))
}
pub fn simulated_recipe(kind: &str) -> CreativeRecipe {
    CreativeRecipe {
        provider: "simulated".into(),
        model: format!("local-{kind}"),
        key_env: None,
        enabled: true,
        estimated_cost_microusd: 0,
        defaults: json!({}),
        input_schema: json!({"type":"object","additionalProperties":false,"required":["prompt","asset_id","kind"],"properties":{"prompt":{"type":"string","minLength":1,"maxLength":4000},"asset_id":{"type":"string","format":"uuid"},"kind":{"type":"string","enum":["video","image"]}}}),
    }
}
