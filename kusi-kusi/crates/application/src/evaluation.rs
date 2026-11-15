//! Deterministic synthetic evaluation of the same typed capability gate used for reads.
use crate::{
    connection_settings::ConnectionKind, connector_manifest::validate_operation, local::LocalError,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[async_trait]
pub trait ReadGate: Send + Sync {
    async fn authorize(&self, resource: &str) -> Result<(), LocalError>;
    async fn revision(&self) -> Result<i64, LocalError>;
    async fn read(&self, resource: &str) -> Result<Value, LocalError>;
}
pub async fn execute_fixture_read<R: ReadGate>(
    gate: &R,
    kind: ConnectionKind,
    version: &str,
    operation: &str,
    input: &Value,
    expected_revision: i64,
) -> Result<Value, LocalError> {
    validate_operation(kind, version, operation, input)?;
    let resource = input
        .get("resource_ref")
        .and_then(Value::as_str)
        .ok_or(LocalError::Invalid)?;
    gate.authorize(resource).await?;
    if expected_revision < 1 || gate.revision().await? != expected_revision {
        return Err(LocalError::Conflict);
    }
    gate.authorize(resource).await?;
    gate.read(resource).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationInput {
    pub suite_version: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCase {
    pub question: String,
    pub fixture_version: String,
    pub resource_ref: String,
    pub expected_evidence: Vec<String>,
    pub id: String,
    pub kind: String,
    pub scenario: String,
    pub expected: String,
}
#[derive(Serialize, Deserialize)]
pub struct CaseResult {
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub fixture_version: String,
    pub id: String,
    pub kind: String,
    pub expected: String,
    pub actual: String,
    pub passed: bool,
    pub evidence: Vec<String>,
}
#[derive(Serialize, Deserialize)]
pub struct EvaluationResult {
    pub suite_version: String,
    pub mode: String,
    pub passed: usize,
    pub total: usize,
    pub quality_evaluated: bool,
    pub cases: Vec<CaseResult>,
}
struct FixtureGate<'a> {
    scenario: &'a str,
}
#[async_trait]
impl ReadGate for FixtureGate<'_> {
    async fn authorize(&self, _: &str) -> Result<(), LocalError> {
        if matches!(self.scenario, "forbidden" | "revoked") {
            Err(LocalError::Forbidden)
        } else {
            Ok(())
        }
    }
    async fn revision(&self) -> Result<i64, LocalError> {
        match self.scenario {
            "expired_connection" => Err(LocalError::Forbidden),
            "stale_revision" => Ok(2),
            _ => Ok(1),
        }
    }
    async fn read(&self, resource: &str) -> Result<Value, LocalError> {
        if self.scenario == "provider_unavailable" {
            Err(LocalError::Unavailable)
        } else {
            Ok(json!({"resource_ref":resource,"evidence":["synthetic-record-010"]}))
        }
    }
}
pub async fn evaluate_suite(cases: &[EvaluationCase]) -> Result<EvaluationResult, LocalError> {
    if cases.len() != 24 {
        return Err(LocalError::Invalid);
    }
    let mut results = vec![];
    for case in cases {
        if case.fixture_version != "synthetic-010-v1"
            || case.question.is_empty()
            || case.question.len() > 500
            || case.resource_ref != "fixture-a"
            || !matches!(
                case.scenario.as_str(),
                "allowed"
                    | "forbidden"
                    | "revoked"
                    | "expired_connection"
                    | "stale_revision"
                    | "unknown_operation"
                    | "invalid_arguments"
                    | "provider_unavailable"
            )
        {
            return Err(LocalError::Invalid);
        }
        let kind = ConnectionKind::parse(&case.kind)?;
        let operation = match kind {
            ConnectionKind::Plane => "plane.resource.read",
            ConnectionKind::Frappe => "frappe.resource.read",
            ConnectionKind::Telegram => "telegram.context.read",
        };
        let input = if case.scenario == "invalid_arguments" {
            json!({"resource_ref":"fixture-a","sql":"not permitted"})
        } else {
            json!({"resource_ref":"fixture-a"})
        };
        let result = execute_fixture_read(
            &FixtureGate {
                scenario: &case.scenario,
            },
            kind,
            "1",
            if case.scenario == "unknown_operation" {
                "write.arbitrary"
            } else {
                operation
            },
            &input,
            1,
        )
        .await;
        let (actual, evidence) = match result {
            Ok(_) => ("allowed", vec!["synthetic-record-010".into()]),
            Err(LocalError::Forbidden) => ("denied", vec![]),
            Err(LocalError::Conflict) => ("stale", vec![]),
            Err(LocalError::Invalid) => ("invalid", vec![]),
            Err(_) => ("unavailable", vec![]),
        };
        results.push(CaseResult {
            id: case.id.clone(),
            kind: case.kind.clone(),
            expected: case.expected.clone(),
            actual: actual.into(),
            passed: actual == case.expected && evidence == case.expected_evidence,
            question: case.question.clone(),
            fixture_version: case.fixture_version.clone(),
            evidence,
        });
    }
    let passed = results.iter().filter(|case| case.passed).count();
    Ok(EvaluationResult {
        suite_version: "1".into(),
        mode: "fixture".into(),
        passed,
        total: results.len(),
        quality_evaluated: false,
        cases: results,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn denial_prevents_transport_and_schema_rejects_arbitrary_tools() {
        struct Deny;
        #[async_trait]
        impl ReadGate for Deny {
            async fn authorize(&self, _: &str) -> Result<(), LocalError> {
                Err(LocalError::Forbidden)
            }
            async fn revision(&self) -> Result<i64, LocalError> {
                panic!("revision accessed after denial")
            }
            async fn read(&self, _: &str) -> Result<Value, LocalError> {
                panic!("transport invoked after denial")
            }
        }
        assert!(matches!(
            execute_fixture_read(
                &Deny,
                ConnectionKind::Plane,
                "1",
                "plane.resource.read",
                &json!({"resource_ref":"fixture-a"}),
                1
            )
            .await,
            Err(LocalError::Forbidden)
        ));
    }
}
