use super::store;
use crate::AppState;
use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
};
use kusi_application::bureaucratic_cases::BureaucraticCaseRepository;
use kusi_application::llm::LlmProvider;
use kusi_application::student_payments::{StudentPaymentError, list_payments, validate_payment};
use kusi_domain::{
    bureaucratic_case::{
        ApproveRequestInput, CaseError, CaseRequestInput, CaseResponseInput, GuidanceInput,
    },
    local::LocalUser,
};
use kusi_infrastructure::student_payments::LocalStudentPaymentsPlatform;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DemoPaymentLookup {
    code: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DemoDniLookup {
    dni: String,
}
fn demo_payment_status(error: StudentPaymentError) -> StatusCode {
    match error {
        StudentPaymentError::Invalid => StatusCode::BAD_REQUEST,
        StudentPaymentError::NotFound => StatusCode::NOT_FOUND,
        StudentPaymentError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}
fn require_local_demo(s: &AppState) -> Result<(), StatusCode> {
    if s.local.store.is_some() {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
pub(super) async fn demo_payments(State(s): State<AppState>) -> Result<Json<Value>, StatusCode> {
    require_local_demo(&s)?;
    let user = LocalStudentPaymentsPlatform::demo_user();
    let platform = LocalStudentPaymentsPlatform;
    let payments = list_payments(&platform, &user)
        .await
        .map_err(demo_payment_status)?;
    Ok(Json(
        json!({"platform": "clon_local", "payments": payments}),
    ))
}
pub(super) async fn demo_payments_by_dni(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<DemoDniLookup>,
) -> Result<Json<Value>, StatusCode> {
    require_local_demo(&s)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if origin != Some(s.local.origin.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.dni.len() != 8 || !input.dni.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if input.dni == "70000002" {
        return Ok(Json(json!({
            "platform": "clon_local",
            "student": {"display_name": "Estudiante ficticio sin pagos", "dni_masked": "70***002", "synthetic": true},
            "payments": []
        })));
    }
    if input.dni != kusi_infrastructure::student_payments::LocalStudentPaymentsPlatform::DEMO_DNI {
        return Err(StatusCode::NOT_FOUND);
    }
    let user = LocalStudentPaymentsPlatform::demo_user();
    let platform = LocalStudentPaymentsPlatform;
    let payments = list_payments(&platform, &user)
        .await
        .map_err(demo_payment_status)?;
    Ok(Json(json!({
        "platform": "clon_local",
        "student": {"display_name": "Estudiante ficticio", "dni_masked": "70***001", "synthetic": true},
        "payments": payments
    })))
}
pub(super) async fn demo_validate_payment(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<DemoPaymentLookup>,
) -> Result<Json<Value>, StatusCode> {
    require_local_demo(&s)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if origin != Some(s.local.origin.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let user = LocalStudentPaymentsPlatform::demo_user();
    let platform = LocalStudentPaymentsPlatform;
    let payment = validate_payment(&platform, &user, &input.code)
        .await
        .map_err(demo_payment_status)?;
    Ok(Json(json!({"platform": "clon_local", "payment": payment})))
}
pub(super) async fn demo_validate_receipt(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, StatusCode> {
    require_local_demo(&s)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if origin != Some(s.local.origin.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !matches!(content_type, "image/jpeg" | "image/png" | "image/webp") {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let bytes = body.to_vec();
    let code = tokio::task::spawn_blocking(move || {
        kusi_infrastructure::student_payments::extract_receipt_reference(&bytes)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .map_err(|error| match error {
        kusi_infrastructure::student_payments::ReceiptOcrError::InvalidImage => {
            StatusCode::BAD_REQUEST
        }
        kusi_infrastructure::student_payments::ReceiptOcrError::NoReference => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        kusi_infrastructure::student_payments::ReceiptOcrError::Unavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
    })?;
    let user = LocalStudentPaymentsPlatform::demo_user();
    let platform = LocalStudentPaymentsPlatform;
    let payment = validate_payment(&platform, &user, &code)
        .await
        .map_err(demo_payment_status)?;
    Ok(Json(json!({
        "platform": "clon_local",
        "validation_method": "local_ocr",
        "detected_reference": payment.receipt_reference_masked,
        "payment": payment
    })))
}
fn case_status(error: CaseError) -> StatusCode {
    match error {
        CaseError::Invalid => StatusCode::BAD_REQUEST,
        CaseError::Forbidden => StatusCode::FORBIDDEN,
        CaseError::NotFound => StatusCode::NOT_FOUND,
        CaseError::Conflict => StatusCode::CONFLICT,
        CaseError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}
pub(super) async fn case_list(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
) -> Result<Json<Value>, StatusCode> {
    let rows = store(&s)?.list_cases(&u).await.map_err(case_status)?;
    Ok(Json(json!({"cases": rows})))
}
pub(super) async fn case_detail(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let row = store(&s)?.case_details(&u, id).await.map_err(case_status)?;
    Ok(Json(json!({"case": row})))
}
pub(super) async fn case_sources(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let rows = store(&s)?.case_sources(&u, id).await.map_err(case_status)?;
    Ok(Json(json!({"sources": rows})))
}
pub(super) async fn case_guidance(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<GuidanceInput>,
) -> Result<Json<Value>, StatusCode> {
    let store = store(&s)?;
    let provider = s
        .llm_profiles
        .get(&u.organization_id)
        .and_then(|profiles| profiles.get("bureaucratic_guidance"))
        .filter(|p| {
            matches!(
                p.provider,
                kusi_infrastructure::llm::Provider::Ollama
                    | kusi_infrastructure::llm::Provider::Vllm
                    | kusi_infrastructure::llm::Provider::Lmstudio
                    | kusi_infrastructure::llm::Provider::OpenaiCompatible
            )
        });
    let provider: Option<&dyn LlmProvider> = provider.map(|p| p as &dyn LlmProvider);
    let result =
        kusi_application::bureaucratic_cases::build_guidance(store, &u, id, input, provider)
            .await
            .map_err(case_status)?;
    Ok(Json(json!({"guidance": result})))
}
pub(super) async fn case_request(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<CaseRequestInput>,
) -> Result<Json<Value>, StatusCode> {
    let result = kusi_application::bureaucratic_cases::upsert_request(store(&s)?, &u, id, input)
        .await
        .map_err(case_status)?;
    Ok(Json(json!({"request": result})))
}
pub(super) async fn case_approve(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path((id, request_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ApproveRequestInput>,
) -> Result<Json<Value>, StatusCode> {
    let result = kusi_application::bureaucratic_cases::approve_request(
        store(&s)?,
        &u,
        id,
        request_id,
        input,
    )
    .await
    .map_err(case_status)?;
    Ok(Json(json!({"request": result})))
}
pub(super) async fn case_simulate_send(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path((id, request_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, StatusCode> {
    let result =
        kusi_application::bureaucratic_cases::simulate_send(store(&s)?, &u, id, request_id)
            .await
            .map_err(case_status)?;
    Ok(Json(
        json!({"request": result, "delivery": "simulated_local_only"}),
    ))
}
pub(super) async fn case_response(
    State(s): State<AppState>,
    Extension(u): Extension<LocalUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<CaseResponseInput>,
) -> Result<Json<Value>, StatusCode> {
    let result = kusi_application::bureaucratic_cases::add_response(store(&s)?, &u, id, input)
        .await
        .map_err(case_status)?;
    Ok(Json(json!({"response": result})))
}
