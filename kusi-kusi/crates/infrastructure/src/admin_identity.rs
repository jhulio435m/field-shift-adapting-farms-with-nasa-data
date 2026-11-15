use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chacha20poly1305::{XChaCha20Poly1305, XNonce, aead::Aead, aead::KeyInit};
use kusi_application::admin_identity::{
    AdminIdentityError, AdminIdentityRepository, AdminLoginAttempt, AdminSession,
};
use kusi_domain::admin_identity::{AdminIdentity, AdminMembership, AdminRole};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgAdminIdentityRepository {
    pool: PgPool,
    state_key: [u8; 32],
}

impl PgAdminIdentityRepository {
    pub fn new(pool: PgPool, state_key: [u8; 32]) -> Self {
        Self { pool, state_key }
    }

    fn seal_attempt_value(&self, value: &str) -> Result<String, AdminIdentityError> {
        let cipher = XChaCha20Poly1305::new((&self.state_key).into());
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| AdminIdentityError::Unavailable)?;
        let nonce = XNonce::from(nonce);
        let ciphertext = cipher
            .encrypt(&nonce, value.as_bytes())
            .map_err(|_| AdminIdentityError::Unavailable)?;
        let mut sealed = nonce.to_vec();
        sealed.extend(ciphertext);
        Ok(URL_SAFE_NO_PAD.encode(sealed))
    }

    fn open_attempt_value(&self, value: &str) -> Result<String, AdminIdentityError> {
        let sealed = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| AdminIdentityError::Unavailable)?;
        if sealed.len() < 40 {
            return Err(AdminIdentityError::Unavailable);
        }
        let nonce_bytes: [u8; 24] = sealed[..24]
            .try_into()
            .map_err(|_| AdminIdentityError::Unavailable)?;
        let nonce = XNonce::from(nonce_bytes);
        let cipher = XChaCha20Poly1305::new((&self.state_key).into());
        let plaintext = cipher
            .decrypt(&nonce, &sealed[24..])
            .map_err(|_| AdminIdentityError::Unavailable)?;
        String::from_utf8(plaintext).map_err(|_| AdminIdentityError::Unavailable)
    }
}

#[async_trait]
impl AdminIdentityRepository for PgAdminIdentityRepository {
    async fn provisioned_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<AdminIdentity>, AdminIdentityError> {
        let rows = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT o.id, o.slug, m.role \
             FROM admin_identities i \
             JOIN admin_memberships m ON m.issuer=i.issuer AND m.subject=i.subject \
             JOIN organizations o ON o.id=m.organization_id \
             WHERE i.issuer=$1 AND i.subject=$2 AND i.disabled_at IS NULL \
               AND m.disabled_at IS NULL ORDER BY o.slug",
        )
        .bind(issuer)
        .bind(subject)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AdminIdentityError::Unavailable)?;

        if rows.is_empty() {
            return Ok(None);
        }
        let memberships = rows
            .into_iter()
            .filter_map(|(organization_id, organization, role)| {
                AdminRole::parse(&role).map(|role| AdminMembership {
                    organization_id,
                    organization,
                    role,
                })
            })
            .collect::<Vec<_>>();
        if memberships.is_empty() {
            return Ok(None);
        }
        Ok(Some(AdminIdentity {
            issuer: issuer.to_owned(),
            subject: subject.to_owned(),
            memberships,
        }))
    }

    async fn create_attempt(&self, attempt: &AdminLoginAttempt) -> Result<(), AdminIdentityError> {
        sqlx::query(
            "DELETE FROM admin_oidc_attempts \
             WHERE consumed_at IS NOT NULL OR expires_at <= now()",
        )
        .execute(&self.pool)
        .await
        .map_err(|_| AdminIdentityError::Unavailable)?;
        let nonce = self.seal_attempt_value(&attempt.nonce)?;
        let code_verifier = self.seal_attempt_value(&attempt.code_verifier)?;
        let result = sqlx::query(
            "INSERT INTO admin_oidc_attempts \
             (state_hash,browser_hash,nonce,code_verifier,expires_at) \
             VALUES ($1,$2,$3,$4,to_timestamp($5))",
        )
        .bind(&attempt.state_hash)
        .bind(&attempt.browser_hash)
        .bind(nonce)
        .bind(code_verifier)
        .bind(attempt.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|_| AdminIdentityError::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(AdminIdentityError::Forbidden);
        }
        Ok(())
    }

    async fn consume_attempt(
        &self,
        state_hash: &str,
        browser_hash: &str,
    ) -> Result<Option<AdminLoginAttempt>, AdminIdentityError> {
        let row = sqlx::query_as::<_, (String, String, String, String, i64)>(
            "DELETE FROM admin_oidc_attempts \
             WHERE state_hash=$1 AND browser_hash=$2 \
               AND expires_at>now() \
             RETURNING state_hash,browser_hash,nonce,code_verifier, \
                       extract(epoch from expires_at)::bigint",
        )
        .bind(state_hash)
        .bind(browser_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AdminIdentityError::Unavailable)?;
        row.map(
            |(state_hash, browser_hash, nonce, code_verifier, expires_at)| {
                Ok(AdminLoginAttempt {
                    state_hash,
                    browser_hash,
                    nonce: self.open_attempt_value(&nonce)?,
                    code_verifier: self.open_attempt_value(&code_verifier)?,
                    expires_at,
                })
            },
        )
        .transpose()
    }

    async fn create_session(
        &self,
        token_hash: &str,
        issuer: &str,
        subject: &str,
        expires_at: i64,
    ) -> Result<(), AdminIdentityError> {
        let result = sqlx::query(
            "INSERT INTO admin_sessions(token_hash,issuer,subject,expires_at) \
             SELECT $1,$2,$3,to_timestamp($4) \
             WHERE EXISTS (SELECT 1 FROM admin_identities i \
               JOIN admin_memberships m USING (issuer,subject) \
               WHERE i.issuer=$2 AND i.subject=$3 AND i.disabled_at IS NULL \
                 AND m.disabled_at IS NULL)",
        )
        .bind(token_hash)
        .bind(issuer)
        .bind(subject)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|_| AdminIdentityError::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(AdminIdentityError::Forbidden);
        }
        Ok(())
    }

    async fn active_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminSession>, AdminIdentityError> {
        let rows = sqlx::query_as::<_, (String, String, Uuid, String, String, i64)>(
            "SELECT i.issuer,i.subject,o.id,o.slug,m.role, \
                    extract(epoch from s.expires_at)::bigint \
             FROM admin_sessions s \
             JOIN admin_identities i ON i.issuer=s.issuer AND i.subject=s.subject \
             JOIN admin_memberships m ON m.issuer=i.issuer AND m.subject=i.subject \
             JOIN organizations o ON o.id=m.organization_id \
             WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>now() \
               AND i.disabled_at IS NULL AND m.disabled_at IS NULL \
             ORDER BY o.slug",
        )
        .bind(token_hash)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AdminIdentityError::Unavailable)?;
        let mut iter = rows.into_iter();
        let Some((issuer, subject, organization_id, organization, role, expires_at)) = iter.next()
        else {
            return Ok(None);
        };
        let mut memberships = Vec::new();
        if let Some(role) = AdminRole::parse(&role) {
            memberships.push(AdminMembership {
                organization_id,
                organization,
                role,
            });
        }
        for (organization_id, organization, role) in
            iter.map(|(_, _, id, slug, role, _)| (id, slug, role))
        {
            if let Some(role) = AdminRole::parse(&role) {
                memberships.push(AdminMembership {
                    organization_id,
                    organization,
                    role,
                });
            }
        }
        if memberships.is_empty() {
            return Ok(None);
        }
        Ok(Some(AdminSession {
            identity: AdminIdentity {
                issuer,
                subject,
                memberships,
            },
            expires_at,
        }))
    }

    async fn revoke_session(&self, token_hash: &str) -> Result<(), AdminIdentityError> {
        sqlx::query(
            "UPDATE admin_sessions SET revoked_at=now() \
             WHERE token_hash=$1 AND revoked_at IS NULL",
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await
        .map_err(|_| AdminIdentityError::Unavailable)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    fn repository() -> PgAdminIdentityRepository {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://user:password@127.0.0.1/kusi_test")
            .expect("test database URL should parse");
        PgAdminIdentityRepository::new(pool, [7_u8; 32])
    }

    #[tokio::test]
    async fn oidc_attempt_secrets_are_randomized_and_round_trip() {
        let repository = repository();
        let first = repository
            .seal_attempt_value("pkce-verifier")
            .expect("attempt value should encrypt");
        let second = repository
            .seal_attempt_value("pkce-verifier")
            .expect("attempt value should encrypt");
        assert_ne!(first, second);
        assert!(!first.contains("pkce-verifier"));
        assert_eq!(
            repository
                .open_attempt_value(&first)
                .expect("attempt value should decrypt"),
            "pkce-verifier"
        );
    }

    #[tokio::test]
    async fn oidc_attempt_cipher_rejects_tampering() {
        let repository = repository();
        let mut sealed = repository
            .seal_attempt_value("nonce-value")
            .expect("attempt value should encrypt");
        sealed.push('x');
        assert!(repository.open_attempt_value(&sealed).is_err());
    }
}
