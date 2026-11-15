use crate::local::LocalStore;
use kusi_application::{
    execution_review::{
        ExecutionAction, ExecutionEvent, ExecutionInput, ExecutionProposal, ExecutionReview,
        ExecutionSummary, projected_state,
    },
    hash_api_key,
    local::LocalError,
};
use kusi_domain::local::LocalUser;
use uuid::Uuid;
#[derive(Clone, sqlx::FromRow)]
struct Stored {
    id: Uuid,
    version: i64,
    status: String,
    channel_id: Uuid,
    conversation_id: Uuid,
    cancel_requested: bool,
    recipient: String,
    content: String,
    updated_at: i64,
    managed_profile_id: Option<Uuid>,
    managed_profile_revision: Option<i64>,
    managed_adapter_version: Option<String>,
}
fn summary(row: Stored) -> ExecutionSummary {
    // Only the digest leaves this boundary, never the message/recipient.
    let digest = hash_api_key(
        &serde_json::json!([
            row.id,
            row.channel_id,
            row.conversation_id,
            row.recipient,
            row.content,
            "local.message.deliver",
            "1",
            row.managed_profile_id,
            row.managed_profile_revision,
            row.managed_adapter_version
        ])
        .to_string(),
    );
    ExecutionSummary {
        id: row.id,
        version: row.version,
        state: if row.status == "sending" && row.cancel_requested {
            "cancel_requested".into()
        } else {
            projected_state(&row.status).into()
        },
        mode: "local_no_delivery",
        proposal_digest: digest,
        updated_at: row.updated_at,
        timeline: vec![],
    }
}
#[async_trait::async_trait]
impl ExecutionReview for LocalStore {
    async fn execution_proposal(
        &self,
        actor: &LocalUser,
        id: Uuid,
    ) -> Result<ExecutionProposal, LocalError> {
        if actor.disabled || !actor.role.may_approve() {
            return Err(LocalError::Forbidden);
        }
        let row=sqlx::query_as::<_,Stored>("SELECT o.id,o.version,o.status,o.channel_id,o.conversation_id,o.cancel_requested,o.recipient,o.content,o.managed_profile_id,o.managed_profile_revision,o.managed_adapter_version,extract(epoch FROM o.updated_at)::bigint AS updated_at FROM local_outbox o JOIN local_conversations c ON c.id=o.conversation_id AND c.organization_id=o.organization_id JOIN local_users u ON u.id=c.user_id AND u.organization_id=c.organization_id WHERE o.organization_id=$1 AND c.user_id=$2 AND o.id=$3 AND NOT u.disabled AND u.role IN ('owner','approver') AND o.status='awaiting_review' AND o.managed_profile_id IS NULL AND (o.channel_id=o.conversation_id OR EXISTS(SELECT 1 FROM local_channel_bindings b WHERE b.organization_id=o.organization_id AND b.channel_id=o.channel_id AND b.sender=o.recipient AND b.user_id=c.user_id AND b.revoked_at IS NULL AND b.expires_at>now()))").bind(actor.organization_id).bind(actor.id).bind(id).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)?;
        let digest = summary(row.clone()).proposal_digest;
        Ok(ExecutionProposal {
            id: row.id,
            version: row.version,
            proposal_digest: digest,
            operation: "local.message.deliver",
            resource_ref: row.conversation_id,
            destination: row.recipient,
            content: row.content,
        })
    }
    async fn execution_list(
        &self,
        actor: &LocalUser,
        cursor: Option<Uuid>,
    ) -> Result<Vec<ExecutionSummary>, LocalError> {
        if actor.disabled {
            return Err(LocalError::Forbidden);
        }
        let rows=sqlx::query_as::<_,Stored>("SELECT o.id,o.version,o.status,o.channel_id,o.conversation_id,o.cancel_requested,o.recipient,o.content,o.managed_profile_id,o.managed_profile_revision,o.managed_adapter_version,extract(epoch FROM o.updated_at)::bigint AS updated_at FROM local_outbox o JOIN local_conversations c ON c.id=o.conversation_id AND c.organization_id=o.organization_id JOIN local_users u ON u.id=c.user_id AND u.organization_id=c.organization_id WHERE o.organization_id=$1 AND c.user_id=$2 AND NOT u.disabled AND ($3::uuid IS NULL OR o.id>$3) ORDER BY o.id LIMIT 50").bind(actor.organization_id).bind(actor.id).bind(cursor).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(rows.into_iter().map(summary).collect())
    }
    async fn execution_detail(
        &self,
        actor: &LocalUser,
        id: Uuid,
    ) -> Result<ExecutionSummary, LocalError> {
        let row=sqlx::query_as::<_,Stored>("SELECT o.id,o.version,o.status,o.channel_id,o.conversation_id,o.cancel_requested,o.recipient,o.content,o.managed_profile_id,o.managed_profile_revision,o.managed_adapter_version,extract(epoch FROM o.updated_at)::bigint AS updated_at FROM local_outbox o JOIN local_conversations c ON c.id=o.conversation_id AND c.organization_id=o.organization_id JOIN local_users u ON u.id=c.user_id AND u.organization_id=c.organization_id WHERE o.organization_id=$1 AND c.user_id=$2 AND o.id=$3 AND NOT u.disabled").bind(actor.organization_id).bind(actor.id).bind(id).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)?;
        let mut result = summary(row);
        let events=sqlx::query_as::<_,(i64,String,i64)>("SELECT version,status,extract(epoch FROM created_at)::bigint FROM managed_execution_events WHERE organization_id=$1 AND execution_id=$2 ORDER BY version DESC LIMIT 50").bind(actor.organization_id).bind(id).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        result.timeline = events
            .into_iter()
            .map(|(version, state, at)| ExecutionEvent {
                version,
                state: projected_state(&state).into(),
                at,
            })
            .collect();
        Ok(result)
    }
    async fn execution_action(
        &self,
        actor: &LocalUser,
        id: Uuid,
        action: ExecutionAction,
        input: &ExecutionInput,
    ) -> Result<ExecutionSummary, LocalError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let row=sqlx::query_as::<_,Stored>("SELECT o.id,o.version,o.status,o.channel_id,o.conversation_id,o.cancel_requested,o.recipient,o.content,o.managed_profile_id,o.managed_profile_revision,o.managed_adapter_version,extract(epoch FROM o.updated_at)::bigint AS updated_at FROM local_outbox o JOIN local_conversations c ON c.id=o.conversation_id AND c.organization_id=o.organization_id JOIN local_users u ON u.id=c.user_id AND u.organization_id=c.organization_id WHERE o.organization_id=$1 AND c.user_id=$2 AND o.id=$3 AND NOT u.disabled FOR UPDATE OF o").bind(actor.organization_id).bind(actor.id).bind(id).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::NotFound)?;
        if row.managed_profile_id.is_some() && matches!(action, ExecutionAction::Review) {
            return Err(LocalError::Forbidden);
        }
        if let Some(profile) = row.managed_profile_id {
            let matching:Option<uuid::Uuid>=sqlx::query_scalar("SELECT id FROM managed_connection_settings WHERE organization_id=$1 AND id=$2 AND revision=$3 AND adapter_version=$4 FOR SHARE").bind(actor.organization_id).bind(profile).bind(row.managed_profile_revision).bind(&row.managed_adapter_version).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            if matching.is_none() {
                return Err(LocalError::Conflict);
            }
        }
        if !matches!(action, ExecutionAction::Cancel) && row.channel_id != row.conversation_id {
            let linked:Option<Uuid>=sqlx::query_scalar("SELECT user_id FROM local_channel_bindings WHERE organization_id=$1 AND channel_id=$2 AND sender=$3 AND user_id=$4 AND revoked_at IS NULL AND expires_at>now() FOR SHARE").bind(actor.organization_id).bind(row.channel_id).bind(&row.recipient).bind(actor.id).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            if linked.is_none() {
                return Err(LocalError::Forbidden);
            }
        }
        let current = summary(row);
        if current.version != input.expected_version {
            return Err(LocalError::Conflict);
        }
        if !matches!(action, ExecutionAction::Cancel)
            && input.proposal_digest.as_deref() != Some(&current.proposal_digest)
        {
            return Err(LocalError::Conflict);
        }
        match action {
            ExecutionAction::Cancel
                if matches!(current.state.as_str(), "pending" | "awaiting_review") =>
            {
                sqlx::query("UPDATE local_outbox SET status='cancelled',updated_at=now() WHERE organization_id=$1 AND id=$2 AND version=$3 AND status IN ('queued','awaiting_review')").bind(actor.organization_id).bind(id).bind(input.expected_version).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            }
            ExecutionAction::Cancel if current.state == "running" => {
                sqlx::query("UPDATE local_outbox SET cancel_requested=true,updated_at=now() WHERE organization_id=$1 AND id=$2 AND version=$3").bind(actor.organization_id).bind(id).bind(input.expected_version).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            }
            ExecutionAction::Cancel if current.state == "cancel_requested" => {}
            ExecutionAction::Review
                if current.state == "awaiting_review" && actor.role.may_approve() =>
            {
                let (state, approved) = match input.decision.as_deref() {
                    Some("approve") => ("queued", Some(current.proposal_digest.clone())),
                    Some("reject") => ("cancelled", None),
                    _ => return Err(LocalError::Invalid),
                };
                sqlx::query("UPDATE local_outbox SET status=$3,approved_digest=$4,updated_at=now() WHERE organization_id=$1 AND id=$2 AND version=$5").bind(actor.organization_id).bind(id).bind(state).bind(approved).bind(input.expected_version).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            }
            ExecutionAction::Resume if current.state == "pending" => {} // Already durably queued; never add/re-send a message.
            ExecutionAction::Cancel
                if matches!(
                    current.state.as_str(),
                    "cancelled" | "completed" | "uncertain"
                ) => {}
            _ => return Err(LocalError::Conflict), // Only explicit review proposals can be approved; uncertain effects require reconciliation.
        }
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.execution_detail(actor, id).await
    }
}
