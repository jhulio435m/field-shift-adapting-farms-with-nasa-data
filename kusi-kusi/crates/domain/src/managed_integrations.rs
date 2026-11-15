//! Connection lifecycle invariants, independent of providers and transports.
use serde::{Deserialize, Serialize};

pub const ADAPTER_VERSION: &str = "1";
pub const VERIFICATION_TTL: i64 = 900;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMode {
    Fixture,
    Provider,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Pending,
    Authorized,
    Running,
    AwaitingReview,
    CancelRequested,
    Cancelled,
    Completed,
    Failed,
    Uncertain,
}
impl ExecutionState {
    pub fn cancel(self) -> Self {
        match self {
            Self::Pending | Self::Authorized | Self::AwaitingReview => Self::Cancelled,
            Self::Running => Self::CancelRequested,
            other => other,
        }
    }
    pub fn resumable(self) -> bool {
        matches!(self, Self::Pending | Self::Authorized)
    }
}
pub fn verification_current(
    expected: i64,
    current: i64,
    version: &str,
    expires_at: i64,
    now: i64,
) -> bool {
    expected > 0 && expected == current && version == ADAPTER_VERSION && expires_at > now
}
pub fn can_enable(mode: VerificationMode, local: bool, current: bool, verified: bool) -> bool {
    !local && mode == VerificationMode::Provider && current && verified
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_expired_and_incompatible_cannot_enable() {
        for (expected, current, version, expiry) in [
            (0, 0, "1", 100),
            (1, 2, "1", 100),
            (1, 1, "2", 100),
            (1, 1, "1", 50),
        ] {
            assert!(!verification_current(
                expected, current, version, expiry, 50
            ));
        }
        assert!(verification_current(1, 1, "1", 51, 50));
        assert!(!can_enable(VerificationMode::Fixture, false, true, true));
        assert!(!can_enable(VerificationMode::Provider, true, true, true));
        assert!(!can_enable(VerificationMode::Provider, false, false, true));
        assert!(can_enable(VerificationMode::Provider, false, true, true));
    }
    #[test]
    fn uncertain_and_completed_are_never_blindly_resumed_or_cancelled() {
        assert!(!ExecutionState::Uncertain.resumable());
        assert_eq!(
            ExecutionState::Uncertain.cancel(),
            ExecutionState::Uncertain
        );
        assert_eq!(
            ExecutionState::Completed.cancel(),
            ExecutionState::Completed
        );
        assert_eq!(
            ExecutionState::Running.cancel(),
            ExecutionState::CancelRequested
        );
        assert_eq!(ExecutionState::Pending.cancel(), ExecutionState::Cancelled);
    }
}
