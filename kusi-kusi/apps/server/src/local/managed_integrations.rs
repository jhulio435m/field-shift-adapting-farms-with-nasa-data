use super::{managed_operations::ManagedError, store};
use crate::AppState;
use axum::{
    Extension, Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use kusi_application::{
    connection_settings::ConnectionKind,
    managed_integrations::{
        self, LifecycleInput, ManagedConnections, VerificationSummary, VerifyInput,
    },
};
use kusi_domain::{local::LocalUser, managed_integrations::VerificationMode};

pub(super) async fn verify(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(k): Path<String>,
    input: Result<Json<VerifyInput>, JsonRejection>,
) -> Result<Json<VerificationSummary>, ManagedError> {
    let Json(input) = input.map_err(|_| ManagedError::Malformed)?;
    managed_integrations::owner(&a)?;
    let kind = ConnectionKind::parse(&k)?;
    if input.mode == VerificationMode::Provider {
        return Err(ManagedError::LocalBlocked);
    }
    Ok(Json(
        managed_integrations::verify(store(&s)?, &a, kind, &input).await?,
    ))
}
pub(super) async fn current(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(k): Path<String>,
) -> Result<Json<VerificationSummary>, ManagedError> {
    Ok(Json(
        store(&s)?
            .connection_status(&a, ConnectionKind::parse(&k)?)
            .await?,
    ))
}
pub(super) async fn disable(
    State(s): State<AppState>,
    Extension(a): Extension<LocalUser>,
    Path(k): Path<String>,
    input: Result<Json<LifecycleInput>, JsonRejection>,
) -> Result<Json<VerificationSummary>, ManagedError> {
    let Json(input) = input.map_err(|_| ManagedError::Malformed)?;
    Ok(Json(
        managed_integrations::disable(store(&s)?, &a, ConnectionKind::parse(&k)?, &input).await?,
    ))
}
pub(super) async fn enable(
    Extension(a): Extension<LocalUser>,
    Path(k): Path<String>,
    input: Result<Json<LifecycleInput>, JsonRejection>,
) -> Result<StatusCode, ManagedError> {
    let Json(input) = input.map_err(|_| ManagedError::Malformed)?;
    managed_integrations::owner(&a)?;
    ConnectionKind::parse(&k)?;
    if input.expected_revision < 1 {
        return Err(kusi_application::local::LocalError::Conflict.into());
    }
    Err(ManagedError::LocalBlocked)
}
pub(super) async fn manifests(
    Extension(a): Extension<LocalUser>,
) -> Result<Json<Vec<kusi_application::connector_manifest::ConnectorManifest>>, ManagedError> {
    managed_integrations::owner(&a)?;
    Ok(Json(kusi_application::connector_manifest::manifests()))
}
