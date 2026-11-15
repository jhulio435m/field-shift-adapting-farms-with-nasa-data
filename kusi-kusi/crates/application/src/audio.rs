use crate::llm::LlmError;
use async_trait::async_trait;
use serde::Serialize;

pub struct AudioInput {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub language: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Transcript {
    pub text: String,
    pub model: String,
}
#[async_trait]
pub trait Transcriber: Send + Sync {
    async fn transcribe(&self, input: AudioInput) -> Result<Transcript, LlmError>;
}
