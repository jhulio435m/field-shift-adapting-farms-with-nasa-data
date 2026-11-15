use super::TokenVault;
use kusi_application::oauth::*;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgOAuthRepository {
    pool: PgPool,
    vault: Arc<TokenVault>,
}
impl PgOAuthRepository {
    pub fn new(pool: PgPool, vault: Arc<TokenVault>) -> Self {
        Self { pool, vault }
    }
}
#[derive(FromRow)]
struct Metadata {
    id: Uuid,
    provider: String,
    external_id: String,
    scopes: String,
    status: String,
    expires_at: i64,
    refresh_expires_at: Option<i64>,
}
impl Metadata {
    fn decode(self) -> Result<SocialConnection, OAuthError> {
        Ok(SocialConnection {
            id: self.id,
            provider: self.provider,
            external_id: self.external_id,
            scopes: serde_json::from_str(&self.scopes).map_err(|_| OAuthError::Storage)?,
            status: self.status,
            expires_at: self.expires_at,
            refresh_expires_at: self.refresh_expires_at,
        })
    }
}
async fn event(
    tx: &mut Transaction<'_, Postgres>,
    org: Uuid,
    id: Option<Uuid>,
    provider: &str,
    event: &str,
    principal: Option<&str>,
) -> Result<(), OAuthError> {
    sqlx::query("INSERT INTO oauth_events (organization_id,connection_id,provider,event,principal_hash) VALUES ($1,$2,$3,$4,$5)").bind(org).bind(id).bind(provider).bind(event).bind(principal).execute(&mut **tx).await.map_err(|_|OAuthError::Storage)?;
    Ok(())
}
impl PgOAuthRepository {
    async fn metadata(&self, org: Uuid, id: Uuid) -> Result<SocialConnection, OAuthError> {
        sqlx::query_as::<_,Metadata>("SELECT id,provider,external_id,scopes,CASE WHEN status='refreshing' AND lease_until<=now() THEN 'needs_reauth' WHEN status='revoking' AND lease_until<=now() THEN 'revoke_pending' WHEN status='active' AND expires_at<=now() THEN 'expired' ELSE status END as status,extract(epoch from expires_at)::bigint as expires_at,extract(epoch from refresh_expires_at)::bigint as refresh_expires_at FROM oauth_connections WHERE organization_id=$1 AND id=$2").bind(org).bind(id).fetch_optional(&self.pool).await.map_err(|_|OAuthError::Storage)?.ok_or(OAuthError::NotFound)?.decode()
    }
}
#[async_trait::async_trait]
impl OAuthRepository for PgOAuthRepository {
    async fn authorization_failed(
        &self,
        pending: &PendingOAuth,
        outcome: &str,
    ) -> Result<(), OAuthError> {
        if !["authorization_denied", "authorization_failed"].contains(&outcome) {
            return Err(OAuthError::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(|_| OAuthError::Storage)?;
        event(
            &mut tx,
            pending.organization_id,
            None,
            &pending.profile.provider,
            outcome,
            Some(&pending.principal_hash),
        )
        .await?;
        tx.commit().await.map_err(|_| OAuthError::Storage)
    }
    async fn begin(
        &self,
        org: Uuid,
        principal: &str,
        state: &str,
        browser: &str,
        profile: &OAuthProfile,
        verifier: Option<&str>,
    ) -> Result<(), OAuthError> {
        let verifier = verifier
            .map(|v| {
                self.vault
                    .encrypt(&format!("{org}:{state}:oauth_verifier"), v.as_bytes())
            })
            .transpose()?;
        let mut tx = self.pool.begin().await.map_err(|_| OAuthError::Storage)?;
        sqlx::query("INSERT INTO oauth_sessions (state_hash,organization_id,provider,principal_hash,browser_hash,profile,verifier_ciphertext) VALUES ($1,$2,$3,$4,$5,$6,$7)").bind(state).bind(org).bind(&profile.provider).bind(principal).bind(browser).bind(serde_json::to_string(profile).map_err(|_|OAuthError::Invalid)?).bind(verifier).execute(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
        event(
            &mut tx,
            org,
            None,
            &profile.provider,
            "authorization_started",
            Some(principal),
        )
        .await?;
        tx.commit().await.map_err(|_| OAuthError::Storage)
    }
    async fn consume(
        &self,
        provider: &str,
        state: &str,
        browser: &str,
    ) -> Result<PendingOAuth, OAuthError> {
        let mut tx = self.pool.begin().await.map_err(|_| OAuthError::Storage)?;
        let row=sqlx::query_as::<_,(Uuid,String,String,Option<String>)>("UPDATE oauth_sessions SET consumed_at=now() WHERE state_hash=$1 AND provider=$2 AND browser_hash=$3 AND consumed_at IS NULL AND expires_at>now() RETURNING organization_id,principal_hash,profile,verifier_ciphertext").bind(state).bind(provider).bind(browser).fetch_optional(&mut *tx).await.map_err(|_|OAuthError::Storage)?.ok_or(OAuthError::InvalidState)?;
        let verifier = row
            .3
            .map(|v| {
                self.vault
                    .decrypt(&format!("{}:{state}:oauth_verifier", row.0), &v)
                    .and_then(|v| String::from_utf8(v).map_err(|_| OAuthError::Storage))
            })
            .transpose()?;
        sqlx::query("UPDATE oauth_sessions SET verifier_ciphertext=NULL WHERE state_hash=$1")
            .bind(state)
            .execute(&mut *tx)
            .await
            .map_err(|_| OAuthError::Storage)?;
        event(
            &mut tx,
            row.0,
            None,
            provider,
            "authorization_consumed",
            Some(&row.1),
        )
        .await?;
        tx.commit().await.map_err(|_| OAuthError::Storage)?;
        Ok(PendingOAuth {
            organization_id: row.0,
            principal_hash: row.1,
            profile: serde_json::from_str(&row.2).map_err(|_| OAuthError::Storage)?,
            verifier,
        })
    }
    async fn save(
        &self,
        pending: PendingOAuth,
        tokens: OAuthTokens,
    ) -> Result<SocialConnection, OAuthError> {
        let org = pending.organization_id;
        let p = pending.profile;
        if tokens.external_id.is_empty()
            || tokens.external_id.len() > 256
            || tokens.access_token.is_empty()
            || tokens.access_token.len() > 16384
            || tokens
                .refresh_token
                .as_ref()
                .is_some_and(|v| v.len() > 16384)
            || tokens.expires_in <= 0
            || tokens.expires_in > 63_072_000
        {
            return Err(OAuthError::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(|_| OAuthError::Storage)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!(
                "{}:{}:{}",
                p.provider, p.client_id, tokens.external_id
            ))
            .execute(&mut *tx)
            .await
            .map_err(|_| OAuthError::Storage)?;
        let previous=sqlx::query_as::<_,(Uuid,Uuid,bool)>("SELECT id,organization_id,COALESCE(lease_until>now(),false) FROM oauth_connections WHERE provider=$1 AND client_id=$2 AND external_id=$3 FOR UPDATE").bind(&p.provider).bind(&p.client_id).bind(&tokens.external_id).fetch_optional(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
        let id = match previous {
            Some((id, owner, busy)) => {
                if owner != org || busy {
                    return Err(OAuthError::Conflict);
                }
                id
            }
            None => Uuid::new_v4(),
        };
        let encrypted = self.vault.encrypt(
            &format!("{org}:{id}:oauth_tokens"),
            &serde_json::to_vec(&tokens).map_err(|_| OAuthError::Storage)?,
        )?;
        sqlx::query("INSERT INTO oauth_connections (id,organization_id,provider,client_id,external_id,scopes,profile,tokens_ciphertext,status,expires_at,refresh_expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'active',now()+$9::bigint*interval '1 second',CASE WHEN $10::bigint IS NULL THEN NULL ELSE now()+$10::bigint*interval '1 second' END) ON CONFLICT (id) DO UPDATE SET scopes=EXCLUDED.scopes,profile=EXCLUDED.profile,tokens_ciphertext=EXCLUDED.tokens_ciphertext,status='active',expires_at=EXCLUDED.expires_at,refresh_expires_at=EXCLUDED.refresh_expires_at,operation_id=NULL,lease_until=NULL,issued_at=now(),updated_at=now()")
   .bind(id).bind(org).bind(&p.provider).bind(&p.client_id).bind(&tokens.external_id).bind(serde_json::to_string(&tokens.scopes).map_err(|_|OAuthError::Storage)?).bind(serde_json::to_string(&p).map_err(|_|OAuthError::Storage)?).bind(encrypted).bind(tokens.expires_in).bind(tokens.refresh_expires_in).execute(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
        event(
            &mut tx,
            org,
            Some(id),
            &p.provider,
            "connected",
            Some(&pending.principal_hash),
        )
        .await?;
        tx.commit().await.map_err(|_| OAuthError::Storage)?;
        self.metadata(org, id).await
    }
    async fn list(&self, org: Uuid) -> Result<Vec<SocialConnection>, OAuthError> {
        sqlx::query_as::<_,Metadata>("SELECT id,provider,external_id,scopes,CASE WHEN status='refreshing' AND lease_until<=now() THEN 'needs_reauth' WHEN status='revoking' AND lease_until<=now() THEN 'revoke_pending' WHEN status='active' AND expires_at<=now() THEN 'expired' ELSE status END as status,extract(epoch from expires_at)::bigint as expires_at,extract(epoch from refresh_expires_at)::bigint as refresh_expires_at FROM oauth_connections WHERE organization_id=$1 ORDER BY updated_at DESC LIMIT 100").bind(org).fetch_all(&self.pool).await.map_err(|_|OAuthError::Storage)?.into_iter().map(Metadata::decode).collect()
    }
    async fn claim(
        &self,
        org: Uuid,
        id: Uuid,
        principal: &str,
        action: &str,
    ) -> Result<ConnectionSecret, OAuthError> {
        if !["refresh", "revoke"].contains(&action) {
            return Err(OAuthError::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(|_| OAuthError::Storage)?;
        let row=sqlx::query_as::<_,(String,String,Option<String>,String,bool,bool,bool,bool)>("SELECT provider,profile,tokens_ciphertext,status,COALESCE(lease_until>now(),false),expires_at>now(),COALESCE(refresh_expires_at>now(),true),issued_at<=now()-interval '24 hours' FROM oauth_connections WHERE organization_id=$1 AND id=$2 FOR UPDATE").bind(org).bind(id).fetch_optional(&mut *tx).await.map_err(|_|OAuthError::Storage)?.ok_or(OAuthError::NotFound)?;
        if row.4 {
            return Err(OAuthError::Conflict);
        }
        if action == "refresh" && row.3 == "refreshing" {
            sqlx::query("UPDATE oauth_connections SET status='needs_reauth',operation_id=NULL,lease_until=NULL WHERE organization_id=$1 AND id=$2").bind(org).bind(id).execute(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
            event(
                &mut tx,
                org,
                Some(id),
                &row.0,
                "refresh_interrupted",
                Some(principal),
            )
            .await?;
            tx.commit().await.map_err(|_| OAuthError::Storage)?;
            return Err(OAuthError::Conflict);
        }
        if action == "refresh"
            && (row.3 != "active"
                || row.0 == "facebook"
                || !row.6
                || (row.0 == "instagram" && (!row.5 || !row.7)))
        {
            return Err(OAuthError::Conflict);
        }
        if action == "revoke" && row.3 == "disconnected" {
            return Err(OAuthError::Conflict);
        }
        let encrypted = row.2.ok_or(OAuthError::Conflict)?;
        let tokens = serde_json::from_slice(
            &self
                .vault
                .decrypt(&format!("{org}:{id}:oauth_tokens"), &encrypted)?,
        )
        .map_err(|_| OAuthError::Storage)?;
        let operation = Uuid::new_v4();
        sqlx::query("UPDATE oauth_connections SET status=$3,operation_id=$4,lease_until=now()+interval '120 seconds',updated_at=now() WHERE organization_id=$1 AND id=$2").bind(org).bind(id).bind(if action=="refresh"{"refreshing"}else{"revoking"}).bind(operation).execute(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
        event(
            &mut tx,
            org,
            Some(id),
            &row.0,
            if action == "refresh" {
                "refresh_started"
            } else {
                "disconnect_started"
            },
            Some(principal),
        )
        .await?;
        tx.commit().await.map_err(|_| OAuthError::Storage)?;
        Ok(ConnectionSecret {
            profile: serde_json::from_str(&row.1).map_err(|_| OAuthError::Storage)?,
            tokens,
            operation_id: operation,
        })
    }
    async fn refreshed(
        &self,
        org: Uuid,
        id: Uuid,
        operation: Uuid,
        tokens: OAuthTokens,
    ) -> Result<(), OAuthError> {
        let encrypted = self.vault.encrypt(
            &format!("{org}:{id}:oauth_tokens"),
            &serde_json::to_vec(&tokens).map_err(|_| OAuthError::Storage)?,
        )?;
        let mut tx = self.pool.begin().await.map_err(|_| OAuthError::Storage)?;
        let provider:Option<String>=sqlx::query_scalar("UPDATE oauth_connections SET scopes=$5,tokens_ciphertext=$6,status='active',expires_at=now()+$7::bigint*interval '1 second',refresh_expires_at=CASE WHEN $8::bigint IS NULL THEN NULL ELSE now()+$8::bigint*interval '1 second' END,operation_id=NULL,lease_until=NULL,issued_at=now(),updated_at=now() WHERE organization_id=$1 AND id=$2 AND operation_id=$3 AND status='refreshing' AND external_id=$4 RETURNING provider")
   .bind(org).bind(id).bind(operation).bind(&tokens.external_id).bind(serde_json::to_string(&tokens.scopes).map_err(|_|OAuthError::Storage)?).bind(encrypted).bind(tokens.expires_in).bind(tokens.refresh_expires_in).fetch_optional(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
        let provider = provider.ok_or(OAuthError::Conflict)?;
        event(&mut tx, org, Some(id), &provider, "refreshed", None).await?;
        tx.commit().await.map_err(|_| OAuthError::Storage)
    }
    async fn finish_action(
        &self,
        org: Uuid,
        id: Uuid,
        operation: Uuid,
        action: &str,
        success: bool,
        principal: &str,
    ) -> Result<(), OAuthError> {
        let (status, kind) = match (action, success) {
            ("refresh", false) => ("needs_reauth", "refresh_failed"),
            ("revoke", true) => ("disconnected", "disconnected"),
            ("revoke", false) => ("revoke_pending", "revocation_pending"),
            _ => return Err(OAuthError::Invalid),
        };
        let mut tx = self.pool.begin().await.map_err(|_| OAuthError::Storage)?;
        let provider:Option<String>=sqlx::query_scalar("UPDATE oauth_connections SET status=$4,tokens_ciphertext=CASE WHEN $4='disconnected' THEN NULL ELSE tokens_ciphertext END,operation_id=NULL,lease_until=NULL,updated_at=now() WHERE organization_id=$1 AND id=$2 AND operation_id=$3 RETURNING provider").bind(org).bind(id).bind(operation).bind(status).fetch_optional(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
        let provider = provider.ok_or(OAuthError::Conflict)?;
        event(&mut tx, org, Some(id), &provider, kind, Some(principal)).await?;
        tx.commit().await.map_err(|_| OAuthError::Storage)
    }
}

/// Optional token maintenance. Never retries an authorization-code exchange or an uncertain refresh.
pub async fn run_refresh_worker(
    repository: PgOAuthRepository,
    provider: super::OfficialOAuthProvider,
    allowed: std::collections::HashMap<Uuid, Vec<String>>,
) {
    loop {
        if refresh_due(&repository, &provider, &allowed).await.is_err() {
            tracing::warn!("OAuth maintenance unavailable; credentials omitted from logs");
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
async fn refresh_due(
    repository: &PgOAuthRepository,
    provider: &impl OAuthProvider,
    allowed: &std::collections::HashMap<Uuid, Vec<String>>,
) -> Result<(), OAuthError> {
    let principal = kusi_application::hash_api_key("oauth-maintenance-worker");
    let mut tx = repository
        .pool
        .begin()
        .await
        .map_err(|_| OAuthError::Storage)?;
    let interrupted=sqlx::query_as::<_,(Uuid,Uuid,String)>("UPDATE oauth_connections SET status=CASE WHEN status='refreshing' THEN 'needs_reauth' ELSE 'revoke_pending' END,operation_id=NULL,lease_until=NULL,updated_at=now() WHERE status IN ('refreshing','revoking') AND lease_until<=now() RETURNING organization_id,id,provider").fetch_all(&mut *tx).await.map_err(|_|OAuthError::Storage)?;
    for (org, id, provider) in interrupted {
        event(
            &mut tx,
            org,
            Some(id),
            &provider,
            "operation_interrupted",
            Some(&principal),
        )
        .await?;
    }
    tx.commit().await.map_err(|_| OAuthError::Storage)?;
    let due=sqlx::query_as::<_,(Uuid,Uuid,String)>("SELECT organization_id,id,provider FROM oauth_connections WHERE status='active' AND expires_at<=now()+interval '10 minutes' AND ((provider='tiktok' AND refresh_expires_at>now()) OR (provider='instagram' AND expires_at>now() AND issued_at<=now()-interval '24 hours')) AND (lease_until IS NULL OR lease_until<=now()) ORDER BY expires_at LIMIT 20").fetch_all(&repository.pool).await.map_err(|_|OAuthError::Storage)?;
    for (org, id, name) in due {
        if !allowed
            .get(&org)
            .is_some_and(|providers| providers.contains(&name))
        {
            continue;
        }
        let Ok(secret) = repository.claim(org, id, &principal, "refresh").await else {
            continue;
        };
        match provider.refresh(&secret.profile, &secret.tokens).await {
            Ok(tokens) => {
                repository
                    .refreshed(org, id, secret.operation_id, tokens)
                    .await?
            }
            Err(_) => {
                repository
                    .finish_action(org, id, secret.operation_id, "refresh", false, &principal)
                    .await?
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct FakeProvider {
        calls: AtomicUsize,
        fail: bool,
    }
    #[async_trait::async_trait]
    impl OAuthProvider for FakeProvider {
        fn authorize_url(
            &self,
            _: &OAuthProfile,
            _: &str,
            _: Option<&str>,
        ) -> Result<String, OAuthError> {
            Err(OAuthError::Invalid)
        }
        async fn exchange(
            &self,
            _: &OAuthProfile,
            _: &str,
            _: Option<&str>,
        ) -> Result<OAuthTokens, OAuthError> {
            Err(OAuthError::Invalid)
        }
        async fn refresh(
            &self,
            p: &OAuthProfile,
            old: &OAuthTokens,
        ) -> Result<OAuthTokens, OAuthError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(OAuthError::Provider);
            }
            Ok(OAuthTokens {
                access_token: "fixture-new-access".into(),
                refresh_token: Some("fixture-new-refresh".into()),
                expires_in: 3600,
                refresh_expires_in: Some(86400),
                external_id: old.external_id.clone(),
                scopes: p.scopes.clone(),
            })
        }
        async fn revoke(&self, _: &OAuthProfile, _: &OAuthTokens) -> Result<(), OAuthError> {
            Ok(())
        }
    }
    #[tokio::test]
    async fn scheduled_refresh_rotates_atomically_and_does_not_retry_uncertain_credentials()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            return Ok(());
        };
        let admin = PgPool::connect(&url).await?;
        let schema = format!("oauth_test_{}", Uuid::new_v4().simple());
        sqlx::QueryBuilder::<Postgres>::new("CREATE SCHEMA ")
            .push(&schema)
            .build()
            .execute(&admin)
            .await?;
        let schema_copy = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
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
        let org = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO organizations (id,name,slug) VALUES ($1,'OAuth Fixture','oauth-fixture')",
        )
        .bind(org)
        .execute(&pool)
        .await?;
        let vault = Arc::new(TokenVault::new(
            "key1".into(),
            std::collections::HashMap::from([("key1".into(), vec![9; 32])]),
        )?);
        let repository = PgOAuthRepository::new(pool.clone(), vault.clone());
        let profile = OAuthProfile {
            provider: "tiktok".into(),
            client_id: "fixture".into(),
            client_secret_env: None,
            redirect_uri: "https://fixture.example/v1/oauth/tiktok/callback".into(),
            scopes: vec!["fixture.scope".into()],
            graph_version: None,
            pkce: false,
        };
        let connection = repository
            .save(
                PendingOAuth {
                    organization_id: org,
                    principal_hash: kusi_application::hash_api_key("fixture"),
                    profile: profile.clone(),
                    verifier: None,
                },
                OAuthTokens {
                    access_token: "fixture-old-access".into(),
                    refresh_token: Some("fixture-old-refresh".into()),
                    expires_in: 3600,
                    refresh_expires_in: Some(86400),
                    external_id: "fixture-user".into(),
                    scopes: profile.scopes.clone(),
                },
            )
            .await?;
        sqlx::query(
            "UPDATE oauth_connections SET expires_at=now()+interval '1 minute' WHERE id=$1",
        )
        .bind(connection.id)
        .execute(&pool)
        .await?;
        let allowed = std::collections::HashMap::from([(org, vec!["tiktok".into()])]);
        let success = FakeProvider {
            calls: AtomicUsize::new(0),
            fail: false,
        };
        refresh_due(&repository, &success, &allowed).await?;
        assert_eq!(success.calls.load(Ordering::SeqCst), 1);
        let ciphertext: String =
            sqlx::query_scalar("SELECT tokens_ciphertext FROM oauth_connections WHERE id=$1")
                .bind(connection.id)
                .fetch_one(&pool)
                .await?;
        let tokens: OAuthTokens = serde_json::from_slice(&vault.decrypt(
            &format!("{org}:{}:oauth_tokens", connection.id),
            &ciphertext,
        )?)?;
        assert_eq!(tokens.refresh_token.as_deref(), Some("fixture-new-refresh"));
        assert_eq!(repository.list(org).await?[0].status, "active");
        sqlx::query(
            "UPDATE oauth_connections SET expires_at=now()+interval '1 minute' WHERE id=$1",
        )
        .bind(connection.id)
        .execute(&pool)
        .await?;
        let uncertain = FakeProvider {
            calls: AtomicUsize::new(0),
            fail: true,
        };
        refresh_due(&repository, &uncertain, &allowed).await?;
        refresh_due(&repository, &uncertain, &allowed).await?;
        assert_eq!(uncertain.calls.load(Ordering::SeqCst), 1);
        assert_eq!(repository.list(org).await?[0].status, "needs_reauth");
        pool.close().await;
        sqlx::QueryBuilder::<Postgres>::new("DROP SCHEMA ")
            .push(&schema)
            .push(" CASCADE")
            .build()
            .execute(&admin)
            .await?;
        admin.close().await;
        Ok(())
    }
}
