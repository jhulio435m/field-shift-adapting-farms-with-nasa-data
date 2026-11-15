use kusi_application::audio::{AudioInput, Transcriber};
use kusi_domain::student_payment::{PaymentState, ProcessState, VoucherUseState};
use kusi_infrastructure::audio::{AudioProfile, AudioProvider};
// Explicit, isolated WhatsApp staging process. Never part of the local workspace router.
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use hmac::{Hmac, Mac};
use kusi_application::student_payments::{list_payments, validate_payment};
use kusi_infrastructure::{
    messaging::{download_whatsapp_image, send_whatsapp},
    student_payments::{LocalStudentPaymentsPlatform, extract_receipt_reference},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use subtle::ConstantTimeEq;

struct Config {
    secret: String,
    verify: String,
    token: String,
    phone: String,
    recipient: String,
    version: String,
    directory: PathBuf,
    serial: tokio::sync::Mutex<()>,
}
fn required(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let value = std::env::var(name)?;
    if value.trim().is_empty() {
        return Err(format!("missing {name}").into());
    }
    Ok(value)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if required("KUSI_WHATSAPP_STAGING_ACK")? != "real-replies-to-test-number" {
        return Err("explicit staging acknowledgement required".into());
    }
    if std::env::var("KUSI_LOCAL_MODE").is_ok_and(|v| v == "true") {
        return Err("cannot run in local workspace mode".into());
    }
    let directory = PathBuf::from(required("KUSI_WHATSAPP_STAGING_DATA")?);
    tokio::fs::create_dir_all(&directory).await?;
    let config = Arc::new(Config {
        secret: required("META_APP_SECRET")?,
        verify: required("META_VERIFY_TOKEN")?,
        token: required("META_ACCESS_TOKEN")?,
        phone: required("META_PHONE_NUMBER_ID")?,
        recipient: required("META_TEST_RECIPIENT")?,
        version: required("META_GRAPH_VERSION")?,
        directory,
        serial: tokio::sync::Mutex::new(()),
    });
    if config.secret.len() < 16
        || config.verify.len() < 16
        || !config
            .recipient
            .split(',')
            .all(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
        || !config.phone.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("invalid staging configuration".into());
    }
    let app = Router::new()
        .route("/webhook", get(verify).post(receive))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18195").await?;
    println!(
        "WhatsApp staging webhook listening on 127.0.0.1:18195; real replies enabled for configured test recipient only"
    );
    axum::serve(listener, app).await?;
    Ok(())
}
async fn verify(
    State(c): State<Arc<Config>>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<String, StatusCode> {
    if q.get("hub.mode").map(String::as_str) != Some("subscribe")
        || !q
            .get("hub.verify_token")
            .is_some_and(|v| bool::from(v.as_bytes().ct_eq(c.verify.as_bytes())))
    {
        return Err(StatusCode::FORBIDDEN);
    }
    q.get("hub.challenge")
        .filter(|v| v.len() <= 512)
        .cloned()
        .ok_or(StatusCode::BAD_REQUEST)
}
fn valid_signature(secret: &str, header: &str, bytes: &[u8]) -> bool {
    let Some(signature) = header
        .strip_prefix("sha256=")
        .and_then(|v| hex::decode(v).ok())
    else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(bytes);
    mac.verify_slice(&signature).is_ok()
}
async fn receive(State(c): State<Arc<Config>>, headers: HeaderMap, body: Bytes) -> StatusCode {
    if !headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| valid_signature(&c.secret, s, &body))
    {
        return StatusCode::UNAUTHORIZED;
    }
    println!("staging: signed webhook received");
    let Ok(payload) = serde_json::from_slice::<Value>(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    if payload["object"] != "whatsapp_business_account" {
        return StatusCode::BAD_REQUEST;
    }
    let Some(entries) = payload["entry"].as_array() else {
        return StatusCode::BAD_REQUEST;
    };
    let _guard = c.serial.lock().await;
    for entry in entries {
        let Some(changes) = entry["changes"].as_array() else {
            continue;
        };
        for change in changes {
            let value = &change["value"];
            if value["metadata"]["phone_number_id"].as_str() != Some(&c.phone) {
                println!("staging: ignored different phone context");
                continue;
            }
            if let Some(statuses) = value["statuses"].as_array() {
                for status in statuses {
                    if let Some(state @ ("sent" | "delivered" | "read" | "failed")) =
                        status["status"].as_str()
                    {
                        println!(
                            "staging: provider delivery status={} error_code={}",
                            state,
                            status["errors"][0]["code"].as_u64().unwrap_or_default()
                        );
                    }
                }
            }
            let Some(messages) = value["messages"].as_array() else {
                continue;
            };
            for message in messages {
                if !message["from"]
                    .as_str()
                    .is_some_and(|sender| c.recipient.split(',').any(|allowed| allowed == sender))
                {
                    println!("staging: ignored unauthorized sender");
                    continue;
                }
                let Some(id) = message["id"]
                    .as_str()
                    .filter(|v| !v.is_empty() && v.len() < 512)
                else {
                    continue;
                };
                // Durable claim BEFORE any external action. Ambiguous/crashed sends never auto-repeat.
                let digest = hex::encode(Sha256::digest(id.as_bytes()));
                let path = c.directory.join(digest);
                let claim = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .await;
                match claim {
                    Ok(file) => {
                        if file.sync_all().await.is_err() {
                            return StatusCode::SERVICE_UNAVAILABLE;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(_) => return StatusCode::SERVICE_UNAVAILABLE,
                }
                let text = reply(&c, message).await;
                let status = if send_whatsapp(
                    &c.version,
                    &c.phone,
                    &c.token,
                    message["from"].as_str().unwrap_or_default(),
                    &text,
                )
                .await
                .is_ok()
                {
                    b"sent".as_slice()
                } else {
                    b"uncertain_or_failed".as_slice()
                };
                if tokio::fs::write(&path, status).await.is_err() {
                    return StatusCode::SERVICE_UNAVAILABLE;
                }
            }
        }
    }
    StatusCode::OK
}
async fn reply(c: &Config, message: &Value) -> String {
    let platform = LocalStudentPaymentsPlatform;
    let actor = LocalStudentPaymentsPlatform::demo_user();
    let code = match message["type"].as_str() {
        Some("image") => {
            let Some(id) = message["image"]["id"].as_str() else {
                return "No pude leer la foto. Envía el código del recibo.".into();
            };
            let Ok(bytes) = download_whatsapp_image(&c.version, id, &c.token).await else {
                return "No pude descargar la foto. Envía el código del recibo.".into();
            };
            match tokio::task::spawn_blocking(move || extract_receipt_reference(&bytes)).await {Ok(Ok(code))=>code,_=>return "No pude reconocer el código. Envía una foto más clara o escribe el código.".into()}
        }
        Some("audio") => {
            println!("staging: audio received");
            if std::env::var("GROQ_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .is_none()
            {
                return "*Notas de voz*\nLa transcripción todavía no está configurada. Por ahora escribe *mis pagos* o envía el código del recibo.".into();
            }
            let Some(id) = message["audio"]["id"].as_str() else {
                return "No pude leer la nota de voz. Intenta escribir tu consulta.".into();
            };
            let mime = message["audio"]["mime_type"].as_str().unwrap_or_default();
            let extension = match mime.split(';').next().unwrap_or_default().trim() {
                "audio/ogg" => "ogg", "audio/mpeg" => "mp3", "audio/mp4" | "audio/aac" => "m4a", "audio/wav" | "audio/x-wav" => "wav", _ => return "Ese formato de audio no está disponible. Envía una nota de voz de WhatsApp.".into(),
            };
            let Ok(bytes) =
                kusi_infrastructure::messaging::download_whatsapp_media(&c.version, id, &c.token)
                    .await
            else {
                return "No pude descargar el audio. Intenta nuevamente o escribe tu consulta."
                    .into();
            };
            let profile = AudioProfile {
                provider: AudioProvider::Groq,
                model: "whisper-large-v3-turbo".into(),
                api_key_env: Some("GROQ_API_KEY".into()),
                endpoint: None,
                timeout_ms: 20000,
            };
            match profile
                .transcribe(AudioInput {
                    bytes,
                    filename: format!("voice.{extension}"),
                    language: Some("es".into()),
                })
                .await
            {
                Ok(transcript)
                    if !transcript.text.trim().is_empty() && transcript.text.len() <= 2000 =>
                {
                    println!("staging: transcription succeeded");
                    transcript.text.trim().to_string()
                }
                _ => {
                    println!("staging: transcription failed");
                    return "No pude transcribir la nota de voz. Intenta nuevamente o escribe *mis pagos*.".into();
                }
            }
        }
        Some("text") => message["text"]["body"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string(),
        _ => {
            return "QORI: envía «mis pagos», un código o una foto del recibo.".into();
        }
    };
    if code == "70000002" {
        return "Este estudiante no tiene pagos registrados.".into();
    }
    if code.len() == 8
        && code.bytes().all(|b| b.is_ascii_digit())
        && code != LocalStudentPaymentsPlatform::DEMO_DNI
    {
        return "DNI no encontrado. Verifica los ocho dígitos e intenta nuevamente.".into();
    }
    if code.to_lowercase().contains("mis pagos") || code == LocalStudentPaymentsPlatform::DEMO_DNI {
        return match list_payments(&platform, &actor).await {
            Ok(payments) => format!(
                "*QORI · Tus pagos*\n\n{}\n\nEnvía un *código* o una *foto del recibo* para consultar su estado.",
                payments
                    .iter()
                    .map(|p| format!(
                        "• *{}*\n  {}\n  *S/ {:.2}*",
                        p.payment_code,
                        p.concept,
                        p.amount_cents as f64 / 100.0
                    ))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ),
            Err(_) => "Consulta no disponible. Intenta nuevamente.".into(),
        };
    }
    let normalized = match kusi_application::student_payments::extract_payment_code(&code) {
        Ok(code) => code,
        Err(_) => return "No pude distinguir un código completo. Dilo dígito por dígito, incluyendo los ceros, o envíalo escrito. También puedes enviar una foto del recibo.".into(),
    };
    let interpreted = if message["type"] == "audio" {
        format!("*Código interpretado:* {normalized}\n\n")
    } else {
        String::new()
    };
    let result: String = match validate_payment(&platform, &actor, &normalized).await {
        Ok(p) => format!("*Pago encontrado*\n\n*Concepto:* {}\n*Código:* {}\n*Monto:* S/ {:.2}\n*Pago:* {}\n*Trámite:* {}\n*Uso del comprobante:* {}\n\nEl pago y el avance del trámite se consultan por separado.", p.concept, p.payment_code, p.amount_cents as f64 / 100.0, payment_label(p.payment_state), process_label(p.process_state), voucher_label(p.voucher_use)),
        Err(_) => "*Consulta sin coincidencia*\n\nNo encontré un pago asociado a ese código.\n\nPrueba con:\n• *mis pagos*\n• *UNCP-EXP-004821*\n• Una foto del recibo\n".into(),
    }
    ;
    format!("{interpreted}{result}")
}
fn voucher_label(state: VoucherUseState) -> &'static str {
    match state {
        VoucherUseState::Available => "Disponible para asociar a un trámite",
        VoucherUseState::Used => {
            "Ya utilizado en este trámite. No válido para iniciar otro trámite con el mismo comprobante."
        }
        VoucherUseState::ReviewRequired => {
            "Asociado a un trámite interrumpido. Reutilización pendiente de revisión administrativa."
        }
    }
}
fn payment_label(state: PaymentState) -> &'static str {
    match state {
        PaymentState::Confirmed => "Confirmado",
        PaymentState::Pending => "Pendiente",
        PaymentState::NotFound => "No encontrado",
        PaymentState::Reversed => "Revertido",
    }
}
fn process_label(state: ProcessState) -> &'static str {
    match state {
        ProcessState::NotStarted => "Sin iniciar",
        ProcessState::InProgress => "En proceso",
        ProcessState::Interrupted => "Interrumpido",
        ProcessState::Completed => "Completado",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn text_lookup_uses_synthetic_platform_and_unknown_code_is_not_confirmed() {
        let c = Config {
            secret: String::new(),
            verify: String::new(),
            token: String::new(),
            phone: String::new(),
            recipient: String::new(),
            version: String::new(),
            directory: PathBuf::new(),
            serial: tokio::sync::Mutex::new(()),
        };
        let found = reply(
            &c,
            &serde_json::json!({"type":"text","text":{"body":"UNCP-EXP-004821"}}),
        )
        .await;
        assert!(found.contains("Pago encontrado") && found.contains("198.00"));
        let empty = reply(
            &c,
            &serde_json::json!({"type":"text","text":{"body":"70000002"}}),
        )
        .await;
        assert!(empty.contains("no tiene pagos registrados"));
        let unknown = reply(
            &c,
            &serde_json::json!({"type":"text","text":{"body":"70000003"}}),
        )
        .await;
        assert!(unknown.contains("DNI no encontrado"));
        let missing = reply(
            &c,
            &serde_json::json!({"type":"text","text":{"body":"UNKNOWN"}}),
        )
        .await;
        assert!(missing.contains("sin coincidencia"));
        let listed = reply(
            &c,
            &serde_json::json!({"type":"text","text":{"body":"mis pagos"}}),
        )
        .await;
        assert!(listed.contains("Tus pagos") && listed.contains("UNCP-EXP-004821"));
    }
    #[test]
    fn statuses_are_presented_in_spanish() {
        assert_eq!(payment_label(PaymentState::Confirmed), "Confirmado");
        assert_eq!(process_label(ProcessState::Interrupted), "Interrumpido");
        assert_eq!(process_label(ProcessState::NotStarted), "Sin iniciar");
        assert!(voucher_label(VoucherUseState::Used).contains("No válido"));
        assert!(voucher_label(VoucherUseState::ReviewRequired).contains("revisión administrativa"));
        assert!(voucher_label(VoucherUseState::Available).contains("Disponible"));
    }
    #[test]
    fn signature_is_bound_to_secret_and_exact_body() {
        let body = b"demo";
        let mut mac = Hmac::<Sha256>::new_from_slice(b"test-secret").unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(valid_signature("test-secret", &signature, body));
        assert!(!valid_signature("other-secret", &signature, body));
        assert!(!valid_signature("test-secret", &signature, b"changed"));
        assert!(!valid_signature("test-secret", "bad", body));
    }
}
