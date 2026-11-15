//! Run explicitly with a disposable loopback TEST_DATABASE_URL.
use kusi_application::{
    CapabilityRegistry, CreateOrganization,
    external_access::{Provision, ProvisionRequest, provision},
    hash_api_key,
    integrations::{AuthorizationService, ExecutionError},
};
use kusi_domain::{Capability, Operation};
use kusi_infrastructure::PgOrganizationRepository;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a disposable loopback TEST_DATABASE_URL; creates an isolated schema"]
async fn postgres_external_acl_scope_expiry_revocation_and_audit()
-> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("TEST_DATABASE_URL")?;
    let url = url::Url::parse(&database_url)?;
    if !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) {
        return Err("loopback test database required".into());
    }
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    let schema = format!("external_acl_{}", Uuid::new_v4().simple());
    sqlx::QueryBuilder::<sqlx::Postgres>::new("CREATE SCHEMA ")
        .push(&schema)
        .build()
        .execute(&admin)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(&search_path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await?;
    let result: Result<(), Box<dyn std::error::Error>> = async {
        sqlx::migrate!("../../migrations").run(&pool).await?;
        let repo = PgOrganizationRepository::new(pool.clone());
        let org = CreateOrganization::new(repo.clone()).execute("Synthetic ACL".into(), "synthetic-acl".into(), "synthetic-key").await?;
        let key = hash_api_key("synthetic-key");
        let cap = Capability { operation: Operation::Read, identifier: "record.read".into(), version:"1".into(), description:"Synthetic".into(), adapter:"system-a".into(), required_permission:"record.read".into(), enabled:true, timeout_ms:1000, input_schema:json!({}), output_schema:json!({}) };
        repo.register(org.id, &cap).await?;
        let input = json!({"resource_ref":"record-a"});
        assert!(repo.authorize_resource(org.id,&key,&cap,&input).await?.is_none());
        let request = |action| ProvisionRequest { organization:org.id, principal_hash:key.clone(), adapter:"system-a".into(), action };
        provision(&repo,&request(Provision::Protect)).await?;
        assert!(matches!(repo.authorize_resource(org.id,&key,&cap,&input).await,Err(ExecutionError::Forbidden)));
        provision(&repo,&request(Provision::Link { subject:"synthetic-subject".into(),ttl_seconds:600,confirmed:true })).await?;
        let grant = || Provision::Grant { capability:"record.read".into(),version:"1".into(),resource_ref:"record-a".into(),ttl_seconds:300,confirmed:true };
        provision(&repo,&request(grant())).await?;
        assert!(repo.authorize_resource(org.id,&key,&cap,&input).await?.is_some());
        for (principal, version, resource) in [("other", "1", "record-a"),(&key,"2","record-a"),(&key,"1","record-b")] {
            let mut modified = cap.clone(); modified.version=version.into();
            assert!(matches!(repo.authorize_resource(org.id,principal,&modified,&json!({"resource_ref":resource})).await,Err(ExecutionError::Forbidden)));
        }
        assert!(matches!(repo.authorize_resource(org.id,&key,&cap,&json!({"resource_ref":"record-a","subject":"override"})).await,Err(ExecutionError::InvalidData)));
        // Another protected system requires its own identity and grant.
        let other = ProvisionRequest { adapter:"system-b".into(),..request(Provision::Protect) };
        provision(&repo,&other).await?;
        let mut other_cap=cap.clone();other_cap.adapter="system-b".into();
        assert!(matches!(repo.authorize_resource(org.id,&key,&other_cap,&input).await,Err(ExecutionError::Forbidden)));
        // Expired grant cannot authorize; direct SQL here only constructs test time.
        sqlx::query("UPDATE external_resource_grants SET created_at=now()-interval '2 hours', expires_at=now()-interval '1 hour'").execute(&pool).await?;
        assert!(matches!(repo.authorize_resource(org.id,&key,&cap,&input).await,Err(ExecutionError::Forbidden)));
        sqlx::query("UPDATE external_resource_grants SET expires_at=now()+interval '1 minute'").execute(&pool).await?;
        provision(&repo,&request(Provision::RevokeGrant { capability:"record.read".into(),version:"1".into(),resource_ref:"record-a".into() })).await?;
        assert!(matches!(repo.authorize_resource(org.id,&key,&cap,&input).await,Err(ExecutionError::Forbidden)));
        assert!(provision(&repo,&request(grant())).await.is_err());
        provision(&repo,&request(Provision::RevokeLink)).await?;
        assert!(provision(&repo,&request(Provision::Link { subject:"replacement".into(),ttl_seconds:600,confirmed:true })).await.is_err());
        let outcomes: Vec<String> = sqlx::query_scalar("SELECT outcome FROM execution_audit").fetch_all(&pool).await?;
        assert!(!outcomes.is_empty());assert!(outcomes.iter().all(|outcome|outcome=="succeeded"));
        Ok(())
    }.await;
    pool.close().await;
    sqlx::QueryBuilder::<sqlx::Postgres>::new("DROP SCHEMA ")
        .push(&schema)
        .push(" CASCADE")
        .build()
        .execute(&admin)
        .await?;
    admin.close().await;
    result
}
