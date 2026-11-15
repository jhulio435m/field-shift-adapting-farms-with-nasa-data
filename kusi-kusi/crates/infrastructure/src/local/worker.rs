use super::LocalStore;
use crate::student_payments::LocalStudentPaymentsPlatform;
use kusi_application::{
    creative::{CreativeRepository, prepare_proposal},
    local::{LocalError, PublicationInput, simulated_recipe},
    student_payments::{list_payments, validate_payment},
};
use kusi_domain::local::{LocalIntent, interpret_local};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Context {
    asset_id: Option<Uuid>,
    creative_job_id: Option<Uuid>,
    publication_id: Option<Uuid>,
}
#[derive(FromRow)]
struct Inbox {
    id: i64,
    event_id: String,
    kind: String,
    content: String,
    asset_id: Option<Uuid>,
    group_id: Option<Uuid>,
    attempts: i32,
}
struct Group {
    org: Uuid,
    channel: Uuid,
    sender: String,
    logical: Uuid,
    lease: Uuid,
    rows: Vec<Inbox>,
}

pub async fn run_worker(store: LocalStore, debounce: i64) {
    let messages = async {
        loop {
            if tick_messages(&store, debounce).await.is_err() {
                tracing::warn!("local message worker unavailable; content omitted");
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    };
    let artifacts = async {
        loop {
            if tick_artifacts(&store).await.is_err() {
                tracing::warn!("local preview worker unavailable; content omitted");
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    };
    let publications = async {
        loop {
            if tick_publications(&store).await.is_err() {
                tracing::warn!("local publication worker unavailable");
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    };
    tokio::join!(messages, artifacts, publications);
}
async fn claim(store: &LocalStore, debounce: i64) -> Result<Option<Group>, LocalError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| LocalError::Unavailable)?;
    let candidate=sqlx::query_as::<_,(Uuid,Uuid,String)>("SELECT i.organization_id,i.channel_id,i.sender FROM channel_inbox i WHERE i.processed_at IS NULL AND i.kind IN ('text','audio','image') AND i.error_code IS NULL AND (i.lease_until IS NULL OR i.lease_until<=now()) AND NOT EXISTS(SELECT 1 FROM channel_inbox a WHERE a.organization_id=i.organization_id AND a.channel_id=i.channel_id AND a.sender=i.sender AND a.processed_at IS NULL AND a.lease_until>now()) GROUP BY i.organization_id,i.channel_id,i.sender HAVING max(i.received_at)<=now()-$1::bigint*interval '1 second' ORDER BY min(i.id) LIMIT 1").bind(debounce).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    let Some((org, channel, sender)) = candidate else {
        return Ok(None);
    };
    let locked: bool =
        sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("conversation:{org}:{channel}:{sender}"))
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
    if !locked {
        return Ok(None);
    }
    let busy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM channel_inbox WHERE organization_id=$1 AND channel_id=$2 AND sender=$3 AND processed_at IS NULL AND lease_until>now())").bind(org).bind(channel).bind(&sender).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    if busy {
        return Ok(None);
    }
    let ready:bool=sqlx::query_scalar("SELECT max(received_at)<=now()-$4::bigint*interval '1 second' FROM channel_inbox WHERE organization_id=$1 AND channel_id=$2 AND sender=$3 AND processed_at IS NULL AND error_code IS NULL").bind(org).bind(channel).bind(&sender).bind(debounce).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    if !ready {
        return Ok(None);
    }
    let first:Option<Uuid>=sqlx::query_scalar("SELECT group_id FROM channel_inbox WHERE organization_id=$1 AND channel_id=$2 AND sender=$3 AND processed_at IS NULL AND error_code IS NULL ORDER BY id LIMIT 1").bind(org).bind(channel).bind(&sender).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    let rows=sqlx::query_as::<_,Inbox>("SELECT id,event_id,kind,content,asset_id,group_id,attempts FROM channel_inbox WHERE organization_id=$1 AND channel_id=$2 AND sender=$3 AND processed_at IS NULL AND error_code IS NULL AND group_id IS NOT DISTINCT FROM $4 ORDER BY id LIMIT 16 FOR UPDATE SKIP LOCKED").bind(org).bind(channel).bind(&sender).bind(first).fetch_all(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    if rows.is_empty() {
        return Ok(None);
    }
    let logical = rows[0].group_id.unwrap_or_else(Uuid::new_v4);
    let lease = Uuid::new_v4();
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    sqlx::query("UPDATE channel_inbox SET group_id=$2,lease_token=$3,lease_until=now()+interval '60 seconds',attempts=attempts+1 WHERE id=ANY($1)").bind(ids).bind(logical).bind(lease).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    tx.commit().await.map_err(|_| LocalError::Unavailable)?;
    Ok(Some(Group {
        org,
        channel,
        sender,
        logical,
        lease,
        rows,
    }))
}
async fn tick_messages(store: &LocalStore, debounce: i64) -> Result<(), LocalError> {
    // Delivery statuses are bookkeeping, never conversational instructions.
    let statuses=sqlx::query_as::<_,(i64,Uuid,Uuid,String,String)>("SELECT id,organization_id,channel_id,event_id,content FROM channel_inbox WHERE kind='status' AND processed_at IS NULL ORDER BY id LIMIT 50").fetch_all(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
    for (id, org, channel, key, status) in statuses {
        if ["sent", "delivered", "read", "failed"].contains(&status.as_str())
            && let Some(raw) = key.strip_prefix("status:")
        {
            let parts: Vec<_> = raw.rsplitn(3, ':').collect();
            if parts.len() == 3 {
                sqlx::query("UPDATE local_outbox SET status=$4,updated_at=now() WHERE organization_id=$1 AND channel_id=$2 AND provider_message_id=$3 AND status NOT IN ('read','simulated')").bind(org).bind(channel).bind(parts[2]).bind(if status=="sent"{"accepted"}else{&status}).execute(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
            }
        }
        sqlx::query("UPDATE channel_inbox SET processed_at=now() WHERE id=$1")
            .bind(id)
            .execute(&store.pool)
            .await
            .map_err(|_| LocalError::Unavailable)?;
    }
    let Some(group) = claim(store, debounce).await? else {
        return Ok(());
    };
    match process(store, &group).await {
        Ok(()) => Ok(()),
        Err(_) if group.rows.iter().any(|r| r.attempts >= 2) => {
            sqlx::query("UPDATE channel_inbox SET processed_at=now(),error_code='local_processing_failed',lease_until=NULL,lease_token=NULL WHERE group_id=$1 AND lease_token=$2").bind(group.logical).bind(group.lease).execute(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
            Ok(())
        }
        Err(error) => {
            sqlx::query("UPDATE channel_inbox SET lease_until=now()+interval '5 seconds' WHERE group_id=$1 AND lease_token=$2").bind(group.logical).bind(group.lease).execute(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
            Err(error)
        }
    }
}
async fn process(store: &LocalStore, group: &Group) -> Result<(), LocalError> {
    let text = group
        .rows
        .iter()
        .filter(|r| r.kind == "text")
        .map(|r| r.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let intent = interpret_local(&text);
    let mut actor_id:Option<Uuid>=sqlx::query_scalar("SELECT u.id FROM local_conversations c JOIN local_users u ON u.id=c.user_id WHERE c.organization_id=$1 AND c.channel_id=$2 AND c.sender=$3 AND c.id=c.channel_id AND NOT u.disabled LIMIT 1").bind(group.org).bind(group.channel).bind(&group.sender).fetch_optional(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
    if actor_id.is_none() {
        actor_id=sqlx::query_scalar("SELECT b.user_id FROM local_channel_bindings b JOIN local_users u ON u.id=b.user_id AND u.organization_id=b.organization_id WHERE b.organization_id=$1 AND b.channel_id=$2 AND b.sender=$3 AND b.revoked_at IS NULL AND b.expires_at>now() AND NOT u.disabled").bind(group.org).bind(group.channel).bind(&group.sender).fetch_optional(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
    }
    let mut linked = false;
    if let LocalIntent::Link(code) = &intent {
        let mut tx = store
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let user:Option<Uuid>=sqlx::query_scalar("UPDATE local_link_codes SET used_at=now() WHERE code_hash=$1 AND organization_id=$2 AND channel_id=$3 AND used_at IS NULL AND expires_at>now() RETURNING user_id").bind(kusi_application::hash_api_key(code)).bind(group.org).bind(group.channel).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        if let Some(user) = user {
            sqlx::query("INSERT INTO local_channel_bindings (organization_id,channel_id,sender,user_id) VALUES ($1,$2,$3,$4) ON CONFLICT (organization_id,channel_id,sender) DO UPDATE SET user_id=EXCLUDED.user_id,updated_at=now(),revoked_at=NULL,expires_at=now()+interval '30 days',version=local_channel_bindings.version+1").bind(group.org).bind(group.channel).bind(&group.sender).bind(user).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            actor_id = Some(user);
            linked = true;
        }
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
    }
    let conversation=sqlx::query_as::<_,(Uuid,String)>("SELECT id,CASE WHEN context_expires_at>now() THEN context ELSE '{}' END FROM local_conversations WHERE organization_id=$1 AND channel_id=$2 AND sender=$3 AND user_id IS NOT DISTINCT FROM $4 ORDER BY created_at DESC LIMIT 1").bind(group.org).bind(group.channel).bind(&group.sender).bind(actor_id).fetch_optional(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
    let (conversation, context) = if let Some(row) = conversation {
        row
    } else {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO local_conversations (id,organization_id,user_id,channel_id,sender,title) VALUES ($1,$2,$3,$4,$5,'Conversación vinculada')").bind(id).bind(group.org).bind(actor_id).bind(group.channel).bind(&group.sender).execute(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
        (id, "{}".into())
    };
    let mut context: Context = serde_json::from_str(&context).unwrap_or_default();
    for row in &group.rows {
        if let Some(asset) = row.asset_id {
            store.asset(group.org, asset).await?;
            context.asset_id = Some(asset);
            context.creative_job_id = None;
        }
    }
    let mut metadata = json!({"simulated":true});
    let response = if linked {
        "Cuenta vinculada. Tus permisos del panel se aplicarán a las próximas solicitudes."
            .to_owned()
    } else if let Some(user) = actor_id
        && let Some(answer) = store
            .adaptive_reply(
                group.org,
                user,
                group.channel,
                &group.sender,
                &text,
                group.logical,
            )
            .await?
    {
        metadata["adaptive_loopback"] = json!(true);
        answer
    } else if actor_id.is_none() {
        "Vincula tu cuenta desde el panel y envía /vincular seguido del código. El número o usuario de mensajería no autoriza consultas.".into()
    } else if group.rows.iter().any(|r| r.kind == "audio") {
        "Recibí una nota de voz. En este entorno local no se descarga ni transcribe audio externo; escribe tu solicitud para continuar.".into()
    } else if group.rows.iter().any(|r| r.kind == "image")
        && !group.rows.iter().any(|r| r.kind == "text")
    {
        "Recibí la foto en el webhook. Esta demo no descarga ni valida el contenido de imágenes de WhatsApp; envía el código UNCP-EXP-004821 o PA57002335 para consultar la trazabilidad ficticia.".into()
    } else {
        let actor = store
            .user(actor_id.ok_or(LocalError::Unauthorized)?)
            .await?;
        match intent{
   LocalIntent::CreateVideo|LocalIntent::CreateImage if actor.role.may_edit()=>{
    if let Some(asset)=context.asset_id{
     store.asset(group.org,asset).await?;let kind=if intent==LocalIntent::CreateVideo{"video"}else{"image"};let recipe=simulated_recipe(kind);
     let proposal=prepare_proposal(&format!("local_product_{kind}"),&recipe,json!({"prompt":text,"asset_id":asset,"kind":kind}),&crate::JsonSchemaValidator).map_err(|_|LocalError::Invalid)?;
     let repo=crate::PgOrganizationRepository::new(store.pool.clone());let job=repo.create(group.org,&kusi_application::hash_api_key(&actor.id.to_string()),group.logical,proposal).await.map_err(|_|LocalError::Unavailable)?;
     context.creative_job_id=Some(job.id);metadata["creative_job_id"]=json!(job.id);
     format!("Preparé el borrador de {kind}. Revisa el producto, los parámetros y el costo en Trabajos creativos. La simulación comenzará solo después de una aprobación autorizada.")
    }else{"Adjunta una foto del producto para preparar el borrador.".into()}
   },
   LocalIntent::PreparePublication(platform) if actor.role.may_edit()=>{
    let generated=if let Some(job)=context.creative_job_id{sqlx::query_scalar::<_,Uuid>("SELECT id FROM local_assets WHERE organization_id=$1 AND creative_job_id=$2").bind(group.org).bind(job).fetch_optional(&store.pool).await.map_err(|_|LocalError::Unavailable)?}else{context.asset_id};
    if let Some(asset)=generated{
     let asset=store.asset(group.org,asset).await?;let caption=text.split_once('|').map(|v|v.1.trim().to_owned()).filter(|s|!s.is_empty()).unwrap_or_else(||format!("Presentamos {}.",asset.label));
     let publication=store.create_publication(&actor,PublicationInput{asset_id:asset.id,platform:platform.into(),caption,privacy:"private".into(),scheduled_at:None,idempotency_key:group.logical}).await?;
     context.publication_id=Some(publication.id);metadata["publication_id"]=json!(publication.id);
     format!("Preparé un borrador para {platform}. Revisa texto, archivo, privacidad y horario en Publicaciones. Requiere una aprobación independiente y su envío será simulado.")
    }else if context.creative_job_id.is_some(){"La pieza todavía no está archivada. Aprueba el trabajo y espera el aviso de vista previa antes de preparar la publicación.".into()}else{"Adjunta primero el archivo que quieres usar en la publicación.".into()}
   },
   LocalIntent::AcademicSummary if actor.role==kusi_domain::local::LocalRole::Viewer=>{
    match store.read_academic_summary(group.org,actor.id).await{
     Ok(summaries)=>{
      let rows=summaries.into_iter().map(|student|{
       let marks=student.marks.into_iter().map(|mark|format!("{}: {}",mark.subject,mark.value)).collect::<Vec<_>>().join(" · ");
       format!("{} ({})\n{}",student.display_label,student.level,marks)
      }).collect::<Vec<_>>().join("\n\n");
      metadata["simulation_kind"]=json!("academic_demo_fixture");
      format!("Resultados académicos simulados · datos ficticios:\n\n{rows}")
     },
     Err(LocalError::Forbidden)=>"No hay estudiantes de demostración vinculados a esta cuenta. Solicita a la escuela que revise el vínculo; no se consultó ningún registro.".into(),
     Err(error)=>return Err(error),
    }
   },
   LocalIntent::AcademicSummary=>"Esta consulta solo está disponible para una cuenta familiar de demostración autorizada. Solicita ayuda a la escuela.".into(),
   LocalIntent::ListStudentPayments=>{
    let platform=LocalStudentPaymentsPlatform;
    let payments=list_payments(&platform,&actor).await.map_err(|_|LocalError::Unavailable)?;
    metadata["simulation_kind"]=json!("student_payment_platform_clone");
    let rows=payments.iter().map(|payment|format!("{} · S/ {:.2} · {} · pago {} · trámite {}",payment.payment_code,payment.amount_cents as f64/100.0,payment.concept,match payment.payment_state{kusi_domain::student_payment::PaymentState::Confirmed=>"confirmado",kusi_domain::student_payment::PaymentState::Pending=>"pendiente",kusi_domain::student_payment::PaymentState::NotFound=>"no encontrado",kusi_domain::student_payment::PaymentState::Reversed=>"revertido"},match payment.process_state{kusi_domain::student_payment::ProcessState::NotStarted=>"sin iniciar",kusi_domain::student_payment::ProcessState::InProgress=>"en curso",kusi_domain::student_payment::ProcessState::Interrupted=>"interrumpido",kusi_domain::student_payment::ProcessState::Completed=>"completado"})).collect::<Vec<_>>().join("\n");
    format!("Pagos del estudiante en la plataforma de demostración:\n{rows}\n\nDatos ficticios; no consultan sistemas reales.")
   },
   LocalIntent::LookupStudentPayment(code)=>{
    let platform=LocalStudentPaymentsPlatform;
    match validate_payment(&platform,&actor,&code).await {
     Ok(payment)=>{
      metadata["simulation_kind"]=json!("student_payment_platform_clone");
      let paid=match payment.payment_state{kusi_domain::student_payment::PaymentState::Confirmed=>"Pago confirmado en la plataforma demo",kusi_domain::student_payment::PaymentState::Pending=>"Pago pendiente de confirmación",kusi_domain::student_payment::PaymentState::NotFound=>"Pago no encontrado",kusi_domain::student_payment::PaymentState::Reversed=>"Pago revertido"};
      let process=match payment.process_state{kusi_domain::student_payment::ProcessState::NotStarted=>"sin iniciar",kusi_domain::student_payment::ProcessState::InProgress=>"en curso",kusi_domain::student_payment::ProcessState::Interrupted=>"interrumpido",kusi_domain::student_payment::ProcessState::Completed=>"completado"};
      format!("{}\nCódigo: {}\nConcepto: {}\nMonto: S/ {:.2}\nEstado del pago: {}\nEstado del trámite: {}\nReferencia: {}\nOrigen: {}. Datos ficticios.",paid,payment.payment_code,payment.concept,payment.amount_cents as f64/100.0,paid,process,payment.receipt_reference_masked,payment.platform_name)
     },
     Err(kusi_application::student_payments::StudentPaymentError::NotFound)=>"No encontré un pago con ese código en la plataforma de demostración. Revisa el código o consulta el historial de pagos.".into(),
     Err(_)=>return Err(LocalError::Invalid),
    }
   },
   LocalIntent::Status=>{
    if let Some(id)=context.creative_job_id{
     let job=crate::PgOrganizationRepository::new(store.pool.clone()).get(group.org,id).await.map_err(|_|LocalError::Unavailable)?;metadata["creative_job_id"]=json!(id);
     format!("El trabajo está en estado {}. Puedes consultar sus parámetros y resultado en Trabajos creativos.",match job.state{kusi_domain::creative::CreativeState::AwaitingApproval=>"pendiente de aprobación",kusi_domain::creative::CreativeState::Approved=>"aprobado",kusi_domain::creative::CreativeState::Submitting=>"enviando",kusi_domain::creative::CreativeState::Queued=>"en cola",kusi_domain::creative::CreativeState::Processing=>"procesando",kusi_domain::creative::CreativeState::Succeeded=>"completado",kusi_domain::creative::CreativeState::Failed=>"fallido",kusi_domain::creative::CreativeState::Unknown=>"incierto",kusi_domain::creative::CreativeState::Canceled=>"cancelado"})
    }else{"Todavía no hay un trabajo asociado a esta conversación.".into()}
   },
   LocalIntent::ApprovalHelp=>"Las aprobaciones se realizan sobre el borrador exacto en el panel. Abre Trabajos creativos o Publicaciones y confirma allí.".into(),
   LocalIntent::Link(_)=>"El código no es válido o ya venció. Genera otro desde Conexiones.".into(),
   LocalIntent::CreateVideo|LocalIntent::CreateImage|LocalIntent::PreparePublication(_)=>"Tu rol puede revisar información, pero no crear borradores. Solicita acceso a un propietario.".into(),
   LocalIntent::Help=>"En la demo de pagos UNCP escribe «mis pagos» o envía el código de un recibo de prueba. En la demo académica consulta «mis calificaciones» o «mis notas» cuando haya una cuenta vinculada. Las demás solicitudes usan fixtures locales; no consultan sistemas reales.".into(),
  }
    };
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| LocalError::Unavailable)?;
    let changed=sqlx::query("UPDATE channel_inbox SET processed_at=now(),lease_until=NULL,lease_token=NULL WHERE group_id=$1 AND lease_token=$2 AND processed_at IS NULL").bind(group.logical).bind(group.lease).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?.rows_affected();
    if changed == 0 {
        return Ok(());
    }
    if group.channel != conversation
        && let Some(user) = actor_id
    {
        let active:Option<Uuid>=sqlx::query_scalar("SELECT b.user_id FROM local_channel_bindings b JOIN local_users u ON u.id=b.user_id AND u.organization_id=b.organization_id WHERE b.organization_id=$1 AND b.channel_id=$2 AND b.sender=$3 AND b.user_id=$4 AND b.revoked_at IS NULL AND b.expires_at>now() AND NOT u.disabled FOR SHARE OF b,u").bind(group.org).bind(group.channel).bind(&group.sender).bind(user).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        if active.is_none() {
            return Err(LocalError::Forbidden);
        }
    }
    for row in &group.rows {
        sqlx::query("INSERT INTO local_messages (organization_id,conversation_id,event_key,direction,content,metadata) VALUES ($1,$2,$3,'in',$4,$5) ON CONFLICT DO NOTHING").bind(group.org).bind(conversation).bind(&row.event_id).bind(&row.content).bind(json!({"asset_id":row.asset_id}).to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    }
    sqlx::query("INSERT INTO local_messages (organization_id,conversation_id,event_key,direction,content,metadata) VALUES ($1,$2,$3,'out',$4,$5) ON CONFLICT DO NOTHING").bind(group.org).bind(conversation).bind(group.logical.to_string()).bind(&response).bind(metadata.to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    sqlx::query("INSERT INTO local_outbox (id,organization_id,conversation_id,channel_id,recipient,content,status) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (id) DO NOTHING").bind(group.logical).bind(group.org).bind(conversation).bind(group.channel).bind(&group.sender).bind(response).bind(if group.channel==conversation{"simulated"}else{"queued"}).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    sqlx::query("UPDATE local_conversations SET context=$3,context_expires_at=now()+interval '24 hours',updated_at=now() WHERE organization_id=$1 AND id=$2").bind(group.org).bind(conversation).bind(serde_json::to_string(&context).map_err(|_|LocalError::Unavailable)?).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    tx.commit().await.map_err(|_| LocalError::Unavailable)
}

async fn tick_artifacts(store: &LocalStore) -> Result<(), LocalError> {
    let job=sqlx::query_as::<_,(Uuid,Uuid,String)>("SELECT j.id,j.organization_id,j.proposal FROM creative_jobs j WHERE j.state='succeeded' AND j.proposal::jsonb->>'provider'='simulated' AND j.proposal::jsonb#>>'{input,asset_id}' IS NOT NULL AND NOT EXISTS(SELECT 1 FROM local_assets a WHERE a.creative_job_id=j.id) AND NOT (j.result::jsonb ? 'artifact_error') ORDER BY j.created_at LIMIT 1").fetch_optional(&store.pool).await.map_err(|_|LocalError::Unavailable)?;
    let Some((job, org, snapshot)) = job else {
        return Ok(());
    };
    // A transaction-scoped advisory lock held during the bounded local renderer prevents duplicate work across processes.
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| LocalError::Unavailable)?;
    let locked: bool =
        sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("artifact:{job}"))
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
    if !locked {
        return Ok(());
    }
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_assets WHERE creative_job_id=$1)")
            .bind(job)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| LocalError::Unavailable)?;
    if exists {
        return Ok(());
    }
    let value: Value = serde_json::from_str(&snapshot).map_err(|_| LocalError::Invalid)?;
    let asset = value["input"]["asset_id"]
        .as_str()
        .and_then(|v| v.parse::<Uuid>().ok())
        .ok_or(LocalError::Invalid)?;
    let source = store.asset(org, asset).await?;
    let path = store.asset_path(org, asset).await?;
    let kind = value["input"]["kind"].as_str().unwrap_or("video");
    let extension = if kind == "image" { "png" } else { "mp4" };
    let key = format!("{job}.{extension}");
    let output = store.root.join("assets").join(&key);
    let temporary = store
        .root
        .join("assets")
        .join(format!("{job}.tmp.{extension}"));
    let font = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
    ]
    .into_iter()
    .find(|p| std::path::Path::new(p).exists())
    .ok_or(LocalError::Unavailable)?;
    let filter = format!(
        "scale=640:640:force_original_aspect_ratio=decrease,pad=640:640:(ow-iw)/2:(oh-ih)/2:color=white,drawbox=x=0:y=570:w=iw:h=70:color=black@0.9:t=fill,drawtext=fontfile={font}:text='SIMULACION LOCAL - NO ES IA':fontcolor=white:fontsize=18:x=(w-text_w)/2:y=595"
    );
    let mut command = tokio::process::Command::new("ffmpeg");
    command.kill_on_drop(true).args([
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-threads",
        "1",
        "-filter_threads",
        "1",
        "-protocol_whitelist",
        "file,pipe",
    ]);
    if source.media_type.starts_with("image/") {
        command.args(["-loop", "1"]);
    } else {
        command.args(["-stream_loop", "-1"]);
    }
    command.arg("-i").arg(path).args(["-an", "-vf", &filter]);
    if kind == "image" {
        command.args(["-frames:v", "1", "-update", "1"]);
    } else {
        command.args([
            "-t",
            "3",
            "-r",
            "24",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ]);
    }
    let rendered = tokio::time::timeout(
        std::time::Duration::from_secs(25),
        command.arg(&temporary).output(),
    )
    .await;
    if !matches!(rendered,Ok(Ok(ref output)) if output.status.success()) {
        sqlx::query("UPDATE creative_jobs SET result=(result::jsonb||jsonb_build_object('artifact_error','preview_failed'))::text WHERE id=$1 AND organization_id=$2").bind(job).bind(org).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        let _ = tokio::fs::remove_file(temporary).await;
        return Ok(());
    }
    tokio::fs::rename(&temporary, &output)
        .await
        .map_err(|_| LocalError::Unavailable)?;
    let bytes = tokio::fs::read(&output)
        .await
        .map_err(|_| LocalError::Unavailable)?;
    let uploader: Uuid = sqlx::query_scalar(
        "SELECT uploaded_by FROM local_assets WHERE organization_id=$1 AND id=$2",
    )
    .bind(org)
    .bind(asset)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| LocalError::Unavailable)?;
    sqlx::query("INSERT INTO local_assets (id,organization_id,uploaded_by,label,filename,media_type,size_bytes,sha256,storage_key,simulated,creative_job_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,true,$1) ON CONFLICT (creative_job_id) DO NOTHING").bind(job).bind(org).bind(uploader).bind(format!("Vista previa · {}",source.label)).bind(format!("vista-previa.{extension}")).bind(if kind=="image"{"image/png"}else{"video/mp4"}).bind(bytes.len() as i64).bind(hex::encode(Sha256::digest(&bytes))).bind(key).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    sqlx::query("UPDATE creative_jobs SET result=$3 WHERE organization_id=$1 AND id=$2").bind(org).bind(job).bind(json!({"simulated":true,"archived":true,"assets":[{"kind":kind,"url":format!("/v1/local/assets/{job}/content"),"asset_id":job}]}).to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    let conversation=sqlx::query_as::<_,(Uuid,Uuid,String)>("SELECT DISTINCT c.id,c.channel_id,c.sender FROM local_conversations c JOIN local_messages m ON m.conversation_id=c.id WHERE c.organization_id=$1 AND m.metadata::jsonb->>'creative_job_id'=$2 LIMIT 1").bind(org).bind(job.to_string()).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    if let Some((conversation, channel, sender)) = conversation {
        let message = "La vista previa simulada está lista y archivada localmente. Puedes revisarla y preparar una publicación con una nueva aprobación.";
        sqlx::query("INSERT INTO local_messages (organization_id,conversation_id,event_key,direction,content,metadata) VALUES ($1,$2,$3,'out',$4,$5) ON CONFLICT DO NOTHING").bind(org).bind(conversation).bind(format!("artifact:{job}")).bind(message).bind(json!({"asset_id":job,"creative_job_id":job,"simulated":true}).to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("INSERT INTO local_outbox (id,organization_id,conversation_id,channel_id,recipient,content,status) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (id) DO NOTHING").bind(job).bind(org).bind(conversation).bind(channel).bind(sender).bind(message).bind(if channel==conversation{"simulated"}else{"queued"}).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    }
    sqlx::query("INSERT INTO local_audit (organization_id,user_id,action,target) VALUES ($1,$2,'preview_archived',$3)").bind(org).bind(uploader).bind(job.to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    tx.commit().await.map_err(|_| LocalError::Unavailable)
}
async fn tick_publications(store: &LocalStore) -> Result<(), LocalError> {
    let mut tx = store
        .pool
        .begin()
        .await
        .map_err(|_| LocalError::Unavailable)?;
    let row=sqlx::query_as::<_,(Uuid,Uuid,Uuid,Uuid)>("SELECT id,organization_id,asset_id,approved_by FROM local_publications WHERE ((state IN ('approved','scheduled') AND (scheduled_at IS NULL OR scheduled_at<=now())) OR (state='publishing' AND lease_until<now())) ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED").fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    let Some((id, org, asset, approver)) = row else {
        return Ok(());
    };
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_users WHERE id=$1 AND organization_id=$2 AND NOT disabled AND role IN ('owner','approver'))").bind(approver).bind(org).fetch_one(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    if !valid {
        sqlx::query("UPDATE local_publications SET state='failed',receipt='approval_revoked',updated_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        return Ok(());
    }
    sqlx::query("UPDATE local_publications SET state='published_simulated',receipt=$2,updated_at=now(),lease_until=NULL WHERE id=$1").bind(id).bind(format!("simulated:{id}")).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    sqlx::query("INSERT INTO local_audit (organization_id,user_id,action,target) VALUES ($1,$2,'publication_simulated',$3)").bind(org).bind(approver).bind(id.to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    let conversation=sqlx::query_as::<_,(Uuid,Uuid,String)>("SELECT DISTINCT c.id,c.channel_id,c.sender FROM local_conversations c JOIN local_messages m ON m.conversation_id=c.id WHERE c.organization_id=$1 AND m.metadata::jsonb->>'publication_id'=$2 LIMIT 1").bind(org).bind(id.to_string()).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    if let Some((conversation, channel, sender)) = conversation {
        let content = "La publicación simulada terminó. No se envió contenido a ninguna red social. Puedes consultar el comprobante local en Publicaciones.";
        sqlx::query("INSERT INTO local_messages (organization_id,conversation_id,event_key,direction,content,metadata) VALUES ($1,$2,$3,'out',$4,$5) ON CONFLICT DO NOTHING").bind(org).bind(conversation).bind(format!("publication:{id}")).bind(content).bind(json!({"publication_id":id,"asset_id":asset,"simulated":true}).to_string()).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        sqlx::query("INSERT INTO local_outbox (id,organization_id,conversation_id,channel_id,recipient,content,status) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (id) DO NOTHING").bind(id).bind(org).bind(conversation).bind(channel).bind(sender).bind(content).bind(if channel==conversation{"simulated"}else{"queued"}).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
    }
    tx.commit().await.map_err(|_| LocalError::Unavailable)
}
