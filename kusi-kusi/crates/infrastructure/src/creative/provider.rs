use kusi_application::creative::{
    CreativeError, CreativeProgress, CreativeProvider, CreativeRecipe, RemoteCreativeRequest,
    SubmissionError,
};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;

pub struct OfficialCreativeProvider {
    pub allow_simulation: bool,
}

pub fn validate_recipe(
    recipe: &CreativeRecipe,
    allow_simulation: bool,
) -> Result<(), CreativeError> {
    if !["higgsfield", "fal", "simulated"].contains(&recipe.provider.as_str())
        || recipe.model.is_empty()
        || recipe.model.len() > 180
        || recipe
            .model
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || !recipe
            .model
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_./".contains(&c))
        || !(0..=100_000_000).contains(&recipe.estimated_cost_microusd)
        || !recipe.defaults.is_object()
        || !recipe.input_schema.is_object()
        || !crate::schema_allowed(&recipe.input_schema)
    {
        return Err(CreativeError::Invalid);
    }
    if recipe.provider == "simulated" {
        if !allow_simulation || recipe.estimated_cost_microusd != 0 {
            return Err(CreativeError::Invalid);
        }
    } else if recipe.enabled {
        if recipe.estimated_cost_microusd == 0 {
            return Err(CreativeError::Invalid);
        }
        let key = credentials(recipe)?;
        if recipe.provider == "higgsfield" && key.split_once(':').is_none() {
            return Err(CreativeError::Invalid);
        }
    }
    Ok(())
}
fn credentials(recipe: &CreativeRecipe) -> Result<String, CreativeError> {
    recipe
        .key_env
        .as_ref()
        .and_then(|name| std::env::var(name).ok())
        .filter(|v| !v.is_empty() && !v.contains(['\n', '\r']))
        .ok_or(CreativeError::Unavailable)
}
fn origin(provider: &str) -> Result<&'static str, CreativeError> {
    match provider {
        "higgsfield" => Ok("https://api.higgsfield.ai"),
        "fal" => Ok("https://queue.fal.run"),
        _ => Err(CreativeError::Invalid),
    }
}
fn allowed_url(
    provider: &str,
    url: &str,
    request_id: &str,
    status: bool,
) -> Result<(), CreativeError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| CreativeError::Invalid)?;
    let base = reqwest::Url::parse(origin(provider)?).map_err(|_| CreativeError::Invalid)?;
    let suffix = if status {
        format!("/requests/{request_id}/status")
    } else {
        format!("/requests/{request_id}")
    };
    if parsed.origin() != base.origin()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !parsed.path().ends_with(&suffix)
    {
        return Err(CreativeError::Invalid);
    }
    Ok(())
}
async fn request(
    recipe: &CreativeRecipe,
    method: reqwest::Method,
    url: &str,
    body: Option<&Value>,
    id: Option<Uuid>,
) -> Result<Value, SubmissionError> {
    let key = credentials(recipe).map_err(|_| SubmissionError::Rejected)?;
    request_with_key(&recipe.provider, method, url, body, id, &key).await
}
async fn request_with_key(
    provider: &str,
    method: reqwest::Method,
    url: &str,
    body: Option<&Value>,
    id: Option<Uuid>,
    key: &str,
) -> Result<Value, SubmissionError> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| SubmissionError::Rejected)?;
    let mut request = client
        .request(method, url)
        .header("Authorization", format!("Key {key}"));
    if let Some(body) = body {
        request = request.json(body);
    }
    if let Some(id) = id
        && provider == "higgsfield"
    {
        request = request.header("Idempotency-Key", id.to_string());
    }
    let mut response = request.send().await.map_err(|_| SubmissionError::Unknown)?;
    match response.status().as_u16() {
        200..=299 => {}
        400..=499 => return Err(SubmissionError::Rejected),
        _ => return Err(SubmissionError::Unknown),
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| SubmissionError::Unknown)?
    {
        if bytes.len() + chunk.len() > 2_097_152 {
            return Err(SubmissionError::Unknown);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| SubmissionError::Unknown)
}
fn remote_from_value(
    recipe: &CreativeRecipe,
    v: &Value,
) -> Result<RemoteCreativeRequest, CreativeError> {
    let request_id = v["request_id"]
        .as_str()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        })
        .ok_or(CreativeError::Invalid)?
        .to_owned();
    let status_url = v["status_url"]
        .as_str()
        .ok_or(CreativeError::Invalid)?
        .to_owned();
    allowed_url(&recipe.provider, &status_url, &request_id, true)?;
    let response_url = if recipe.provider == "fal" {
        let url = v["response_url"]
            .as_str()
            .ok_or(CreativeError::Invalid)?
            .to_owned();
        allowed_url(&recipe.provider, &url, &request_id, false)?;
        Some(url)
    } else {
        None
    };
    Ok(RemoteCreativeRequest {
        request_id,
        status_url,
        response_url,
    })
}
fn outputs(v: &Value) -> Result<Value, CreativeError> {
    let mut assets = Vec::new();
    for kind in ["images", "video", "audio", "audios"] {
        let values: Vec<&Value> = match v.get(kind) {
            Some(Value::Array(a)) => a.iter().collect(),
            Some(v) => vec![v],
            None => vec![],
        };
        for value in values {
            let url = value["url"].as_str().ok_or(CreativeError::Invalid)?;
            let parsed = reqwest::Url::parse(url).map_err(|_| CreativeError::Invalid)?;
            if parsed.scheme() != "https"
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || parsed.host_str().is_none()
            {
                return Err(CreativeError::Invalid);
            }
            assets.push(json!({"kind":kind,"url":url}));
        }
    }
    if assets.is_empty() || assets.len() > 20 {
        return Err(CreativeError::Invalid);
    }
    Ok(json!({"assets":assets,"expires_at":v.get("expires_at")}))
}
#[async_trait::async_trait]
impl CreativeProvider for OfficialCreativeProvider {
    async fn submit(
        &self,
        recipe: &CreativeRecipe,
        input: &Value,
        id: Uuid,
    ) -> Result<RemoteCreativeRequest, SubmissionError> {
        if std::env::var("KUSI_LOCAL_MODE").is_ok_and(|v| v == "true")
            && recipe.provider != "simulated"
        {
            return Err(SubmissionError::Rejected);
        }
        validate_recipe(recipe, self.allow_simulation).map_err(|_| SubmissionError::Rejected)?;
        if recipe.provider == "simulated" {
            return Ok(RemoteCreativeRequest {
                request_id: id.to_string(),
                status_url: format!("simulated:{id}"),
                response_url: None,
            });
        }
        let url = format!(
            "{}/{}",
            origin(&recipe.provider).map_err(|_| SubmissionError::Rejected)?,
            recipe.model
        );
        let response = request(recipe, reqwest::Method::POST, &url, Some(input), Some(id)).await?;
        remote_from_value(recipe, &response).map_err(|_| SubmissionError::Unknown)
    }
    async fn poll(
        &self,
        recipe: &CreativeRecipe,
        remote: &RemoteCreativeRequest,
    ) -> Result<CreativeProgress, CreativeError> {
        if std::env::var("KUSI_LOCAL_MODE").is_ok_and(|v| v == "true")
            && recipe.provider != "simulated"
        {
            return Err(CreativeError::Unavailable);
        }
        if recipe.provider == "simulated" {
            if !self.allow_simulation {
                return Err(CreativeError::Unavailable);
            }
            return Ok(CreativeProgress::Completed(
                json!({"simulated":true,"assets":[],"message":"Fixture job: no media generated and no money spent"}),
            ));
        }
        allowed_url(
            &recipe.provider,
            &remote.status_url,
            &remote.request_id,
            true,
        )?;
        let v = request(recipe, reqwest::Method::GET, &remote.status_url, None, None)
            .await
            .map_err(|_| CreativeError::Unavailable)?;
        if v["request_id"]
            .as_str()
            .is_some_and(|id| id != remote.request_id)
        {
            return Err(CreativeError::Invalid);
        }
        match (recipe.provider.as_str(), v["status"].as_str()) {
            ("higgsfield", Some("queued")) | ("fal", Some("IN_QUEUE")) => {
                Ok(CreativeProgress::Pending)
            }
            ("higgsfield", Some("in_progress")) | ("fal", Some("IN_PROGRESS")) => {
                Ok(CreativeProgress::Processing)
            }
            ("higgsfield", Some("failed" | "nsfw" | "canceled")) => Ok(CreativeProgress::Failed),
            ("higgsfield", Some("completed")) => Ok(CreativeProgress::Completed(outputs(&v)?)),
            ("fal", Some("COMPLETED")) => {
                if v.get("error").is_some_and(|e| !e.is_null()) {
                    return Ok(CreativeProgress::Failed);
                }
                let url = remote.response_url.as_ref().ok_or(CreativeError::Invalid)?;
                allowed_url("fal", url, &remote.request_id, false)?;
                let result = request(recipe, reqwest::Method::GET, url, None, None)
                    .await
                    .map_err(|_| CreativeError::Unavailable)?;
                if result.get("error").is_some_and(|e| !e.is_null()) {
                    return Ok(CreativeProgress::Failed);
                }
                Ok(CreativeProgress::Completed(outputs(&result)?))
            }
            _ => Err(CreativeError::Invalid),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn recipe(provider: &str) -> CreativeRecipe {
        CreativeRecipe {
            provider: provider.into(),
            model: "fixture/model".into(),
            key_env: None,
            input_schema: json!({"type":"object"}),
            defaults: json!({}),
            estimated_cost_microusd: 0,
            enabled: false,
        }
    }
    #[test]
    fn status_urls_cannot_exfiltrate_credentials() {
        assert!(
            allowed_url(
                "fal",
                "https://attacker.example/requests/abc/status",
                "abc",
                true
            )
            .is_err()
        );
        assert!(
            allowed_url(
                "fal",
                "https://queue.fal.run/owner/model/requests/abc/status",
                "abc",
                true
            )
            .is_ok()
        );
        assert!(
            allowed_url(
                "fal",
                "https://queue.fal.run/owner/model/requests/other/status",
                "abc",
                true
            )
            .is_err()
        );
        assert!(
            allowed_url(
                "higgsfield",
                "https://api.higgsfield.ai/requests/abc/status?secret=x",
                "abc",
                true
            )
            .is_err()
        );
    }
    #[test]
    fn native_submit_contracts_and_results_are_normalized() {
        let hf=remote_from_value(&recipe("higgsfield"),&json!({"request_id":"abc","status_url":"https://api.higgsfield.ai/requests/abc/status"})).expect("HF fixture");
        assert_eq!(hf.request_id, "abc");
        let fal=remote_from_value(&recipe("fal"),&json!({"request_id":"abc","status_url":"https://queue.fal.run/fal-ai/model/requests/abc/status","response_url":"https://queue.fal.run/fal-ai/model/requests/abc"})).expect("fal fixture");
        assert!(fal.response_url.is_some());
        assert_eq!(
            outputs(&json!({"video":{"url":"https://cdn.example/video.mp4"}})).expect("output")["assets"]
                [0]["kind"],
            "video"
        );
        assert!(outputs(&json!({"video":{"url":"javascript:alert(1)"}})).is_err());
    }
    #[tokio::test]
    async fn submit_transport_uses_official_auth_and_preserves_uncertain_failures() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for (provider, status) in [
            ("higgsfield", 200),
            ("fal", 200),
            ("fal", 503),
            ("higgsfield", 422),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("fixture listener");
            let address = listener.local_addr().expect("fixture address");
            let request_id = Uuid::new_v4();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.expect("fixture connection");
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let n = stream.read(&mut buffer).await.expect("fixture read");
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    assert!(bytes.len() < 65536);
                    if let Some(split) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..split]).to_ascii_lowercase();
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .expect("fixture content length");
                        if bytes.len() >= split + 4 + length {
                            assert!(headers.contains("authorization: key fixture:id"));
                            assert_eq!(
                                headers.contains("idempotency-key:"),
                                provider == "higgsfield"
                            );
                            let body: Value =
                                serde_json::from_slice(&bytes[split + 4..split + 4 + length])
                                    .expect("fixture body");
                            assert_eq!(body, json!({"prompt":"fixture"}));
                            break;
                        }
                    }
                }
                let body=json!({"request_id":"abc","status_url":"https://api.higgsfield.ai/requests/abc/status"}).to_string();
                stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.expect("fixture write");
            });
            let result = request_with_key(
                provider,
                reqwest::Method::POST,
                &format!("http://{address}/model"),
                Some(&json!({"prompt":"fixture"})),
                Some(request_id),
                "fixture:id",
            )
            .await;
            match status {
                200 => assert!(result.is_ok()),
                503 => assert!(matches!(result, Err(SubmissionError::Unknown))),
                422 => assert!(matches!(result, Err(SubmissionError::Rejected))),
                _ => unreachable!(),
            }
            server.await.expect("fixture server");
        }
    }
}
