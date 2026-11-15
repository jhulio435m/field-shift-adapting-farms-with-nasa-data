//! Explicit staging composition. Organization/source/identity never come from incoming text.
use kusi_application::adaptive::{Actor, Assistant, AuthorizedEnvironment, Environment, Event};
use kusi_domain::adaptive::{Error, Reply};
use kusi_infrastructure::adaptive::{FileState, LanguagePlanner, RuntimeConfig, Sources};
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    event: Option<Event>,
}
async fn run() -> Result<Reply, Error> {
    if std::env::var("KUSI_ADAPTIVE_STAGING_ACK").as_deref() != Ok("authorized-environment")
        || std::env::var("KUSI_LOCAL_MODE").as_deref() == Ok("true")
    {
        return Err(Error::Forbidden);
    }
    let path = std::env::args().nth(1).ok_or(Error::Invalid)?;
    let config = RuntimeConfig::load(std::path::Path::new(&path))?;
    if std::env::var("KUSI_ADAPTIVE_REQUIRE_ACL").as_deref() == Ok("true")
        && config.access.is_none()
    {
        return Err(Error::Forbidden);
    }
    let mut raw = Vec::new();
    std::io::stdin()
        .take(65537)
        .read_to_end(&mut raw)
        .map_err(|_| Error::Invalid)?;
    if raw.len() > 65536 {
        return Err(Error::Invalid);
    }
    let input: Input = serde_json::from_slice(&raw).map_err(|_| Error::Invalid)?;
    let actor = config.actor();
    let managed_fixture = config.managed_fixture;
    let sources = Sources::new(actor.clone(), config.sources, config.bridge_script);
    let planner = LanguagePlanner {
        profile: config.llm,
    };
    let state = FileState {
        root: config.state_directory,
    };
    if let Some(access) = config.access {
        let url = std::env::var(&access.database_url_env).map_err(|_| Error::Unavailable)?;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .map_err(|_| Error::Unavailable)?;
        let repository = kusi_infrastructure::PgOrganizationRepository::new(pool.clone());
        let result = if let Some(fixture) = managed_fixture {
            let store = kusi_infrastructure::local::LocalStore::new(
                pool.clone(),
                fixture.storage_directory,
            )
            .await
            .map_err(|_| Error::Unavailable)?;
            let sources = sources.with_managed_fixture(store, fixture.kind)?;
            let environment =
                AuthorizedEnvironment::new(sources, repository, actor.clone(), access.resources)?;
            respond(environment, planner, state, &actor, input).await
        } else {
            let environment =
                AuthorizedEnvironment::new(sources, repository, actor.clone(), access.resources)?;
            respond(environment, planner, state, &actor, input).await
        };
        pool.close().await;
        result
    } else {
        respond(sources, planner, state, &actor, input).await
    }
}

async fn respond<E: Environment>(
    environment: E,
    planner: LanguagePlanner,
    state: FileState,
    actor: &Actor,
    input: Input,
) -> Result<Reply, Error> {
    let engine = Assistant {
        environment,
        planner,
        state,
    };
    if let Some(event) = input.event {
        if input.message.is_some() || input.request_id.is_some() {
            return Err(Error::Invalid);
        }
        engine.event(actor, &event).await
    } else {
        let request = input
            .request_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        engine
            .handle(
                actor,
                &input.message.ok_or(Error::Invalid)?,
                &request,
                chrono::Local::now().date_naive(),
            )
            .await
    }
}
#[tokio::main]
async fn main() {
    let result = tokio::time::timeout(std::time::Duration::from_secs(90), run()).await;
    let reply = match result {
        Ok(Ok(reply)) => reply,
        Ok(Err(Error::Forbidden)) => Reply::message(
            "blocked",
            "No existe autorización vigente para este entorno o recurso.",
        ),
        Ok(Err(Error::Limited)) => Reply::message(
            "deferred",
            "Se alcanzó el límite del proveedor. Conserva el objetivo y vuelve a solicitarlo cuando se restablezca la cuota.",
        ),
        Ok(Err(Error::Incomplete)) => Reply::message(
            "clarification",
            "El catálogo o los datos exceden el límite de ejecución. Acota la fuente o el periodo para obtener un resultado completo.",
        ),
        Ok(Err(Error::Invalid)) => Reply::message(
            "clarification",
            "La petición o configuración no cumple el contrato del entorno. No se ejecutó un plan válido.",
        ),
        _ => Reply::message(
            "unavailable",
            "No pude completar la operación dentro de los límites configurados. No voy a inventar un resultado ni reintentarlo automáticamente.",
        ),
    };
    match serde_json::to_string(&reply) {
        Ok(json) => println!("{json}"),
        Err(_) => std::process::exit(1),
    }
}
