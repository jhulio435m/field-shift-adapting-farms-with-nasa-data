//! Inspection and safe cancellation/recovery of the existing outbox, never a second queue.
use crate::local::LocalError;
use async_trait::async_trait;
use kusi_domain::local::LocalUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Serialize)]
pub struct ExecutionSummary {
    pub id: Uuid,
    pub version: i64,
    pub state: String,
    pub mode: &'static str,
    pub proposal_digest: String,
    pub updated_at: i64,
    pub timeline: Vec<ExecutionEvent>,
}
#[derive(Serialize)]
pub struct ExecutionEvent {
    pub version: i64,
    pub state: String,
    pub at: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionInput {
    pub expected_version: i64,
    pub proposal_digest: Option<String>,
    pub decision: Option<String>,
}
#[derive(Clone, Copy)]
pub enum ExecutionAction {
    Cancel,
    Resume,
    Review,
}
#[async_trait]
pub trait ExecutionReview: Send + Sync {
    async fn execution_proposal(
        &self,
        actor: &LocalUser,
        id: Uuid,
    ) -> Result<ExecutionProposal, LocalError>;
    async fn execution_list(
        &self,
        actor: &LocalUser,
        cursor: Option<Uuid>,
    ) -> Result<Vec<ExecutionSummary>, LocalError>;
    async fn execution_detail(
        &self,
        actor: &LocalUser,
        id: Uuid,
    ) -> Result<ExecutionSummary, LocalError>;
    async fn execution_action(
        &self,
        actor: &LocalUser,
        id: Uuid,
        action: ExecutionAction,
        input: &ExecutionInput,
    ) -> Result<ExecutionSummary, LocalError>;
}
pub async fn action<R: ExecutionReview>(
    repo: &R,
    actor: &LocalUser,
    id: Uuid,
    action: ExecutionAction,
    input: &ExecutionInput,
) -> Result<ExecutionSummary, LocalError> {
    if actor.disabled {
        return Err(LocalError::Forbidden);
    }
    if input.expected_version < 1 {
        return Err(LocalError::Conflict);
    }
    if matches!(action, ExecutionAction::Review) && !actor.role.may_approve() {
        return Err(LocalError::Forbidden);
    }
    if !matches!(action, ExecutionAction::Cancel)
        && !input
            .proposal_digest
            .as_ref()
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(LocalError::Invalid);
    }
    repo.execution_action(actor, id, action, input).await
}
pub fn projected_state(status: &str) -> &str {
    match status {
        "queued" => "pending",
        "sending" => "running",
        "unknown" => "uncertain",
        "simulated" | "accepted" | "delivered" | "read" => "completed",
        other => other,
    }
}

#[derive(Serialize)]
pub struct ExecutionProposal {
    pub id: Uuid,
    pub version: i64,
    pub proposal_digest: String,
    pub operation: &'static str,
    pub resource_ref: Uuid,
    pub destination: String,
    pub content: String,
}
