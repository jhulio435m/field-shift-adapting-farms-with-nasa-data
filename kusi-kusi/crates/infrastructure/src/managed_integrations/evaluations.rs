use crate::local::LocalStore;
use kusi_application::{
    evaluation::{EvaluationCase, EvaluationResult, evaluate_suite},
    local::LocalError,
    managed_integrations::owner,
};
use kusi_domain::local::LocalUser;
use uuid::Uuid;
const CASES: &str =
    include_str!("../../../../specs/010-managed-integrations/fixtures/evaluations.json");
impl LocalStore {
    pub async fn run_evaluation(
        &self,
        actor: &LocalUser,
        version: &str,
    ) -> Result<(Uuid, EvaluationResult), LocalError> {
        owner(actor)?;
        if version != "1" {
            return Err(LocalError::Invalid);
        }
        let cases: Vec<EvaluationCase> =
            serde_json::from_str(CASES).map_err(|_| LocalError::Unavailable)?;
        let result = evaluate_suite(&cases).await?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO managed_evaluation_runs(id,organization_id,actor_id,suite_version,result) VALUES($1,$2,$3,$4,$5)").bind(id).bind(actor.organization_id).bind(actor.id).bind(version).bind(serde_json::to_value(&result).map_err(|_|LocalError::Unavailable)?).execute(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok((id, result))
    }
    pub async fn evaluation_result(
        &self,
        actor: &LocalUser,
        id: Uuid,
    ) -> Result<EvaluationResult, LocalError> {
        owner(actor)?;
        let value: serde_json::Value = sqlx::query_scalar(
            "SELECT result FROM managed_evaluation_runs WHERE organization_id=$1 AND id=$2",
        )
        .bind(actor.organization_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| LocalError::Unavailable)?
        .ok_or(LocalError::NotFound)?;
        serde_json::from_value(value).map_err(|_| LocalError::Unavailable)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn suite_checks_expected_decisions_and_evidence() {
        let mut cases: Vec<EvaluationCase> = serde_json::from_str(CASES).expect("valid fixtures");
        let result = evaluate_suite(&cases).await.expect("evaluation");
        assert_eq!(result.passed, 24);
        assert_eq!(
            result
                .cases
                .iter()
                .filter(|c| !c.evidence.is_empty())
                .count(),
            3
        );
        cases[0].expected = "denied".into();
        assert_eq!(evaluate_suite(&cases).await.expect("evaluation").passed, 23);
    }
}
