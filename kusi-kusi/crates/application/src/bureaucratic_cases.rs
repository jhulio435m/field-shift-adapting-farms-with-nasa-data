use async_trait::async_trait;
use kusi_domain::{
    bureaucratic_case::{
        ApproveRequestInput, CaseDetails, CaseError, CaseGuidance, CaseRequest, CaseRequestInput,
        CaseResponse, CaseResponseInput, CaseSource, CaseSummary, GuidanceInput,
    },
    local::{LocalRole, LocalUser},
};
use uuid::Uuid;

#[async_trait]
pub trait BureaucraticCaseRepository: Send + Sync {
    async fn list_cases(&self, actor: &LocalUser) -> Result<Vec<CaseSummary>, CaseError>;
    async fn case_details(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
    ) -> Result<CaseDetails, CaseError>;
    async fn case_details_by_tracking_code(
        &self,
        actor: &LocalUser,
        tracking_code: &str,
    ) -> Result<CaseDetails, CaseError>;
    async fn case_sources(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
    ) -> Result<Vec<CaseSource>, CaseError>;
    async fn upsert_request(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        input: CaseRequestInput,
    ) -> Result<CaseRequest, CaseError>;
    async fn approve_request(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        request_id: Uuid,
        input: ApproveRequestInput,
    ) -> Result<CaseRequest, CaseError>;
    async fn simulate_send(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        request_id: Uuid,
    ) -> Result<CaseRequest, CaseError>;
    async fn add_response(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        input: CaseResponseInput,
    ) -> Result<CaseResponse, CaseError>;
}

pub async fn list_cases<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
) -> Result<Vec<CaseSummary>, CaseError> {
    repository.list_cases(actor).await
}

pub async fn case_details<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    case_id: Uuid,
) -> Result<CaseDetails, CaseError> {
    repository.case_details(actor, case_id).await
}

pub async fn trace_by_code<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    tracking_code: &str,
) -> Result<CaseDetails, CaseError> {
    if tracking_code.len() > 40 || tracking_code.is_empty() {
        return Err(CaseError::Invalid);
    }
    repository
        .case_details_by_tracking_code(actor, tracking_code)
        .await
}

pub async fn case_sources<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    case_id: Uuid,
) -> Result<Vec<CaseSource>, CaseError> {
    repository.case_sources(actor, case_id).await
}

pub async fn build_guidance<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    case_id: Uuid,
    input: GuidanceInput,
    provider: Option<&dyn crate::llm::LlmProvider>,
) -> Result<CaseGuidance, CaseError> {
    input.validate()?;
    let details = repository.case_details(actor, case_id).await?;
    let sources = details.sources;
    if sources.is_empty() {
        return Err(CaseError::Unavailable);
    }
    let citation_sources = sources
        .iter()
        .map(|source| kusi_domain::bureaucratic_case::CaseCitation {
            source_id: source.id,
            source_key: source.key.clone(),
            version: source.version.clone(),
            page_or_section: source.page_or_section.clone(),
            quote: source.citation_excerpt.clone(),
            validity: source.validity,
        })
        .collect::<Vec<_>>();
    let prompt_context = sources
        .iter()
        .map(|source| {
            format!(
                "{} {} {} Estado de vigencia: {:?}.",
                source.key, source.title, source.citation_excerpt, source.validity
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let (summary, assistant_mode) = if let Some(provider) = provider {
        let request = crate::llm::InferenceRequest {
            instructions: "Responde en español claro y breve. Usa exclusivamente los extractos de fuente y hechos de demostración incluidos en la entrada. No concluyas que un pago es vigente, reembolsable o reutilizable. No declares que hubo una infracción ni tomes decisiones de la universidad. Indica lo que debe confirmar el área responsable. No propongas ni invoques herramientas.".to_owned(),
            input: format!(
                "Pregunta: {}\nHechos ficticios: pago de S/ 198 asociado a PA57002335; el trámite se interrumpió por motivos de salud.\nExtractos: {}",
                input.question.trim(), prompt_context
            ),
            tools: Vec::new(),
        };
        let result = provider
            .generate(&request)
            .await
            .map_err(|_| CaseError::Unavailable)?;
        if !result.tools.is_empty() || result.text.trim().is_empty() || result.text.len() > 3000 {
            return Err(CaseError::Unavailable);
        }
        (result.text, "local_model".to_owned())
    } else {
        (
            "El TUPA UNCP 2024 identifica el pago de S/ 198 con la constancia de expedito PA57002335. La vigencia o reutilización de ese pago después de interrumpir el trámite debe confirmarse con Tesorería; el TUPA también lista un procedimiento de devolución de tasas, pero no determina el resultado de este caso específico.".to_owned(),
            "deterministic_demo".to_owned(),
        )
    };
    Ok(CaseGuidance {
        summary,
        next_steps: vec![
            "Confirma con Tesorería si el comprobante de PA57002335 conserva vigencia o debe tramitarse de otra manera.".to_owned(),
            "Solicita a la Facultad el estado actual del trámite y los requisitos que siguen pendientes.".to_owned(),
            "Pide al área responsable confirmar si el TUPA 2024 tuvo modificatorias posteriores.".to_owned(),
        ],
        open_questions: vec![
            "¿Cuál es el estado del trámite después de la interrupción?".to_owned(),
            "¿El pago de S/ 198 puede aplicarse al paso pendiente, o corresponde solicitar devolución?".to_owned(),
            "¿Qué evidencia mínima requiere la UNCP para registrar la interrupción por salud?".to_owned(),
        ],
        citations: citation_sources,
        assistant_mode,
    })
}

pub async fn upsert_request<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    case_id: Uuid,
    input: CaseRequestInput,
) -> Result<CaseRequest, CaseError> {
    input.validate()?;
    repository.upsert_request(actor, case_id, input).await
}

pub async fn approve_request<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    case_id: Uuid,
    request_id: Uuid,
    input: ApproveRequestInput,
) -> Result<CaseRequest, CaseError> {
    if !input.confirm_share || input.version < 1 || input.digest.len() != 64 {
        return Err(CaseError::Invalid);
    }
    repository
        .approve_request(actor, case_id, request_id, input)
        .await
}

pub async fn simulate_send<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    case_id: Uuid,
    request_id: Uuid,
) -> Result<CaseRequest, CaseError> {
    repository.simulate_send(actor, case_id, request_id).await
}

pub async fn add_response<R: BureaucraticCaseRepository>(
    repository: &R,
    actor: &LocalUser,
    case_id: Uuid,
    input: CaseResponseInput,
) -> Result<CaseResponse, CaseError> {
    input.validate()?;
    if !matches!(actor.role, LocalRole::Owner | LocalRole::Approver) {
        return Err(CaseError::Forbidden);
    }
    repository.add_response(actor, case_id, input).await
}
