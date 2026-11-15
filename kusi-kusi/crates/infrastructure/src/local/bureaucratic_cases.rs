use super::LocalStore;
use async_trait::async_trait;
use kusi_application::bureaucratic_cases::BureaucraticCaseRepository;
use kusi_domain::{
    bureaucratic_case::{
        ApproveRequestInput, CaseDetails, CaseError, CaseEvent, CaseRequest, CaseRequestInput,
        CaseResponse, CaseResponseInput, CaseSource, CaseStatus, CaseSummary, SourceValidity,
    },
    local::{LocalRole, LocalUser},
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

const TUPA_VERSION: &str = "TUPA UNCP 2024 / Res. 3226-R-2024";
const TUPA_URL: &str = "https://files.uncp.edu.pe/La%20Universidad/Resoluciones/R/3226-R-2024-.pdf";
const TUPA_SHA256: &str = "9235e6813006846338c46d6d5bbeeeb4668f833285d18c180461308975dbbdd7";

struct SourceFixture {
    key: &'static str,
    title: &'static str,
    page: &'static str,
    excerpt: &'static str,
}

const SOURCE_FIXTURES: [SourceFixture; 3] = [
    SourceFixture {
        key: "PA57002335",
        title: "Constancia de expedito para optar título profesional",
        page: "pág. 156",
        excerpt: "El TUPA lista la constancia de expedito PA57002335 por S/ 198. El desglose indicado es S/ 5 constancia, S/ 2 ficha estadística, S/ 188 derecho de diploma de título profesional y S/ 3 trámite documentario. Plazo consignado: 5 días hábiles.",
    },
    SourceFixture {
        key: "PA570004D8",
        title: "Otorgar el título profesional",
        page: "pág. 95",
        excerpt: "El TUPA lista PA570004D8, con solicitud dirigida al Decano, acta de sustentación de tesis y pago de S/ 3 por trámite documentario. Plazo consignado: 30 días hábiles.",
    },
    SourceFixture {
        key: "PA57002563",
        title: "Devolución de tasas educacionales",
        page: "pág. 265",
        excerpt: "El TUPA lista PA57002563: solicitud dirigida al jefe de la Sub Unidad de Tesorería, comprobante de pago original y S/ 3 por trámite documentario. Plazo consignado: 15 días hábiles. El listado no determina que el pago de PA57002335 sea elegible para devolución tras una interrupción.",
    },
];

impl LocalStore {
    async fn ensure_bureaucratic_fixture(&self, actor: &LocalUser) -> Result<(), CaseError> {
        for source in SOURCE_FIXTURES {
            sqlx::query(
                "INSERT INTO local_bureaucratic_sources \
                 (organization_id,source_key,title,official_url,resolution,version,validity_status, \
                  page_or_section,citation_excerpt,document_sha256) \
                 VALUES ($1,$2,$3,$4,$5,$6,'needs_confirmation',$7,$8,$9) \
                 ON CONFLICT (organization_id,source_key) DO NOTHING",
            )
            .bind(actor.organization_id)
            .bind(source.key)
            .bind(source.title)
            .bind(TUPA_URL)
            .bind("Resolución 3226-R-2024")
            .bind(TUPA_VERSION)
            .bind(source.page)
            .bind(source.excerpt)
            .bind(TUPA_SHA256)
            .execute(&self.pool)
            .await
            .map_err(|_| CaseError::Unavailable)?;
        }

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM local_bureaucratic_cases \
             WHERE organization_id=$1 AND workflow_key='uncp_titulacion_demo')",
        )
        .bind(actor.organization_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        if exists {
            return Ok(());
        }

        let requester: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM local_users WHERE organization_id=$1 AND role='viewer' AND NOT disabled \
             ORDER BY created_at,id LIMIT 1",
        )
        .bind(actor.organization_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        let staff: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM local_users WHERE organization_id=$1 AND role IN ('owner','approver') \
             AND NOT disabled ORDER BY CASE role WHEN 'owner' THEN 0 ELSE 1 END,created_at,id LIMIT 1",
        )
        .bind(actor.organization_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        let requester = requester.unwrap_or(actor.id);
        let staff = staff.unwrap_or({
            if matches!(actor.role, LocalRole::Owner | LocalRole::Approver) {
                actor.id
            } else {
                requester
            }
        });
        let case_id = Uuid::new_v4();
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        let insert = sqlx::query(
            "INSERT INTO local_bureaucratic_cases \
             (id,organization_id,requester_user_id,assigned_staff_user_id,workflow_key,tracking_code,title,status, \
              payment_amount_cents,payment_currency,payment_concept,payment_reference_masked, \
              health_interruption_summary,receipt_fixture_label) \
             VALUES ($1,$2,$3,$4,'uncp_titulacion_demo','UNCP-EXP-004821', \
              'Retomar la titulación - caso ficticio UNCP','needs_information',19800,'PEN', \
              'Constancia de expedito PA57002335 (comprobante sintético)', \
              '****4821','Interrumpido por motivos de salud', \
              'Comprobante ficticio, conceptos desglosados y datos enmascarados') \
             ON CONFLICT DO NOTHING",
        )
        .bind(case_id)
        .bind(actor.organization_id)
        .bind(requester)
        .bind(staff)
        .execute(&mut *transaction)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        if insert.rows_affected() == 1 {
            self.insert_event(
                &mut transaction,
                actor.organization_id,
                case_id,
                None,
                "case_opened",
                "system",
                "Caso sintético UNCP disponible para la demostración.",
                None,
            )
            .await?;
            self.insert_event(
                &mut transaction,
                actor.organization_id,
                case_id,
                Some(requester),
                "source_added",
                "requester",
                "El caso incluye un comprobante ficticio asociado a PA57002335.",
                Some("PA57002335"),
            )
            .await?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_event(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        organization: Uuid,
        case_id: Uuid,
        actor: Option<Uuid>,
        event_type: &str,
        actor_type: &str,
        summary: &str,
        source_key: Option<&str>,
    ) -> Result<(), CaseError> {
        sqlx::query(
            "INSERT INTO local_bureaucratic_case_events \
             (organization_id,case_id,actor_user_id,event_type,actor_type,summary,source_key) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(organization)
        .bind(case_id)
        .bind(actor)
        .bind(event_type)
        .bind(actor_type)
        .bind(summary)
        .bind(source_key)
        .execute(&mut **transaction)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        Ok(())
    }

    async fn accessible_case(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
    ) -> Result<(bool, bool), CaseError> {
        let row = sqlx::query_as::<_, (bool, bool)>(
            "SELECT requester_user_id=$3, assigned_staff_user_id=$3 \
             FROM local_bureaucratic_cases WHERE organization_id=$1 AND id=$2",
        )
        .bind(actor.organization_id)
        .bind(case_id)
        .bind(actor.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?
        .ok_or(CaseError::NotFound)?;
        if !row.0 && !row.1 {
            return Err(CaseError::NotFound);
        }
        Ok(row)
    }

    async fn read_sources(&self, org: Uuid) -> Result<Vec<CaseSource>, CaseError> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, String, String, String, String, String, Option<String>, Option<i64>)>(
            "SELECT id,source_key,title,official_url,resolution,version,validity_status, \
                    page_or_section,citation_excerpt, \
                    (SELECT u.display_name FROM local_users u WHERE u.organization_id=s.organization_id AND u.id=s.verified_by), \
                    extract(epoch FROM verified_at)::bigint \
             FROM local_bureaucratic_sources s WHERE organization_id=$1 \
             ORDER BY array_position(ARRAY['PA57002335','PA570004D8','PA57002563']::text[],source_key)",
        )
        .bind(org)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        rows.into_iter()
            .map(
                |(
                    id,
                    key,
                    title,
                    url,
                    resolution,
                    version,
                    validity,
                    page,
                    excerpt,
                    verified_by,
                    verified_at,
                )| {
                    let validity =
                        SourceValidity::parse(&validity).ok_or(CaseError::Unavailable)?;
                    Ok(CaseSource {
                        id,
                        key,
                        title,
                        official_url: url,
                        resolution,
                        version,
                        validity,
                        page_or_section: page,
                        citation_excerpt: excerpt,
                        verified_by,
                        verified_at,
                    })
                },
            )
            .collect()
    }

    async fn read_request(
        &self,
        organization: Uuid,
        case_id: Uuid,
    ) -> Result<Option<CaseRequest>, CaseError> {
        let row = sqlx::query_as::<_, (Uuid, i32, String, String, bool, String, String, bool, i64)>(
            "SELECT r.id,r.version,r.recipient_unit,r.body,r.include_receipt_fixture,r.digest,r.state, \
                    EXISTS(SELECT 1 FROM local_bureaucratic_sharing_consents c \
                      WHERE c.organization_id=r.organization_id AND c.request_id=r.id \
                        AND c.request_version=r.version AND c.revoked_at IS NULL), \
                    extract(epoch FROM r.created_at)::bigint \
             FROM local_bureaucratic_requests r \
             WHERE r.organization_id=$1 AND r.case_id=$2",
        )
        .bind(organization)
        .bind(case_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        Ok(row.map(
            |(
                id,
                version,
                recipient_unit,
                body,
                include_receipt_fixture,
                digest,
                state,
                consented,
                created_at,
            )| CaseRequest {
                id,
                version,
                recipient_unit,
                body,
                include_receipt_fixture,
                digest,
                state,
                consented,
                created_at,
            },
        ))
    }

    async fn read_responses(
        &self,
        organization: Uuid,
        case_id: Uuid,
    ) -> Result<Vec<CaseResponse>, CaseError> {
        sqlx::query_as::<_, (Uuid, String, String, String, Vec<String>, i64)>(
            "SELECT r.id,r.response_type,r.body,u.display_name,r.citation_keys, \
                    extract(epoch FROM r.created_at)::bigint \
             FROM local_bureaucratic_responses r \
             JOIN local_users u ON u.organization_id=r.organization_id AND u.id=r.responder_user_id \
             WHERE r.organization_id=$1 AND r.case_id=$2 ORDER BY r.created_at,r.id",
        )
        .bind(organization)
        .bind(case_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)
        .map(|rows| rows.into_iter().map(|(id,response_type,body,actor_label,citation_keys,created_at)| CaseResponse {
            id, response_type, body, actor_label, citation_keys, created_at,
        }).collect())
    }

    async fn request_in_transaction(
        transaction: &mut Transaction<'_, Postgres>,
        organization: Uuid,
        case_id: Uuid,
        request_id: Uuid,
    ) -> Result<CaseRequest, CaseError> {
        let row = sqlx::query_as::<_, (i32, String, String, bool, String, String, bool, i64)>(
            "SELECT r.version,r.recipient_unit,r.body,r.include_receipt_fixture,r.digest,r.state, \
                    EXISTS(SELECT 1 FROM local_bureaucratic_sharing_consents c \
                     WHERE c.organization_id=r.organization_id AND c.request_id=r.id \
                       AND c.request_version=r.version AND c.revoked_at IS NULL), \
                    extract(epoch FROM r.created_at)::bigint \
             FROM local_bureaucratic_requests r \
             WHERE r.organization_id=$1 AND r.case_id=$2 AND r.id=$3",
        )
        .bind(organization)
        .bind(case_id)
        .bind(request_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|_| CaseError::Unavailable)?
        .ok_or(CaseError::NotFound)?;
        Ok(CaseRequest {
            id: request_id,
            version: row.0,
            recipient_unit: row.1,
            body: row.2,
            include_receipt_fixture: row.3,
            digest: row.4,
            state: row.5,
            consented: row.6,
            created_at: row.7,
        })
    }
}

#[async_trait]
impl BureaucraticCaseRepository for LocalStore {
    async fn list_cases(&self, actor: &LocalUser) -> Result<Vec<CaseSummary>, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, bool, bool, i64)>(
            "SELECT id,title,workflow_key,status,requester_user_id=$2,assigned_staff_user_id=$2, \
                    extract(epoch FROM updated_at)::bigint \
             FROM local_bureaucratic_cases WHERE organization_id=$1 \
               AND (requester_user_id=$2 OR assigned_staff_user_id=$2) \
             ORDER BY updated_at DESC,id",
        )
        .bind(actor.organization_id)
        .bind(actor.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        rows.into_iter()
            .map(
                |(id, title, workflow_key, status, is_requester, is_assigned_staff, updated_at)| {
                    Ok(CaseSummary {
                        id,
                        title,
                        workflow_key,
                        status: CaseStatus::parse(&status).ok_or(CaseError::Unavailable)?,
                        is_requester,
                        is_assigned_staff,
                        updated_at,
                    })
                },
            )
            .collect()
    }

    async fn case_details(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
    ) -> Result<CaseDetails, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        let (is_requester, is_assigned_staff) = self.accessible_case(actor, case_id).await?;
        let row = sqlx::query_as::<_, (String,String,String,String,i64,String,String,String,String,i64)>(
            "SELECT title,workflow_key,tracking_code,status,payment_amount_cents,payment_currency,payment_concept, \
                    payment_reference_masked,health_interruption_summary, \
                    extract(epoch FROM updated_at)::bigint \
             FROM local_bureaucratic_cases WHERE organization_id=$1 AND id=$2",
        )
        .bind(actor.organization_id)
        .bind(case_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?
        .ok_or(CaseError::NotFound)?;
        let events = sqlx::query_as::<_, (Uuid,String,String,Option<i64>,Option<String>)>(
            "SELECT e.id,e.event_type, \
                    COALESCE(u.display_name, CASE e.actor_type WHEN 'system' THEN 'Sistema de demostración' ELSE 'Usuario' END), \
                    extract(epoch FROM e.created_at)::bigint,e.source_key \
             FROM local_bureaucratic_case_events e \
             LEFT JOIN local_users u ON u.organization_id=e.organization_id AND u.id=e.actor_user_id \
             WHERE e.organization_id=$1 AND e.case_id=$2 ORDER BY e.created_at,e.id",
        )
        .bind(actor.organization_id)
        .bind(case_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?
        .into_iter()
        .map(|(id,event_type,actor_label,occurred_at,source_key)| CaseEvent {
            id,event_type,actor_label,
            summary: match source_key.as_deref() {
                Some("PA57002335") => "El caso incluye un comprobante ficticio de PA57002335.".to_owned(),
                _ => "Actividad registrada en el expediente de demostración.".to_owned(),
            },
            occurred_at: occurred_at.unwrap_or_default(),
            source_key,
        })
        .collect();
        let sources = self.read_sources(actor.organization_id).await?;
        let request = self.read_request(actor.organization_id, case_id).await?;
        let responses = self.read_responses(actor.organization_id, case_id).await?;
        Ok(CaseDetails {
            id: case_id,
            title: row.0,
            workflow_key: row.1,
            tracking_code: row.2,
            status: CaseStatus::parse(&row.3).ok_or(CaseError::Unavailable)?,
            is_requester,
            is_assigned_staff,
            payment_amount_cents: row.4,
            payment_currency: row.5,
            payment_concept: row.6,
            payment_reference_masked: row.7,
            health_interruption_summary: row.8,
            events,
            sources,
            request,
            responses,
            updated_at: row.9,
        })
    }

    async fn case_details_by_tracking_code(
        &self,
        actor: &LocalUser,
        tracking_code: &str,
    ) -> Result<CaseDetails, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        let case_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM local_bureaucratic_cases WHERE organization_id=$1 AND (tracking_code=$2 OR ($2='PA57002335' AND workflow_key='uncp_titulacion_demo')) \
             AND (requester_user_id=$3 OR assigned_staff_user_id=$3)",
        )
        .bind(actor.organization_id)
        .bind(tracking_code)
        .bind(actor.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| CaseError::Unavailable)?
        .ok_or(CaseError::NotFound)?;
        self.case_details(actor, case_id).await
    }

    async fn case_sources(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
    ) -> Result<Vec<CaseSource>, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        self.accessible_case(actor, case_id).await?;
        self.read_sources(actor.organization_id).await
    }

    async fn upsert_request(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        input: CaseRequestInput,
    ) -> Result<CaseRequest, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        let (is_requester, _) = self.accessible_case(actor, case_id).await?;
        if !is_requester {
            return Err(CaseError::Forbidden);
        }
        let digest = kusi_application::hash_api_key(
            &serde_json::to_string(&(
                actor.organization_id,
                case_id,
                &input.recipient_unit,
                &input.body,
                input.include_receipt_fixture,
            ))
            .map_err(|_| CaseError::Invalid)?,
        );
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        let old_id = sqlx::query_scalar::<_,Uuid>(
            "SELECT id FROM local_bureaucratic_requests WHERE organization_id=$1 AND case_id=$2 FOR UPDATE",
        )
        .bind(actor.organization_id)
        .bind(case_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        let request_id = Uuid::new_v4();
        if let Some(old_id) = old_id {
            sqlx::query(
                "UPDATE local_bureaucratic_sharing_consents SET revoked_at=now() \
                         WHERE organization_id=$1 AND request_id=$2 AND revoked_at IS NULL",
            )
            .bind(actor.organization_id)
            .bind(old_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| CaseError::Unavailable)?;
            sqlx::query("UPDATE local_bureaucratic_requests SET version=version+1,recipient_unit=$3,body=$4, \
                         include_receipt_fixture=$5,digest=$6,state='draft',approved_by=NULL,approved_at=NULL, \
                         shared_at=NULL,updated_at=now() WHERE organization_id=$1 AND case_id=$2")
                .bind(actor.organization_id).bind(case_id).bind(&input.recipient_unit).bind(&input.body)
                .bind(input.include_receipt_fixture).bind(&digest).execute(&mut *transaction).await
                .map_err(|_| CaseError::Unavailable)?;
        } else {
            sqlx::query(
                "INSERT INTO local_bureaucratic_requests \
                         (id,organization_id,case_id,requester_user_id,recipient_unit,body, \
                          include_receipt_fixture,digest) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
            )
            .bind(request_id)
            .bind(actor.organization_id)
            .bind(case_id)
            .bind(actor.id)
            .bind(&input.recipient_unit)
            .bind(&input.body)
            .bind(input.include_receipt_fixture)
            .bind(&digest)
            .execute(&mut *transaction)
            .await
            .map_err(|_| CaseError::Unavailable)?;
        }
        sqlx::query(
            "UPDATE local_bureaucratic_cases SET status='ready_for_review',updated_at=now() \
                     WHERE organization_id=$1 AND id=$2",
        )
        .bind(actor.organization_id)
        .bind(case_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        self.insert_event(
            &mut transaction,
            actor.organization_id,
            case_id,
            Some(actor.id),
            if old_id.is_some() {
                "request_edited"
            } else {
                "request_drafted"
            },
            "requester",
            "Se preparó una consulta para revisión de la persona.",
            None,
        )
        .await?;
        let request_id = old_id.unwrap_or(request_id);
        let result = Self::request_in_transaction(
            &mut transaction,
            actor.organization_id,
            case_id,
            request_id,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        Ok(result)
    }

    async fn approve_request(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        request_id: Uuid,
        input: ApproveRequestInput,
    ) -> Result<CaseRequest, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        let (is_requester, _) = self.accessible_case(actor, case_id).await?;
        if !is_requester {
            return Err(CaseError::Forbidden);
        }
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        let row = sqlx::query_as::<_,(i32,String,String,String,String)>(
            "SELECT version,digest,state,recipient_unit,requester_user_id::text \
             FROM local_bureaucratic_requests WHERE organization_id=$1 AND case_id=$2 AND id=$3 FOR UPDATE",
        )
        .bind(actor.organization_id).bind(case_id).bind(request_id).fetch_optional(&mut *transaction).await
        .map_err(|_| CaseError::Unavailable)?.ok_or(CaseError::NotFound)?;
        if row.0 != input.version
            || row.1 != input.digest
            || row.2 != "draft"
            || row.4 != actor.id.to_string()
        {
            return Err(CaseError::Conflict);
        }
        sqlx::query("UPDATE local_bureaucratic_requests SET state='approved',approved_by=$4,approved_at=now(),updated_at=now() \
                     WHERE organization_id=$1 AND case_id=$2 AND id=$3")
            .bind(actor.organization_id).bind(case_id).bind(request_id).bind(actor.id)
            .execute(&mut *transaction).await.map_err(|_|CaseError::Unavailable)?;
        sqlx::query("INSERT INTO local_bureaucratic_sharing_consents \
                     (organization_id,request_id,request_version,recipient_unit,scope,consented_by) \
                     VALUES ($1,$2,$3,$4,'request_body_and_selected_receipt_fixture',$5) \
                     ON CONFLICT (organization_id,request_id,request_version) DO UPDATE SET \
                       recipient_unit=EXCLUDED.recipient_unit,scope=EXCLUDED.scope,consented_by=EXCLUDED.consented_by, \
                       consented_at=now(),revoked_at=NULL")
            .bind(actor.organization_id).bind(request_id).bind(input.version).bind(row.3).bind(actor.id)
            .execute(&mut *transaction).await.map_err(|_|CaseError::Unavailable)?;
        self.insert_event(
            &mut transaction,
            actor.organization_id,
            case_id,
            Some(actor.id),
            "request_approved",
            "requester",
            "La persona revisó el borrador y consintió compartir esta versión.",
            None,
        )
        .await?;
        let result = Self::request_in_transaction(
            &mut transaction,
            actor.organization_id,
            case_id,
            request_id,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        Ok(result)
    }

    async fn simulate_send(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        request_id: Uuid,
    ) -> Result<CaseRequest, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        let (is_requester, _) = self.accessible_case(actor, case_id).await?;
        if !is_requester {
            return Err(CaseError::Forbidden);
        }
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        let row = sqlx::query_as::<_,(String,i32,Option<Uuid>,bool)>(
            "SELECT r.state,r.version,r.approved_by,EXISTS(SELECT 1 FROM local_bureaucratic_sharing_consents c \
              WHERE c.organization_id=r.organization_id AND c.request_id=r.id AND c.request_version=r.version \
                AND c.revoked_at IS NULL AND c.consented_by=$4) \
             FROM local_bureaucratic_requests r WHERE r.organization_id=$1 AND r.case_id=$2 AND r.id=$3 FOR UPDATE",
        )
        .bind(actor.organization_id).bind(case_id).bind(request_id).bind(actor.id)
        .fetch_optional(&mut *transaction).await.map_err(|_|CaseError::Unavailable)?.ok_or(CaseError::NotFound)?;
        if row.0 == "simulated_sent" {
            let result = Self::request_in_transaction(
                &mut transaction,
                actor.organization_id,
                case_id,
                request_id,
            )
            .await?;
            transaction
                .commit()
                .await
                .map_err(|_| CaseError::Unavailable)?;
            return Ok(result);
        }
        if row.0 != "approved" || row.2 != Some(actor.id) || !row.3 {
            return Err(CaseError::Forbidden);
        }
        sqlx::query("UPDATE local_bureaucratic_requests SET state='simulated_sent',shared_at=now(),updated_at=now() \
                     WHERE organization_id=$1 AND case_id=$2 AND id=$3")
            .bind(actor.organization_id).bind(case_id).bind(request_id).execute(&mut *transaction).await
            .map_err(|_|CaseError::Unavailable)?;
        sqlx::query(
            "UPDATE local_bureaucratic_cases SET status='awaiting_institution',updated_at=now() \
                     WHERE organization_id=$1 AND id=$2",
        )
        .bind(actor.organization_id)
        .bind(case_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| CaseError::Unavailable)?;
        self.insert_event(
            &mut transaction,
            actor.organization_id,
            case_id,
            Some(actor.id),
            "request_simulated_sent",
            "system",
            "La consulta quedó registrada como envío simulado local.",
            None,
        )
        .await?;
        let result = Self::request_in_transaction(
            &mut transaction,
            actor.organization_id,
            case_id,
            request_id,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        Ok(result)
    }

    async fn add_response(
        &self,
        actor: &LocalUser,
        case_id: Uuid,
        input: CaseResponseInput,
    ) -> Result<CaseResponse, CaseError> {
        self.ensure_bureaucratic_fixture(actor).await?;
        let (_, is_assigned_staff) = self.accessible_case(actor, case_id).await?;
        if !is_assigned_staff || !matches!(actor.role, LocalRole::Owner | LocalRole::Approver) {
            return Err(CaseError::Forbidden);
        }
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        let state: Option<String> = sqlx::query_scalar(
            "SELECT status FROM local_bureaucratic_cases WHERE organization_id=$1 AND id=$2 FOR UPDATE",
        ).bind(actor.organization_id).bind(case_id).fetch_optional(&mut *transaction).await
            .map_err(|_|CaseError::Unavailable)?;
        if state.as_deref() != Some("awaiting_institution") {
            return Err(CaseError::Conflict);
        }
        let id = Uuid::new_v4();
        let result=sqlx::query_as::<_,(i64,String)>(
            "INSERT INTO local_bureaucratic_responses \
             (id,organization_id,case_id,responder_user_id,response_type,body,citation_keys) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING extract(epoch FROM created_at)::bigint,responder_user_id::text",
        ).bind(id).bind(actor.organization_id).bind(case_id).bind(actor.id).bind(&input.response_type)
            .bind(&input.body).bind(&input.citation_keys).fetch_one(&mut *transaction).await
            .map_err(|_|CaseError::Unavailable)?;
        sqlx::query("UPDATE local_bureaucratic_cases SET status='answered',updated_at=now() WHERE organization_id=$1 AND id=$2")
            .bind(actor.organization_id).bind(case_id).execute(&mut *transaction).await.map_err(|_|CaseError::Unavailable)?;
        self.insert_event(
            &mut transaction,
            actor.organization_id,
            case_id,
            Some(actor.id),
            "response_recorded",
            "staff",
            "Personal administrativo registró una respuesta atribuida.",
            input.citation_keys.first().map(String::as_str),
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| CaseError::Unavailable)?;
        Ok(CaseResponse {
            id,
            response_type: input.response_type,
            body: input.body,
            actor_label: actor.display_name.clone(),
            citation_keys: input.citation_keys,
            created_at: result.0,
        })
    }
}
