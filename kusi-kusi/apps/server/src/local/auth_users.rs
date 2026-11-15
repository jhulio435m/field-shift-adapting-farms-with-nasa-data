use super::{cookie, csrf, status, store};
use crate::AppState;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use kusi_domain::local::{LocalRole, LocalUser};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Login {
    organization: String,
    username: String,
    password: String,
}
pub(super) async fn login(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(i): Json<Login>,
) -> Result<Response, StatusCode> {
    if h.get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v != s.local.origin)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let (u, t) = store(&s)?
        .login(&i.organization, &i.username, i.password)
        .await
        .map_err(status)?;
    let mut r = Json(json!({"user":u,"csrf_token":csrf(&s,&t)?})).into_response();
    r.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "kusi_local_session={t}; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800"
        ))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(r)
}
pub(super) async fn session(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    h: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let t = cookie(&h)
        .or_else(|| crate::bearer_token(&h))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(json!({"user":u,"csrf_token":csrf(&s,t)?})))
}
pub(super) async fn logout(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    h: HeaderMap,
) -> Result<Response, StatusCode> {
    let t = cookie(&h)
        .or_else(|| crate::bearer_token(&h))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    store(&s)?.logout(&u, t).await.map_err(status)?;
    let mut r = StatusCode::NO_CONTENT.into_response();
    r.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "kusi_local_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        ),
    );
    Ok(r)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UserInput {
    username: String,
    display_name: String,
    password: String,
    role: LocalRole,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Bootstrap {
    username: String,
    display_name: String,
    password: String,
}
pub(super) async fn bootstrap(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(i): Json<Bootstrap>,
) -> Result<Json<LocalUser>, StatusCode> {
    crate::authorize_admin(&s, &h).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let org = kusi_application::GetOrganizationByKey::new(s.repository.clone())
        .execute(crate::bearer_token(&h).ok_or(StatusCode::UNAUTHORIZED)?)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(Json(
        store(&s)?
            .bootstrap(org.id, &i.username, &i.display_name, i.password, true)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn users(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Vec<LocalUser>>, StatusCode> {
    Ok(Json(store(&s)?.users(&u).await.map_err(status)?))
}
pub(super) async fn list_local_sessions(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    h: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let token = cookie(&h)
        .or_else(|| crate::bearer_token(&h))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let sessions = store(&s)?
        .active_sessions(&u, token)
        .await
        .map_err(status)?;
    Ok(Json(json!({"sessions":sessions})))
}
pub(super) async fn revoke_local_session(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let token = cookie(&h)
        .or_else(|| crate::bearer_token(&h))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    store(&s)?
        .revoke_local_session(&u, token, id)
        .await
        .map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn create_user(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(i): Json<UserInput>,
) -> Result<Json<LocalUser>, StatusCode> {
    Ok(Json(
        store(&s)?
            .create_user(&u, &i.username, &i.display_name, i.password, i.role)
            .await
            .map_err(status)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateUser {
    role: LocalRole,
    disabled: bool,
}
pub(super) async fn update_user(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(i): Json<UpdateUser>,
) -> Result<Json<LocalUser>, StatusCode> {
    Ok(Json(
        store(&s)?
            .update_user(&u, id, i.role, i.disabled)
            .await
            .map_err(status)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Password {
    password: String,
}
pub(super) async fn reset_password(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(i): Json<Password>,
) -> Result<StatusCode, StatusCode> {
    store(&s)?
        .reset_password(&u, id, i.password)
        .await
        .map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn overview(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        store(&s)?
            .overview(u.organization_id)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn activity(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        store(&s)?
            .activity(u.organization_id)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn settings(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Json<Value> {
    let channels: Vec<_> = s
        .channels
        .iter()
        .filter(|(_, c)| c.organization_id == u.organization_id)
        .map(|(id, c)| json!({"id":id,"provider":c.provider}))
        .collect();
    Json(
        json!({"mode":"local","external_publication":false,"channels":channels,"llm_profiles":s.llm_profiles.get(&u.organization_id).map(|p|p.keys().collect::<Vec<_>>()).unwrap_or_default(),"audio_profiles":s.audio_profiles.get(&u.organization_id).map(|p|p.keys().collect::<Vec<_>>()).unwrap_or_default(),"debounce_seconds":3,"session_hours":8}),
    )
}
pub(super) async fn workspaces(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(store(&s)?.workspaces(&u).await.map_err(status)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Workspace {
    name: String,
    slug: String,
    username: String,
    display_name: String,
    password: String,
}
pub(super) async fn create_workspace(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(i): Json<Workspace>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        store(&s)?
            .create_workspace(
                &u,
                &i.name,
                &i.slug,
                &i.username,
                &i.display_name,
                i.password,
            )
            .await
            .map_err(status)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Retention {
    days: i64,
    #[serde(default)]
    confirm: bool,
}
pub(super) async fn retention_preview(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(i): Json<Retention>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        store(&s)?
            .retention_preview(&u, i.days)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn retention_apply(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(i): Json<Retention>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        store(&s)?
            .apply_retention(&u, i.days, i.confirm)
            .await
            .map_err(status)?,
    ))
}
