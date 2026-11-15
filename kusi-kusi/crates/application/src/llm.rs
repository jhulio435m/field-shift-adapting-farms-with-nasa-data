//! Provider-neutral inference. Tool proposals never authorize or execute business operations.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceRequest {
    pub instructions: String,
    pub input: String,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}
#[derive(Debug, Serialize)]
pub struct ToolProposal {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}
#[derive(Debug, Serialize)]
pub struct InferenceResult {
    pub text: String,
    pub tools: Vec<ToolProposal>,
    pub model: String,
    pub finish_reason: String,
    pub usage: Value,
}
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("invalid provider configuration")]
    Configuration,
    #[error("invalid inference request or unsupported capability")]
    InvalidRequest,
    #[error("provider credentials rejected")]
    Authentication,
    #[error("provider rate limit exceeded")]
    RateLimited,
    #[error("provider unavailable")]
    Unavailable,
    #[error("invalid, refused, or incomplete provider response")]
    InvalidResponse,
}
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: &InferenceRequest) -> Result<InferenceResult, LlmError>;
}
