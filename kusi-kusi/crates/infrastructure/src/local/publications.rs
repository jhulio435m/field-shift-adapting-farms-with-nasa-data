use super::LocalStore;
use kusi_application::local::{LocalError, LocalPublication, PublicationInput, publication_digest};
use kusi_domain::local::LocalUser;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
struct PublicationRow {
    id: Uuid,
    idempotency_key: Uuid,
    asset_id: Uuid,
    platform: String,
    caption: String,
    privacy: String,
    scheduled_at: Option<i64>,
    revision: i32,
    approval_digest: String,
    state: String,
    receipt: Option<String>,
    created_at: i64,
}
impl From<PublicationRow> for LocalPublication {
    fn from(r: PublicationRow) -> Self {
        Self {
            id: r.id,
            idempotency_key: r.idempotency_key,
            asset_id: r.asset_id,
            platform: r.platform,
            caption: r.caption,
            privacy: r.privacy,
            scheduled_at: r.scheduled_at,
            revision: r.revision,
            approval_digest: r.approval_digest,
            state: r.state,
            receipt: r.receipt,
            created_at: r.created_at,
        }
    }
}
fn validate(input: &PublicationInput) -> Result<(), LocalError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| LocalError::Unavailable)?
        .as_secs() as i64;
    if !["instagram", "facebook", "tiktok"].contains(&input.platform.as_str())
        || !["private", "friends", "public"].contains(&input.privacy.as_str())
        || input.caption.trim().is_empty()
        || input.caption.chars().count() > 2200
        || input
            .scheduled_at
            .is_some_and(|t| t < now - 60 || t > now + 366 * 86400)
    {
        return Err(LocalError::Invalid);
    }
    Ok(())
}
impl LocalStore {
    pub async fn publications(&self, org: Uuid) -> Result<Vec<LocalPublication>, LocalError> {
        Ok(sqlx::query_as::<_,PublicationRow>("SELECT id,idempotency_key,asset_id,platform,caption,privacy,extract(epoch from scheduled_at)::bigint as scheduled_at,revision,approval_digest,state,receipt,extract(epoch from created_at)::bigint as created_at FROM local_publications WHERE organization_id=$1 ORDER BY created_at DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?.into_iter().map(Into::into).collect())
    }
    pub async fn publication(&self, org: Uuid, id: Uuid) -> Result<LocalPublication, LocalError> {
        sqlx::query_as::<_,PublicationRow>("SELECT id,idempotency_key,asset_id,platform,caption,privacy,extract(epoch from scheduled_at)::bigint as scheduled_at,revision,approval_digest,state,receipt,extract(epoch from created_at)::bigint as created_at FROM local_publications WHERE organization_id=$1 AND id=$2").bind(org).bind(id).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?.map(Into::into).ok_or(LocalError::NotFound)
    }
    pub async fn create_publication(
        &self,
        actor: &LocalUser,
        input: PublicationInput,
    ) -> Result<LocalPublication, LocalError> {
        if !actor.role.may_edit() {
            return Err(LocalError::Forbidden);
        }
        validate(&input)?;
        self.asset(actor.organization_id, input.asset_id).await?;
        let id = Uuid::new_v4();
        let digest = publication_digest(actor.organization_id, id, 1, &input)?;
        let snapshot = serde_json::to_string(&input).map_err(|_| LocalError::Invalid)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let inserted=sqlx::query("INSERT INTO local_publications (id,organization_id,created_by,asset_id,platform,snapshot,caption,privacy,scheduled_at,approval_digest,idempotency_key) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,to_timestamp($9::double precision),$10,$11) ON CONFLICT (organization_id,idempotency_key) DO NOTHING").bind(id).bind(actor.organization_id).bind(actor.id).bind(input.asset_id).bind(&input.platform).bind(&snapshot).bind(&input.caption).bind(&input.privacy).bind(input.scheduled_at.map(|v|v as f64)).bind(digest).bind(input.idempotency_key).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?.rows_affected();
        let (actual,existing)=sqlx::query_as::<_,(Uuid,String)>("SELECT id,snapshot FROM local_publications WHERE organization_id=$1 AND idempotency_key=$2").bind(actor.organization_id).bind(input.idempotency_key).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        if snapshot != existing {
            return Err(LocalError::Conflict);
        }
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        if inserted == 1 {
            self.audit(
                actor.organization_id,
                Some(actor.id),
                "publication_drafted",
                &actual.to_string(),
            )
            .await?;
        }
        self.publication(actor.organization_id, actual).await
    }
    pub async fn edit_publication(
        &self,
        actor: &LocalUser,
        id: Uuid,
        input: PublicationInput,
    ) -> Result<LocalPublication, LocalError> {
        if !actor.role.may_edit() {
            return Err(LocalError::Forbidden);
        }
        validate(&input)?;
        self.asset(actor.organization_id, input.asset_id).await?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let (revision,state,idem)=sqlx::query_as::<_,(i32,String,Uuid)>("SELECT revision,state,idempotency_key FROM local_publications WHERE organization_id=$1 AND id=$2 FOR UPDATE").bind(actor.organization_id).bind(id).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)?;
        if !["draft", "approved", "scheduled", "failed"].contains(&state.as_str())
            || idem != input.idempotency_key
        {
            return Err(LocalError::Conflict);
        }
        let revision = revision.checked_add(1).ok_or(LocalError::Conflict)?;
        let digest = publication_digest(actor.organization_id, id, revision, &input)?;
        sqlx::query("UPDATE local_publications SET asset_id=$3,platform=$4,snapshot=$5,caption=$6,privacy=$7,scheduled_at=to_timestamp($8::double precision),revision=$9,approval_digest=$10,state='draft',approved_by=NULL,approved_at=NULL,approval_expires_at=now()+interval '30 minutes',updated_at=now() WHERE organization_id=$1 AND id=$2").bind(actor.organization_id).bind(id).bind(input.asset_id).bind(&input.platform).bind(serde_json::to_string(&input).map_err(|_|LocalError::Invalid)?).bind(&input.caption).bind(&input.privacy).bind(input.scheduled_at.map(|v|v as f64)).bind(revision).bind(digest).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "publication_revised",
            &id.to_string(),
        )
        .await?;
        self.publication(actor.organization_id, id).await
    }
    pub async fn approve_publication(
        &self,
        actor: &LocalUser,
        id: Uuid,
        digest: &str,
        confirm: bool,
    ) -> Result<LocalPublication, LocalError> {
        if !actor.role.may_approve() {
            return Err(LocalError::Forbidden);
        }
        if !confirm || digest.len() != 64 {
            return Err(LocalError::Invalid);
        }
        let changed=sqlx::query("UPDATE local_publications SET state=CASE WHEN scheduled_at>now() THEN 'scheduled' ELSE 'approved' END,approved_by=$3,approved_at=now(),updated_at=now() WHERE organization_id=$1 AND id=$2 AND state='draft' AND approval_digest=$4 AND approval_expires_at>now()").bind(actor.organization_id).bind(id).bind(actor.id).bind(digest).execute(&self.pool).await.map_err(|_|LocalError::Unavailable)?.rows_affected();
        if changed != 1 {
            return Err(LocalError::Conflict);
        }
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "publication_approved",
            &id.to_string(),
        )
        .await?;
        self.publication(actor.organization_id, id).await
    }
    pub async fn cancel_publication(
        &self,
        actor: &LocalUser,
        id: Uuid,
    ) -> Result<LocalPublication, LocalError> {
        if !actor.role.may_edit() && !actor.role.may_approve() {
            return Err(LocalError::Forbidden);
        }
        let changed=sqlx::query("UPDATE local_publications SET state='canceled',updated_at=now() WHERE organization_id=$1 AND id=$2 AND state IN ('draft','approved','scheduled')").bind(actor.organization_id).bind(id).execute(&self.pool).await.map_err(|_|LocalError::Unavailable)?.rows_affected();
        if changed != 1 {
            return Err(LocalError::Conflict);
        }
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "publication_canceled",
            &id.to_string(),
        )
        .await?;
        self.publication(actor.organization_id, id).await
    }
}
