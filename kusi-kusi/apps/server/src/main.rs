mod admin_auth;
mod capabilities;
mod channels;
mod creative;
mod local;
mod oauth;
use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kusi_application::{CreateOrganization, GetOrganizationByKey, hash_api_key};
use kusi_infrastructure::PgOrganizationRepository;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use subtle::ConstantTimeEq;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    local: local::Settings,
    admin_auth: admin_auth::Settings,
    repository: PgOrganizationRepository,
    pool: PgPool,
    admin_key_hash: String,
    channels: std::collections::HashMap<Uuid, channels::Channel>,
    creative_recipes: creative::Recipes,
    oauth: oauth::Settings,
    creative_worker_enabled: bool,
    llm_profiles: kusi_infrastructure::llm::Profiles,
    audio_profiles: kusi_infrastructure::audio::AudioProfiles,
    destinations: std::collections::HashMap<Uuid, std::collections::HashMap<String, String>>,
    allowed_hosts: Vec<String>,
    system_profiles: std::sync::Arc<kusi_infrastructure::system_connectors::SystemProfiles>,
}

#[derive(Deserialize)]
struct CreateOrganizationRequest {
    name: String,
    slug: String,
}

#[derive(Serialize)]
struct CreateOrganizationResponse {
    organization: kusi_domain::Organization,
    api_key: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "service": "kusi-server"}))
}

async fn ready(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Unavailable)?;
    Ok(Json(
        serde_json::json!({"status": "ready", "database": "ok"}),
    ))
}

async fn create_organization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateOrganizationRequest>,
) -> Result<(StatusCode, Json<CreateOrganizationResponse>), ApiError> {
    authorize_admin(&state, &headers)?;
    let api_key = format!("kk_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let organization = CreateOrganization::new(state.repository.clone())
        .execute(input.name, input.slug, &api_key)
        .await
        .map_err(ApiError::from)?;
    Ok((
        StatusCode::CREATED,
        Json(CreateOrganizationResponse {
            organization,
            api_key,
        }),
    ))
}

async fn current_organization(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<kusi_domain::Organization>, ApiError> {
    let key = bearer_token(&headers).ok_or(ApiError::Unauthorized)?;
    let organization = GetOrganizationByKey::new(state.repository)
        .execute(key)
        .await
        .map_err(|error| match error {
            kusi_application::ApplicationError::NotFound => ApiError::Unauthorized,
            other => ApiError::from(other),
        })?;
    Ok(Json(organization))
}

async fn capabilities(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<kusi_domain::Capability>>, ApiError> {
    use kusi_application::CapabilityRegistry;
    let key = bearer_token(&headers).ok_or(ApiError::Unauthorized)?;
    let org = GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    Ok(Json(
        state
            .repository
            .list(org.id)
            .await
            .map_err(ApiError::from)?,
    ))
}

async fn register_capability(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(capability): Json<kusi_domain::Capability>,
) -> Result<StatusCode, ApiError> {
    use kusi_application::CapabilityRegistry;
    authorize_admin(&state, &headers)?;
    let key = bearer_token(&headers).ok_or(ApiError::Unauthorized)?;
    let org = GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    if !capability.is_valid()
        || !kusi_infrastructure::schema_allowed(&capability.input_schema)
        || !kusi_infrastructure::schema_allowed(&capability.output_schema)
    {
        return Err(ApiError::BadRequest);
    }
    state
        .repository
        .register(org.id, &capability)
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn execute_capability(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<capabilities::ExecutionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    capabilities::execute(state.repository, headers, request, move |organization| {
        kusi_infrastructure::system_connectors::RuntimeConnector {
            organization,
            systems: state.system_profiles,
            http: kusi_infrastructure::http_connector::HttpConnector {
                destinations: state
                    .destinations
                    .get(&organization)
                    .cloned()
                    .unwrap_or_default(),
                allowed_hosts: state.allowed_hosts,
            },
        }
    })
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantRequest {
    permission: String,
}
async fn grant_permission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GrantRequest>,
) -> Result<StatusCode, ApiError> {
    authorize_admin(&state, &headers)?;
    let key = bearer_token(&headers).ok_or(ApiError::Unauthorized)?;
    let organization = GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    if request.permission.is_empty() || request.permission.len() > 120 {
        return Err(ApiError::BadRequest);
    }
    sqlx::query("INSERT INTO capability_grants (organization_id, principal_hash, permission) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING").bind(organization.id).bind(hash_api_key(key)).bind(request.permission).execute(&state.pool).await.map_err(|_| ApiError::Unavailable)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GenerateRequest {
    profile: String,
    request: kusi_application::llm::InferenceRequest,
}
async fn generate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<GenerateRequest>,
) -> Result<Json<kusi_application::llm::InferenceResult>, ApiError> {
    use kusi_application::llm::LlmProvider;
    authorize_admin(&state, &headers)?;
    let key = bearer_token(&headers).ok_or(ApiError::Unauthorized)?;
    let org = GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    let profile = state
        .llm_profiles
        .get(&org.id)
        .and_then(|profiles| profiles.get(&input.profile))
        .ok_or(ApiError::Forbidden)?;
    let result = profile
        .generate(&input.request)
        .await
        .map_err(|error| match error {
            kusi_application::llm::LlmError::InvalidRequest => ApiError::BadRequest,
            _ => ApiError::Unavailable,
        })?;
    Ok(Json(result))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TranscriptionQuery {
    profile: String,
    filename: String,
    language: Option<String>,
}
async fn transcribe(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<TranscriptionQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<kusi_application::audio::Transcript>, ApiError> {
    use kusi_application::audio::{AudioInput, Transcriber};
    authorize_admin(&state, &headers)?;
    let key = bearer_token(&headers).ok_or(ApiError::Unauthorized)?;
    let org = GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    let profile = state
        .audio_profiles
        .get(&org.id)
        .and_then(|p| p.get(&query.profile))
        .ok_or(ApiError::Forbidden)?;
    let result = profile
        .transcribe(AudioInput {
            filename: query.filename,
            bytes: body.to_vec(),
            language: query.language,
        })
        .await
        .map_err(|e| match e {
            kusi_application::llm::LlmError::InvalidRequest => ApiError::BadRequest,
            _ => ApiError::Unavailable,
        })?;
    Ok(Json(result))
}

async fn openapi() -> Response<Body> {
    (
        [(axum::http::header::CONTENT_TYPE, "application/yaml")],
        include_str!("../../../contracts/openapi/openapi.yaml"),
    )
        .into_response()
}

fn authorize_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    if headers
        .get("x-kusi-local-admin")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| bool::from(v.as_bytes().ct_eq(state.local.marker.as_bytes())))
    {
        return Ok(());
    }
    let presented = headers
        .get("x-admin-key")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let presented_hash = hash_api_key(presented);
    if bool::from(
        presented_hash
            .as_bytes()
            .ct_eq(state.admin_key_hash.as_bytes()),
    ) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

#[derive(Debug)]
enum ApiError {
    Unauthorized,
    Forbidden,
    BadRequest,
    Conflict,
    NotFound,
    Unavailable,
}

impl From<kusi_application::ApplicationError> for ApiError {
    fn from(error: kusi_application::ApplicationError) -> Self {
        match error {
            kusi_application::ApplicationError::Domain(_) => Self::BadRequest,
            kusi_application::ApplicationError::Conflict => Self::Conflict,
            kusi_application::ApplicationError::NotFound => Self::NotFound,
            kusi_application::ApplicationError::Storage => Self::Unavailable,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "operation forbidden"),
            Self::BadRequest => (StatusCode::BAD_REQUEST, "invalid organization"),
            Self::Conflict => (StatusCode::CONFLICT, "organization slug already exists"),
            Self::NotFound => (StatusCode::NOT_FOUND, "organization not found"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "service unavailable"),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")?;
    let admin_key = std::env::var("KUSI_ADMIN_API_KEY")?;
    if admin_key.len() < 32 {
        return Err("KUSI_ADMIN_API_KEY must contain at least 32 characters".into());
    }
    let bind_address =
        std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    let destinations: std::collections::HashMap<Uuid, std::collections::HashMap<String, String>> =
        match std::env::var("KUSI_HTTP_DESTINATIONS") {
            Ok(value) => serde_json::from_str(&value)?,
            Err(std::env::VarError::NotPresent) => std::collections::HashMap::new(),
            Err(error) => return Err(error.into()),
        };
    let allowed_hosts = std::env::var("KUSI_HTTP_ALLOWED_HOSTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect();
    let llm_profiles: kusi_infrastructure::llm::Profiles = match std::env::var("KUSI_LLM_PROFILES")
    {
        Ok(value) => serde_json::from_str(&value)?,
        Err(std::env::VarError::NotPresent) => std::collections::HashMap::new(),
        Err(error) => return Err(error.into()),
    };
    for profiles in llm_profiles.values() {
        for profile in profiles.values() {
            profile.validate()?;
        }
    }
    let audio_profiles: kusi_infrastructure::audio::AudioProfiles =
        match std::env::var("KUSI_AUDIO_PROFILES") {
            Ok(value) => serde_json::from_str(&value)?,
            Err(std::env::VarError::NotPresent) => std::collections::HashMap::new(),
            Err(error) => return Err(error.into()),
        };
    for profiles in audio_profiles.values() {
        for profile in profiles.values() {
            profile.validate()?;
        }
    }
    let creative_simulation = std::env::var("KUSI_CREATIVE_SIMULATION")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()?;
    let creative_worker_enabled = std::env::var("KUSI_CREATIVE_WORKER")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()?;
    let creative_recipes = creative::load(creative_simulation)?;
    let local = local::load(pool.clone(), &bind_address).await?;
    let system_hosts: Vec<String> = std::env::var("KUSI_SYSTEM_ALLOWED_HOSTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .map(str::to_owned)
        .collect();
    let allow_private_systems = std::env::var("KUSI_SYSTEM_PRIVATE_HTTP")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()?;
    let system_profiles = std::sync::Arc::new(
        kusi_infrastructure::system_connectors::SystemProfiles::load(
            &std::env::var("KUSI_SYSTEM_PROFILES").unwrap_or_else(|_| "{}".into()),
            &system_hosts,
            allow_private_systems,
            local.store.is_some(),
        )?,
    );
    for (organization, adapters) in &destinations {
        if adapters
            .keys()
            .any(|adapter| system_profiles.contains(*organization, adapter))
        {
            return Err("system and generic HTTP adapter identifiers must not overlap".into());
        }
    }
    let admin_auth = admin_auth::Settings::load(local.store.is_some()).await?;
    if local.store.is_some()
        && creative_recipes
            .values()
            .any(|r| r.values().any(|r| r.provider != "simulated"))
    {
        return Err("local mode only allows simulated creative recipes".into());
    }
    let local_worker = local.store.clone();
    let creative_worker_repository = PgOrganizationRepository::new(pool.clone());
    let oauth = oauth::load()?;
    let channels = channels::load()?;
    let oauth_maintenance = (oauth.clone(), pool.clone());
    let state = AppState {
        local,
        admin_auth,
        oauth,
        creative_recipes,
        creative_worker_enabled,
        llm_profiles,
        audio_profiles,
        channels,
        destinations,
        allowed_hosts,
        system_profiles,
        repository: PgOrganizationRepository::new(pool.clone()),
        pool,
        admin_key_hash: hash_api_key(&admin_key),
    };

    let app = Router::new()
        .merge(admin_auth::routes())
        .merge(local::routes())
        .route("/health/live", get(health))
        .route("/v1/oauth/{provider}/start", post(oauth::start))
        .route("/v1/oauth/{provider}/callback", get(oauth::callback))
        .route("/v1/oauth/connections", get(oauth::list))
        .route("/v1/oauth/connections/{id}/refresh", post(oauth::refresh))
        .route("/v1/oauth/connections/{id}/disconnect", post(oauth::disconnect))
        .route("/v1/creative/recipes", get(creative::recipes))
        .route("/v1/creative/budget", get(creative::budget).post(creative::set_budget))
        .route("/v1/creative/jobs", get(creative::list).post(creative::create))
        .route("/v1/creative/jobs/{id}", get(creative::get))
        .route("/v1/creative/jobs/{id}/approve", post(creative::approve))
        .route("/v1/creative/jobs/{id}/cancel", post(creative::cancel))
        .route("/v1/ai/generate", post(generate))
        .route("/v1/ai/transcribe", post(transcribe).layer(axum::extract::DefaultBodyLimit::max(10*1024*1024)))
        .route(
            "/webhooks/{id}",
            get(channels::verify).post(channels::receive),
        )
        .route("/v1/channels/{id}/send", post(channels::send))
        .route("/v1/capabilities/execute", post(execute_capability))
        .route("/v1/capabilities/grants", post(grant_permission))
        .route(
            "/v1/capabilities",
            get(capabilities).post(register_capability),
        )
        .route("/health/ready", get(ready))
        .route("/openapi.yaml", get(openapi))
        .route("/v1/organizations", post(create_organization))
        .route("/v1/organizations/current", get(current_organization))
        .layer(middleware::from_fn_with_state(state.clone(), local::guard))
        .layer(middleware::from_fn(request_id))
        .layer(TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<Body>| tracing::info_span!("http_request", method = %request.method(), path = %request.uri().path())))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    if creative_worker_enabled {
        tokio::spawn(kusi_infrastructure::creative::run_worker(
            creative_worker_repository,
            kusi_infrastructure::creative::OfficialCreativeProvider {
                allow_simulation: creative_simulation,
            },
        ));
    }
    if let Some(store) = local_worker {
        tokio::spawn(kusi_infrastructure::local::run_worker(store, 3));
    } else {
        oauth::spawn_maintenance(oauth_maintenance.0, oauth_maintenance.1);
    }
    tracing::info!(address = %bind_address, "Kusi Kusi API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn request_id(mut request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    if let Ok(value) = request_id.parse() {
        request.headers_mut().insert("x-request-id", value);
    }
    let mut response = next.run(request).await;
    if let Ok(value) = request_id.parse() {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
