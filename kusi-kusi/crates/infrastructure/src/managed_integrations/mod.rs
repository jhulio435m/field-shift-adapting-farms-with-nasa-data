//! Driven implementations for managed connections. No provider network in local mode.
pub mod probes;
use crate::local::LocalStore;
use kusi_application::{
    connection_settings::ConnectionKind,
    local::LocalError,
    managed_integrations::{ManagedConnections, VerificationSummary, owner},
};
use kusi_domain::{local::LocalUser, managed_integrations::VerificationMode};
use sqlx::Row;
use uuid::Uuid;

async fn lock(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &LocalUser,
    kind: ConnectionKind,
) -> Result<(Uuid, i64, Vec<String>), LocalError> {
    owner(actor)?;
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_users WHERE id=$1 AND organization_id=$2 AND role='owner' AND NOT disabled)").bind(actor.id).bind(actor.organization_id).fetch_one(&mut **tx).await.map_err(|_|LocalError::Unavailable)?;
    if !active {
        return Err(LocalError::Forbidden);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!(
            "managed-connection:{}:{}",
            actor.organization_id,
            kind.name()
        ))
        .execute(&mut **tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
    sqlx::query_as("SELECT id,revision,credential_fields FROM managed_connection_settings WHERE organization_id=$1 AND kind=$2 FOR UPDATE").bind(actor.organization_id).bind(kind.name()).fetch_optional(&mut **tx).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)
}
#[async_trait::async_trait]
impl ManagedConnections for LocalStore {
    async fn verify_fixture(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        revision: i64,
    ) -> Result<VerificationSummary, LocalError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let (profile, current, fields) = lock(&mut tx, actor, kind).await?;
        if revision != current {
            return Err(LocalError::Conflict);
        }
        let ready = probes::credentials_present(kind, &fields);
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO managed_connection_verifications(id,organization_id,profile_id,profile_revision,adapter_version,mode,status,scope_summary,error_code) VALUES($1,$2,$3,$4,'1','fixture',$5,'synthetic_identity_only',$6)").bind(id).bind(actor.organization_id).bind(profile).bind(current).bind(if ready{"verified"}else{"failed"}).bind(if ready{None}else{Some("missing_credentials")}).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("INSERT INTO local_audit(organization_id,user_id,action,target) VALUES($1,$2,'connection_fixture_verified',$3)").bind(actor.organization_id).bind(actor.id).bind(kind.name()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.connection_status(actor, kind).await
    }
    async fn connection_status(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
    ) -> Result<VerificationSummary, LocalError> {
        owner(actor)?;
        let row=sqlx::query("SELECT p.revision,p.adapter_version,v.id,v.mode,CASE WHEN v.profile_revision=p.revision AND v.adapter_version=p.adapter_version AND v.expires_at>now() AND v.status<>'stale' THEN v.status ELSE 'not_verified' END AS status,extract(epoch FROM v.checked_at)::bigint AS checked_at,extract(epoch FROM v.expires_at)::bigint AS expires_at,COALESCE(v.scope_summary,'not_checked') AS scope_summary,v.error_code FROM managed_connection_settings p LEFT JOIN LATERAL (SELECT * FROM managed_connection_verifications WHERE profile_id=p.id AND organization_id=p.organization_id ORDER BY checked_at DESC,id DESC LIMIT 1) v ON true WHERE p.organization_id=$1 AND p.kind=$2").bind(actor.organization_id).bind(kind.name()).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)?;
        let mode: Option<String> = row.try_get("mode").map_err(|_| LocalError::Unavailable)?;
        Ok(VerificationSummary {
            id: row.try_get("id").map_err(|_| LocalError::Unavailable)?,
            profile_revision: row
                .try_get("revision")
                .map_err(|_| LocalError::Unavailable)?,
            adapter_version: row
                .try_get("adapter_version")
                .map_err(|_| LocalError::Unavailable)?,
            mode: mode.map(|m| {
                if m == "fixture" {
                    VerificationMode::Fixture
                } else {
                    VerificationMode::Provider
                }
            }),
            status: row.try_get("status").map_err(|_| LocalError::Unavailable)?,
            checked_at: row
                .try_get("checked_at")
                .map_err(|_| LocalError::Unavailable)?,
            expires_at: row
                .try_get("expires_at")
                .map_err(|_| LocalError::Unavailable)?,
            scope_summary: row
                .try_get("scope_summary")
                .map_err(|_| LocalError::Unavailable)?,
            error_code: row
                .try_get("error_code")
                .map_err(|_| LocalError::Unavailable)?,
            runtime_active: false,
        })
    }
    async fn disable_connection(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        revision: i64,
    ) -> Result<VerificationSummary, LocalError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let (id, current, _) = lock(&mut tx, actor, kind).await?;
        if current != revision {
            return Err(LocalError::Conflict);
        }
        sqlx::query("UPDATE managed_connection_settings SET enabled_revision=NULL,disabled_at=now() WHERE id=$1 AND organization_id=$2").bind(id).bind(actor.organization_id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.connection_status(actor, kind).await
    }
}
#[async_trait::async_trait]
impl kusi_application::managed_integrations::FixtureConnectionResolver for LocalStore {
    async fn fixture_revision(
        &self,
        actor: &kusi_application::adaptive::Actor,
        kind: ConnectionKind,
    ) -> Result<i64, kusi_domain::adaptive::Error> {
        use kusi_domain::adaptive::Error;
        if !actor.read {
            return Err(Error::Forbidden);
        }
        sqlx::query_scalar("SELECT p.revision FROM managed_connection_settings p WHERE p.organization_id=$1 AND p.kind=$2 AND p.adapter_version='1' AND EXISTS(SELECT 1 FROM organization_api_keys k WHERE k.organization_id=p.organization_id AND k.key_hash=$3 AND k.revoked_at IS NULL AND (k.expires_at IS NULL OR k.expires_at>now())) AND EXISTS(SELECT 1 FROM managed_connection_verifications v WHERE v.profile_id=p.id AND v.organization_id=p.organization_id AND v.profile_revision=p.revision AND v.adapter_version=p.adapter_version AND v.mode='fixture' AND v.status='verified' AND v.expires_at>now())").bind(actor.organization).bind(kind.name()).bind(&actor.principal).fetch_optional(&self.pool).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Forbidden)
    }
}

pub mod channel;

pub mod executions;

pub mod evaluations;

pub mod memory;

pub mod reads;
