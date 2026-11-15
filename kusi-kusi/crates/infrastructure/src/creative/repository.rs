use super::super::PgOrganizationRepository;
use kusi_application::creative::{CreativeError, CreativeRepository, approval_digest};
use kusi_domain::creative::{CreativeJob, CreativeProposal};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(FromRow)]
pub(super) struct JobRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub state: String,
    pub proposal: String,
    pub approval_digest: String,
    pub approval_expires_at: i64,
    pub remote_request: Option<String>,
    pub result: String,
    pub error_code: Option<String>,
}
impl JobRow {
    pub fn decode(self) -> Result<CreativeJob, CreativeError> {
        let remote = self
            .remote_request
            .as_deref()
            .map(serde_json::from_str::<kusi_application::creative::RemoteCreativeRequest>)
            .transpose()
            .map_err(|_| CreativeError::Unavailable)?;
        Ok(CreativeJob {
            id: self.id,
            organization_id: self.organization_id,
            state: serde_json::from_value(serde_json::Value::String(self.state))
                .map_err(|_| CreativeError::Unavailable)?,
            proposal: serde_json::from_str(&self.proposal)
                .map_err(|_| CreativeError::Unavailable)?,
            approval_digest: self.approval_digest,
            approval_expires_at: self.approval_expires_at,
            remote_request_id: remote.map(|v| v.request_id),
            result: serde_json::from_str(&self.result).map_err(|_| CreativeError::Unavailable)?,
            error_code: self.error_code,
        })
    }
}
pub(super) async fn event(
    tx: &mut Transaction<'_, Postgres>,
    org: Uuid,
    id: Uuid,
    kind: &str,
    principal: Option<&str>,
) -> Result<(), CreativeError> {
    sqlx::query("INSERT INTO creative_events (organization_id,job_id,event,principal_hash) VALUES ($1,$2,$3,$4)").bind(org).bind(id).bind(kind).bind(principal).execute(&mut **tx).await.map_err(|_|CreativeError::Unavailable)?;
    Ok(())
}

#[async_trait::async_trait]
impl CreativeRepository for PgOrganizationRepository {
    async fn list(&self, org: Uuid) -> Result<Vec<CreativeJob>, CreativeError> {
        let rows=sqlx::query_as::<_,JobRow>("SELECT id, organization_id, state, proposal, approval_digest, extract(epoch from approval_expires_at)::bigint as approval_expires_at, remote_request, result, error_code FROM creative_jobs WHERE organization_id=$1 ORDER BY created_at DESC LIMIT 50").bind(org).fetch_all(&self.pool).await.map_err(|_|CreativeError::Unavailable)?;
        rows.into_iter().map(JobRow::decode).collect()
    }
    async fn set_daily_budget(
        &self,
        org: Uuid,
        principal: &str,
        limit: i64,
    ) -> Result<(), CreativeError> {
        if !(0..=1_000_000_000).contains(&limit) {
            return Err(CreativeError::Invalid);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| CreativeError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(org.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|_| CreativeError::Unavailable)?;
        sqlx::query("INSERT INTO creative_budgets (organization_id,daily_limit_microusd) VALUES ($1,$2) ON CONFLICT (organization_id) DO UPDATE SET daily_limit_microusd=EXCLUDED.daily_limit_microusd,updated_at=now()").bind(org).bind(limit).execute(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
        sqlx::query("INSERT INTO creative_budget_changes (organization_id,principal_hash,daily_limit_microusd) VALUES ($1,$2,$3)").bind(org).bind(principal).bind(limit).execute(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
        tx.commit().await.map_err(|_| CreativeError::Unavailable)
    }
    async fn daily_budget(&self, org: Uuid) -> Result<(i64, i64), CreativeError> {
        let limit:i64=sqlx::query_scalar("SELECT COALESCE((SELECT daily_limit_microusd FROM creative_budgets WHERE organization_id=$1),0)").bind(org).fetch_one(&self.pool).await.map_err(|_|CreativeError::Unavailable)?;
        let reserved:i64=sqlx::query_scalar("SELECT COALESCE(SUM(((proposal::jsonb)->>'estimated_cost_microusd')::bigint),0)::bigint FROM creative_jobs WHERE organization_id=$1 AND approved_at>=date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' AND state NOT IN ('failed','canceled')").bind(org).fetch_one(&self.pool).await.map_err(|_|CreativeError::Unavailable)?;
        Ok((limit, reserved))
    }
    async fn create(
        &self,
        org: Uuid,
        principal: &str,
        idempotency: Uuid,
        proposal: CreativeProposal,
    ) -> Result<CreativeJob, CreativeError> {
        let id = Uuid::new_v4();
        let digest = approval_digest(org, id, &proposal)?;
        let body = serde_json::to_string(&proposal).map_err(|_| CreativeError::Invalid)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| CreativeError::Unavailable)?;
        let inserted=sqlx::query("INSERT INTO creative_jobs (id,organization_id,idempotency_key,principal_hash,proposal,approval_digest) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (organization_id,idempotency_key) DO NOTHING")
   .bind(id).bind(org).bind(idempotency).bind(principal).bind(&body).bind(&digest).execute(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?.rows_affected();
        let row = sqlx::query_as::<_, JobRow>("SELECT id, organization_id, state, proposal, approval_digest, extract(epoch from approval_expires_at)::bigint as approval_expires_at, remote_request, result, error_code FROM creative_jobs WHERE organization_id=$1 AND idempotency_key=$2")
        .bind(org)
        .bind(idempotency)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| CreativeError::Unavailable)?;
        if row.proposal != body {
            return Err(CreativeError::Conflict);
        }
        if inserted == 1 {
            event(&mut tx, org, id, "created", Some(principal)).await?;
        }
        tx.commit().await.map_err(|_| CreativeError::Unavailable)?;
        row.decode()
    }
    async fn get(&self, org: Uuid, id: Uuid) -> Result<CreativeJob, CreativeError> {
        sqlx::query_as::<_, JobRow>("SELECT id, organization_id, state, proposal, approval_digest, extract(epoch from approval_expires_at)::bigint as approval_expires_at, remote_request, result, error_code FROM creative_jobs WHERE organization_id=$1 AND id=$2")
        .bind(org)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CreativeError::Unavailable)?
        .ok_or(CreativeError::NotFound)?
        .decode()
    }
    async fn approve(
        &self,
        org: Uuid,
        id: Uuid,
        principal: &str,
        digest: &str,
        max_cost: i64,
    ) -> Result<CreativeJob, CreativeError> {
        if max_cost < 0 {
            return Err(CreativeError::Invalid);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| CreativeError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(org.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|_| CreativeError::Unavailable)?;
        let within_budget:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM creative_jobs j WHERE j.organization_id=$1 AND j.id=$2 AND (j.state<>'awaiting_approval' OR (((j.proposal::jsonb)->>'estimated_cost_microusd')::bigint + COALESCE((SELECT SUM(((p.proposal::jsonb)->>'estimated_cost_microusd')::bigint) FROM creative_jobs p WHERE p.organization_id=$1 AND p.approved_at>=date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' AND p.state NOT IN ('failed','canceled')),0) <= COALESCE((SELECT daily_limit_microusd FROM creative_budgets WHERE organization_id=$1),0))))").bind(org).bind(id).fetch_one(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
        if !within_budget {
            return Err(CreativeError::Conflict);
        }
        let changed=sqlx::query("UPDATE creative_jobs SET state='approved',approved_by_hash=$3,approved_at=now(),updated_at=now() WHERE organization_id=$1 AND id=$2 AND state='awaiting_approval' AND approval_expires_at>now() AND approval_digest=$4 AND ((proposal::jsonb)->>'estimated_cost_microusd')::bigint <= $5")
  .bind(org).bind(id).bind(principal).bind(digest).bind(max_cost).execute(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?.rows_affected();
        if changed == 1 {
            event(&mut tx, org, id, "approved", Some(principal)).await?;
        } else {
            let repeat:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM creative_jobs WHERE organization_id=$1 AND id=$2 AND approved_by_hash=$3 AND approval_digest=$4 AND ((proposal::jsonb)->>'estimated_cost_microusd')::bigint <= $5)").bind(org).bind(id).bind(principal).bind(digest).bind(max_cost).fetch_one(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?;
            if !repeat {
                return Err(CreativeError::Conflict);
            }
        }
        tx.commit().await.map_err(|_| CreativeError::Unavailable)?;
        self.get(org, id).await
    }
    async fn cancel(
        &self,
        org: Uuid,
        id: Uuid,
        principal: &str,
    ) -> Result<CreativeJob, CreativeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| CreativeError::Unavailable)?;
        let changed=sqlx::query("UPDATE creative_jobs SET state='canceled',updated_at=now() WHERE organization_id=$1 AND id=$2 AND state IN ('awaiting_approval','approved')").bind(org).bind(id).execute(&mut *tx).await.map_err(|_|CreativeError::Unavailable)?.rows_affected();
        if changed != 1 {
            return Err(CreativeError::Conflict);
        }
        event(&mut tx, org, id, "canceled", Some(principal)).await?;
        tx.commit().await.map_err(|_| CreativeError::Unavailable)?;
        self.get(org, id).await
    }
}
