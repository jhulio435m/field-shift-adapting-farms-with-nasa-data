pub mod adaptive;
pub mod admin_identity;
pub mod admin_oidc;
pub mod audio;
pub mod creative;
pub mod external_access;
pub mod llm;
pub mod local;
pub mod messaging;
pub mod oauth;
pub mod student_payments;
pub mod system_connectors;
#[async_trait::async_trait]
impl kusi_application::ChannelInbox for PgOrganizationRepository {
    async fn accept(
        &self,
        organization: uuid::Uuid,
        channel: uuid::Uuid,
        events: &[kusi_domain::ChannelEvent],
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| ApplicationError::Storage)?;
        for event in events {
            sqlx::query("INSERT INTO channel_inbox (organization_id, channel_id, event_id, sender, kind, content) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (channel_id, event_id) DO NOTHING")
   .bind(organization).bind(channel).bind(&event.event_id).bind(&event.sender).bind(&event.kind).bind(&event.content).execute(&mut *tx).await.map_err(|_| ApplicationError::Storage)?;
        }
        tx.commit().await.map_err(|_| ApplicationError::Storage)
    }
}
#[async_trait::async_trait]
impl kusi_application::integrations::AuditRepository for PgOrganizationRepository {
    async fn record(
        &self,
        event: uuid::Uuid,
        organization: uuid::Uuid,
        capability: &str,
        version: &str,
        outcome: &str,
    ) -> Result<(), kusi_application::integrations::ExecutionError> {
        sqlx::query("INSERT INTO execution_audit (event_id, organization_id, capability, version, outcome) VALUES ($1,$2,$3,$4,$5)").bind(event).bind(organization).bind(capability).bind(version).bind(outcome).execute(&self.pool).await.map_err(|_| kusi_application::integrations::ExecutionError::Unavailable)?;
        Ok(())
    }
}
#[async_trait::async_trait]
impl kusi_application::integrations::AuthorizationService for PgOrganizationRepository {
    async fn authorize_resource(
        &self,
        organization: uuid::Uuid,
        principal: &str,
        capability: &kusi_domain::Capability,
        input: &serde_json::Value,
    ) -> Result<
        Option<kusi_domain::external_access::ResourceRead>,
        kusi_application::integrations::ExecutionError,
    > {
        self.authorize_external_resource(organization, principal, capability, input)
            .await
    }
    async fn permits(&self, organization: uuid::Uuid, principal: &str, permission: &str) -> bool {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM capability_grants g JOIN organization_api_keys k ON k.key_hash = g.principal_hash AND k.organization_id = g.organization_id WHERE g.organization_id = $1 AND g.principal_hash = $2 AND g.permission = $3 AND k.revoked_at IS NULL AND (k.expires_at IS NULL OR k.expires_at > now()))")
   .bind(organization).bind(principal).bind(permission).fetch_one(&self.pool).await.unwrap_or(false)
    }
}
pub struct JsonSchemaValidator;
impl kusi_application::integrations::SchemaValidator for JsonSchemaValidator {
    fn validate(&self, schema: &serde_json::Value, value: &serde_json::Value) -> bool {
        schema_allowed(schema)
            && jsonschema::validator_for(schema).is_ok_and(|validator| validator.is_valid(value))
    }
}

pub fn schema_allowed(schema: &serde_json::Value) -> bool {
    fn safe(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(map) => {
                !map.contains_key("$ref")
                    && !map.contains_key("$dynamicRef")
                    && map.values().all(safe)
            }
            serde_json::Value::Array(values) => values.iter().all(safe),
            _ => true,
        }
    }
    schema.to_string().len() <= 16384 && safe(schema) && jsonschema::validator_for(schema).is_ok()
}
pub mod http_connector;
use async_trait::async_trait;
use kusi_application::{ApplicationError, OrganizationRepository};
use kusi_domain::Organization;
use sqlx::PgPool;

#[derive(Clone)]
pub struct PgOrganizationRepository {
    pool: PgPool,
}

impl PgOrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrganizationRepository for PgOrganizationRepository {
    async fn create(
        &self,
        organization: &Organization,
        key_hash: &str,
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| ApplicationError::Storage)?;
        let result = sqlx::query("INSERT INTO organizations (id, name, slug) VALUES ($1, $2, $3)")
            .bind(organization.id)
            .bind(&organization.name)
            .bind(&organization.slug)
            .execute(&mut *tx)
            .await;
        if let Err(error) = result {
            if let sqlx::Error::Database(db_error) = &error
                && db_error.constraint() == Some("organizations_slug_key")
            {
                return Err(ApplicationError::Conflict);
            }
            return Err(ApplicationError::Storage);
        }
        sqlx::query(
            "INSERT INTO organization_api_keys (organization_id, key_hash) VALUES ($1, $2)",
        )
        .bind(organization.id)
        .bind(key_hash)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApplicationError::Storage)?;
        tx.commit().await.map_err(|_| ApplicationError::Storage)
    }

    async fn find_by_api_key_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<Organization>, ApplicationError> {
        sqlx::query_as::<_, (uuid::Uuid, String, String)>(
            "SELECT o.id, o.name, o.slug FROM organizations o JOIN organization_api_keys k ON k.organization_id = o.id WHERE k.key_hash = $1 AND k.revoked_at IS NULL AND (k.expires_at IS NULL OR k.expires_at > now())"
        ).bind(key_hash).fetch_optional(&self.pool).await.map_err(|_| ApplicationError::Storage)
            .map(|row| row.map(|(id, name, slug)| Organization { id, name, slug }))
    }
}

#[async_trait]
impl kusi_application::CapabilityRegistry for PgOrganizationRepository {
    async fn register(
        &self,
        organization: uuid::Uuid,
        c: &kusi_domain::Capability,
    ) -> Result<(), ApplicationError> {
        sqlx::query("INSERT INTO capabilities (organization_id, identifier, version, description, adapter, required_permission, enabled, timeout_ms, input_schema, output_schema) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (organization_id, identifier, version) DO UPDATE SET description = EXCLUDED.description, adapter = EXCLUDED.adapter, required_permission = EXCLUDED.required_permission, enabled = EXCLUDED.enabled, timeout_ms = EXCLUDED.timeout_ms, input_schema = EXCLUDED.input_schema, output_schema = EXCLUDED.output_schema")
   .bind(organization).bind(&c.identifier).bind(&c.version).bind(&c.description).bind(&c.adapter).bind(&c.required_permission).bind(c.enabled).bind(i64::from(c.timeout_ms) as i32).bind(c.input_schema.to_string()).bind(c.output_schema.to_string()).execute(&self.pool).await.map_err(|_| ApplicationError::Storage)?;
        Ok(())
    }
    async fn list(
        &self,
        organization: uuid::Uuid,
    ) -> Result<Vec<kusi_domain::Capability>, ApplicationError> {
        let rows = sqlx::query_as::<_, (String,String,String,String,String,bool,i32,String,String)>("SELECT identifier, version, description, adapter, required_permission, enabled, timeout_ms, input_schema, output_schema FROM capabilities WHERE organization_id = $1 ORDER BY identifier, version").bind(organization).fetch_all(&self.pool).await.map_err(|_| ApplicationError::Storage)?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    identifier,
                    version,
                    description,
                    adapter,
                    required_permission,
                    enabled,
                    timeout_ms,
                    input_schema,
                    output_schema,
                )| kusi_domain::Capability {
                    operation: kusi_domain::Operation::Read,
                    identifier,
                    version,
                    description,
                    adapter,
                    required_permission,
                    enabled,
                    timeout_ms: timeout_ms as u32,
                    input_schema: serde_json::from_str(&input_schema)
                        .unwrap_or(serde_json::Value::Bool(false)),
                    output_schema: serde_json::from_str(&output_schema)
                        .unwrap_or(serde_json::Value::Bool(false)),
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kusi_domain::Organization;
    use uuid::Uuid;

    #[tokio::test]
    async fn postgres_repository_resolves_only_the_matching_tenant_key()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
            eprintln!("skipping PostgreSQL integration test: TEST_DATABASE_URL is not set");
            return Ok(());
        };
        let pool = PgPool::connect(&database_url).await?;
        sqlx::migrate!("../../migrations").run(&pool).await?;
        let repository = PgOrganizationRepository::new(pool.clone());
        let slug = format!("test-{}", Uuid::new_v4().simple());
        let organization = Organization::new("Integration Test Org".into(), slug)?;
        let key_hash = "test-key-hash-a";

        repository.create(&organization, key_hash).await?;
        let resolved = repository.find_by_api_key_hash(key_hash).await?;
        let unrelated = repository.find_by_api_key_hash("test-key-hash-b").await?;
        assert_eq!(resolved, Some(organization.clone()));
        assert_eq!(unrelated, None);

        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(organization.id)
            .execute(&pool)
            .await?;
        pool.close().await;
        Ok(())
    }
}

pub mod managed_integrations;
