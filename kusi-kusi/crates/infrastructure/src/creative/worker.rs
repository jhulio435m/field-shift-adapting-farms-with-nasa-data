use super::repository::{JobRow, event};
use crate::PgOrganizationRepository;
use kusi_application::creative::{
    CreativeError, CreativeProgress, CreativeProvider, CreativeRecipe, RemoteCreativeRequest,
    SubmissionError,
};
use kusi_domain::creative::CreativeProposal;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

// Each claim is committed before any external call. Stale submissions never return to approved.
pub async fn run_worker(
    repository: PgOrganizationRepository,
    provider: super::OfficialCreativeProvider,
) {
    loop {
        if tick(&repository.pool, &provider).await.is_err() {
            tracing::warn!(
                "creative worker database or state error; no request automatically resubmitted"
            );
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}
async fn tick(pool: &PgPool, provider: &impl CreativeProvider) -> Result<(), CreativeError> {
    let mut tx = pool.begin().await.map_err(|_| CreativeError::Unavailable)?;
    let stale=sqlx::query_as::<_,(Uuid,Uuid)>("UPDATE creative_jobs SET state='unknown',error_code='submission_interrupted',lease_token=NULL,lease_until=NULL,updated_at=now() WHERE state='submitting' AND lease_until<now() RETURNING organization_id,id").fetch_all(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
    for (org, id) in stale {
        event(&mut tx, org, id, "unknown", None).await?;
    }
    let row=sqlx::query_as::<_,JobRow>("SELECT id, organization_id, state, proposal, approval_digest, extract(epoch from approval_expires_at)::bigint as approval_expires_at, remote_request, result, error_code FROM creative_jobs WHERE state IN ('approved','queued','processing') AND next_attempt_at<=now() AND (lease_until IS NULL OR lease_until<now()) ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED").fetch_optional(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
    let Some(row) = row else {
        tx.commit().await.map_err(|_| CreativeError::Unavailable)?;
        return Ok(());
    };
    let lease = Uuid::new_v4();
    let submitting = row.state == "approved";
    sqlx::query("UPDATE creative_jobs SET state=CASE WHEN state='approved' THEN 'submitting' ELSE state END,lease_token=$2,lease_until=now()+interval '120 seconds',updated_at=now() WHERE id=$1").bind(row.id).bind(lease).execute(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
    if submitting {
        event(&mut tx, row.organization_id, row.id, "submitting", None).await?;
    }
    tx.commit().await.map_err(|_| CreativeError::Unavailable)?;
    let proposal: CreativeProposal =
        serde_json::from_str(&row.proposal).map_err(|_| CreativeError::Unavailable)?;
    if kusi_application::creative::approval_digest(row.organization_id, row.id, &proposal)?
        != row.approval_digest
    {
        return finish(
            pool,
            &row,
            lease,
            "failed",
            None,
            None,
            Some("invalid_snapshot"),
        )
        .await;
    }
    let recipe: CreativeRecipe =
        serde_json::from_value(proposal.configuration).map_err(|_| CreativeError::Unavailable)?;
    if submitting {
        match provider.submit(&recipe, &proposal.input, row.id).await {
            Ok(remote) => finish(pool, &row, lease, "queued", Some(remote), None, None).await,
            Err(SubmissionError::Rejected) => {
                finish(
                    pool,
                    &row,
                    lease,
                    "failed",
                    None,
                    None,
                    Some("provider_rejected"),
                )
                .await
            }
            Err(SubmissionError::Unknown) => {
                finish(
                    pool,
                    &row,
                    lease,
                    "unknown",
                    None,
                    None,
                    Some("submission_uncertain"),
                )
                .await
            }
        }
    } else {
        let remote: RemoteCreativeRequest = serde_json::from_str(
            row.remote_request
                .as_deref()
                .ok_or(CreativeError::Unavailable)?,
        )
        .map_err(|_| CreativeError::Unavailable)?;
        match provider.poll(&recipe, &remote).await {
            Ok(CreativeProgress::Pending) => {
                finish(pool, &row, lease, "queued", None, None, None).await
            }
            Ok(CreativeProgress::Processing) => {
                finish(pool, &row, lease, "processing", None, None, None).await
            }
            Ok(CreativeProgress::Completed(result)) => {
                finish(pool, &row, lease, "succeeded", None, Some(result), None).await
            }
            Ok(CreativeProgress::Failed) => {
                finish(
                    pool,
                    &row,
                    lease,
                    "failed",
                    None,
                    None,
                    Some("generation_failed"),
                )
                .await
            }
            Err(_) => {
                let mut tx = pool.begin().await.map_err(|_| CreativeError::Unavailable)?;
                let state:Option<String>=sqlx::query_scalar("UPDATE creative_jobs SET poll_failures=poll_failures+1,state=CASE WHEN poll_failures>=9 THEN 'unknown' ELSE state END,error_code='status_unavailable',lease_token=NULL,lease_until=NULL,next_attempt_at=now()+interval '30 seconds',updated_at=now() WHERE id=$1 AND lease_token=$2 RETURNING state").bind(row.id).bind(lease).fetch_optional(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
                if state.as_deref() == Some("unknown") {
                    event(&mut tx, row.organization_id, row.id, "unknown", None).await?;
                }
                tx.commit().await.map_err(|_| CreativeError::Unavailable)
            }
        }
    }
}
async fn finish(
    pool: &PgPool,
    row: &JobRow,
    lease: Uuid,
    state: &str,
    remote: Option<RemoteCreativeRequest>,
    result: Option<Value>,
    error: Option<&str>,
) -> Result<(), CreativeError> {
    let remote = remote
        .map(|v| serde_json::to_string(&v))
        .transpose()
        .map_err(|_| CreativeError::Unavailable)?;
    let result = result.map(|v| v.to_string());
    let mut tx = pool.begin().await.map_err(|_| CreativeError::Unavailable)?;
    let changed=sqlx::query("UPDATE creative_jobs SET state=$3,remote_request=COALESCE($4,remote_request),result=COALESCE($5,result),error_code=$6,lease_token=NULL,lease_until=NULL,poll_failures=0,next_attempt_at=now()+interval '5 seconds',updated_at=now() WHERE id=$1 AND lease_token=$2")
 .bind(row.id).bind(lease).bind(state).bind(remote).bind(result).bind(error).execute(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?.rows_affected();
    if changed == 1 && row.state != state {
        event(&mut tx, row.organization_id, row.id, state, None).await?;
    }
    tx.commit().await.map_err(|_| CreativeError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kusi_application::creative::{CreativeRepository, approval_digest};
    use kusi_domain::{
        Organization,
        creative::{CreativeJob, CreativeState},
    };
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeProvider {
        submits: AtomicUsize,
        uncertain: bool,
    }
    #[async_trait::async_trait]
    impl CreativeProvider for FakeProvider {
        async fn submit(
            &self,
            _: &CreativeRecipe,
            _: &Value,
            id: Uuid,
        ) -> Result<RemoteCreativeRequest, SubmissionError> {
            self.submits.fetch_add(1, Ordering::SeqCst);
            if self.uncertain {
                return Err(SubmissionError::Unknown);
            }
            Ok(RemoteCreativeRequest {
                request_id: id.to_string(),
                status_url: format!("simulated:{id}"),
                response_url: None,
            })
        }
        async fn poll(
            &self,
            _: &CreativeRecipe,
            _: &RemoteCreativeRequest,
        ) -> Result<CreativeProgress, CreativeError> {
            Ok(CreativeProgress::Completed(
                json!({"simulated":true,"assets":[]}),
            ))
        }
    }
    async fn proposal(
        repository: &PgOrganizationRepository,
        org: Uuid,
        principal: &str,
    ) -> Result<CreativeJob, CreativeError> {
        let recipe = CreativeRecipe {
            provider: "simulated".into(),
            model: "fixture".into(),
            key_env: None,
            input_schema: json!({"type":"object"}),
            defaults: json!({}),
            estimated_cost_microusd: 0,
            enabled: true,
        };
        repository
            .create(
                org,
                principal,
                Uuid::new_v4(),
                CreativeProposal {
                    recipe: "fixture".into(),
                    provider: recipe.provider.clone(),
                    model: recipe.model.clone(),
                    input: json!({"prompt":"fictional"}),
                    estimated_cost_microusd: 0,
                    configuration: serde_json::to_value(recipe)
                        .map_err(|_| CreativeError::Invalid)?,
                },
            )
            .await
    }
    #[tokio::test]
    async fn durable_queue_approval_isolation_and_ambiguous_recovery()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            return Ok(());
        };
        let admin = PgPool::connect(&url).await?;
        let schema = format!("creative_test_{}", Uuid::new_v4().simple());
        // The identifier is generated exclusively from a UUID, never request input.
        sqlx::QueryBuilder::<sqlx::Postgres>::new("CREATE SCHEMA ")
            .push(&schema)
            .build()
            .execute(&admin)
            .await?;
        let schema_copy = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |connection, _| {
                let schema = schema_copy.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(schema)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::migrate!("../../migrations").run(&pool).await?;
        let repository = PgOrganizationRepository::new(pool.clone());
        let org = Organization::new("Creative Fixture".into(), "creative-fixture".into())?;
        let principal = kusi_application::hash_api_key("fictional-key");
        kusi_application::OrganizationRepository::create(&repository, &org, &principal).await?;
        let job = proposal(&repository, org.id, &principal).await?;
        assert!(matches!(
            repository.get(Uuid::new_v4(), job.id).await,
            Err(CreativeError::NotFound)
        ));
        assert!(matches!(
            repository
                .approve(org.id, job.id, &principal, "wrong-digest", 0)
                .await,
            Err(CreativeError::Conflict)
        ));
        assert!(matches!(
            repository
                .approve(org.id, job.id, &principal, &job.approval_digest, -1)
                .await,
            Err(CreativeError::Invalid)
        ));
        repository
            .approve(org.id, job.id, &principal, &job.approval_digest, 0)
            .await?;
        let provider = FakeProvider {
            submits: AtomicUsize::new(0),
            uncertain: false,
        };
        let (a, b) = tokio::join!(tick(&pool, &provider), tick(&pool, &provider));
        a?;
        b?;
        assert_eq!(provider.submits.load(Ordering::SeqCst), 1);
        assert_eq!(
            repository.get(org.id, job.id).await?.state,
            CreativeState::Queued
        );
        assert!(repository.cancel(org.id, job.id, &principal).await.is_err());
        sqlx::query("UPDATE creative_jobs SET next_attempt_at=now() WHERE id=$1")
            .bind(job.id)
            .execute(&pool)
            .await?;
        tick(&pool, &provider).await?;
        assert_eq!(
            repository.get(org.id, job.id).await?.state,
            CreativeState::Succeeded
        );
        repository
            .approve(org.id, job.id, &principal, &job.approval_digest, 0)
            .await?;
        tick(&pool, &provider).await?;
        assert_eq!(provider.submits.load(Ordering::SeqCst), 1);
        let cancel = proposal(&repository, org.id, &principal).await?;
        repository.cancel(org.id, cancel.id, &principal).await?;
        assert!(
            repository
                .approve(org.id, cancel.id, &principal, &cancel.approval_digest, 0)
                .await
                .is_err()
        );
        let expired = proposal(&repository, org.id, &principal).await?;
        sqlx::query(
            "UPDATE creative_jobs SET approval_expires_at=now()-interval '1 second' WHERE id=$1",
        )
        .bind(expired.id)
        .execute(&pool)
        .await?;
        assert!(
            repository
                .approve(org.id, expired.id, &principal, &expired.approval_digest, 0)
                .await
                .is_err()
        );
        let interrupted = proposal(&repository, org.id, &principal).await?;
        sqlx::query("UPDATE creative_jobs SET state='submitting',lease_until=now()-interval '1 second' WHERE id=$1").bind(interrupted.id).execute(&pool).await?;
        tick(&pool, &provider).await?;
        assert_eq!(
            repository.get(org.id, interrupted.id).await?.state,
            CreativeState::Unknown
        );
        assert_eq!(provider.submits.load(Ordering::SeqCst), 1);
        let ambiguous = proposal(&repository, org.id, &principal).await?;
        repository
            .approve(
                org.id,
                ambiguous.id,
                &principal,
                &ambiguous.approval_digest,
                0,
            )
            .await?;
        let unknown = FakeProvider {
            submits: AtomicUsize::new(0),
            uncertain: true,
        };
        tick(&pool, &unknown).await?;
        tick(&pool, &unknown).await?;
        assert_eq!(
            repository.get(org.id, ambiguous.id).await?.state,
            CreativeState::Unknown
        );
        assert_eq!(unknown.submits.load(Ordering::SeqCst), 1);
        let idem = Uuid::new_v4();
        let snapshot = job.proposal;
        let first = repository
            .create(org.id, &principal, idem, snapshot.clone())
            .await?;
        let repeated = repository
            .create(org.id, &principal, idem, snapshot.clone())
            .await?;
        assert_eq!(first.id, repeated.id);
        let mut changed = snapshot;
        changed.input = json!({"prompt":"changed"});
        assert!(matches!(
            repository.create(org.id, &principal, idem, changed).await,
            Err(CreativeError::Conflict)
        ));
        assert_eq!(
            first.approval_digest,
            approval_digest(org.id, first.id, &first.proposal)?
        );
        repository.set_daily_budget(org.id, &principal, 100).await?;
        let mut paid_snapshot = first.proposal.clone();
        paid_snapshot.estimated_cost_microusd = 60;
        paid_snapshot.configuration["estimated_cost_microusd"] = json!(60);
        let budget_a = repository
            .create(org.id, &principal, Uuid::new_v4(), paid_snapshot.clone())
            .await?;
        let budget_b = repository
            .create(org.id, &principal, Uuid::new_v4(), paid_snapshot)
            .await?;
        assert!(
            repository
                .approve(
                    org.id,
                    budget_a.id,
                    &principal,
                    &budget_a.approval_digest,
                    59
                )
                .await
                .is_err()
        );
        let (a, b) = tokio::join!(
            repository.approve(
                org.id,
                budget_a.id,
                &principal,
                &budget_a.approval_digest,
                60
            ),
            repository.approve(
                org.id,
                budget_b.id,
                &principal,
                &budget_b.approval_digest,
                60
            )
        );
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        assert_eq!(repository.daily_budget(org.id).await?, (100, 60));
        pool.close().await;
        sqlx::QueryBuilder::<sqlx::Postgres>::new("DROP SCHEMA ")
            .push(&schema)
            .push(" CASCADE")
            .build()
            .execute(&admin)
            .await?;
        admin.close().await;
        Ok(())
    }
}
