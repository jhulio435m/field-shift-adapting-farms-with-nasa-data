//! Local administration of existing durable operations. No provider dispatch here.
use super::store;
use crate::AppState;
use axum::{
    Extension, Json,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kusi_application::{
    authorized_memory::{
        AuthorizedMemoryRepository, MemoryPreference, MemorySummary, PolicyDecision, PolicyInput,
    },
    channel_operations::{BindingRevision, ChannelOperations, ChannelSummary},
    evaluation::{EvaluationInput, EvaluationResult},
    execution_review::{self, ExecutionAction, ExecutionInput, ExecutionReview, ExecutionSummary},
    local::LocalError,
};
use kusi_domain::local::LocalUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) enum ManagedError {
    Local(LocalError),
    Malformed,
    LocalBlocked,
}
impl From<StatusCode> for ManagedError {
    fn from(_: StatusCode) -> Self {
        Self::Local(LocalError::NotFound)
    }
}
impl From<LocalError> for ManagedError {
    fn from(error: LocalError) -> Self {
        Self::Local(error)
    }
}
impl IntoResponse for ManagedError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::Malformed => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_request"),
            Self::LocalBlocked => (StatusCode::FORBIDDEN, "local_external_blocked"),
            Self::Local(error) => {
                let code = match &error {
                    LocalError::Invalid => "invalid_request",
                    LocalError::Forbidden => "forbidden",
                    LocalError::Unauthorized => "unauthorized",
                    LocalError::Conflict => "revision_conflict",
                    LocalError::NotFound => "not_found",
                    LocalError::Unavailable => "unavailable",
                    LocalError::RateLimited => "rate_limited",
                };
                (super::status(error), code)
            }
        };
        let mut response = (
            status,
            Json(serde_json::json!({"code":code,"correlation_id":Uuid::new_v4()})),
        )
            .into_response();
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        );
        response
    }
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    cursor: Option<Uuid>,
}
fn input<T>(result: Result<Json<T>, JsonRejection>) -> Result<T, ManagedError> {
    result
        .map(|Json(value)| value)
        .map_err(|_| ManagedError::Malformed)
}
pub(super) async fn channel_list(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Query(page): Query<Page>,
) -> Result<Json<ChannelSummary>, ManagedError> {
    Ok(Json(store(&s)?.telegram_bindings(&a, page.cursor).await?))
}
pub(super) async fn revoke_binding(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    body: Result<Json<BindingRevision>, JsonRejection>,
) -> Result<StatusCode, ManagedError> {
    kusi_application::channel_operations::revoke(store(&s)?, &a, id, &input(body)?).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn executions(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Query(page): Query<Page>,
) -> Result<Json<Vec<ExecutionSummary>>, ManagedError> {
    Ok(Json(store(&s)?.execution_list(&a, page.cursor).await?))
}
pub(super) async fn execution(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<ExecutionSummary>, ManagedError> {
    Ok(Json(store(&s)?.execution_detail(&a, id).await?))
}
pub(super) async fn execution_action(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path((id, action)): Path<(Uuid, String)>,
    body: Result<Json<ExecutionInput>, JsonRejection>,
) -> Result<Json<ExecutionSummary>, ManagedError> {
    let action = match action.as_str() {
        "cancel" => ExecutionAction::Cancel,
        "resume" => ExecutionAction::Resume,
        "review" => ExecutionAction::Review,
        _ => return Err(LocalError::NotFound.into()),
    };
    Ok(Json(
        execution_review::action(store(&s)?, &a, id, action, &input(body)?).await?,
    ))
}
#[derive(Serialize)]
pub(super) struct EvaluationCreated {
    id: Uuid,
    result: EvaluationResult,
}
pub(super) async fn evaluate(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    body: Result<Json<EvaluationInput>, JsonRejection>,
) -> Result<Json<EvaluationCreated>, ManagedError> {
    let (id, result) = store(&s)?
        .run_evaluation(&a, &input(body)?.suite_version)
        .await?;
    Ok(Json(EvaluationCreated { id, result }))
}
pub(super) async fn evaluation(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<EvaluationResult>, ManagedError> {
    Ok(Json(store(&s)?.evaluation_result(&a, id).await?))
}
pub(super) async fn memories(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
) -> Result<Json<Vec<MemorySummary>>, ManagedError> {
    Ok(Json(store(&s)?.memory_list(&a).await?))
}
pub(super) async fn forget(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ManagedError> {
    store(&s)?.forget_memory(&a, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn memory_preference(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
) -> Result<Json<MemoryPreference>, ManagedError> {
    Ok(Json(store(&s)?.memory_preference(&a).await?))
}
pub(super) async fn set_memory_preference(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    body: Result<Json<MemoryPreference>, JsonRejection>,
) -> Result<StatusCode, ManagedError> {
    kusi_application::authorized_memory::set_preference(store(&s)?, &a, &input(body)?).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn policy(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    body: Result<Json<PolicyInput>, JsonRejection>,
) -> Result<Json<PolicyDecision>, ManagedError> {
    Ok(Json(
        kusi_application::authorized_memory::simulate(store(&s)?, &a, &input(body)?).await?,
    ))
}
pub(super) async fn managed_query(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    headers: axum::http::HeaderMap,
    body: Result<Json<kusi_application::managed_integrations::ManagedQueryInput>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ManagedError> {
    let token = crate::bearer_token(&headers).ok_or(LocalError::Unauthorized)?;
    let actor = kusi_application::adaptive::Actor {
        organization: a.organization_id,
        principal: kusi_application::hash_api_key(token),
        read: !a.disabled,
        subscribe: false,
    };
    Ok(Json(
        store(&s)?
            .execute_managed_fixture(&actor, &input(body)?)
            .await?,
    ))
}

pub(super) async fn execution_proposal(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<kusi_application::execution_review::ExecutionProposal>, ManagedError> {
    Ok(Json(store(&s)?.execution_proposal(&a, id).await?))
}
