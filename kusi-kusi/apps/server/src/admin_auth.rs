use crate::AppState;
use axum::{
    Json, Router,
    body::Body,
    extract::{Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use hmac::{Hmac, Mac};
use kusi_application::{admin_identity::*, hash_api_key};
use kusi_infrastructure::{admin_identity::PgAdminIdentityRepository, admin_oidc::OidcProvider};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct Settings {
    pub local_mode: bool,
    pub origin: String,
    csrf_secret: String,
    state_key: [u8; 32],
    provider: Option<Arc<OidcProvider>>,
}

impl Settings {
    pub async fn load(local_mode: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let oidc_keys = [
            "KUSI_ADMIN_OIDC_ISSUER",
            "KUSI_ADMIN_OIDC_CLIENT_ID",
            "KUSI_ADMIN_OIDC_CLIENT_SECRET",
            "KUSI_ADMIN_OIDC_REDIRECT_URI",
        ];
        let oidc_configured = oidc_keys.iter().any(|key| std::env::var(key).is_ok());
        if local_mode && oidc_configured {
            return Err("OIDC must remain disabled in local mode".into());
        }
        let configured_origin = std::env::var("KUSI_ADMIN_PUBLIC_ORIGIN").ok();
        if oidc_configured && configured_origin.is_none() {
            return Err("KUSI_ADMIN_PUBLIC_ORIGIN is required when OIDC is enabled".into());
        }
        let public_origin = configured_origin
            .or_else(|| std::env::var("KUSI_LOCAL_ORIGIN").ok())
            .unwrap_or_else(|| "http://127.0.0.1:3000".to_owned());
        validate_public_origin(&public_origin)?;
        if oidc_configured
            && !public_origin.starts_with("https://")
            && !is_loopback_origin(&public_origin)
        {
            return Err("OIDC admin sessions require an HTTPS public origin".into());
        }
        if oidc_configured {
            let callback = std::env::var("KUSI_ADMIN_OIDC_REDIRECT_URI")?;
            let expected = format!(
                "{}/v1/admin/auth/oidc/callback",
                public_origin.trim_end_matches('/')
            );
            if callback != expected {
                return Err("OIDC callback must match the configured admin public origin".into());
            }
        }
        let state_key = if oidc_configured {
            let encoded = std::env::var("KUSI_ADMIN_OIDC_STATE_KEY")?;
            let decoded = hex::decode(encoded)?;
            decoded
                .try_into()
                .map_err(|_| "KUSI_ADMIN_OIDC_STATE_KEY must decode to 32 bytes")?
        } else {
            let random = kusi_infrastructure::oauth::random_secret()?;
            let digest = Sha256::digest(random.as_bytes());
            let mut key = [0_u8; 32];
            key.copy_from_slice(&digest);
            key
        };
        let provider = OidcProvider::load(local_mode).await?.map(Arc::new);
        let csrf_secret = kusi_infrastructure::oauth::random_secret()?;
        Ok(Self {
            local_mode,
            origin: public_origin,
            csrf_secret,
            state_key,
            provider,
        })
    }
}

fn is_loopback_origin(raw: &str) -> bool {
    url::Url::parse(raw).ok().is_some_and(|url| {
        url.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
        })
    })
}

fn validate_public_origin(raw: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = url::Url::parse(raw)?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/"
        || !(parsed.scheme() == "https" || (parsed.scheme() == "http" && is_loopback_origin(raw)))
    {
        return Err("admin public origin must be HTTPS or loopback HTTP origin".into());
    }
    Ok(())
}

fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn csrf_token(secret: &str, session: &str) -> Result<String, StatusCode> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    mac.update(session.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn equal(left: &str, right: &str) -> bool {
    bool::from(left.as_bytes().ct_eq(right.as_bytes()))
}

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| part.trim().strip_prefix(&format!("{name}=")))
}

fn cookie_header(
    name: &str,
    value: &str,
    path: &str,
    max_age: i64,
    secure: bool,
) -> Result<HeaderValue, StatusCode> {
    let secure_flag = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{name}={value}; HttpOnly; SameSite=Lax; Path={path}; Max-Age={max_age}{secure_flag}"
    ))
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

fn is_secure_cookie(settings: &Settings) -> bool {
    settings.origin.starts_with("https://")
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/auth/mode", get(mode))
        .route("/v1/admin/auth/oidc/start", get(start))
        .route("/v1/admin/auth/oidc/callback", get(callback))
        .route("/v1/admin/auth/session", get(session))
        .route("/v1/admin/auth/logout", post(logout))
        .layer(middleware::from_fn(private_headers))
}

async fn private_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn mode(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "mode": if state.local.store.is_some() { "local" } else { "admin" },
        "oidc_enabled": state.local.store.is_none() && state.admin_auth.provider.is_some()
    }))
}

async fn start(State(state): State<AppState>) -> Result<Response, StatusCode> {
    if state.local.store.is_some() || state.admin_auth.local_mode {
        return Err(StatusCode::NOT_FOUND);
    }
    let provider = state
        .admin_auth
        .provider
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let login = provider.begin();
    let browser =
        kusi_infrastructure::oauth::random_secret().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let now = unix_time()?;
    let attempt = AdminLoginAttempt {
        state_hash: digest(&login.state),
        browser_hash: digest(&browser),
        nonce: login.nonce,
        code_verifier: login.code_verifier,
        expires_at: now.saturating_add(600),
    };
    PgAdminIdentityRepository::new(state.pool.clone(), state.admin_auth.state_key)
        .create_attempt(&attempt)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut response = Redirect::temporary(&login.url).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie_header(
            "kusi_admin_oidc_browser",
            &browser,
            "/v1/admin/auth/oidc",
            600,
            is_secure_cookie(&state.admin_auth),
        )?,
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    #[serde(rename = "iss")]
    issuer: Option<String>,
}

async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Response {
    match callback_inner(state.clone(), headers, query).await {
        Ok(response) => response,
        Err(_) => {
            let mut response = Redirect::temporary("/admin?error=login_failed").into_response();
            if let Ok(value) = cookie_header(
                "kusi_admin_oidc_browser",
                "",
                "/v1/admin/auth/oidc",
                0,
                is_secure_cookie(&state.admin_auth),
            ) {
                response.headers_mut().insert(header::SET_COOKIE, value);
            }
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
    }
}

async fn callback_inner(
    state: AppState,
    headers: HeaderMap,
    query: CallbackQuery,
) -> Result<Response, StatusCode> {
    if state.local.store.is_some() || state.admin_auth.local_mode {
        return Err(StatusCode::NOT_FOUND);
    }
    let provider = state
        .admin_auth
        .provider
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let browser = cookie(&headers, "kusi_admin_oidc_browser").ok_or(StatusCode::BAD_REQUEST)?;
    let state_value = query
        .state
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let attempt = PgAdminIdentityRepository::new(state.pool.clone(), state.admin_auth.state_key)
        .consume_attempt(&digest(&state_value), &digest(browser))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if query.error.is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let code = query
        .code
        .filter(|value| !value.is_empty() && value.len() <= 4096)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let verified = provider
        .complete(code, attempt.code_verifier, attempt.nonce)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if query
        .issuer
        .as_deref()
        .is_some_and(|issuer| issuer != verified.issuer)
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let repository = PgAdminIdentityRepository::new(state.pool.clone(), state.admin_auth.state_key);
    let identity = resolve_identity(&repository, &verified.issuer, &verified.subject)
        .await
        .map_err(|error| match error {
            AdminIdentityError::Forbidden => StatusCode::FORBIDDEN,
            AdminIdentityError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        })?;
    let session_token =
        kusi_infrastructure::oauth::random_secret().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    repository
        .create_session(
            &hash_api_key(&session_token),
            &identity.issuer,
            &identity.subject,
            unix_time()?.saturating_add(8 * 60 * 60),
        )
        .await
        .map_err(|error| match error {
            AdminIdentityError::Forbidden => StatusCode::FORBIDDEN,
            AdminIdentityError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        })?;
    let mut response = Redirect::temporary("/admin").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie_header(
            "kusi_admin_session",
            &session_token,
            "/",
            8 * 60 * 60,
            is_secure_cookie(&state.admin_auth),
        )?,
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        cookie_header(
            "kusi_admin_oidc_browser",
            "",
            "/v1/admin/auth/oidc",
            0,
            is_secure_cookie(&state.admin_auth),
        )?,
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

fn session_cookie(headers: &HeaderMap) -> Result<String, StatusCode> {
    cookie(headers, "kusi_admin_session")
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or(StatusCode::UNAUTHORIZED)
}

async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = session_cookie(&headers)?;
    let current = current_session(
        &PgAdminIdentityRepository::new(state.pool.clone(), state.admin_auth.state_key),
        &hash_api_key(&token),
    )
    .await
    .map_err(|error| match error {
        AdminIdentityError::Forbidden => StatusCode::UNAUTHORIZED,
        AdminIdentityError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    let csrf = csrf_token(&state.admin_auth.csrf_secret, &token)?;
    Ok(Json(serde_json::json!({
        "memberships": current.identity.memberships,
        "csrf_token": csrf,
        "expires_at": current.expires_at
    })))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, StatusCode> {
    let token = session_cookie(&headers)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;
    let presented = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;
    if !equal(origin, &state.admin_auth.origin)
        || !equal(
            presented,
            &csrf_token(&state.admin_auth.csrf_secret, &token)?,
        )
    {
        return Err(StatusCode::FORBIDDEN);
    }
    PgAdminIdentityRepository::new(state.pool.clone(), state.admin_auth.state_key)
        .revoke_session(&hash_api_key(&token))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie_header(
            "kusi_admin_session",
            "",
            "/",
            0,
            is_secure_cookie(&state.admin_auth),
        )?,
    );
    Ok(response)
}

fn unix_time() -> Result<i64, StatusCode> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
