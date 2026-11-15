use super::{status, store};
use crate::AppState;
use axum::{
    Extension, Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use kusi_application::connection_settings::{
    self, ConnectionInput, ConnectionKind, ConnectionSummary,
};
use kusi_domain::local::LocalUser;
use serde::Deserialize;

pub(super) async fn list(
    State(state): State<AppState>,
    Extension(actor): Extension<LocalUser>,
) -> Result<Json<Vec<ConnectionSummary>>, StatusCode> {
    connection_settings::list(store(&state)?, &actor)
        .await
        .map(Json)
        .map_err(status)
}
pub(super) async fn save(
    State(state): State<AppState>,
    Extension(actor): Extension<LocalUser>,
    Path(kind): Path<String>,
    input: Result<Json<ConnectionInput>, JsonRejection>,
) -> Result<Json<ConnectionSummary>, StatusCode> {
    let Json(input) = input.map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let kind = ConnectionKind::parse(&kind).map_err(status)?;
    connection_settings::save(store(&state)?, &actor, kind, &input)
        .await
        .map(Json)
        .map_err(status)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Revision {
    expected_revision: i64,
}
pub(super) async fn clear(
    State(state): State<AppState>,
    Extension(actor): Extension<LocalUser>,
    Path(kind): Path<String>,
    input: Result<Json<Revision>, JsonRejection>,
) -> Result<Json<ConnectionSummary>, StatusCode> {
    let Json(input) = input.map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let kind = ConnectionKind::parse(&kind).map_err(status)?;
    connection_settings::clear(store(&state)?, &actor, kind, input.expected_revision)
        .await
        .map(Json)
        .map_err(status)
}
