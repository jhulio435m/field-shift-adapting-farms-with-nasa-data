use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreativeState {
    AwaitingApproval,
    Approved,
    Submitting,
    Queued,
    Processing,
    Succeeded,
    Failed,
    Unknown,
    Canceled,
}
impl CreativeState {
    pub fn can_cancel(self) -> bool {
        matches!(self, Self::AwaitingApproval | Self::Approved)
    }
    pub fn can_approve(self) -> bool {
        self == Self::AwaitingApproval
    }
}

/// An immutable generation proposal; approval binds to its digest and estimated cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeProposal {
    pub recipe: String,
    pub provider: String,
    pub model: String,
    pub input: Value,
    pub estimated_cost_microusd: i64,
    pub configuration: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeJob {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub state: CreativeState,
    pub proposal: CreativeProposal,
    pub approval_digest: String,
    pub approval_expires_at: i64,
    pub remote_request_id: Option<String>,
    pub result: Value,
    pub error_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_unsubmitted_jobs_can_be_approved_or_canceled() {
        assert!(CreativeState::AwaitingApproval.can_approve());
        assert!(CreativeState::Approved.can_cancel());
        for state in [
            CreativeState::Submitting,
            CreativeState::Queued,
            CreativeState::Processing,
            CreativeState::Succeeded,
            CreativeState::Unknown,
        ] {
            assert!(!state.can_cancel());
            assert!(!state.can_approve());
        }
    }
}
