//! Explicitly opt-in migration/race acceptance; never uses a workspace database.
use kusi_application::{
    CreateOrganization,
    adaptive::Actor,
    connection_settings::{
        ConnectionInput, ConnectionKind, ConnectionSettingsRepository, CredentialPatch,
    },
    hash_api_key,
    managed_integrations::{FixtureConnectionResolver, ManagedConnections},
};
use kusi_domain::local::{LocalRole, LocalUser};
use kusi_infrastructure::{PgOrganizationRepository, local::LocalStore};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;
#[tokio::test]
#[ignore = "requires disposable loopback TEST_DATABASE_URL; creates and drops its own schema"]
async fn migrations_revision_races_expiry_and_tenant_foreign_keys()
-> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("TEST_DATABASE_URL")?;
    if !matches!(
        url::Url::parse(&url)?.host_str(),
        Some("127.0.0.1" | "localhost" | "[::1]")
    ) {
        return Err("loopback database required".into());
    }
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("managed_{}", Uuid::new_v4().simple());
    sqlx::QueryBuilder::<sqlx::Postgres>::new("CREATE SCHEMA ")
        .push(&schema)
        .build()
        .execute(&admin)
        .await?;
    let search = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .after_connect(move |connection, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    let root = std::env::temp_dir().join(format!("kusi-managed-test-{}", Uuid::new_v4()));
    let result:Result<(),Box<dyn std::error::Error>>=async{
  sqlx::migrate!("../../migrations").run(&pool).await?;
  let repo=PgOrganizationRepository::new(pool.clone());
  let org=CreateOrganization::new(repo.clone()).execute("Synthetic".into(),format!("managed-{}",Uuid::new_v4().simple()),"synthetic-key").await?;
  let other=CreateOrganization::new(repo).execute("Other".into(),format!("other-{}",Uuid::new_v4().simple()),"other-synthetic-key").await?;
  let user=LocalUser{id:Uuid::new_v4(),organization_id:org.id,organization:org.slug,username:"synthetic".into(),display_name:"Synthetic".into(),role:LocalRole::Owner,platform_admin:false,disabled:false};
  sqlx::query("INSERT INTO local_users(id,organization_id,username,display_name,password_hash,role) VALUES($1,$2,'synthetic','Synthetic','unused','owner')").bind(user.id).bind(org.id).execute(&pool).await?;
  let store=LocalStore::new(pool.clone(),root.clone()).await?;
  let input=|revision|ConnectionInput{label:"Plane fixture".into(),base_url:Some("https://synthetic.example.invalid".into()),workspace:None,bot_username:None,webhook_url:None,expected_revision:revision,credentials:CredentialPatch{api_key:Some("synthetic-plane-key".into()),..Default::default()}};
  store.save_connection_settings(&user,ConnectionKind::Plane,&input(None)).await?;
  store.verify_fixture(&user,ConnectionKind::Plane,1).await?;
  let actor=Actor{organization:org.id,principal:hash_api_key("synthetic-key"),read:true,subscribe:false};
  assert_eq!(store.fixture_revision(&actor,ConnectionKind::Plane).await?,1);
  let profile:Uuid=sqlx::query_scalar("SELECT id FROM managed_connection_settings WHERE organization_id=$1").bind(org.id).fetch_one(&pool).await?;
  sqlx::query("INSERT INTO managed_connection_verifications(id,organization_id,profile_id,profile_revision,adapter_version,mode,status) VALUES($1,$2,$3,1,'1','fixture','pending')").bind(Uuid::new_v4()).bind(org.id).bind(profile).execute(&pool).await?;
  store.save_connection_settings(&user,ConnectionKind::Plane,&input(Some(1))).await?;
  let all_stale:bool=sqlx::query_scalar("SELECT bool_and(status='stale') FROM managed_connection_verifications").fetch_one(&pool).await?;assert!(all_stale);
  let left=input(Some(2));let right=input(Some(2));
  let (a,b)=tokio::join!(store.save_connection_settings(&user,ConnectionKind::Plane,&left),store.save_connection_settings(&user,ConnectionKind::Plane,&right));
  assert_ne!(a.is_ok(),b.is_ok(),"CAS allowed two concurrent writes");
  store.verify_fixture(&user,ConnectionKind::Plane,3).await?;
  sqlx::query("UPDATE managed_connection_verifications SET expires_at=checked_at WHERE profile_revision=3").execute(&pool).await?;
  assert_eq!(store.connection_status(&user,ConnectionKind::Plane).await?.status,"not_verified");
  assert!(store.fixture_revision(&actor,ConnectionKind::Plane).await.is_err());
  let cross=sqlx::query("INSERT INTO managed_connection_verifications(id,organization_id,profile_id,profile_revision,adapter_version,mode,status) VALUES($1,$2,$3,3,'1','fixture','verified')").bind(Uuid::new_v4()).bind(other.id).bind(profile).execute(&pool).await;
  assert!(cross.is_err(),"cross-tenant profile reference accepted");
  let restarted=LocalStore::new(pool.clone(),root.clone()).await?;
  assert_eq!(restarted.list_connection_settings(&user).await?[0].revision,3);
  Ok(())
 }.await;
    pool.close().await;
    sqlx::QueryBuilder::<sqlx::Postgres>::new("DROP SCHEMA ")
        .push(schema)
        .push(" CASCADE")
        .build()
        .execute(&admin)
        .await?;
    admin.close().await;
    if root.exists() {
        std::fs::remove_dir_all(root)?;
    }
    result
}
