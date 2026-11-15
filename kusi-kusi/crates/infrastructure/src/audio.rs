use kusi_application::{
    audio::{AudioInput, Transcriber, Transcript},
    llm::LlmError,
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioProvider {
    Groq,
    Openai,
    Local,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioProfile {
    pub provider: AudioProvider,
    pub model: String,
    pub api_key_env: Option<String>,
    pub endpoint: Option<String>,
    pub timeout_ms: u64,
}
pub type AudioProfiles = HashMap<Uuid, HashMap<String, AudioProfile>>;
impl AudioProfile {
    pub fn validate(&self) -> Result<(), LlmError> {
        if self.model.is_empty()
            || self.model.len() > 200
            || !(100..=120000).contains(&self.timeout_ms)
        {
            return Err(LlmError::Configuration);
        }
        let url = reqwest::Url::parse(&self.url()?).map_err(|_| LlmError::Configuration)?;
        if url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(LlmError::Configuration);
        }
        match self.provider {
            AudioProvider::Local => {
                if !(url.scheme() == "https"
                    || (url.scheme() == "http"
                        && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
                {
                    return Err(LlmError::Configuration);
                }
            }
            _ => {
                if self.endpoint.is_some() || self.key()?.is_none() {
                    return Err(LlmError::Configuration);
                }
            }
        }
        Ok(())
    }
    fn url(&self) -> Result<String, LlmError> {
        Ok(match self.provider {
            AudioProvider::Groq => "https://api.groq.com/openai/v1/audio/transcriptions".into(),
            AudioProvider::Openai => "https://api.openai.com/v1/audio/transcriptions".into(),
            AudioProvider::Local => self.endpoint.clone().ok_or(LlmError::Configuration)?,
        })
    }
    fn key(&self) -> Result<Option<String>, LlmError> {
        self.api_key_env
            .as_ref()
            .map(|k| {
                std::env::var(k)
                    .ok()
                    .filter(|v| !v.is_empty())
                    .ok_or(LlmError::Configuration)
            })
            .transpose()
    }
    fn form(&self, input: AudioInput) -> Result<reqwest::multipart::Form, LlmError> {
        let extension = input
            .filename
            .rsplit('.')
            .next()
            .ok_or(LlmError::InvalidRequest)?;
        if input.bytes.is_empty()
            || input.bytes.len() > 10 * 1024 * 1024
            || input.filename.len() > 120
            || !input
                .filename
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
            || ![
                "flac", "mp3", "mp4", "mpeg", "mpga", "m4a", "ogg", "wav", "webm",
            ]
            .contains(&extension)
        {
            return Err(LlmError::InvalidRequest);
        }
        let part = reqwest::multipart::Part::bytes(input.bytes).file_name(input.filename);
        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "json");
        if let Some(language) = input.language {
            if language.len() != 2 || !language.bytes().all(|c| c.is_ascii_lowercase()) {
                return Err(LlmError::InvalidRequest);
            }
            form = form.text("language", language);
        }
        Ok(form)
    }
}
#[async_trait::async_trait]
impl Transcriber for AudioProfile {
    async fn transcribe(&self, input: AudioInput) -> Result<Transcript, LlmError> {
        self.validate()?;
        let form = self.form(input)?;
        let client = reqwest::Client::builder()
            .user_agent("kusi-kusi/0.1")
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|_| LlmError::Configuration)?;
        let mut request = client.post(self.url()?).multipart(form);
        if let Some(key) = self.key()? {
            request = request.bearer_auth(key);
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
            if bytes.len() + chunk.len() > 1048576 {
                return Err(LlmError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| LlmError::InvalidResponse)?;
        let text = value["text"]
            .as_str()
            .ok_or(LlmError::InvalidResponse)?
            .to_owned();
        Ok(Transcript {
            text,
            model: self.model.clone(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_audio_invalid_languages_and_path_filenames() {
        let p = AudioProfile {
            provider: AudioProvider::Local,
            model: "fixture".into(),
            api_key_env: None,
            endpoint: Some("http://127.0.0.1:1234/audio/transcriptions".into()),
            timeout_ms: 1000,
        };
        assert!(
            p.form(AudioInput {
                filename: "voice.ogg".into(),
                bytes: vec![],
                language: None
            })
            .is_err()
        );
        assert!(
            p.form(AudioInput {
                filename: "../voice.ogg".into(),
                bytes: vec![1],
                language: None
            })
            .is_err()
        );
        assert!(
            p.form(AudioInput {
                filename: "voice.ogg".into(),
                bytes: vec![1],
                language: Some("spanish".into())
            })
            .is_err()
        );
        assert!(
            p.form(AudioInput {
                filename: "voice.ogg".into(),
                bytes: vec![1],
                language: Some("es".into())
            })
            .is_ok()
        );
    }
}
