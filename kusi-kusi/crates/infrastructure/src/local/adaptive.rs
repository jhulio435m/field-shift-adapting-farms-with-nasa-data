//! Opt-in local adaptive composition. Profile identity comes from verified channel binding.
use super::LocalStore;
use crate::adaptive::{FileState, LanguagePlanner, RuntimeConfig, Sources};
use kusi_application::{adaptive::Assistant, hash_api_key, local::LocalError};
use uuid::Uuid;

impl LocalStore {
    pub async fn adaptive_reply(
        &self,
        org: Uuid,
        user: Uuid,
        channel: Uuid,
        sender: &str,
        message: &str,
        request: Uuid,
    ) -> Result<Option<String>, LocalError> {
        let binding = hash_api_key(&format!("{channel}:{sender}"));
        let path = self
            .root
            .join("adaptive-bindings")
            .join(org.to_string())
            .join(user.to_string())
            .join(format!("{binding}.json"));
        if !path.try_exists().map_err(|_| LocalError::Unavailable)? {
            return Ok(None);
        }
        let result = async {
            let config = RuntimeConfig::load(&path).map_err(|_| LocalError::Forbidden)?;
            config
                .validate_loopback()
                .map_err(|_| LocalError::Forbidden)?;
            let active = self.user(user).await?;
            if active.disabled
                || active.organization_id != org
                || config.organization != org
                || config.principal != user.to_string()
                || config.binding_hash != binding
            {
                return Err(LocalError::Forbidden);
            }
            let actor = config.actor();
            let engine = Assistant {
                environment: Sources::new(actor.clone(), config.sources, config.bridge_script),
                planner: LanguagePlanner {
                    profile: config.llm,
                },
                state: FileState {
                    root: config.state_directory,
                },
            };
            let reply = engine
                .handle(
                    &actor,
                    message,
                    &request.to_string(),
                    chrono::Local::now().date_naive(),
                )
                .await
                .map_err(|_| LocalError::Unavailable)?;
            Ok(reply.text)
        };
        let text = match tokio::time::timeout(std::time::Duration::from_secs(50), result).await {
            Ok(Ok(text)) => text,
            Ok(Err(LocalError::Forbidden)) => "La conexión o delegación de este chat no está vigente. No se consultó el comercio.".into(),
            _ => "La fuente o el intérprete local no pudieron producir una respuesta verificable. No inventaré datos; revisa la conexión antes de volver a consultar.".into(),
        };
        Ok(Some(text))
    }
}
