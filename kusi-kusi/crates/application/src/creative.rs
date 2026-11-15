use async_trait::async_trait;
use kusi_domain::creative::{CreativeJob, CreativeProposal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CreativeError {
    #[error("invalid creative request")]
    Invalid,
    #[error("creative job not found")]
    NotFound,
    #[error("creative job conflicts with current state or approval")]
    Conflict,
    #[error("creative operation unavailable")]
    Unavailable,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreativeRecipe {
    pub provider: String,
    pub model: String,
    pub key_env: Option<String>,
    pub input_schema: Value,
    pub defaults: Value,
    pub estimated_cost_microusd: i64,
    pub enabled: bool,
}

pub fn approval_digest(
    organization: Uuid,
    id: Uuid,
    proposal: &CreativeProposal,
) -> Result<String, CreativeError> {
    let snapshot =
        serde_json::to_string(&(organization, id, proposal)).map_err(|_| CreativeError::Invalid)?;
    Ok(crate::hash_api_key(&snapshot))
}

#[async_trait]
pub trait CreativeRepository: Send + Sync {
    async fn list(&self, organization: Uuid) -> Result<Vec<CreativeJob>, CreativeError>;
    async fn set_daily_budget(
        &self,
        organization: Uuid,
        principal: &str,
        microusd: i64,
    ) -> Result<(), CreativeError>;
    async fn daily_budget(&self, organization: Uuid) -> Result<(i64, i64), CreativeError>;
    async fn create(
        &self,
        organization: Uuid,
        principal: &str,
        idempotency: Uuid,
        proposal: CreativeProposal,
    ) -> Result<CreativeJob, CreativeError>;
    async fn get(&self, organization: Uuid, id: Uuid) -> Result<CreativeJob, CreativeError>;
    async fn approve(
        &self,
        organization: Uuid,
        id: Uuid,
        principal: &str,
        digest: &str,
        max_cost_microusd: i64,
    ) -> Result<CreativeJob, CreativeError>;
    async fn cancel(
        &self,
        organization: Uuid,
        id: Uuid,
        principal: &str,
    ) -> Result<CreativeJob, CreativeError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCreativeRequest {
    pub request_id: String,
    pub status_url: String,
    pub response_url: Option<String>,
}
#[derive(Debug)]
pub enum CreativeProgress {
    Pending,
    Processing,
    Completed(Value),
    Failed,
}
#[derive(Debug)]
pub enum SubmissionError {
    Rejected,
    Unknown,
}

#[async_trait]
pub trait CreativeProvider: Send + Sync {
    async fn submit(
        &self,
        recipe: &CreativeRecipe,
        input: &Value,
        idempotency: Uuid,
    ) -> Result<RemoteCreativeRequest, SubmissionError>;
    async fn poll(
        &self,
        recipe: &CreativeRecipe,
        remote: &RemoteCreativeRequest,
    ) -> Result<CreativeProgress, CreativeError>;
}

/// Merge only administrator-owned defaults, then validate the complete immutable provider input.
pub fn prepare_proposal(
    name: &str,
    recipe: &CreativeRecipe,
    input: Value,
    validator: &impl crate::integrations::SchemaValidator,
) -> Result<CreativeProposal, CreativeError> {
    if !recipe.enabled
        || name.is_empty()
        || name.len() > 80
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        || input.to_string().len() > 32768
        || recipe.estimated_cost_microusd < 0
    {
        return Err(CreativeError::Invalid);
    }
    let mut payload = recipe
        .defaults
        .as_object()
        .cloned()
        .ok_or(CreativeError::Invalid)?;
    payload.extend(input.as_object().ok_or(CreativeError::Invalid)?.clone());
    let input = Value::Object(payload);
    if !validator.validate(&recipe.input_schema, &input) {
        return Err(CreativeError::Invalid);
    }
    Ok(CreativeProposal {
        recipe: name.into(),
        provider: recipe.provider.clone(),
        model: recipe.model.clone(),
        input,
        estimated_cost_microusd: recipe.estimated_cost_microusd,
        configuration: serde_json::to_value(recipe).map_err(|_| CreativeError::Invalid)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_is_bound_to_tenant_payload_and_cost() {
        let tenant = Uuid::new_v4();
        let proposal = CreativeProposal {
            recipe: "product-video".into(),
            provider: "simulated".into(),
            model: "fixture".into(),
            input: serde_json::json!({"prompt":"fixture"}),
            estimated_cost_microusd: 10,
            configuration: Value::Null,
        };
        let id = Uuid::new_v4();
        let digest = approval_digest(tenant, id, &proposal).expect("fixture");
        assert_ne!(
            digest,
            approval_digest(Uuid::new_v4(), id, &proposal).expect("fixture")
        );
        let mut changed = proposal.clone();
        changed.input = serde_json::json!({"prompt":"changed"});
        assert_ne!(
            digest,
            approval_digest(tenant, id, &changed).expect("fixture")
        );
        changed = proposal;
        changed.estimated_cost_microusd += 1;
        assert_ne!(
            digest,
            approval_digest(tenant, id, &changed).expect("fixture")
        );
    }
}
