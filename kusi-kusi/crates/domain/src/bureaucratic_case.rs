use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Draft,
    NeedsInformation,
    ReadyForReview,
    AwaitingInstitution,
    Answered,
    Closed,
}

impl CaseStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::NeedsInformation => "needs_information",
            Self::ReadyForReview => "ready_for_review",
            Self::AwaitingInstitution => "awaiting_institution",
            Self::Answered => "answered",
            Self::Closed => "closed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "needs_information" => Some(Self::NeedsInformation),
            "ready_for_review" => Some(Self::ReadyForReview),
            "awaiting_institution" => Some(Self::AwaitingInstitution),
            "answered" => Some(Self::Answered),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceValidity {
    Unverified,
    Current,
    NeedsConfirmation,
    Superseded,
}

impl SourceValidity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Current => "current",
            Self::NeedsConfirmation => "needs_confirmation",
            Self::Superseded => "superseded",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "unverified" => Some(Self::Unverified),
            "current" => Some(Self::Current),
            "needs_confirmation" => Some(Self::NeedsConfirmation),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseSource {
    pub id: Uuid,
    pub key: String,
    pub title: String,
    pub official_url: String,
    pub resolution: String,
    pub version: String,
    pub validity: SourceValidity,
    pub page_or_section: String,
    pub citation_excerpt: String,
    pub verified_by: Option<String>,
    pub verified_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseCitation {
    pub source_id: Uuid,
    pub source_key: String,
    pub version: String,
    pub page_or_section: String,
    pub quote: String,
    pub validity: SourceValidity,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseSummary {
    pub id: Uuid,
    pub title: String,
    pub workflow_key: String,
    pub status: CaseStatus,
    pub is_requester: bool,
    pub is_assigned_staff: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseEvent {
    pub id: Uuid,
    pub event_type: String,
    pub actor_label: String,
    pub summary: String,
    pub occurred_at: i64,
    pub source_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseDetails {
    pub id: Uuid,
    pub tracking_code: String,
    pub title: String,
    pub workflow_key: String,
    pub status: CaseStatus,
    pub is_requester: bool,
    pub is_assigned_staff: bool,
    pub payment_amount_cents: i64,
    pub payment_currency: String,
    pub payment_concept: String,
    pub payment_reference_masked: String,
    pub health_interruption_summary: String,
    pub events: Vec<CaseEvent>,
    pub sources: Vec<CaseSource>,
    pub request: Option<CaseRequest>,
    pub responses: Vec<CaseResponse>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseGuidance {
    pub summary: String,
    pub next_steps: Vec<String>,
    pub open_questions: Vec<String>,
    pub citations: Vec<CaseCitation>,
    pub assistant_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidanceInput {
    pub question: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseRequestInput {
    pub recipient_unit: String,
    pub body: String,
    pub include_receipt_fixture: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseRequest {
    pub id: Uuid,
    pub version: i32,
    pub recipient_unit: String,
    pub body: String,
    pub include_receipt_fixture: bool,
    pub digest: String,
    pub state: String,
    pub consented: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproveRequestInput {
    pub version: i32,
    pub digest: String,
    pub confirm_share: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseResponseInput {
    pub response_type: String,
    pub body: String,
    pub citation_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResponse {
    pub id: Uuid,
    pub response_type: String,
    pub body: String,
    pub actor_label: String,
    pub citation_keys: Vec<String>,
    pub created_at: i64,
}

#[derive(Debug, Error)]
pub enum CaseError {
    #[error("invalid case request")]
    Invalid,
    #[error("case access denied")]
    Forbidden,
    #[error("case not found")]
    NotFound,
    #[error("case state changed")]
    Conflict,
    #[error("case service unavailable")]
    Unavailable,
}

impl GuidanceInput {
    pub fn validate(&self) -> Result<(), CaseError> {
        let question = self.question.trim();
        if question.is_empty() || question.len() > 1200 {
            return Err(CaseError::Invalid);
        }
        Ok(())
    }
}

impl CaseRequestInput {
    pub fn validate(&self) -> Result<(), CaseError> {
        if self.recipient_unit.trim().is_empty()
            || self.recipient_unit.len() > 120
            || self.body.trim().is_empty()
            || self.body.len() > 4000
        {
            return Err(CaseError::Invalid);
        }
        Ok(())
    }
}

impl CaseResponseInput {
    pub fn validate(&self) -> Result<(), CaseError> {
        if !matches!(
            self.response_type.as_str(),
            "request_information" | "guidance" | "referral" | "institutional_decision"
        ) || self.body.trim().is_empty()
            || self.body.len() > 4000
            || self.citation_keys.len() > 8
        {
            return Err(CaseError::Invalid);
        }
        if self
            .citation_keys
            .iter()
            .any(|key| !matches!(key.as_str(), "PA57002335" | "PA570004D8" | "PA57002563"))
        {
            return Err(CaseError::Invalid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_case_inputs_and_known_source_citations() {
        assert!(
            GuidanceInput {
                question: "¿Cómo retomo el trámite?".into()
            }
            .validate()
            .is_ok()
        );
        assert!(
            GuidanceInput {
                question: "   ".into()
            }
            .validate()
            .is_err()
        );
        assert!(
            CaseRequestInput {
                recipient_unit: "Tesorería".into(),
                body: "Consulta sobre PA57002335".into(),
                include_receipt_fixture: true
            }
            .validate()
            .is_ok()
        );
        assert!(
            CaseResponseInput {
                response_type: "guidance".into(),
                body: "Consulte con Tesorería".into(),
                citation_keys: vec!["PA57002563".into()]
            }
            .validate()
            .is_ok()
        );
        assert!(
            CaseResponseInput {
                response_type: "institutional_decision".into(),
                body: "Pago confirmado".into(),
                citation_keys: vec!["invented".into()]
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn parses_only_declared_statuses() {
        assert_eq!(
            CaseStatus::parse("awaiting_institution"),
            Some(CaseStatus::AwaitingInstitution)
        );
        assert_eq!(
            SourceValidity::parse("needs_confirmation"),
            Some(SourceValidity::NeedsConfirmation)
        );
        assert_eq!(SourceValidity::parse("currentish"), None);
    }
}
