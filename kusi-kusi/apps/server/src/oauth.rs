use crate::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use kusi_application::oauth::{OAuthError, OAuthProfile, OAuthProvider, OAuthRepository};
use kusi_infrastructure::oauth::{
    OfficialOAuthProvider, PgOAuthRepository, TokenVault, pkce_challenge, random_secret,
    validate_profile,
};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Clone)]
pub struct Settings {
    profiles: HashMap<Uuid, HashMap<String, OAuthProfile>>,
    vault: Option<Arc<TokenVault>>,
    simulation: bool,
    refresh_enabled: bool,
}
pub fn load() -> Result<Settings, Box<dyn std::error::Error>> {
    let profiles: HashMap<Uuid, HashMap<String, OAuthProfile>> =
        match std::env::var("KUSI_OAUTH_PROFILES") {
            Ok(v) => serde_json::from_str(&v)?,
            Err(std::env::VarError::NotPresent) => HashMap::new(),
            Err(e) => return Err(e.into()),
        };
    let simulation = std::env::var("KUSI_OAUTH_SIMULATION")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()?;
    for tenant in profiles.values() {
        for (provider, p) in tenant {
            if provider != &p.provider {
                return Err("OAuth provider key mismatch".into());
            }
            validate_profile(p, simulation)?;
        }
    }
    let vault = match std::env::var("KUSI_OAUTH_KEYRING") {
        Ok(value) => {
            let refs: HashMap<String, String> = serde_json::from_str(&value)?;
            let mut keys = HashMap::new();
            for (id, name) in refs {
                keys.insert(id, hex::decode(std::env::var(name)?)?);
            }
            Some(Arc::new(TokenVault::new(
                std::env::var("KUSI_OAUTH_ACTIVE_KEY")?,
                keys,
            )?))
        }
        Err(std::env::VarError::NotPresent) if profiles.is_empty() => None,
        _ => return Err("OAuth requires an encryption keyring".into()),
    };
    let refresh_enabled = std::env::var("KUSI_OAUTH_REFRESH_WORKER")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()?;
    Ok(Settings {
        refresh_enabled,
        profiles,
        vault,
        simulation,
    })
}
fn store(state: &AppState) -> Result<PgOAuthRepository, StatusCode> {
    Ok(PgOAuthRepository::new(
        state.pool.clone(),
        state.oauth.vault.clone().ok_or(StatusCode::NOT_FOUND)?,
    ))
}
fn status(error: OAuthError) -> StatusCode {
    match error {
        OAuthError::Invalid => StatusCode::BAD_REQUEST,
        OAuthError::InvalidState => StatusCode::FORBIDDEN,
        OAuthError::NotFound => StatusCode::NOT_FOUND,
        OAuthError::Conflict => StatusCode::CONFLICT,
        OAuthError::Provider => StatusCode::BAD_GATEWAY,
        OAuthError::Storage => StatusCode::SERVICE_UNAVAILABLE,
    }
}
async fn identity(state: &AppState, headers: &HeaderMap) -> Result<(Uuid, String), StatusCode> {
    crate::authorize_admin(state, headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let key = crate::bearer_token(headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let org = kusi_application::GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok((org.id, kusi_application::hash_api_key(key)))
}
fn protected_json(value: impl serde::Serialize) -> Response {
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        Json(value),
    )
        .into_response()
}
pub async fn start(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (org, principal) = identity(&state, &headers).await?;
    let profile = state
        .oauth
        .profiles
        .get(&org)
        .and_then(|p| p.get(&provider))
        .ok_or(StatusCode::NOT_FOUND)?;
    let token = random_secret().map_err(status)?;
    let browser = random_secret().map_err(status)?;
    let verifier = if profile.pkce {
        Some(random_secret().map_err(status)?)
    } else {
        None
    };
    let challenge = verifier.as_deref().map(pkce_challenge);
    let url = OfficialOAuthProvider {
        simulation: state.oauth.simulation,
    }
    .authorize_url(profile, &token, challenge.as_deref())
    .map_err(status)?;
    store(&state)?
        .begin(
            org,
            &principal,
            &kusi_application::hash_api_key(&token),
            &kusi_application::hash_api_key(&browser),
            profile,
            verifier.as_deref(),
        )
        .await
        .map_err(status)?;
    let mut response =
        protected_json(json!({"authorization_url":url,"expires_in":600,"provider":provider}));
    response.headers_mut().insert(
        header::SET_COOKIE,
        format!("__Host-kusi_oauth={browser}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=600")
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}
#[derive(Deserialize)]
pub struct Callback {
    state: String,
    code: Option<String>,
    error: Option<String>,
}
fn browser_cookie(headers: &HeaderMap) -> Result<String, StatusCode> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        let header = header.to_str().map_err(|_| StatusCode::FORBIDDEN)?;
        for part in header.split(';') {
            if let Some(value) = part.trim().strip_prefix("__Host-kusi_oauth=") {
                if found.is_some()
                    || value.len() != 43
                    || !value
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                {
                    return Err(StatusCode::FORBIDDEN);
                }
                found = Some(value.to_owned());
            }
        }
    }
    found.ok_or(StatusCode::FORBIDDEN)
}
pub async fn callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Query(query): Query<Callback>,
) -> Result<Response, StatusCode> {
    if query.state.len() != 43
        || !query
            .state
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let cookie = browser_cookie(&headers)?;
    let repository = store(&state)?;
    let pending = repository
        .consume(
            &provider,
            &kusi_application::hash_api_key(&query.state),
            &kusi_application::hash_api_key(&cookie),
        )
        .await
        .map_err(status)?;
    if query.error.is_some() {
        repository
            .authorization_failed(&pending, "authorization_denied")
            .await
            .map_err(status)?;
        return Err(StatusCode::BAD_REQUEST);
    }
    let code = query.code.as_deref().ok_or(StatusCode::BAD_REQUEST)?;
    let tokens = match (OfficialOAuthProvider {
        simulation: state.oauth.simulation,
    })
    .exchange(&pending.profile, code, pending.verifier.as_deref())
    .await
    {
        Ok(tokens) => tokens,
        Err(error) => {
            repository
                .authorization_failed(&pending, "authorization_failed")
                .await
                .map_err(status)?;
            return Err(status(error));
        }
    };
    let connection = repository.save(pending, tokens).await.map_err(status)?;
    let mut response = protected_json(connection);
    response.headers_mut().insert(
        header::SET_COOKIE,
        "__Host-kusi_oauth=; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=0"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (org, _) = identity(&state, &headers).await?;
    Ok(protected_json(
        store(&state)?.list(org).await.map_err(status)?,
    ))
}
pub async fn refresh(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (org, principal) = identity(&state, &headers).await?;
    let repository = store(&state)?;
    let secret = repository
        .claim(org, id, &principal, "refresh")
        .await
        .map_err(status)?;
    let result = OfficialOAuthProvider {
        simulation: state.oauth.simulation,
    }
    .refresh(&secret.profile, &secret.tokens)
    .await;
    match result {
        Ok(tokens) => repository
            .refreshed(org, id, secret.operation_id, tokens)
            .await
            .map_err(status)?,
        Err(_) => {
            repository
                .finish_action(org, id, secret.operation_id, "refresh", false, &principal)
                .await
                .map_err(status)?;
            return Err(StatusCode::BAD_GATEWAY);
        }
    }
    Ok(protected_json(
        json!({"status":"refreshed","connection_id":id}),
    ))
}
pub async fn disconnect(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (org, principal) = identity(&state, &headers).await?;
    let repository = store(&state)?;
    let secret = repository
        .claim(org, id, &principal, "revoke")
        .await
        .map_err(status)?;
    let success = OfficialOAuthProvider {
        simulation: state.oauth.simulation,
    }
    .revoke(&secret.profile, &secret.tokens)
    .await
    .is_ok();
    repository
        .finish_action(org, id, secret.operation_id, "revoke", success, &principal)
        .await
        .map_err(status)?;
    let code = if success {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    let mut response = protected_json(
        json!({"connection_id":id,"status":if success{"disconnected"}else{"revoke_pending"},"local_access_enabled":false}),
    );
    *response.status_mut() = code;
    Ok(response)
}

pub fn spawn_maintenance(settings: Settings, pool: sqlx::PgPool) {
    if !settings.refresh_enabled {
        return;
    }
    if let Some(vault) = settings.vault {
        let allowed = settings
            .profiles
            .into_iter()
            .map(|(org, profiles)| (org, profiles.into_keys().collect()))
            .collect();
        tokio::spawn(kusi_infrastructure::oauth::run_refresh_worker(
            PgOAuthRepository::new(pool, vault),
            OfficialOAuthProvider {
                simulation: settings.simulation,
            },
            allowed,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_cookie_rejects_missing_duplicate_and_wrong_length() {
        assert!(browser_cookie(&HeaderMap::new()).is_err());
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "__Host-kusi_oauth=short".parse().expect("fixture"),
        );
        assert!(browser_cookie(&headers).is_err());
        let nonce = "x".repeat(43);
        headers.insert(
            header::COOKIE,
            format!("__Host-kusi_oauth={nonce}")
                .parse()
                .expect("fixture"),
        );
        assert_eq!(browser_cookie(&headers).expect("cookie"), nonce);
        headers.insert(
            header::COOKIE,
            format!("__Host-kusi_oauth={nonce}; __Host-kusi_oauth={nonce}")
                .parse()
                .expect("fixture"),
        );
        assert!(browser_cookie(&headers).is_err());
    }
}
