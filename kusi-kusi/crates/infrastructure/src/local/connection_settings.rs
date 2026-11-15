use super::LocalStore;
use crate::oauth::TokenVault;
use kusi_application::{
    connection_settings::{
        ConnectionInput, ConnectionKind, ConnectionSettingsRepository, ConnectionSummary,
    },
    local::LocalError,
};
use kusi_domain::local::LocalUser;
use sqlx::FromRow;
use std::{
    collections::{BTreeMap, HashMap},
    io::{Read, Write},
    path::Path,
    sync::Arc,
};
use uuid::Uuid;

pub(super) async fn load_vault(
    pool: &sqlx::PgPool,
    root: &Path,
) -> Result<Arc<TokenVault>, LocalError> {
    let path = root.join("connection-vault.key");
    if !path.exists() {
        let existing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM managed_connection_settings WHERE credential_ciphertext IS NOT NULL) OR EXISTS(SELECT 1 FROM authorized_memories WHERE ciphertext IS NOT NULL)").fetch_one(pool).await.map_err(|_|LocalError::Unavailable)?;
        if existing {
            return Err(LocalError::Unavailable);
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                let random = crate::oauth::random_secret().map_err(|_| LocalError::Unavailable)?;
                let key = kusi_application::hash_api_key(&random);
                file.write_all(key.as_bytes())
                    .and_then(|_| file.sync_all())
                    .map_err(|_| LocalError::Unavailable)?;
                std::fs::File::open(root)
                    .and_then(|file| file.sync_all())
                    .map_err(|_| LocalError::Unavailable)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(LocalError::Unavailable),
        }
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| LocalError::Unavailable)?;
    if !metadata.is_file() || metadata.len() != 64 {
        return Err(LocalError::Unavailable);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LocalError::Unavailable);
        }
    }
    let mut raw = String::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(65).read_to_string(&mut raw))
        .map_err(|_| LocalError::Unavailable)?;
    let key = hex::decode(raw).map_err(|_| LocalError::Unavailable)?;
    TokenVault::new("v1".into(), HashMap::from([("v1".into(), key)]))
        .map(Arc::new)
        .map_err(|_| LocalError::Unavailable)
}

#[derive(FromRow)]
struct Stored {
    id: Uuid,
    kind: String,
    label: String,
    base_url: Option<String>,
    workspace: Option<String>,
    bot_username: Option<String>,
    webhook_url: Option<String>,
    credential_ciphertext: Option<String>,
    credential_fields: Vec<String>,
    revision: i64,
}
impl Stored {
    fn summary(self) -> Result<ConnectionSummary, LocalError> {
        let kind = ConnectionKind::parse(&self.kind)?;
        let present = |field: &str| self.credential_fields.iter().any(|name| name == field);
        let configured = match kind {
            ConnectionKind::Plane => present("api_key"),
            ConnectionKind::Frappe => present("api_key") && present("api_secret"),
            ConnectionKind::Telegram => present("bot_token") && present("webhook_secret"),
        };
        Ok(ConnectionSummary {
            kind,
            label: self.label,
            base_url: self.base_url,
            workspace: self.workspace,
            bot_username: self.bot_username,
            webhook_url: self.webhook_url,
            revision: self.revision,
            credential_fields: self.credential_fields,
            configured,
            runtime_active: false,
            verification: "not_verified",
        })
    }
}
fn aad(org: Uuid, id: Uuid, kind: ConnectionKind, revision: i64) -> String {
    format!("managed-connection:{org}:{id}:{}:{revision}", kind.name())
}
async fn lock(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &LocalUser,
    kind: ConnectionKind,
) -> Result<(), LocalError> {
    let owner=sqlx::query_scalar::<_,Uuid>("SELECT id FROM local_users WHERE id=$1 AND organization_id=$2 AND role='owner' AND NOT disabled FOR SHARE").bind(actor.id).bind(actor.organization_id).fetch_optional(&mut **tx).await.map_err(|_|LocalError::Unavailable)?;
    if owner.is_none() {
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
    Ok(())
}

#[async_trait::async_trait]
impl ConnectionSettingsRepository for LocalStore {
    async fn list_connection_settings(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<ConnectionSummary>, LocalError> {
        if !actor.role.is_owner() || actor.disabled {
            return Err(LocalError::Forbidden);
        }
        // Deliberately do not select ciphertext on the read/response path.
        sqlx::query_as::<_,Stored>("SELECT id,kind,label,base_url,workspace,bot_username,webhook_url,NULL::text AS credential_ciphertext,credential_fields,revision FROM managed_connection_settings WHERE organization_id=$1 ORDER BY kind")
            .bind(actor.organization_id).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?.into_iter().map(Stored::summary).collect()
    }
    async fn save_connection_settings(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        input: &ConnectionInput,
    ) -> Result<ConnectionSummary, LocalError> {
        input.validate(kind)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        lock(&mut tx, actor, kind).await?;
        let old=sqlx::query_as::<_,Stored>("SELECT id,kind,label,base_url,workspace,bot_username,webhook_url,credential_ciphertext,credential_fields,revision FROM managed_connection_settings WHERE organization_id=$1 AND kind=$2 FOR UPDATE").bind(actor.organization_id).bind(kind.name()).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        if old.as_ref().map(|row| row.revision) != input.expected_revision {
            return Err(LocalError::Conflict);
        }
        let id = old.as_ref().map(|row| row.id).unwrap_or_else(Uuid::new_v4);
        let revision = old
            .as_ref()
            .map(|row| row.revision)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(LocalError::Conflict)?;
        let mut secrets: BTreeMap<String, String> = match old
            .as_ref()
            .and_then(|row| row.credential_ciphertext.as_ref())
        {
            Some(ciphertext) => {
                let previous = old.as_ref().ok_or(LocalError::Unavailable)?;
                let raw = self
                    .connection_vault
                    .decrypt(
                        &aad(actor.organization_id, id, kind, previous.revision),
                        ciphertext,
                    )
                    .map_err(|_| LocalError::Unavailable)?;
                serde_json::from_slice(&raw).map_err(|_| LocalError::Unavailable)?
            }
            None => BTreeMap::new(),
        };
        for (name, value) in input.credentials.values() {
            secrets.insert(name.into(), value.into());
        }
        if kind == ConnectionKind::Telegram
            && secrets.contains_key("bot_token")
            && !secrets.contains_key("webhook_secret")
        {
            secrets.insert(
                "webhook_secret".into(),
                crate::oauth::random_secret().map_err(|_| LocalError::Unavailable)?,
            );
        }
        let fields: Vec<_> = secrets.keys().cloned().collect();
        let ciphertext = if secrets.is_empty() {
            None
        } else {
            Some(
                self.connection_vault
                    .encrypt(
                        &aad(actor.organization_id, id, kind, revision),
                        &serde_json::to_vec(&secrets).map_err(|_| LocalError::Unavailable)?,
                    )
                    .map_err(|_| LocalError::Unavailable)?,
            )
        };
        let result=sqlx::query_as::<_,Stored>("INSERT INTO managed_connection_settings(id,organization_id,kind,label,base_url,workspace,bot_username,webhook_url,credential_ciphertext,credential_fields,revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT(organization_id,kind) DO UPDATE SET label=EXCLUDED.label,base_url=EXCLUDED.base_url,workspace=EXCLUDED.workspace,bot_username=EXCLUDED.bot_username,webhook_url=EXCLUDED.webhook_url,credential_ciphertext=EXCLUDED.credential_ciphertext,credential_fields=EXCLUDED.credential_fields,revision=EXCLUDED.revision,updated_at=now() RETURNING id,kind,label,base_url,workspace,bot_username,webhook_url,credential_ciphertext,credential_fields,revision")
            .bind(id).bind(actor.organization_id).bind(kind.name()).bind(&input.label).bind(&input.base_url).bind(&input.workspace).bind(&input.bot_username).bind(&input.webhook_url).bind(ciphertext).bind(fields).bind(revision).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("INSERT INTO local_audit(organization_id,user_id,action,target) VALUES($1,$2,'connection_settings_saved',$3)").bind(actor.organization_id).bind(actor.id).bind(kind.name()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        result.summary()
    }
    async fn clear_connection_credentials(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        revision: i64,
    ) -> Result<ConnectionSummary, LocalError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        lock(&mut tx, actor, kind).await?;
        let row=sqlx::query_as::<_,Stored>("UPDATE managed_connection_settings SET credential_ciphertext=NULL,credential_fields='{}',revision=revision+1,updated_at=now() WHERE organization_id=$1 AND kind=$2 AND revision=$3 RETURNING id,kind,label,base_url,workspace,bot_username,webhook_url,credential_ciphertext,credential_fields,revision").bind(actor.organization_id).bind(kind.name()).bind(revision).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?.ok_or(LocalError::Conflict)?;
        sqlx::query("INSERT INTO local_audit(organization_id,user_id,action,target) VALUES($1,$2,'connection_credentials_cleared',$3)").bind(actor.organization_id).bind(actor.id).bind(kind.name()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        row.summary()
    }
}
