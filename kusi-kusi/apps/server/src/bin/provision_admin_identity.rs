use kusi_domain::admin_identity::AdminRole;
use sqlx::postgres::PgPoolOptions;
use std::env;
use url::Url;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Admin identity provisioning failed: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let issuer = env::var("KUSI_ADMIN_PROVISION_ISSUER")?;
    let subject = env::var("KUSI_ADMIN_PROVISION_SUBJECT")?;
    let organization_slug = env::var("KUSI_ADMIN_PROVISION_ORGANIZATION")?;
    let role_value = env::var("KUSI_ADMIN_PROVISION_ROLE")?;
    let role = AdminRole::parse(&role_value).ok_or("Unsupported admin role")?;
    if subject.is_empty() || subject.len() > 255 || issuer.len() > 2048 {
        return Err("Issuer or subject length is invalid".into());
    }
    validate_issuer(&issuer)?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    let organization_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM organizations WHERE slug=$1")
            .bind(&organization_slug)
            .fetch_optional(&pool)
            .await?
            .ok_or("Organization not found")?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO admin_identities(issuer,subject) VALUES ($1,$2) \
         ON CONFLICT (issuer,subject) DO NOTHING",
    )
    .bind(&issuer)
    .bind(&subject)
    .execute(&mut *tx)
    .await?;
    let membership = sqlx::query(
        "INSERT INTO admin_memberships(issuer,subject,organization_id,role) \
         SELECT $1,$2,$3,$4 WHERE EXISTS (SELECT 1 FROM admin_identities \
           WHERE issuer=$1 AND subject=$2 AND disabled_at IS NULL) \
         ON CONFLICT (issuer,subject,organization_id) DO NOTHING",
    )
    .bind(&issuer)
    .bind(&subject)
    .bind(organization_id)
    .bind(role.as_str())
    .execute(&mut *tx)
    .await?;
    if membership.rows_affected() != 1 {
        return Err("Identity membership already exists; no changes were made".into());
    }
    tx.commit().await?;
    pool.close().await;
    println!("Administrative identity provisioned.");
    Ok(())
}

fn validate_issuer(issuer: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse(issuer)?;
    let host = url.host_str().unwrap_or_default();
    let loopback = host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1";
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || (url.scheme() == "http" && loopback))
    {
        return Err("Issuer must use HTTPS except for loopback test issuers".into());
    }
    Ok(())
}
