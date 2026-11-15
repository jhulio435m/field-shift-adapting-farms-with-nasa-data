//! Real loopback HTTP + PostgreSQL, with an injected synthetic driven adapter.
//! No production setting enables this adapter or relaxes HTTPS/SSRF rules.
use super::*;
use axum::{Router, routing::post};
use kusi_application::{
    CapabilityRegistry, CreateOrganization,
    external_access::{Provision, ProvisionRequest, provision},
    integrations::ExecutionError,
};
use kusi_domain::{Capability, Operation};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone)]
struct SyntheticReader {
    organization: Uuid,
    invocations: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl ConnectorExecutor for SyntheticReader {
    async fn execute(&self, _: &str, input: &Value, _: u32) -> Result<Value, ExecutionError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        // One intentionally mislabelled record verifies output scope validation.
        let resource = if input["resource_ref"] == "record-mislabelled" {
            "different-resource"
        } else {
            input["resource_ref"]
                .as_str()
                .ok_or(ExecutionError::InvalidData)?
        };
        Ok(
            json!({"resource_ref":resource,"data":{"synthetic":true,"organization":self.organization}}),
        )
    }
}

async fn request(
    client: &reqwest::Client,
    base: &str,
    key: Option<&str>,
    body: &Value,
    expected: u16,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut request = client
        .post(format!("{base}/v1/capabilities/execute"))
        .json(body);
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request.send().await?;
    // Report only status codes on failure, never request bodies or credentials.
    assert_eq!(response.status().as_u16(), expected);
    if expected == 422 {
        return Ok(Value::Null);
    }
    Ok(response.json().await?)
}

#[tokio::test]
#[ignore = "requires disposable loopback TEST_DATABASE_URL and loopback HTTP sockets"]
async fn http_external_acl_acceptance() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("TEST_DATABASE_URL")?;
    let url = url::Url::parse(&database_url)?;
    if !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) {
        return Err("disposable loopback database required".into());
    }
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    let schema = format!("external_http_{}", Uuid::new_v4().simple());
    // The identifier is generated solely from a UUID; never from caller input.
    sqlx::QueryBuilder::<sqlx::Postgres>::new("CREATE SCHEMA ")
        .push(&schema)
        .build()
        .execute(&admin)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(3)
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
    let mut server = None;
    let result: Result<(), Box<dyn std::error::Error>> = async {
        sqlx::migrate!("../../migrations").run(&pool).await?;
        let repo = PgOrganizationRepository::new(pool.clone());
        let org = CreateOrganization::new(repo.clone()).execute("Synthetic HTTP A".into(),"synthetic-http-a".into(),"synthetic-http-key-a").await?;
        let other = CreateOrganization::new(repo.clone()).execute("Synthetic HTTP B".into(),"synthetic-http-b".into(),"synthetic-http-key-b").await?;
        let hash = hash_api_key("synthetic-http-key-a");
        let other_hash = hash_api_key("synthetic-http-key-b");
        let cap = Capability {
            operation: Operation::Read, identifier:"record.read".into(),version:"1".into(),description:"Synthetic resource".into(),adapter:"system-a".into(),required_permission:"record.read".into(),enabled:true,timeout_ms:1000,
            // Deliberately permissive registration: resource authorization must
            // independently reject extra arguments and a wrong output envelope.
            input_schema:json!({}),output_schema:json!({}),
        };
        for (organization, principal_hash) in [(org.id,&hash),(other.id,&other_hash)] {
            repo.register(organization,&cap).await?;
            sqlx::query("INSERT INTO capability_grants(organization_id,principal_hash,permission) VALUES($1,$2,'record.read')").bind(organization).bind(principal_hash).execute(&pool).await?;
            provision(&repo,&ProvisionRequest { organization,principal_hash:principal_hash.clone(),adapter:"system-a".into(),action:Provision::Protect }).await?;
        }
        let operation = |action| ProvisionRequest { organization:org.id,principal_hash:hash.clone(),adapter:"system-a".into(),action };
        let invocations = Arc::new(AtomicUsize::new(0));
        let count = invocations.clone();
        // Same authentication, deserialization, execution, authorization and error
        // mapping used by production; only driven connector composition differs.
        let app = Router::new().route("/v1/capabilities/execute",post(move |headers: HeaderMap, Json(body): Json<ExecutionRequest>| {
            let repo = repo.clone(); let count = count.clone();
            async move { execute(repo,headers,body,move |organization| SyntheticReader { organization,invocations:count }).await }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}",listener.local_addr()?);
        server = Some(tokio::spawn(async move { axum::serve(listener,app).await }));
        let client = reqwest::Client::builder().no_proxy().timeout(std::time::Duration::from_secs(5)).build()?;
        let read = json!({"identifier":"record.read","version":"1","input":{"resource_ref":"record-a"}});
        request(&client,&base,None,&read,401).await?;
        request(&client,&base,Some("unknown-key"),&read,401).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&read,403).await?;
        let repo = PgOrganizationRepository::new(pool.clone());
        provision(&repo,&operation(Provision::Link { subject:"synthetic-external-person".into(),ttl_seconds:600,confirmed:true })).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&read,403).await?;
        let grant = |resource: &str| Provision::Grant { capability:"record.read".into(),version:"1".into(),resource_ref:resource.into(),ttl_seconds:300,confirmed:true };
        provision(&repo,&operation(grant("record-a"))).await?;
        let output = request(&client,&base,Some("synthetic-http-key-a"),&read,200).await?;
        assert_eq!(output["data"]["resource_ref"],"record-a");
        assert_eq!(output["data"]["data"]["organization"],json!(org.id));
        assert_eq!(invocations.load(Ordering::SeqCst),1);
        request(&client,&base,Some("synthetic-http-key-b"),&read,403).await?;
        let mut changed = read.clone();changed["input"]["resource_ref"]=json!("record-b");
        request(&client,&base,Some("synthetic-http-key-a"),&changed,403).await?;
        changed=read.clone();changed["input"]["subject"]=json!("override");
        request(&client,&base,Some("synthetic-http-key-a"),&changed,400).await?;
        changed=read.clone();changed["organization"]=json!(other.id);
        request(&client,&base,Some("synthetic-http-key-a"),&changed,422).await?;
        let mut version2=cap.clone();version2.version="2".into();repo.register(org.id,&version2).await?;
        changed=read.clone();changed["version"]=json!("2");
        request(&client,&base,Some("synthetic-http-key-a"),&changed,403).await?;
        assert_eq!(invocations.load(Ordering::SeqCst),1);
        // Expiry uses server time and does not require sleeping.
        sqlx::query("UPDATE external_resource_grants SET created_at=now()-interval '2 hours',expires_at=now()-interval '1 hour'").execute(&pool).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&read,403).await?;
        sqlx::query("UPDATE external_resource_grants SET expires_at=now()+interval '1 minute'").execute(&pool).await?;
        // Existing global permission remains necessary even with an exact ACL.
        sqlx::query("DELETE FROM capability_grants WHERE organization_id=$1").bind(org.id).execute(&pool).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&read,403).await?;
        sqlx::query("INSERT INTO capability_grants(organization_id,principal_hash,permission) VALUES($1,$2,'record.read')").bind(org.id).bind(&hash).execute(&pool).await?;
        // Wrong output is rejected after exactly one additional invocation.
        provision(&repo,&operation(grant("record-mislabelled"))).await?;
        changed=read.clone();changed["input"]["resource_ref"]=json!("record-mislabelled");
        request(&client,&base,Some("synthetic-http-key-a"),&changed,400).await?;
        assert_eq!(invocations.load(Ordering::SeqCst),2);
        provision(&repo,&operation(Provision::RevokeGrant { capability:"record.read".into(),version:"1".into(),resource_ref:"record-a".into() })).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&read,403).await?;
        provision(&repo,&operation(Provision::RevokeLink)).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&changed,403).await?;
        assert_eq!(invocations.load(Ordering::SeqCst),2);
        // A policy storage error is unavailable, never an unprotected fallback.
        sqlx::query("ALTER TABLE external_resource_policies RENAME TO unavailable_policies").execute(&pool).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&read,503).await?;
        sqlx::query("ALTER TABLE unavailable_policies RENAME TO external_resource_policies").execute(&pool).await?;
        sqlx::query("UPDATE organization_api_keys SET revoked_at=now() WHERE organization_id=$1").bind(org.id).execute(&pool).await?;
        request(&client,&base,Some("synthetic-http-key-a"),&read,401).await?;
        assert_eq!(invocations.load(Ordering::SeqCst),2);
        let outcomes: Vec<String> = sqlx::query_scalar("SELECT outcome FROM execution_audit WHERE capability='record.read'").fetch_all(&pool).await?;
        for outcome in ["succeeded","forbidden","invalid_data","unavailable"] { assert!(outcomes.iter().any(|item|item==outcome)); }
        Ok(())
    }.await;
    if let Some(server) = server {
        server.abort();
        let _ = server.await;
    }
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
