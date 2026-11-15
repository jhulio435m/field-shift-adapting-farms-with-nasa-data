//! Trusted operator entrypoint. Input is a private JSON file, never command-line secrets.
use kusi_application::external_access::{ProvisionRequest, provision};
use kusi_infrastructure::PgOrganizationRepository;
use sqlx::postgres::PgPoolOptions;
use std::{env, io::Read, path::PathBuf};

#[tokio::main]
async fn main() {
    if run().await.is_err() {
        // Never print SQL errors, file contents, subjects or database credentials.
        eprintln!("External access provisioning failed; no operation was committed.");
        std::process::exit(1);
    }
    println!("External access operation committed.");
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env::var("KUSI_EXTERNAL_ACCESS_REQUEST_FILE")?);
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.len() > 8192 {
        return Err("invalid request file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("request file must be private".into());
        }
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(8193)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8192 {
        return Err("request file too large".into());
    }
    let request: ProvisionRequest = serde_json::from_slice(&bytes)?;
    request.validate()?;
    let database_url = env::var("DATABASE_URL")?;
    // This command is deliberately local-only; production provisioning needs a separate rollout.
    let parsed = url::Url::parse(&database_url)?;
    if !matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) {
        return Err("only loopback databases are supported".into());
    }
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    // Migrations are applied by the normal server lifecycle, never implicitly here.
    provision(&PgOrganizationRepository::new(pool.clone()), &request).await?;
    pool.close().await;
    Ok(())
}
