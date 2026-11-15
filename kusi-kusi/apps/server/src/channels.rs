use crate::AppState;
use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hmac::{Hmac, Mac};
use kusi_application::ChannelInbox;
use kusi_domain::ChannelEvent;
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Telegram,
    Whatsapp,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub organization_id: Uuid,
    pub provider: Provider,
    pub external_id: String,
    pub secret_env: String,
    pub verify_token_env: Option<String>,
}

pub fn load() -> Result<HashMap<Uuid, Channel>, Box<dyn std::error::Error>> {
    let channels: HashMap<Uuid, Channel> = match std::env::var("KUSI_CHANNELS") {
        Ok(value) => serde_json::from_str(&value)?,
        Err(std::env::VarError::NotPresent) => HashMap::new(),
        Err(error) => return Err(error.into()),
    };
    for channel in channels.values() {
        let secret = std::env::var(&channel.secret_env)?;
        if secret.len() < 16 || channel.external_id.is_empty() {
            return Err("invalid channel configuration".into());
        }
        if matches!(channel.provider, Provider::Telegram)
            && (secret.len() > 256
                || !secret
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'))
        {
            return Err("invalid Telegram webhook secret".into());
        }
        if matches!(channel.provider, Provider::Whatsapp) {
            let name = channel
                .verify_token_env
                .as_ref()
                .ok_or("missing Meta verification token reference")?;
            if std::env::var(name)?.len() < 16 {
                return Err("invalid Meta verification token".into());
            }
        }
    }
    Ok(channels)
}

pub async fn verify(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<String, StatusCode> {
    let channel = state.channels.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    if !matches!(channel.provider, Provider::Whatsapp) {
        return Err(StatusCode::NOT_FOUND);
    }
    let token = channel
        .verify_token_env
        .as_ref()
        .and_then(|name| std::env::var(name).ok())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let presented = query.get("hub.verify_token").ok_or(StatusCode::FORBIDDEN)?;
    if query.get("hub.mode").map(String::as_str) != Some("subscribe")
        || !bool::from(token.as_bytes().ct_eq(presented.as_bytes()))
    {
        return Err(StatusCode::FORBIDDEN);
    }
    query
        .get("hub.challenge")
        .filter(|value| value.len() <= 512)
        .cloned()
        .ok_or(StatusCode::BAD_REQUEST)
}

pub async fn receive(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    let channel = state.channels.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let secret = std::env::var(&channel.secret_env).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    match channel.provider {
        Provider::Telegram => {
            let presented = headers
                .get("x-telegram-bot-api-secret-token")
                .and_then(|v| v.to_str().ok())
                .ok_or(StatusCode::UNAUTHORIZED)?;
            if !bool::from(secret.as_bytes().ct_eq(presented.as_bytes())) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        Provider::Whatsapp => {
            let signature = headers
                .get("x-hub-signature-256")
                .and_then(|v| v.to_str().ok())
                .ok_or(StatusCode::UNAUTHORIZED)?;
            if !valid_meta_signature(secret.as_bytes(), &body, signature) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }
    let payload: Value = serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let events = normalize(channel, &payload)?;
    state
        .repository
        .accept(channel.organization_id, id, &events)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(StatusCode::OK)
}

fn valid_meta_signature(secret: &[u8], body: &[u8], signature: &str) -> bool {
    let Some(signature) = signature
        .strip_prefix("sha256=")
        .and_then(|value| hex::decode(value).ok())
    else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&signature).is_ok()
}

fn normalize(channel: &Channel, payload: &Value) -> Result<Vec<ChannelEvent>, StatusCode> {
    let mut events = Vec::new();
    match channel.provider {
        Provider::Telegram => {
            let update = payload
                .get("update_id")
                .and_then(Value::as_i64)
                .ok_or(StatusCode::BAD_REQUEST)?;
            if let Some(message) = payload.get("message") {
                // Group messages do not establish the identity of a private user.
                if message.pointer("/chat/type").and_then(Value::as_str) != Some("private") {
                    return Ok(events);
                }
                let sender = message
                    .pointer("/from/id")
                    .and_then(Value::as_i64)
                    .ok_or(StatusCode::BAD_REQUEST)?
                    .to_string();
                let chat = message
                    .pointer("/chat/id")
                    .and_then(Value::as_i64)
                    .ok_or(StatusCode::BAD_REQUEST)?
                    .to_string();
                if sender != chat {
                    return Err(StatusCode::BAD_REQUEST);
                }
                if let Some(text) = message.get("text").and_then(Value::as_str) {
                    events.push(ChannelEvent {
                        event_id: update.to_string(),
                        sender,
                        kind: "text".into(),
                        content: text.into(),
                    });
                } else if let Some(file) = message.pointer("/voice/file_id").and_then(Value::as_str)
                {
                    events.push(ChannelEvent {
                        event_id: update.to_string(),
                        sender,
                        kind: "audio".into(),
                        content: file.into(),
                    });
                }
            }
        }
        Provider::Whatsapp => {
            if payload.get("object").and_then(Value::as_str) != Some("whatsapp_business_account") {
                return Err(StatusCode::BAD_REQUEST);
            }
            for entry in payload
                .get("entry")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                for change in entry
                    .get("changes")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let Some(value) = change.get("value") else {
                        continue;
                    };
                    if value
                        .pointer("/metadata/phone_number_id")
                        .and_then(Value::as_str)
                        != Some(channel.external_id.as_str())
                    {
                        return Err(StatusCode::FORBIDDEN);
                    }
                    for message in value
                        .get("messages")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let event_id = message
                            .get("id")
                            .and_then(Value::as_str)
                            .ok_or(StatusCode::BAD_REQUEST)?
                            .to_owned();
                        let sender = message
                            .get("from")
                            .and_then(Value::as_str)
                            .ok_or(StatusCode::BAD_REQUEST)?
                            .to_owned();
                        if let Some(text) = message.pointer("/text/body").and_then(Value::as_str) {
                            events.push(ChannelEvent {
                                event_id,
                                sender,
                                kind: "text".into(),
                                content: text.into(),
                            });
                        } else if let Some(media) =
                            message.pointer("/audio/id").and_then(Value::as_str)
                        {
                            events.push(ChannelEvent {
                                event_id,
                                sender,
                                kind: "audio".into(),
                                content: media.into(),
                            });
                        } else if let Some(_media_id) =
                            message.pointer("/image/id").and_then(Value::as_str)
                        {
                            events.push(ChannelEvent {
                                event_id: event_id.clone(),
                                sender: sender.clone(),
                                kind: "image".into(),
                                content: "image_attachment_received".into(),
                            });
                            if let Some(caption) =
                                message.pointer("/image/caption").and_then(Value::as_str)
                            {
                                events.push(ChannelEvent {
                                    event_id: format!("{event_id}:caption"),
                                    sender,
                                    kind: "text".into(),
                                    content: caption.into(),
                                });
                            }
                        }
                    }
                    for status in value
                        .get("statuses")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let id = status
                            .get("id")
                            .and_then(Value::as_str)
                            .ok_or(StatusCode::BAD_REQUEST)?;
                        let kind = status
                            .get("status")
                            .and_then(Value::as_str)
                            .ok_or(StatusCode::BAD_REQUEST)?;
                        let timestamp = status
                            .get("timestamp")
                            .and_then(Value::as_str)
                            .ok_or(StatusCode::BAD_REQUEST)?;
                        events.push(ChannelEvent {
                            event_id: format!("status:{id}:{kind}:{timestamp}"),
                            sender: String::new(),
                            kind: "status".into(),
                            content: kind.into(),
                        });
                    }
                }
            }
        }
    }
    if events.len() > 100
        || events
            .iter()
            .any(|e| e.content.len() > 16384 || e.event_id.len() > 512 || e.sender.len() > 128)
    {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(events)
}

// Explicit operator dispatch, not automatic conversational processing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendRequest {
    pub recipient: String,
    pub text: String,
    pub graph_version: Option<String>,
}

pub async fn send(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<SendRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::authorize_admin(&state, &headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let key = crate::bearer_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let org = kusi_application::GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let channel = state.channels.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    if channel.organization_id != org.id {
        return Err(StatusCode::FORBIDDEN);
    }
    // Credentials must be server-owned; callers cannot select arbitrary environment variables.
    let token = std::env::var(format!("KUSI_CHANNEL_TOKEN_{}", id.simple()))
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let value = match channel.provider {
        Provider::Telegram => {
            kusi_infrastructure::messaging::send_telegram(&token, &input.recipient, &input.text)
                .await
        }
        Provider::Whatsapp => {
            kusi_infrastructure::messaging::send_whatsapp(
                input
                    .graph_version
                    .as_deref()
                    .ok_or(StatusCode::BAD_REQUEST)?,
                &channel.external_id,
                &token,
                &input.recipient,
                &input.text,
            )
            .await
        }
    }
    .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn meta_signature_requires_exact_original_body() {
        let mut mac = Hmac::<Sha256>::new_from_slice(b"fixture-secret").expect("test key");
        mac.update(b"{\"x\":1}");
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(valid_meta_signature(
            b"fixture-secret",
            b"{\"x\":1}",
            &signature
        ));
        assert!(!valid_meta_signature(
            b"fixture-secret",
            b"{\"x\":2}",
            &signature
        ));
        assert!(!valid_meta_signature(
            b"wrong-key",
            b"{\"x\":1}",
            &signature
        ));
    }
    #[test]
    fn telegram_normalizes_unverified_sender() {
        let channel = Channel {
            organization_id: Uuid::new_v4(),
            provider: Provider::Telegram,
            external_id: "bot".into(),
            secret_env: "fixture".into(),
            verify_token_env: None,
        };
        let events=normalize(&channel,&serde_json::json!({"update_id":12,"message":{"chat":{"id":42,"type":"private"},"from":{"id":42},"text":"fixture"}})).expect("valid fixture");
        assert_eq!(events[0].event_id, "12");
        assert_eq!(events[0].sender, "42");
    }
}
