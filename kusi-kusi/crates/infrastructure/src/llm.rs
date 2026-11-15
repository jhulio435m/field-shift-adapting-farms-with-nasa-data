use kusi_application::llm::{
    InferenceRequest, InferenceResult, LlmError, LlmProvider, ToolProposal,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Openai,
    Gemini,
    Claude,
    Openrouter,
    Nvidia,
    Groq,
    Ollama,
    Vllm,
    Lmstudio,
    OpenaiCompatible,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub provider: Provider,
    pub model: String,
    pub api_key_env: Option<String>,
    pub endpoint: Option<String>,
    pub max_output_tokens: u32,
    pub timeout_ms: u64,
    #[serde(default)]
    pub tools_supported: bool,
    pub anthropic_version: Option<String>,
    pub routing_providers: Option<Vec<String>>,
}

pub type Profiles = HashMap<Uuid, HashMap<String, Profile>>;

impl Profile {
    pub fn validate(&self) -> Result<(), LlmError> {
        if self.model.is_empty()
            || self.model.len() > 200
            || !(1..=32768).contains(&self.max_output_tokens)
            || !(100..=120000).contains(&self.timeout_ms)
        {
            return Err(LlmError::Configuration);
        }
        let url = reqwest::Url::parse(&self.url()?).map_err(|_| LlmError::Configuration)?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(LlmError::Configuration);
        }
        match self.provider {
            Provider::Ollama | Provider::Vllm | Provider::Lmstudio | Provider::OpenaiCompatible => {
                if self.endpoint.is_none()
                    || !(url.scheme() == "https"
                        || (url.scheme() == "http"
                            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
                {
                    return Err(LlmError::Configuration);
                }
            }
            _ => {
                if self.endpoint.is_some() {
                    return Err(LlmError::Configuration);
                }
                self.key()?.ok_or(LlmError::Configuration)?;
            }
        }
        if matches!(self.provider, Provider::Gemini)
            && !self
                .model
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        {
            return Err(LlmError::Configuration);
        }
        if matches!(self.provider, Provider::Claude)
            && self.anthropic_version.as_ref().is_none_or(|v| {
                v.len() != 10 || !v.bytes().all(|c| c.is_ascii_digit() || c == b'-')
            })
        {
            return Err(LlmError::Configuration);
        }
        if matches!(self.provider, Provider::Openrouter)
            && self
                .routing_providers
                .as_ref()
                .is_none_or(|v| v.is_empty() || v.iter().any(|s| s.is_empty()))
        {
            return Err(LlmError::Configuration);
        }
        Ok(())
    }
    fn key(&self) -> Result<Option<String>, LlmError> {
        self.api_key_env
            .as_ref()
            .map(|name| {
                std::env::var(name)
                    .ok()
                    .filter(|s| !s.is_empty())
                    .ok_or(LlmError::Configuration)
            })
            .transpose()
    }
    fn url(&self) -> Result<String, LlmError> {
        Ok(match self.provider {
            Provider::Openai => "https://api.openai.com/v1/responses".into(),
            Provider::Claude => "https://api.anthropic.com/v1/messages".into(),
            Provider::Gemini => format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                self.model
            ),
            Provider::Openrouter => "https://openrouter.ai/api/v1/chat/completions".into(),
            Provider::Groq => "https://api.groq.com/openai/v1/chat/completions".into(),
            Provider::Nvidia => "https://integrate.api.nvidia.com/v1/chat/completions".into(),
            _ => self.endpoint.clone().ok_or(LlmError::Configuration)?,
        })
    }
    fn payload(&self, r: &InferenceRequest) -> Value {
        let tools: Vec<Value> = r
            .tools
            .iter()
            .map(|t| json!({"name":t.name,"description":t.description,"parameters":t.input_schema}))
            .collect();
        match self.provider {
            Provider::Openai => {
                let mut body = json!({"model":self.model,"instructions":r.instructions,"input":r.input,"max_output_tokens":self.max_output_tokens,"store":false});
                if !tools.is_empty() {
                    body["tools"] = json!(
                        tools
                            .into_iter()
                            .map(|mut t| {
                                t["type"] = json!("function");
                                t["strict"] = json!(false);
                                t
                            })
                            .collect::<Vec<_>>()
                    );
                }
                body
            }
            Provider::Claude => {
                let mut body = json!({"model":self.model,"system":r.instructions,"messages":[{"role":"user","content":r.input}],"max_tokens":self.max_output_tokens});
                if !tools.is_empty() {
                    body["tools"]=json!(r.tools.iter().map(|t|json!({"name":t.name,"description":t.description,"input_schema":t.input_schema})).collect::<Vec<_>>());
                }
                body
            }
            Provider::Gemini => {
                let mut body = json!({"systemInstruction":{"parts":[{"text":r.instructions}]},"contents":[{"role":"user","parts":[{"text":r.input}]}],"generationConfig":{"maxOutputTokens":self.max_output_tokens}});
                if !tools.is_empty() {
                    body["tools"] = json!([{"functionDeclarations":tools}]);
                }
                body
            }
            Provider::Ollama => {
                let mut body = json!({"model":self.model,"stream":false,"messages":[{"role":"system","content":r.instructions},{"role":"user","content":r.input}],"options":{"num_predict":self.max_output_tokens}});
                if !tools.is_empty() {
                    body["tools"] = json!(
                        tools
                            .into_iter()
                            .map(|t| json!({"type":"function","function":t}))
                            .collect::<Vec<_>>()
                    );
                }
                body
            }
            _ => {
                let mut body = json!({"model":self.model,"stream":false,"messages":[{"role":"system","content":r.instructions},{"role":"user","content":r.input}],"max_tokens":self.max_output_tokens});
                if !tools.is_empty() {
                    body["tools"] = json!(
                        tools
                            .into_iter()
                            .map(|t| json!({"type":"function","function":t}))
                            .collect::<Vec<_>>()
                    );
                }
                if matches!(self.provider, Provider::Openrouter) {
                    body["provider"] = json!({"allow_fallbacks":false,"require_parameters":true,"only":self.routing_providers,"data_collection":"deny"});
                }
                body
            }
        }
    }
    fn parse(&self, v: &Value) -> Result<InferenceResult, LlmError> {
        if v.get("error").is_some() {
            return Err(LlmError::InvalidResponse);
        }
        let mut text = String::new();
        let mut proposals = Vec::new();
        let finish;
        match self.provider {
            Provider::Openai => {
                finish = v["status"]
                    .as_str()
                    .ok_or(LlmError::InvalidResponse)?
                    .to_owned();
                if finish != "completed" {
                    return Err(LlmError::InvalidResponse);
                }
                for item in v["output"].as_array().ok_or(LlmError::InvalidResponse)? {
                    if item["type"] == "function_call" {
                        proposals.push(proposal(
                            &item["call_id"],
                            &item["name"],
                            &item["arguments"],
                            proposals.len(),
                        )?);
                    }
                    for part in item["content"].as_array().into_iter().flatten() {
                        if part["type"] == "refusal" {
                            return Err(LlmError::InvalidResponse);
                        }
                        if let Some(t) = part["text"].as_str() {
                            text.push_str(t);
                        }
                    }
                }
            }
            Provider::Claude => {
                finish = v["stop_reason"]
                    .as_str()
                    .ok_or(LlmError::InvalidResponse)?
                    .to_owned();
                if !matches!(finish.as_str(), "end_turn" | "tool_use" | "stop_sequence") {
                    return Err(LlmError::InvalidResponse);
                }
                for part in v["content"].as_array().ok_or(LlmError::InvalidResponse)? {
                    if part["type"] == "tool_use" {
                        proposals.push(proposal(
                            &part["id"],
                            &part["name"],
                            &part["input"],
                            proposals.len(),
                        )?);
                    } else if let Some(t) = part["text"].as_str() {
                        text.push_str(t);
                    }
                }
            }
            Provider::Gemini => {
                let candidate = &v["candidates"][0];
                finish = candidate["finishReason"]
                    .as_str()
                    .ok_or(LlmError::InvalidResponse)?
                    .to_owned();
                if finish != "STOP" {
                    return Err(LlmError::InvalidResponse);
                }
                for part in candidate["content"]["parts"]
                    .as_array()
                    .ok_or(LlmError::InvalidResponse)?
                {
                    if let Some(call) = part.get("functionCall") {
                        proposals.push(proposal(
                            &call["id"],
                            &call["name"],
                            &call["args"],
                            proposals.len(),
                        )?);
                    } else if let Some(t) = part["text"].as_str()
                        && part["thought"] != true
                    {
                        text.push_str(t);
                    }
                }
            }
            _ => {
                let (message, reason) = if matches!(self.provider, Provider::Ollama) {
                    if v["done"] != true {
                        return Err(LlmError::InvalidResponse);
                    }
                    (&v["message"], v["done_reason"].as_str().unwrap_or("stop"))
                } else {
                    (
                        &v["choices"][0]["message"],
                        v["choices"][0]["finish_reason"]
                            .as_str()
                            .ok_or(LlmError::InvalidResponse)?,
                    )
                };
                finish = reason.to_owned();
                if !matches!(reason, "stop" | "tool_calls") {
                    return Err(LlmError::InvalidResponse);
                }
                if message.get("refusal").is_some_and(|v| !v.is_null()) {
                    return Err(LlmError::InvalidResponse);
                }
                if let Some(t) = message["content"].as_str() {
                    text.push_str(t);
                }
                for call in message["tool_calls"].as_array().into_iter().flatten() {
                    proposals.push(proposal(
                        &call["id"],
                        &call["function"]["name"],
                        &call["function"]["arguments"],
                        proposals.len(),
                    )?);
                }
            }
        }
        if text.is_empty() && proposals.is_empty() {
            return Err(LlmError::InvalidResponse);
        }
        Ok(InferenceResult {
            text,
            tools: proposals,
            model: self.model.clone(),
            finish_reason: finish,
            usage: v
                .get("usage")
                .or_else(|| v.get("usageMetadata"))
                .cloned()
                .unwrap_or(Value::Null),
        })
    }
}
fn proposal(
    id: &Value,
    name: &Value,
    args: &Value,
    index: usize,
) -> Result<ToolProposal, LlmError> {
    let arguments = if let Some(raw) = args.as_str() {
        serde_json::from_str(raw).map_err(|_| LlmError::InvalidResponse)?
    } else {
        args.clone()
    };
    if !arguments.is_object() {
        return Err(LlmError::InvalidResponse);
    }
    Ok(ToolProposal {
        id: id
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("proposal-{index}")),
        name: name.as_str().ok_or(LlmError::InvalidResponse)?.to_owned(),
        arguments,
    })
}

#[async_trait::async_trait]
impl LlmProvider for Profile {
    async fn generate(&self, r: &InferenceRequest) -> Result<InferenceResult, LlmError> {
        self.validate()?;
        if r.input.is_empty()
            || r.input.len() + r.instructions.len() > 65536
            || r.tools.len() > 32
            || (!r.tools.is_empty() && !self.tools_supported)
        {
            return Err(LlmError::InvalidRequest);
        }
        for tool in &r.tools {
            if tool.name.is_empty()
                || tool.name.len() > 64
                || !tool
                    .name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                || !crate::schema_allowed(&tool.input_schema)
            {
                return Err(LlmError::InvalidRequest);
            }
        }
        let client = reqwest::Client::builder()
            .user_agent("KusiKusi/0.1")
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|_| LlmError::Configuration)?;
        let mut request = client.post(self.url()?).json(&self.payload(r));
        if let Some(key) = self.key()? {
            request = match self.provider {
                Provider::Claude => request.header("x-api-key", key),
                Provider::Gemini => request.header("x-goog-api-key", key),
                _ => request.bearer_auth(key),
            };
        }
        if matches!(self.provider, Provider::Claude) {
            request = request.header(
                "anthropic-version",
                self.anthropic_version
                    .as_ref()
                    .ok_or(LlmError::Configuration)?,
            );
        }
        let mut response = request.send().await.map_err(|_| LlmError::Unavailable)?;
        match response.status().as_u16() {
            401 | 403 => return Err(LlmError::Authentication),
            429 => return Err(LlmError::RateLimited),
            200..=299 => {}
            _ => return Err(LlmError::Unavailable),
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| LlmError::Unavailable)? {
            if bytes.len() + chunk.len() > 2097152 {
                return Err(LlmError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        let value = serde_json::from_slice(&bytes).map_err(|_| LlmError::InvalidResponse)?;
        let result = self.parse(&value)?;
        for proposed in &result.tools {
            let tool = r
                .tools
                .iter()
                .find(|t| t.name == proposed.name)
                .ok_or(LlmError::InvalidResponse)?;
            use kusi_application::integrations::SchemaValidator;
            if !crate::JsonSchemaValidator.validate(&tool.input_schema, &proposed.arguments) {
                return Err(LlmError::InvalidResponse);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(provider: Provider) -> Profile {
        Profile {
            provider,
            model: "fixture-model".into(),
            api_key_env: None,
            endpoint: None,
            max_output_tokens: 100,
            timeout_ms: 5000,
            tools_supported: true,
            anthropic_version: Some("2023-06-01".into()),
            routing_providers: Some(vec!["fixture-backend".into()]),
        }
    }
    fn request() -> InferenceRequest {
        InferenceRequest {
            instructions: "trusted instructions".into(),
            input: "fictional request".into(),
            tools: Vec::new(),
        }
    }
    #[test]
    fn all_protocols_decode_text_and_reject_truncation() {
        let fixtures = [
            (
                Provider::Openai,
                json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"fixture"}]}]}),
            ),
            (
                Provider::Gemini,
                json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"fixture"}]}}]}),
            ),
            (
                Provider::Claude,
                json!({"stop_reason":"end_turn","content":[{"type":"text","text":"fixture"}]}),
            ),
            (
                Provider::Ollama,
                json!({"done":true,"done_reason":"stop","message":{"content":"fixture"}}),
            ),
        ];
        for (provider, body) in fixtures {
            assert_eq!(
                profile(provider)
                    .parse(&body)
                    .expect("fixture response")
                    .text,
                "fixture"
            );
        }
        for provider in [
            Provider::Openrouter,
            Provider::Nvidia,
            Provider::Vllm,
            Provider::Lmstudio,
            Provider::OpenaiCompatible,
        ] {
            let p = profile(provider);
            assert_eq!(
                p.parse(
                    &json!({"choices":[{"finish_reason":"stop","message":{"content":"fixture"}}]})
                )
                .expect("fixture response")
                .text,
                "fixture"
            );
            assert!(
                p.parse(
                    &json!({"choices":[{"finish_reason":"length","message":{"content":"partial"}}]})
                )
                .is_err()
            );
        }
        assert!(
            profile(Provider::Openai)
                .parse(&json!({"status":"incomplete","output":[]}))
                .is_err()
        );
        assert!(
            profile(Provider::Gemini)
                .parse(&json!({"promptFeedback":{"blockReason":"SAFETY"}}))
                .is_err()
        );
        assert!(
            profile(Provider::Claude)
                .parse(&json!({"stop_reason":"max_tokens","content":[]}))
                .is_err()
        );
    }
    #[test]
    fn payloads_keep_system_instructions_separate_and_disable_remote_storage_or_fallback() {
        let r = request();
        let openai = profile(Provider::Openai).payload(&r);
        assert_eq!(openai["instructions"], r.instructions);
        assert_eq!(openai["store"], false);
        let gemini = profile(Provider::Gemini).payload(&r);
        assert_eq!(
            gemini["systemInstruction"]["parts"][0]["text"],
            r.instructions
        );
        let claude = profile(Provider::Claude).payload(&r);
        assert_eq!(claude["system"], r.instructions);
        let router = profile(Provider::Openrouter).payload(&r);
        assert_eq!(router["provider"]["allow_fallbacks"], false);
    }
    #[test]
    fn tool_calls_decode_native_shapes_without_execution() {
        for (provider, body) in [
            (
                Provider::Openai,
                json!({"status":"completed","output":[{"type":"function_call","call_id":"call-1","name":"lookup","arguments":"{\"id\":1}"}]}),
            ),
            (
                Provider::Claude,
                json!({"stop_reason":"tool_use","content":[{"type":"tool_use","id":"call-1","name":"lookup","input":{"id":1}}]}),
            ),
            (
                Provider::Gemini,
                json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"functionCall":{"name":"lookup","args":{"id":1}}}]}}]}),
            ),
            (
                Provider::Ollama,
                json!({"done":true,"message":{"tool_calls":[{"function":{"name":"lookup","arguments":{"id":1}}}]}}),
            ),
            (
                Provider::Openrouter,
                json!({"choices":[{"finish_reason":"tool_calls","message":{"tool_calls":[{"id":"call-1","function":{"name":"lookup","arguments":"{\"id\":1}"}}]}}]}),
            ),
        ] {
            let result = profile(provider).parse(&body).expect("tool fixture");
            assert_eq!(result.tools[0].arguments, json!({"id":1}));
        }
    }
    #[tokio::test]
    async fn local_compatible_transport_sends_and_receives_real_http() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fixture listener");
        let address = listener.local_addr().expect("fixture address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("fixture connection");
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let n = stream.read(&mut buffer).await.expect("fixture request");
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(split) = bytes.windows(4).position(|p| p == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..split]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .expect("content length");
                    if bytes.len() >= split + 4 + length {
                        let body: Value =
                            serde_json::from_slice(&bytes[split + 4..split + 4 + length])
                                .expect("request JSON");
                        assert_eq!(body["messages"][1]["content"], "fictional request");
                        break;
                    }
                }
            }
            let body =
                json!({"choices":[{"finish_reason":"stop","message":{"content":"fixture"}}]})
                    .to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.expect("fixture response");
        });
        let mut p = profile(Provider::OpenaiCompatible);
        p.endpoint = Some(format!("http://{address}/v1/chat/completions"));
        assert_eq!(
            p.generate(&request()).await.expect("local inference").text,
            "fixture"
        );
        server.await.expect("fixture server");
    }
    #[test]
    fn remote_endpoint_override_and_insecure_network_endpoints_are_rejected() {
        let mut p = profile(Provider::Openai);
        p.endpoint = Some("https://attacker.example/responses".into());
        assert!(p.validate().is_err());
        let mut local = profile(Provider::Ollama);
        local.endpoint = Some("http://10.0.0.1:11434/api/chat".into());
        assert!(local.validate().is_err());
    }
}
