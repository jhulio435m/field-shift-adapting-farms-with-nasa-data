mod auth_users;
mod cases_payments;
mod connection_settings;
mod content;
mod managed_integrations;
mod managed_operations;

use crate::AppState;
use auth_users::*;
use axum::{
    Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use cases_payments::*;
use content::*;
use kusi_application::local::LocalError;
use kusi_domain::local::LocalUser;
use kusi_infrastructure::local::{LocalStore, SessionUser};
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct Settings {
    pub store: Option<LocalStore>,
    pub marker: String,
    pub origin: String,
}
fn validate_loopback(bind: &str, origin: &str) -> Result<(), Box<dyn std::error::Error>> {
    let address: std::net::SocketAddr = bind.parse()?;
    let frontend: std::net::SocketAddr = origin
        .strip_prefix("http://")
        .ok_or("local origin requires HTTP loopback")?
        .parse()?;
    if !address.ip().is_loopback() || !frontend.ip().is_loopback() {
        return Err("local mode requires loopback HTTP bind and frontend origin".into());
    }
    Ok(())
}
pub async fn load(pool: sqlx::PgPool, bind: &str) -> Result<Settings, Box<dyn std::error::Error>> {
    let enabled = std::env::var("KUSI_LOCAL_MODE")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()?;
    let origin = std::env::var("KUSI_LOCAL_ORIGIN").unwrap_or_else(|_| format!("http://{bind}"));
    if enabled {
        validate_loopback(bind, &origin)?;
    }
    let store = if enabled {
        Some(
            LocalStore::new(
                pool,
                std::env::var("KUSI_LOCAL_STORAGE")
                    .unwrap_or_else(|_| ".local-data".into())
                    .into(),
            )
            .await?,
        )
    } else {
        None
    };
    Ok(Settings {
        store,
        marker: kusi_infrastructure::oauth::random_secret()?,
        origin,
    })
}
pub(super) fn status(e: LocalError) -> StatusCode {
    match e {
        LocalError::Invalid => StatusCode::BAD_REQUEST,
        LocalError::Unauthorized => StatusCode::UNAUTHORIZED,
        LocalError::Forbidden => StatusCode::FORBIDDEN,
        LocalError::NotFound => StatusCode::NOT_FOUND,
        LocalError::Conflict => StatusCode::CONFLICT,
        LocalError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        LocalError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}
pub(super) fn store(s: &AppState) -> Result<&LocalStore, StatusCode> {
    s.local.store.as_ref().ok_or(StatusCode::NOT_FOUND)
}
pub(super) fn cookie(h: &HeaderMap) -> Option<&str> {
    h.get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|p| p.trim().strip_prefix("kusi_local_session="))
}
pub(super) fn csrf(s: &AppState, t: &str) -> Result<String, StatusCode> {
    use hmac::{Hmac, Mac};
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(s.admin_key_hash.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    mac.update(t.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}
fn same(a: &str, b: &str) -> bool {
    bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}
pub async fn guard(State(s): State<AppState>, mut req: Request, next: Next) -> Response {
    req.headers_mut().remove("x-kusi-local-admin");
    let Some(storage) = s.local.store.as_ref() else {
        if req.uri().path().starts_with("/local/auth/")
            || req.uri().path().starts_with("/v1/local/")
        {
            return StatusCode::NOT_FOUND.into_response();
        }
        return next.run(req).await;
    };
    if req.uri().path() == "/v1/capabilities/execute"
        || req.uri().path().starts_with("/v1/ai/")
        || req.uri().path().starts_with("/v1/oauth/") && req.method() != "GET"
        || req.uri().path().starts_with("/v1/channels/")
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let token = if req.uri().path() == "/local/auth/login" {
        None
    } else {
        cookie(req.headers())
            .map(str::to_owned)
            .or_else(|| crate::bearer_token(req.headers()).map(str::to_owned))
    };
    if let Some(token) = token {
        match storage.session(&token).await {
            Ok(SessionUser::Active(user)) => {
                let path = req.uri().path();
                let method = req.method().as_str();
                if !user.role.permits(method, path, user.platform_admin) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                if method != "GET" && method != "HEAD" {
                    let origin = req
                        .headers()
                        .get(header::ORIGIN)
                        .and_then(|v| v.to_str().ok());
                    let presented = req
                        .headers()
                        .get("x-csrf-token")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if origin.is_some_and(|v| v != s.local.origin)
                        || !csrf(&s, &token).is_ok_and(|expected| same(presented, &expected))
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                }
                if let Err(e) = storage
                    .rate_limit(&format!("session:{}", user.id), 180, 60)
                    .await
                {
                    return status(e).into_response();
                }
                let Ok(auth) = HeaderValue::from_str(&format!("Bearer {token}")) else {
                    return StatusCode::UNAUTHORIZED.into_response();
                };
                let Ok(marker) = HeaderValue::from_str(&s.local.marker) else {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                };
                req.headers_mut().insert(header::AUTHORIZATION, auth);
                req.headers_mut().insert("x-kusi-local-admin", marker);
                req.extensions_mut().insert(user);
            }
            Ok(SessionUser::Missing) if cookie(req.headers()).is_none() => {}
            Ok(_) => return StatusCode::UNAUTHORIZED.into_response(),
            Err(e) => return status(e).into_response(),
        }
    }
    if ((req.uri().path().starts_with("/v1/local/") && req.uri().path() != "/v1/local/bootstrap")
        || matches!(
            req.uri().path(),
            "/local/auth/session" | "/local/auth/logout"
        ))
        && req.extensions().get::<LocalUser>().is_none()
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
pub fn routes() -> Router<AppState> {
    let managed = Router::new()
        .route(
            "/v1/local/managed-executions/{id}/proposal",
            get(managed_operations::execution_proposal),
        )
        .route(
            "/v1/local/managed-queries",
            post(managed_operations::managed_query).layer(DefaultBodyLimit::max(32768)),
        )
        .route(
            "/v1/local/channel-operations/telegram",
            get(managed_operations::channel_list),
        )
        .route(
            "/v1/local/channel-operations/telegram/bindings/{id}/revoke",
            post(managed_operations::revoke_binding),
        )
        .route(
            "/v1/local/managed-executions",
            get(managed_operations::executions),
        )
        .route(
            "/v1/local/managed-executions/{id}",
            get(managed_operations::execution),
        )
        .route(
            "/v1/local/managed-executions/{id}/{action}",
            post(managed_operations::execution_action),
        )
        .route(
            "/v1/local/integration-evaluations",
            post(managed_operations::evaluate),
        )
        .route(
            "/v1/local/integration-evaluations/{id}",
            get(managed_operations::evaluation),
        )
        .route(
            "/v1/local/authorized-memories",
            get(managed_operations::memories),
        )
        .route(
            "/v1/local/authorized-memories/preferences",
            get(managed_operations::memory_preference)
                .put(managed_operations::set_memory_preference),
        )
        .route(
            "/v1/local/authorized-memories/{id}",
            axum::routing::delete(managed_operations::forget),
        )
        .route(
            "/v1/local/policy-simulations",
            post(managed_operations::policy),
        )
        .route(
            "/v1/local/connector-manifests",
            get(managed_integrations::manifests),
        )
        .route(
            "/v1/local/connection-settings/{kind}/verify",
            post(managed_integrations::verify),
        )
        .route(
            "/v1/local/connection-settings/{kind}/status",
            get(managed_integrations::current),
        )
        .route(
            "/v1/local/connection-settings/{kind}/enable",
            post(managed_integrations::enable),
        )
        .route(
            "/v1/local/connection-settings/{kind}/disable",
            post(managed_integrations::disable),
        )
        .layer(DefaultBodyLimit::max(32768));
    Router::new()
        .merge(managed)
        .route(
            "/v1/local/connection-settings",
            get(connection_settings::list),
        )
        .route(
            "/v1/local/connection-settings/{kind}",
            axum::routing::put(connection_settings::save).layer(DefaultBodyLimit::max(32768)),
        )
        .route(
            "/v1/local/connection-settings/{kind}/credentials",
            axum::routing::delete(connection_settings::clear).layer(DefaultBodyLimit::max(32768)),
        )
        .route("/local/auth/login", post(login))
        .route("/local/auth/session", get(session))
        .route("/local/auth/logout", post(logout))
        .route("/v1/local/bootstrap", post(bootstrap))
        .route("/v1/local/overview", get(overview))
        .route("/v1/demo/payments", get(demo_payments))
        .route("/v1/demo/payments/by-dni", post(demo_payments_by_dni))
        .route("/v1/demo/payments/validate", post(demo_validate_payment))
        .route(
            "/v1/demo/payments/validate-receipt",
            post(demo_validate_receipt).layer(DefaultBodyLimit::max(5 * 1024 * 1024)),
        )
        .route("/v1/local/cases", get(case_list))
        .route("/v1/local/cases/{id}", get(case_detail))
        .route("/v1/local/cases/{id}/sources", get(case_sources))
        .route("/v1/local/cases/{id}/guidance", post(case_guidance))
        .route("/v1/local/cases/{id}/requests", post(case_request))
        .route(
            "/v1/local/cases/{id}/requests/{request_id}/approve",
            post(case_approve),
        )
        .route(
            "/v1/local/cases/{id}/requests/{request_id}/simulate-send",
            post(case_simulate_send),
        )
        .route("/v1/local/cases/{id}/responses", post(case_response))
        .route("/v1/local/activity", get(activity))
        .route("/v1/local/settings", get(settings))
        .route("/v1/local/users", get(users).post(create_user))
        .route("/v1/local/sessions", get(list_local_sessions))
        .route(
            "/v1/local/sessions/{id}",
            axum::routing::delete(revoke_local_session),
        )
        .route("/v1/local/users/{id}", axum::routing::patch(update_user))
        .route("/v1/local/users/{id}/password", post(reset_password))
        .route(
            "/v1/local/assets",
            get(assets)
                .post(upload)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/v1/local/assets/{id}", get(asset).delete(delete_asset))
        .route("/v1/local/assets/{id}/content", get(content))
        .route(
            "/v1/local/conversations",
            get(conversations).post(create_conversation),
        )
        .route(
            "/v1/local/conversations/{id}/messages",
            get(messages).post(send_message),
        )
        .route("/v1/local/link-codes", post(link_code))
        .route("/v1/local/academic/demo-students", get(demo_students))
        .route("/v1/local/resource-access/resources", get(resource_catalog))
        .route(
            "/v1/local/resource-access/grants",
            get(resource_grants).post(create_resource_grant),
        )
        .route(
            "/v1/local/resource-access/grants/{user_id}/{resource_ref}",
            axum::routing::delete(revoke_resource_grant),
        )
        .route("/v1/local/resource-access/execute", post(execute_resource))
        .route("/v1/local/resource-access/audit", get(resource_audit))
        .route(
            "/v1/local/academic/guardian-links",
            get(guardian_links).post(create_guardian_link),
        )
        .route(
            "/v1/local/academic/guardian-links/{user_id}/{student_ref}",
            axum::routing::delete(remove_guardian_link),
        )
        .route(
            "/v1/local/publications",
            get(publications).post(create_publication),
        )
        .route(
            "/v1/local/publications/{id}",
            get(publication).patch(edit_publication),
        )
        .route(
            "/v1/local/publications/{id}/approve",
            post(approve_publication),
        )
        .route(
            "/v1/local/publications/{id}/cancel",
            post(cancel_publication),
        )
        .route(
            "/v1/local/workspaces",
            get(workspaces).post(create_workspace),
        )
        .route("/v1/local/retention/preview", post(retention_preview))
        .route("/v1/local/retention/apply", post(retention_apply))
}

#[cfg(test)]
mod tests {
    use super::validate_loopback;
    #[test]
    fn separate_loopback_ports_are_allowed() {
        assert!(validate_loopback("127.0.0.1:8080", "http://127.0.0.1:3000").is_ok());
        assert!(validate_loopback("[::1]:8080", "http://[::1]:3000").is_ok());
    }
    #[test]
    fn external_bind_and_origins_are_rejected() {
        for (bind, origin) in [
            ("0.0.0.0:8080", "http://127.0.0.1:3000"),
            ("127.0.0.1:8080", "http://192.0.2.1:3000"),
            ("127.0.0.1:8080", "http://127.0.0.1:3000/extra"),
            ("127.0.0.1:8080", "https://example.com"),
        ] {
            assert!(validate_loopback(bind, origin).is_err());
        }
    }
}
