//! Explicit, tenant-owned, read-only platform adapters. No caller-selected URLs or credentials.
use crate::http_connector::{HttpConnector, public_ip};
use kusi_application::integrations::{ConnectorExecutor, ExecutionError};
use reqwest::{Client, Method, Response, StatusCode, header::SET_COOKIE};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
    net::SocketAddr,
    path::Path,
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
#[error("invalid system connector configuration")]
pub struct ConfigurationError;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Kind {
    FrappeIdentity,
    PlaneIdentity,
    WppconnectHealth,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    kind: Kind,
    base_url: String,
    host: String,
    credential_file: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    username: String,
    password: String,
}

struct Destination {
    kind: Kind,
    base_url: reqwest::Url,
    host: String,
    credentials: Option<Credentials>,
    private: bool,
}

/// Immutable server-owned configuration, loaded once; credentials are never Debug/Serialize.
#[derive(Default)]
pub struct SystemProfiles(HashMap<Uuid, HashMap<String, Destination>>);

impl SystemProfiles {
    pub fn load(
        configuration: &str,
        allowed_hosts: &[String],
        allow_private_http: bool,
        local_mode: bool,
    ) -> Result<Self, ConfigurationError> {
        let profiles: HashMap<Uuid, HashMap<String, Profile>> =
            serde_json::from_str(configuration).map_err(|_| ConfigurationError)?;
        if local_mode && profiles.values().any(|profiles| !profiles.is_empty()) {
            return Err(ConfigurationError);
        }
        let mut destinations = HashMap::new();
        for (organization, profiles) in profiles {
            let mut tenant = HashMap::new();
            for (adapter, profile) in profiles {
                if adapter.is_empty() || adapter.len() > 120 {
                    return Err(ConfigurationError);
                }
                let (base_url, private) =
                    validate_destination(&profile, allowed_hosts, allow_private_http)?;
                let credentials = match (&profile.kind, &profile.credential_file) {
                    (Kind::WppconnectHealth, None) => None,
                    (Kind::FrappeIdentity | Kind::PlaneIdentity, Some(path)) => {
                        Some(read_credentials(Path::new(path))?)
                    }
                    _ => return Err(ConfigurationError),
                };
                tenant.insert(
                    adapter,
                    Destination {
                        kind: profile.kind,
                        base_url,
                        host: profile.host,
                        credentials,
                        private,
                    },
                );
            }
            destinations.insert(organization, tenant);
        }
        Ok(Self(destinations))
    }

    pub fn contains(&self, organization: Uuid, adapter: &str) -> bool {
        self.0
            .get(&organization)
            .is_some_and(|profiles| profiles.contains_key(adapter))
    }
}

fn read_credentials(path: &Path) -> Result<Credentials, ConfigurationError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ConfigurationError)?;
    if !metadata.is_file() || metadata.len() > 8192 {
        return Err(ConfigurationError);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(ConfigurationError);
        }
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| ConfigurationError)?
        .take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| ConfigurationError)?;
    if bytes.len() > 8192 {
        return Err(ConfigurationError);
    }
    let credentials: Credentials =
        serde_json::from_slice(&bytes).map_err(|_| ConfigurationError)?;
    if credentials.username.is_empty()
        || credentials.username.len() > 254
        || credentials.password.is_empty()
        || credentials.password.len() > 1024
    {
        return Err(ConfigurationError);
    }
    Ok(credentials)
}

fn validate_destination(
    profile: &Profile,
    allowed_hosts: &[String],
    allow_private_http: bool,
) -> Result<(reqwest::Url, bool), ConfigurationError> {
    if !allowed_hosts.contains(&profile.host) {
        return Err(ConfigurationError);
    }
    let authority = reqwest::Url::parse(&format!("https://{}", profile.host))
        .map_err(|_| ConfigurationError)?;
    if authority.host_str() != Some(profile.host.as_str())
        || authority.port().is_some()
        || authority.path() != "/"
        || !authority.username().is_empty()
        || authority.password().is_some()
        || authority.query().is_some()
        || authority.fragment().is_some()
    {
        return Err(ConfigurationError);
    }
    let url = reqwest::Url::parse(&profile.base_url).map_err(|_| ConfigurationError)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfigurationError);
    }
    let private = url.scheme() == "http"
        && allow_private_http
        && matches!(url.host(), Some(url::Host::Ipv4(ip)) if ip.is_private() || ip.is_loopback());
    let public = url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.host_str() == Some(profile.host.as_str());
    if !private && !public {
        return Err(ConfigurationError);
    }
    Ok((url, private))
}

/// Dispatch is selected only after application authorization/schema validation.
pub struct RuntimeConnector {
    pub organization: Uuid,
    pub systems: Arc<SystemProfiles>,
    pub http: HttpConnector,
}

#[async_trait::async_trait]
impl ConnectorExecutor for RuntimeConnector {
    async fn execute(
        &self,
        adapter: &str,
        input: &Value,
        timeout_ms: u32,
    ) -> Result<Value, ExecutionError> {
        if let Some(destination) = self
            .systems
            .0
            .get(&self.organization)
            .and_then(|profiles| profiles.get(adapter))
        {
            // A permissive registered schema must never enable arguments on these fixed reads.
            if input != &json!({}) || !(100..=30000).contains(&timeout_ms) {
                return Err(ExecutionError::InvalidData);
            }
            tokio::time::timeout(
                Duration::from_millis(u64::from(timeout_ms)),
                execute(destination, timeout_ms),
            )
            .await
            .map_err(|_| ExecutionError::Unavailable)?
        } else {
            self.http.execute(adapter, input, timeout_ms).await
        }
    }
}

async fn client(destination: &Destination, timeout_ms: u32) -> Result<Client, ExecutionError> {
    let mut builder = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("kusi-kusi/0.1")
        .timeout(Duration::from_millis(u64::from(timeout_ms)));
    if !destination.private {
        let host = destination
            .base_url
            .host_str()
            .ok_or(ExecutionError::Forbidden)?;
        let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, 443))
            .await
            .map_err(|_| ExecutionError::Unavailable)?
            .collect();
        if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
            return Err(ExecutionError::Forbidden);
        }
        builder = builder.resolve_to_addrs(host, &addresses);
    } else if !matches!(destination.base_url.host(), Some(url::Host::Ipv4(ip)) if ip.is_private() || ip.is_loopback())
    {
        return Err(ExecutionError::Forbidden);
    }
    builder.build().map_err(|_| ExecutionError::Unavailable)
}

struct Session<'a> {
    destination: &'a Destination,
    client: Client,
    cookies: BTreeMap<String, String>,
}

impl Session<'_> {
    async fn call(
        &mut self,
        method: Method,
        path: &str,
        form: Option<&[(&str, &str)]>,
        csrf: Option<&str>,
    ) -> Result<Response, ExecutionError> {
        let url = self
            .destination
            .base_url
            .join(path)
            .map_err(|_| ExecutionError::Forbidden)?;
        let cookies = self
            .cookies
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        let origin = if self.destination.private {
            format!("https://{}", self.destination.host)
        } else {
            self.destination.base_url.origin().ascii_serialization()
        };
        let mut request = self
            .client
            .request(method, url)
            .header("Host", &self.destination.host)
            .header("Origin", &origin)
            .header("Referer", format!("{origin}/"))
            .header("Cookie", cookies);
        if self.destination.private {
            request = request.header("X-Forwarded-Proto", "https");
        }
        if let Some(form) = form {
            request = request.form(form);
        }
        if let Some(csrf) = csrf {
            request = request.header("X-CSRFToken", csrf);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ExecutionError::Unavailable)?;
        for header in response.headers().get_all(SET_COOKIE) {
            let value = header.to_str().map_err(|_| ExecutionError::InvalidData)?;
            if let Some((key, value)) = value
                .split(';')
                .next()
                .and_then(|part| part.split_once('='))
            {
                self.cookies.insert(key.into(), value.into());
            }
        }
        Ok(response)
    }
}

async fn json_response(mut response: Response) -> Result<Value, ExecutionError> {
    if !response.status().is_success() {
        return Err(ExecutionError::InvalidData);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ExecutionError::Unavailable)?
    {
        if bytes.len() + chunk.len() > 65536 {
            return Err(ExecutionError::InvalidData);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| ExecutionError::InvalidData)
}

fn require(value: bool) -> Result<(), ExecutionError> {
    if value {
        Ok(())
    } else {
        Err(ExecutionError::InvalidData)
    }
}

async fn plane_csrf(session: &mut Session<'_>) -> Result<String, ExecutionError> {
    let value = json_response(
        session
            .call(Method::GET, "/auth/get-csrf-token/", None, None)
            .await?,
    )
    .await?;
    value["csrf_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or(ExecutionError::InvalidData)
}

async fn execute(destination: &Destination, timeout_ms: u32) -> Result<Value, ExecutionError> {
    let mut session = Session {
        destination,
        client: client(destination, timeout_ms).await?,
        cookies: BTreeMap::new(),
    };
    if destination.kind == Kind::WppconnectHealth {
        let result =
            json_response(session.call(Method::GET, "/healthz", None, None).await?).await?;
        require(result["message"] == "OK")?;
        return Ok(json!({"kind":"wppconnect_health","reachable":true,"authenticated":false}));
    }
    let credentials = destination
        .credentials
        .as_ref()
        .ok_or(ExecutionError::Forbidden)?;
    if destination.kind == Kind::FrappeIdentity {
        let login = session
            .call(
                Method::POST,
                "/api/method/login",
                Some(&[
                    ("usr", &credentials.username),
                    ("pwd", &credentials.password),
                ]),
                None,
            )
            .await?;
        require(login.status() == StatusCode::OK)?;
        let result = async {
            let identity = json_response(
                session
                    .call(
                        Method::GET,
                        "/api/method/frappe.auth.get_logged_user",
                        None,
                        None,
                    )
                    .await?,
            )
            .await?;
            require(identity["message"] == credentials.username)?;
            Ok(json!({"kind":"frappe_identity","reachable":true,"authenticated":true}))
        }
        .await;
        // Always attempt logout, including identity mismatch/invalid response.
        let logout = session
            .call(Method::POST, "/api/method/logout", Some(&[]), None)
            .await?;
        require(logout.status() == StatusCode::OK)?;
        result
    } else {
        let csrf = plane_csrf(&mut session).await?;
        let login = session
            .call(
                Method::POST,
                "/auth/sign-in/",
                Some(&[
                    ("email", &credentials.username),
                    ("password", &credentials.password),
                ]),
                Some(&csrf),
            )
            .await?;
        require(login.status() == StatusCode::FOUND)?;
        let result = async {
            let identity = json_response(session.call(Method::GET, "/api/users/me/", None, None).await?).await?;
            require(identity["email"] == credentials.username)?;
            let workspaces = json_response(session.call(Method::GET, "/api/users/me/workspaces/", None, None).await?).await?;
            let count = workspaces.as_array().ok_or(ExecutionError::InvalidData)?.len();
            Ok(json!({"kind":"plane_identity","reachable":true,"authenticated":true,"workspace_count":count}))
        }.await;
        let csrf = plane_csrf(&mut session).await?;
        let logout = session
            .call(Method::POST, "/auth/sign-out/", Some(&[]), Some(&csrf))
            .await?;
        require(logout.status() == StatusCode::FOUND)?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(base: &str, host: &str) -> Profile {
        Profile {
            kind: Kind::WppconnectHealth,
            base_url: base.into(),
            host: host.into(),
            credential_file: None,
        }
    }

    #[test]
    fn private_destinations_need_explicit_opt_in_and_cannot_use_dns_or_metadata() {
        let allowed = vec!["service.example".into()];
        assert!(
            validate_destination(
                &profile("http://127.0.0.1:8090", "service.example"),
                &allowed,
                false
            )
            .is_err()
        );
        assert!(
            validate_destination(
                &profile("http://172.19.0.8:8000", "service.example"),
                &allowed,
                true
            )
            .is_ok()
        );
        for base in [
            "http://169.254.169.254",
            "http://service.example",
            "http://8.8.8.8",
            "http://127.0.0.1/path",
            "http://user:pass@127.0.0.1",
            "http://127.0.0.1?target=other",
            "http://127.0.0.1#fragment",
            "http://[::1]",
        ] {
            assert!(
                validate_destination(&profile(base, "service.example"), &allowed, true).is_err(),
                "{base}"
            );
        }
        assert!(
            validate_destination(
                &profile("http://127.0.0.1", "other.example"),
                &allowed,
                true
            )
            .is_err()
        );
    }

    #[test]
    fn public_destinations_require_matching_allowlisted_https_authority() {
        let allowed = vec!["service.example".into()];
        assert!(
            validate_destination(
                &profile("https://service.example", "service.example"),
                &allowed,
                false
            )
            .is_ok()
        );
        for base in [
            "https://other.example",
            "https://service.example:444",
            "https://service.example/path",
        ] {
            assert!(
                validate_destination(&profile(base, "service.example"), &allowed, false).is_err()
            );
        }
    }

    #[tokio::test]
    async fn local_configuration_tenant_scope_and_arguments_fail_before_transport() {
        let tenant = Uuid::new_v4();
        let other = Uuid::new_v4();
        let value = json!({tenant.to_string():{"health":{"kind":"wppconnect_health","base_url":"http://127.0.0.1:9","host":"service.example"}}}).to_string();
        let allowed = vec!["service.example".into()];
        assert!(SystemProfiles::load(&value, &allowed, true, true).is_err());
        let systems = Arc::new(
            SystemProfiles::load(&value, &allowed, true, false).expect("fixture profiles"),
        );
        assert!(systems.contains(tenant, "health"));
        assert!(!systems.contains(other, "health"));
        let connector = RuntimeConnector {
            organization: tenant,
            systems: systems.clone(),
            http: HttpConnector {
                destinations: HashMap::new(),
                allowed_hosts: Vec::new(),
            },
        };
        assert!(matches!(
            connector
                .execute("health", &json!({"url":"http://metadata"}), 1000)
                .await,
            Err(ExecutionError::InvalidData)
        ));
        let connector = RuntimeConnector {
            organization: other,
            systems,
            http: HttpConnector {
                destinations: HashMap::new(),
                allowed_hosts: Vec::new(),
            },
        };
        assert!(matches!(
            connector.execute("health", &json!({}), 1000).await,
            Err(ExecutionError::Forbidden)
        ));
    }

    #[test]
    fn credential_files_require_private_regular_bounded_json()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory =
            std::env::temp_dir().join(format!("kusi-credential-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory)?;
        let path = directory.join("credentials.json");
        std::fs::write(
            &path,
            r#"{"username":"synthetic@example.invalid","password":"fictional"}"#,
        )?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
        assert!(read_credentials(&path).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        assert!(read_credentials(&path).is_ok());
        let link = directory.join("link.json");
        symlink(&path, &link)?;
        assert!(read_credentials(&link).is_err());
        std::fs::write(&path, vec![b'a'; 8193])?;
        assert!(read_credentials(&path).is_err());
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }
}
