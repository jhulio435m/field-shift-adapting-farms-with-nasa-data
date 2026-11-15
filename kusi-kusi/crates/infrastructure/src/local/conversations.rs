use super::LocalStore;
use kusi_application::local::LocalError;
use kusi_domain::local::LocalUser;
use serde_json::{Value, json};
use uuid::Uuid;

impl LocalStore {
    pub async fn conversations(&self, actor: &LocalUser) -> Result<Value, LocalError> {
        let rows=sqlx::query_as::<_,(Uuid,String,Option<Uuid>,Uuid,i64)>("SELECT id,title,user_id,channel_id,extract(epoch from updated_at)::bigint FROM local_conversations WHERE organization_id=$1 AND (user_id=$2 OR $3) ORDER BY updated_at DESC LIMIT 100").bind(actor.organization_id).bind(actor.id).bind(actor.role.is_owner()).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(json!(rows.into_iter().map(|(id,title,user,channel,updated)|json!({"id":id,"title":title,"user_id":user,"local":id==channel,"updated_at":updated})).collect::<Vec<_>>()))
    }
    pub async fn create_conversation(
        &self,
        actor: &LocalUser,
        title: &str,
    ) -> Result<Value, LocalError> {
        if !actor.role.may_edit() {
            return Err(LocalError::Forbidden);
        }
        if title.trim().is_empty() || title.chars().count() > 100 {
            return Err(LocalError::Invalid);
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO local_conversations (id,organization_id,user_id,channel_id,sender,title) VALUES ($1,$2,$3,$1,$4,$5)").bind(id).bind(actor.organization_id).bind(actor.id).bind(actor.id.to_string()).bind(title.trim()).execute(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(json!({"id":id,"title":title,"user_id":actor.id,"local":true}))
    }
    pub async fn messages(&self, actor: &LocalUser, id: Uuid) -> Result<Value, LocalError> {
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_conversations WHERE organization_id=$1 AND id=$2 AND (user_id=$3 OR $4))").bind(actor.organization_id).bind(id).bind(actor.id).bind(actor.role.is_owner()).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        if !exists {
            return Err(LocalError::NotFound);
        }
        let rows=sqlx::query_as::<_,(i64,String,String,String,i64)>("SELECT id,direction,content,metadata,extract(epoch from created_at)::bigint FROM local_messages WHERE organization_id=$1 AND conversation_id=$2 ORDER BY id LIMIT 500").bind(actor.organization_id).bind(id).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        let pending:i64=sqlx::query_scalar("SELECT count(*) FROM channel_inbox i JOIN local_conversations c ON c.channel_id=i.channel_id AND c.organization_id=i.organization_id AND c.sender=i.sender WHERE c.id=$1 AND c.organization_id=$2 AND i.processed_at IS NULL").bind(id).bind(actor.organization_id).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(
            json!({"messages":rows.into_iter().map(|(id,direction,content,metadata,created)|json!({"id":id,"direction":direction,"content":content,"metadata":serde_json::from_str::<Value>(&metadata).unwrap_or(Value::Null),"created_at":created})).collect::<Vec<_>>(),"pending":pending}),
        )
    }
    pub async fn send_local_message(
        &self,
        actor: &LocalUser,
        id: Uuid,
        text: &str,
        asset: Option<Uuid>,
        client_id: Uuid,
    ) -> Result<(), LocalError> {
        if !actor.role.may_edit() {
            return Err(LocalError::Forbidden);
        }
        if text.trim().is_empty() || text.chars().count() > 4000 {
            return Err(LocalError::Invalid);
        }
        if let Some(asset) = asset {
            self.asset(actor.organization_id, asset).await?;
        }
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_conversations WHERE organization_id=$1 AND id=$2 AND user_id=$3 AND channel_id=id)").bind(actor.organization_id).bind(id).bind(actor.id).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        if !exists {
            return Err(LocalError::NotFound);
        }
        let key = client_id.to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("INSERT INTO channel_inbox (organization_id,channel_id,event_id,sender,kind,content,asset_id) VALUES ($1,$2,$3,$4,'text',$5,$6) ON CONFLICT (channel_id,event_id) DO NOTHING").bind(actor.organization_id).bind(id).bind(&key).bind(actor.id.to_string()).bind(text.trim()).bind(asset).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        let (old,old_asset)=sqlx::query_as::<_,(String,Option<Uuid>)>("SELECT content,asset_id FROM channel_inbox WHERE organization_id=$1 AND channel_id=$2 AND event_id=$3").bind(actor.organization_id).bind(id).bind(&key).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        if old != text.trim() || old_asset != asset {
            return Err(LocalError::Conflict);
        }
        sqlx::query("INSERT INTO local_messages (organization_id,conversation_id,event_key,direction,content,metadata) VALUES ($1,$2,$3,'in',$4,$5) ON CONFLICT DO NOTHING").bind(actor.organization_id).bind(id).bind(key).bind(text.trim()).bind(json!({"asset_id":asset}).to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query(
            "UPDATE local_conversations SET updated_at=now() WHERE organization_id=$1 AND id=$2",
        )
        .bind(actor.organization_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)
    }
    pub async fn create_link_code(
        &self,
        actor: &LocalUser,
        channel: Uuid,
    ) -> Result<String, LocalError> {
        self.create_link_code_for(actor, actor.id, channel).await
    }

    pub async fn create_link_code_for(
        &self,
        actor: &LocalUser,
        user_id: Uuid,
        channel: Uuid,
    ) -> Result<String, LocalError> {
        if !actor.role.may_edit() {
            return Err(LocalError::Forbidden);
        }
        if user_id != actor.id {
            if !actor.role.is_owner() {
                return Err(LocalError::Forbidden);
            }
            let eligible: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM local_users WHERE organization_id=$1 AND id=$2 \
                 AND role='viewer' AND NOT disabled)",
            )
            .bind(actor.organization_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| LocalError::Unavailable)?;
            if !eligible {
                return Err(LocalError::NotFound);
            }
        }
        let code = Uuid::new_v4().simple().to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query(
            "UPDATE local_link_codes SET used_at=now() \
             WHERE organization_id=$1 AND user_id=$2 AND channel_id=$3 AND used_at IS NULL",
        )
        .bind(actor.organization_id)
        .bind(user_id)
        .bind(channel)
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("INSERT INTO local_link_codes (code_hash,organization_id,user_id,channel_id) VALUES ($1,$2,$3,$4)")
            .bind(kusi_application::hash_api_key(&code))
            .bind(actor.organization_id)
            .bind(user_id)
            .bind(channel)
            .execute(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query(
            "INSERT INTO local_audit (organization_id,user_id,action,target) \
             VALUES ($1,$2,'link_code_created',$3)",
        )
        .bind(actor.organization_id)
        .bind(actor.id)
        .bind(format!("{channel}:{user_id}"))
        .execute(&mut *tx)
        .await
        .map_err(|_| LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        Ok(code)
    }
    pub async fn overview(&self, org: Uuid) -> Result<Value, LocalError> {
        let value:String=sqlx::query_scalar("SELECT json_build_object('assets',(SELECT count(*) FROM local_assets WHERE organization_id=$1),'pending_messages',(SELECT count(*) FROM channel_inbox WHERE organization_id=$1 AND processed_at IS NULL),'approval_jobs',(SELECT count(*) FROM creative_jobs WHERE organization_id=$1 AND state='awaiting_approval'),'completed_jobs',(SELECT count(*) FROM creative_jobs WHERE organization_id=$1 AND state='succeeded'),'publication_drafts',(SELECT count(*) FROM local_publications WHERE organization_id=$1 AND state='draft'),'publications_simulated',(SELECT count(*) FROM local_publications WHERE organization_id=$1 AND state='published_simulated'),'users',(SELECT count(*) FROM local_users WHERE organization_id=$1 AND NOT disabled),'failed_messages',(SELECT count(*) FROM channel_inbox WHERE organization_id=$1 AND error_code IS NOT NULL),'pending_delivery',(SELECT count(*) FROM local_outbox WHERE organization_id=$1 AND status='queued'))::text").bind(org).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        serde_json::from_str(&value).map_err(|_| LocalError::Unavailable)
    }
    pub async fn activity(&self, org: Uuid) -> Result<Value, LocalError> {
        let rows=sqlx::query_as::<_,(i64,String,String,String)>("SELECT extract(epoch from a.created_at)::bigint as happened_at,a.action,a.target,COALESCE(u.display_name,'Sistema') as actor FROM local_audit a LEFT JOIN local_users u ON u.id=a.user_id WHERE a.organization_id=$1 UNION ALL SELECT extract(epoch from occurred_at)::bigint,'creative_'||event,job_id::text,'Sistema' FROM creative_events WHERE organization_id=$1 UNION ALL SELECT extract(epoch from occurred_at)::bigint,'integration_'||outcome,capability,'Sistema' FROM execution_audit WHERE organization_id=$1 UNION ALL SELECT extract(epoch from occurred_at)::bigint,'oauth_'||event,provider,'Sistema' FROM oauth_events WHERE organization_id=$1 ORDER BY happened_at DESC LIMIT 150").bind(org).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(json!(rows.into_iter().map(|(at,action,target,actor)|json!({"at":at,"action":action,"target":target,"actor":actor})).collect::<Vec<_>>()))
    }
    pub async fn workspaces(&self, actor: &LocalUser) -> Result<Value, LocalError> {
        if !actor.platform_admin {
            return Err(LocalError::Forbidden);
        }
        let rows=sqlx::query_as::<_,(Uuid,String,String,i64)>("SELECT o.id,o.name,o.slug,(SELECT count(*) FROM local_users u WHERE u.organization_id=o.id AND NOT u.disabled) FROM organizations o ORDER BY o.created_at DESC LIMIT 100").fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(json!(
            rows.into_iter()
                .map(
                    |(id, name, slug, users)| json!({"id":id,"name":name,"slug":slug,"users":users})
                )
                .collect::<Vec<_>>()
        ))
    }
    pub async fn create_workspace(
        &self,
        actor: &LocalUser,
        name: &str,
        slug: &str,
        username: &str,
        display_name: &str,
        password: String,
    ) -> Result<Value, LocalError> {
        if !actor.platform_admin {
            return Err(LocalError::Forbidden);
        }
        let organization = kusi_domain::Organization::new(name.into(), slug.into())
            .map_err(|_| LocalError::Invalid)?;
        // Bootstrap validation runs before inserting the organization; no raw service key is exposed.
        if username.len() < 3
            || username.len() > 64
            || !username
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c))
            || display_name.trim().is_empty()
            || display_name.chars().count() > 80
        {
            return Err(LocalError::Invalid);
        }
        let password_hash = super::auth::password_hash(password).await?;
        let owner = Uuid::new_v4();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("INSERT INTO organizations (id,name,slug) VALUES ($1,$2,$3)")
            .bind(organization.id)
            .bind(&organization.name)
            .bind(&organization.slug)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if e.as_database_error()
                    .is_some_and(|e| e.is_unique_violation())
                {
                    LocalError::Conflict
                } else {
                    LocalError::Unavailable
                }
            })?;
        sqlx::query("INSERT INTO local_users (id,organization_id,username,display_name,password_hash,role) VALUES ($1,$2,$3,$4,$5,'owner')").bind(owner).bind(organization.id).bind(username).bind(display_name.trim()).bind(password_hash).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "workspace_created",
            &organization.id.to_string(),
        )
        .await?;
        Ok(json!({"organization":organization,"owner_username":username}))
    }
    pub async fn retention_preview(
        &self,
        actor: &LocalUser,
        days: i64,
    ) -> Result<Value, LocalError> {
        if !actor.role.is_owner() {
            return Err(LocalError::Forbidden);
        }
        if !(1..=3650).contains(&days) {
            return Err(LocalError::Invalid);
        }
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM channel_inbox WHERE organization_id=$1 AND processed_at IS NOT NULL AND received_at<now()-$2::bigint*interval '1 day' AND content<>'[eliminado por retencion]'").bind(actor.organization_id).bind(days).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(json!({"days":days,"processed_events_to_redact":count,"preserves_event_ids":true}))
    }
    pub async fn apply_retention(
        &self,
        actor: &LocalUser,
        days: i64,
        confirm: bool,
    ) -> Result<Value, LocalError> {
        self.retention_preview(actor, days).await?;
        if !confirm {
            return Err(LocalError::Invalid);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let changed=sqlx::query("UPDATE channel_inbox SET content='[eliminado por retencion]' WHERE organization_id=$1 AND processed_at IS NOT NULL AND received_at<now()-$2::bigint*interval '1 day'").bind(actor.organization_id).bind(days).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?.rows_affected();
        sqlx::query("DELETE FROM local_messages WHERE organization_id=$1 AND created_at<now()-$2::bigint*interval '1 day'").bind(actor.organization_id).bind(days).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("UPDATE local_conversations SET context='{}',context_expires_at=now() WHERE organization_id=$1 AND updated_at<now()-$2::bigint*interval '1 day'").bind(actor.organization_id).bind(days).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("DELETE FROM oauth_sessions WHERE organization_id=$1 AND expires_at<now()-interval '1 day'").bind(actor.organization_id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("DELETE FROM authorized_memories WHERE organization_id=$1 AND (deleted_at IS NOT NULL OR expires_at<=now())").bind(actor.organization_id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        self.audit(
            actor.organization_id,
            Some(actor.id),
            "retention_applied",
            &days.to_string(),
        )
        .await?;
        Ok(json!({"redacted_events":changed}))
    }
}
