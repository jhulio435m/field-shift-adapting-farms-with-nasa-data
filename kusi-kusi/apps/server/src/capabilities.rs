//! Capability HTTP boundary; connector selection remains server-owned.
use crate::{ApiError, bearer_token};
use axum::{Json, http::HeaderMap};
use kusi_application::{
    GetOrganizationByKey, hash_api_key,
    integrations::{ConnectorExecutor, ExecuteCapability},
};
use kusi_infrastructure::PgOrganizationRepository;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExecutionRequest {
    identifier: String,
    version: String,
    input: serde_json::Value,
}

pub(super) async fn execute<C: ConnectorExecutor>(
    repository: PgOrganizationRepository,
    headers: HeaderMap,
    request: ExecutionRequest,
    connector: impl FnOnce(Uuid) -> C,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = bearer_token(&headers).ok_or(ApiError::Unauthorized)?;
    let organization = GetOrganizationByKey::new(repository.clone())
        .execute(key)
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    if request.identifier.len() > 120 || request.version.len() > 32 {
        return Err(ApiError::BadRequest);
    }
    let executor = ExecuteCapability {
        registry: repository.clone(),
        authorization: repository.clone(),
        audit: repository,
        validator: kusi_infrastructure::JsonSchemaValidator,
        connector: connector(organization.id),
    };
    let output = executor
        .execute(
            organization.id,
            &hash_api_key(key),
            &request.identifier,
            &request.version,
            &request.input,
        )
        .await
        .map_err(|error| match error {
            kusi_application::integrations::ExecutionError::Forbidden => ApiError::Forbidden,
            kusi_application::integrations::ExecutionError::InvalidData => ApiError::BadRequest,
            kusi_application::integrations::ExecutionError::Unavailable => ApiError::Unavailable,
        })?;
    Ok(Json(
        serde_json::json!({"capability":request.identifier,"version":request.version,"data":output,"source":"configured_http_connector"}),
    ))
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;
