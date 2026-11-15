use super::{status, store};
use crate::AppState;
use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use kusi_application::local::PublicationInput;
use kusi_domain::local::LocalUser;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) async fn assets(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Vec<kusi_application::local::LocalAsset>>, StatusCode> {
    Ok(Json(
        store(&s)?.assets(u.organization_id).await.map_err(status)?,
    ))
}
#[derive(Deserialize)]
pub(super) struct Upload {
    label: String,
    filename: String,
}
pub(super) async fn upload(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Query(i): Query<Upload>,
    b: Bytes,
) -> Result<Json<kusi_application::local::LocalAsset>, StatusCode> {
    Ok(Json(
        store(&s)?
            .upload(&u, &i.label, &i.filename, b.to_vec())
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn asset(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<kusi_application::local::LocalAsset>, StatusCode> {
    Ok(Json(
        store(&s)?
            .asset(u.organization_id, id)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn content(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let (a, b) = store(&s)?
        .asset_bytes(u.organization_id, id)
        .await
        .map_err(status)?;
    let mut r = b.into_response();
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&a.media_type).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(r)
}
pub(super) async fn delete_asset(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    store(&s)?.delete_asset(&u, id).await.map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn conversations(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(store(&s)?.conversations(&u).await.map_err(status)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Title {
    title: String,
}
pub(super) async fn create_conversation(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(i): Json<Title>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        store(&s)?
            .create_conversation(&u, &i.title)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn messages(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(store(&s)?.messages(&u, id).await.map_err(status)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Message {
    content: String,
    asset_id: Option<Uuid>,
    client_id: Uuid,
}
pub(super) async fn send_message(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(i): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    store(&s)?
        .send_local_message(&u, id, &i.content, i.asset_id, i.client_id)
        .await
        .map_err(status)?;
    Ok(StatusCode::ACCEPTED)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Link {
    channel_id: Uuid,
    user_id: Option<Uuid>,
}
pub(super) async fn link_code(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(i): Json<Link>,
) -> Result<Json<Value>, StatusCode> {
    if s.channels
        .get(&i.channel_id)
        .is_none_or(|c| c.organization_id != u.organization_id)
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let code = store(&s)?
        .create_link_code_for(&u, i.user_id.unwrap_or(u.id), i.channel_id)
        .await
        .map_err(status)?;
    Ok(Json(json!({"code":code,"expires_in_seconds":600})))
}
pub(super) async fn demo_students(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Vec<kusi_infrastructure::local::DemoStudent>>, StatusCode> {
    Ok(Json(store(&s)?.demo_students(&u).await.map_err(status)?))
}
pub(super) async fn resource_catalog(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        json!({"resources": store(&s)?.resource_catalog(&u).await.map_err(status)?}),
    ))
}
pub(super) async fn resource_grants(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        json!({"grants": store(&s)?.resource_grants(&u).await.map_err(status)?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateResourceGrant {
    user_id: Uuid,
    resource_ref: String,
    authorization_verified: bool,
}
pub(super) async fn create_resource_grant(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(input): Json<CreateResourceGrant>,
) -> Result<StatusCode, StatusCode> {
    store(&s)?
        .grant_resource(
            &u,
            input.user_id,
            &input.resource_ref,
            input.authorization_verified,
        )
        .await
        .map_err(status)?;
    Ok(StatusCode::CREATED)
}
pub(super) async fn revoke_resource_grant(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path((user_id, resource_ref)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    store(&s)?
        .revoke_resource(&u, user_id, &resource_ref)
        .await
        .map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExecuteResource {
    resource_ref: String,
}
pub(super) async fn execute_resource(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(input): Json<ExecuteResource>,
) -> Result<Json<Value>, StatusCode> {
    let result = store(&s)?
        .execute_resource(&u, &input.resource_ref)
        .await
        .map_err(status)?;
    Ok(Json(
        json!({"capability":"demo.resource.read","result":result,"simulated":true}),
    ))
}
pub(super) async fn resource_audit(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(
        json!({"events":store(&s)?.resource_audit(&u).await.map_err(status)?}),
    ))
}
pub(super) async fn guardian_links(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Vec<kusi_infrastructure::local::GuardianStudentLink>>, StatusCode> {
    Ok(Json(store(&s)?.guardian_links(&u).await.map_err(status)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GuardianLinkInput {
    user_id: Uuid,
    student_ref: String,
    relationship_verified: bool,
}
pub(super) async fn create_guardian_link(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(input): Json<GuardianLinkInput>,
) -> Result<StatusCode, StatusCode> {
    store(&s)?
        .create_guardian_link(
            &u,
            input.user_id,
            &input.student_ref,
            input.relationship_verified,
        )
        .await
        .map_err(status)?;
    Ok(StatusCode::CREATED)
}
pub(super) async fn remove_guardian_link(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path((user_id, student_ref)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    store(&s)?
        .remove_guardian_link(&u, user_id, &student_ref)
        .await
        .map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn publications(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Vec<kusi_application::local::LocalPublication>>, StatusCode> {
    Ok(Json(
        store(&s)?
            .publications(u.organization_id)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn publication(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<kusi_application::local::LocalPublication>, StatusCode> {
    Ok(Json(
        store(&s)?
            .publication(u.organization_id, id)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn create_publication(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Json(i): Json<PublicationInput>,
) -> Result<Json<kusi_application::local::LocalPublication>, StatusCode> {
    Ok(Json(
        store(&s)?.create_publication(&u, i).await.map_err(status)?,
    ))
}
pub(super) async fn edit_publication(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(i): Json<PublicationInput>,
) -> Result<Json<kusi_application::local::LocalPublication>, StatusCode> {
    Ok(Json(
        store(&s)?
            .edit_publication(&u, id, i)
            .await
            .map_err(status)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Approval {
    confirm: bool,
    approval_digest: String,
}
pub(super) async fn approve_publication(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(i): Json<Approval>,
) -> Result<Json<kusi_application::local::LocalPublication>, StatusCode> {
    Ok(Json(
        store(&s)?
            .approve_publication(&u, id, &i.approval_digest, i.confirm)
            .await
            .map_err(status)?,
    ))
}
pub(super) async fn cancel_publication(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<kusi_application::local::LocalPublication>, StatusCode> {
    Ok(Json(
        store(&s)?
            .cancel_publication(&u, id)
            .await
            .map_err(status)?,
    ))
}
