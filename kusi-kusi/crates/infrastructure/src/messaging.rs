//! Official provider text senders. Never expose provider error URLs (Telegram tokens are in paths).
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
#[error("messaging provider rejected or could not deliver the request")]
pub struct SendError;

pub async fn send_telegram(token: &str, chat_id: &str, text: &str) -> Result<Value, SendError> {
    if token.is_empty()
        || !token
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b":_-".contains(&c))
        || text.is_empty()
        || text.chars().count() > 4096
    {
        return Err(SendError);
    }
    send(
        &format!("https://api.telegram.org/bot{token}/sendMessage"),
        None,
        json!({"chat_id":chat_id,"text":text}),
    )
    .await
}

pub async fn send_whatsapp(
    version: &str,
    phone_id: &str,
    token: &str,
    recipient: &str,
    text: &str,
) -> Result<Value, SendError> {
    if !version.starts_with('v')
        || !version[1..]
            .bytes()
            .all(|c| c.is_ascii_digit() || c == b'.')
        || version.len() < 4
        || phone_id.is_empty()
        || !phone_id.bytes().all(|c| c.is_ascii_digit())
        || text.is_empty()
        || text.chars().count() > 4096
    {
        return Err(SendError);
    }
    send(
        &format!("https://graph.facebook.com/{version}/{phone_id}/messages"),
        Some(token),
        json!({"messaging_product":"whatsapp","to":recipient,"type":"text","text":{"body":text}}),
    )
    .await
}

async fn send(url: &str, token: Option<&str>, payload: Value) -> Result<Value, SendError> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| SendError)?;
    let mut request = client.post(url).json(&payload);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let mut response = request.send().await.map_err(|_| SendError)?;
    if !response.status().is_success() {
        return Err(SendError);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| SendError)? {
        if bytes.len() + chunk.len() > 65536 {
            return Err(SendError);
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| SendError)?;
    if value.get("ok") == Some(&Value::Bool(false)) || value.get("error").is_some() {
        return Err(SendError);
    }
    Ok(value)
}

/// Download bounded Meta media without forwarding credentials to arbitrary hosts or redirects.
pub async fn download_whatsapp_media(
    version: &str,
    media_id: &str,
    token: &str,
) -> Result<Vec<u8>, SendError> {
    if !media_id.bytes().all(|b| b.is_ascii_digit())
        || media_id.is_empty()
        || !version.starts_with('v')
        || !version[1..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.')
    {
        return Err(SendError);
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| SendError)?;
    let metadata = client
        .get(format!("https://graph.facebook.com/{version}/{media_id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| SendError)?;
    let mut metadata = metadata.error_for_status().map_err(|_| SendError)?;
    let mut raw = Vec::new();
    while let Some(chunk) = metadata.chunk().await.map_err(|_| SendError)? {
        if raw.len() + chunk.len() > 65536 {
            return Err(SendError);
        }
        raw.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&raw).map_err(|_| SendError)?;
    let url =
        reqwest::Url::parse(value["url"].as_str().ok_or(SendError)?).map_err(|_| SendError)?;
    let host = url.host_str().ok_or(SendError)?;
    if url.scheme() != "https"
        || !(host == "lookaside.fbsbx.com" || host.ends_with(".fbcdn.net"))
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(SendError);
    }
    let mut response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| SendError)?
        .error_for_status()
        .map_err(|_| SendError)?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| SendError)? {
        if bytes.len() + chunk.len() > 5 * 1024 * 1024 {
            return Err(SendError);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Image compatibility entrypoint; shares bounded authenticated media transport.
pub async fn download_whatsapp_image(
    version: &str,
    media_id: &str,
    token: &str,
) -> Result<Vec<u8>, SendError> {
    download_whatsapp_media(version, media_id, token).await
}
