//! Opt-in remote acceptance runner. Never loaded by the local workspace server.
//! Uses synthetic accounts and the real ExecuteCapability application use case.
use async_trait::async_trait;
use kusi_application::{
    ApplicationError, CapabilityRegistry, GetOrganizationByKey, OrganizationRepository,
    hash_api_key,
    integrations::{
        AuditRepository, AuthorizationService, ConnectorExecutor, ExecuteCapability,
        ExecutionError, SchemaValidator,
    },
};
use kusi_domain::{Capability, Operation, Organization};
use reqwest::{Client, Method, Response, StatusCode, header::SET_COOKIE};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Destination {
    base: String,
    host: String,
    email: Option<String>,
    password: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    systems: BTreeMap<String, Destination>,
}

struct Registry {
    organization: Uuid,
}

#[async_trait]
impl CapabilityRegistry for Registry {
    async fn register(&self, _: Uuid, _: &Capability) -> Result<(), ApplicationError> {
        Err(ApplicationError::Storage)
    }

    async fn list(&self, organization: Uuid) -> Result<Vec<Capability>, ApplicationError> {
        if organization != self.organization {
            return Ok(Vec::new());
        }
        Ok(["crm", "erp", "plane", "wppconnect"]
            .into_iter()
            .map(|system| Capability {
                operation: Operation::Read,
                identifier: format!("acceptance.{system}.read"),
                version: "1".into(),
                description: "Read only synthetic identity or service health".into(),
                adapter: system.into(),
                required_permission: format!("acceptance.{system}.read"),
                enabled: true,
                timeout_ms: 30000,
                input_schema: json!({"type":"object","additionalProperties":false}),
                output_schema: json!({
                    "type":"object","additionalProperties":false,
                    "required":["system","check","ok"],
                    "properties":{
                        "system":{"const":system},
                        "check":{"const":if system == "wppconnect" {"health"} else {"identity"}},
                        "ok":{"const":true}
                    }
                }),
            })
            .collect())
    }
}

#[derive(Clone)]
struct Identity {
    organization: Organization,
    key_hash: String,
}

#[async_trait]
impl OrganizationRepository for Identity {
    async fn create(&self, _: &Organization, _: &str) -> Result<(), ApplicationError> {
        Err(ApplicationError::Storage)
    }

    async fn find_by_api_key_hash(
        &self,
        hash: &str,
    ) -> Result<Option<Organization>, ApplicationError> {
        Ok((hash == self.key_hash).then(|| self.organization.clone()))
    }
}

struct Grants {
    organization: Uuid,
    principal: String,
}

#[async_trait]
impl AuthorizationService for Grants {
    async fn authorize_resource(
        &self,
        _: Uuid,
        _: &str,
        _: &Capability,
        _: &Value,
    ) -> Result<Option<kusi_domain::external_access::ResourceRead>, ExecutionError> {
        // This isolated acceptance harness exposes only fixed identity/health reads.
        Ok(None)
    }
    async fn permits(&self, organization: Uuid, principal: &str, permission: &str) -> bool {
        organization == self.organization
            && principal == self.principal
            && ["crm", "erp", "plane", "wppconnect"]
                .iter()
                .any(|system| permission == format!("acceptance.{system}.read"))
    }
}

struct Validator;
impl SchemaValidator for Validator {
    fn validate(&self, schema: &Value, value: &Value) -> bool {
        jsonschema::validator_for(schema).is_ok_and(|validator| validator.is_valid(value))
    }
}

#[derive(Default)]
struct Audit(Mutex<Vec<(Uuid, String)>>);
#[async_trait]
impl AuditRepository for Audit {
    async fn record(
        &self,
        event: Uuid,
        _: Uuid,
        _: &str,
        _: &str,
        outcome: &str,
    ) -> Result<(), ExecutionError> {
        self.0
            .lock()
            .map_err(|_| ExecutionError::Unavailable)?
            .push((event, outcome.into()));
        Ok(())
    }
}

struct LiveAdapters {
    configuration: Configuration,
    calls: Arc<AtomicUsize>,
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
        let cookies = self
            .cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.destination.base))
            .header("Host", &self.destination.host)
            .header("Origin", format!("https://{}", self.destination.host))
            .header("Referer", format!("https://{}/", self.destination.host))
            .header("X-Forwarded-Proto", "https")
            .header("Cookie", cookies);
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
        eprintln!(
            "acceptance {} {path}: {}",
            self.destination.host,
            response.status()
        );
        for cookie in response.headers().get_all(SET_COOKIE) {
            let cookie = cookie.to_str().map_err(|_| ExecutionError::InvalidData)?;
            if let Some((name, value)) = cookie.split(';').next().and_then(|s| s.split_once('=')) {
                self.cookies.insert(name.into(), value.into());
            }
        }
        Ok(response)
    }
}

async fn bounded_json(mut response: Response) -> Result<Value, ExecutionError> {
    if !response.status().is_success() {
        return Err(ExecutionError::InvalidData);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ExecutionError::Unavailable)?
    {
        if body.len() + chunk.len() > 65536 {
            return Err(ExecutionError::InvalidData);
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| ExecutionError::InvalidData)
}

fn require(condition: bool) -> Result<(), ExecutionError> {
    if condition {
        Ok(())
    } else {
        Err(ExecutionError::InvalidData)
    }
}

#[async_trait]
impl ConnectorExecutor for LiveAdapters {
    async fn execute(
        &self,
        adapter: &str,
        input: &Value,
        timeout_ms: u32,
    ) -> Result<Value, ExecutionError> {
        require(input == &json!({}))?;
        let destination = self
            .configuration
            .systems
            .get(adapter)
            .ok_or(ExecutionError::Forbidden)?;
        let client = Client::builder()
            .no_proxy()
            .user_agent("kusi-kusi-acceptance/0.1")
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(u64::from(timeout_ms)))
            .build()
            .map_err(|_| ExecutionError::Unavailable)?;
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut session = Session {
            destination,
            client,
            cookies: BTreeMap::new(),
        };
        let check = if adapter == "wppconnect" {
            let health =
                bounded_json(session.call(Method::GET, "/healthz", None, None).await?).await?;
            require(health["message"] == "OK")?;
            let denied = session
                .call(
                    Method::GET,
                    "/api/kusi-qa-20261108/check-connection-session",
                    None,
                    None,
                )
                .await?;
            require(matches!(denied.status().as_u16(), 401 | 403))?;
            "health"
        } else {
            let email = destination
                .email
                .as_deref()
                .ok_or(ExecutionError::Forbidden)?;
            let password = destination
                .password
                .as_deref()
                .ok_or(ExecutionError::Forbidden)?;
            require(email == "kusi-qa-20261108@example.invalid")?;
            if adapter == "plane" {
                let csrf = bounded_json(
                    session
                        .call(Method::GET, "/auth/get-csrf-token/", None, None)
                        .await?,
                )
                .await?;
                let token = csrf["csrf_token"]
                    .as_str()
                    .ok_or(ExecutionError::InvalidData)?;
                let login = session
                    .call(
                        Method::POST,
                        "/auth/sign-in/",
                        Some(&[("email", email), ("password", password)]),
                        Some(token),
                    )
                    .await?;
                require(login.status() == StatusCode::FOUND)?;
                let identity_result = async {
                    let identity = bounded_json(
                        session
                            .call(Method::GET, "/api/users/me/", None, None)
                            .await?,
                    )
                    .await?;
                    require(identity["email"] == email)?;
                    let workspaces = bounded_json(
                        session
                            .call(Method::GET, "/api/users/me/workspaces/", None, None)
                            .await?,
                    )
                    .await?;
                    require(workspaces == json!([]))
                }
                .await;
                let csrf = bounded_json(
                    session
                        .call(Method::GET, "/auth/get-csrf-token/", None, None)
                        .await?,
                )
                .await?;
                let logout = session
                    .call(
                        Method::POST,
                        "/auth/sign-out/",
                        Some(&[]),
                        csrf["csrf_token"].as_str(),
                    )
                    .await?;
                require(logout.status() == StatusCode::FOUND)?;
                identity_result?;
            } else {
                let login = session
                    .call(
                        Method::POST,
                        "/api/method/login",
                        Some(&[("usr", email), ("pwd", password)]),
                        None,
                    )
                    .await?;
                require(login.status() == StatusCode::OK)?;
                let identity_result = async {
                    let identity = bounded_json(
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
                    require(identity["message"] == email)?;
                    let denied = session
                        .call(Method::GET, "/api/resource/User/Administrator", None, None)
                        .await?;
                    require(matches!(denied.status().as_u16(), 401 | 403))
                }
                .await;
                let logout = session
                    .call(Method::POST, "/api/method/logout", Some(&[]), None)
                    .await?;
                require(logout.status() == StatusCode::OK)?;
                identity_result?;
            }
            "identity"
        };
        Ok(json!({"system":adapter,"check":check,"ok":true}))
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("KUSI_REMOTE_ACCEPTANCE").as_deref() != Ok("synthetic-read-only") {
        return Err("requires explicit KUSI_REMOTE_ACCEPTANCE=synthetic-read-only".into());
    }
    let path = std::env::args()
        .nth(1)
        .ok_or("requires private configuration path")?;
    let configuration: Configuration = serde_json::from_slice(&std::fs::read(path)?)?;
    if configuration.systems.len() != 4 {
        return Err("requires exactly four configured test systems".into());
    }
    for destination in configuration.systems.values() {
        let url = reqwest::Url::parse(&destination.base)?;
        let Some(url::Host::Ipv4(ip)) = url.host() else {
            return Err("verification destinations must be literal private IPv4 addresses".into());
        };
        if url.scheme() != "http"
            || !(ip.is_private() || ip.is_loopback())
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("invalid isolated verification destination".into());
        }
    }
    let organization =
        Organization::new("Remote synthetic QA".into(), "remote-synthetic-qa".into())?;
    let key = Uuid::new_v4().to_string();
    let principal = hash_api_key(&key);
    let identity = Identity {
        organization,
        key_hash: principal.clone(),
    };
    let resolved = GetOrganizationByKey::new(identity.clone())
        .execute(&key)
        .await?;
    if GetOrganizationByKey::new(identity)
        .execute("invalid-key")
        .await
        .is_ok()
    {
        return Err("invalid credential resolved a tenant".into());
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = ExecuteCapability {
        registry: Registry {
            organization: resolved.id,
        },
        authorization: Grants {
            organization: resolved.id,
            principal: principal.clone(),
        },
        validator: Validator,
        connector: LiveAdapters {
            configuration,
            calls: calls.clone(),
        },
        audit: Audit::default(),
    };
    let mut results = Vec::new();
    for system in ["crm", "erp", "plane", "wppconnect"] {
        let capability = format!("acceptance.{system}.read");
        let before = calls.load(Ordering::SeqCst);
        for (tenant, actor, input, version, expected_invalid) in [
            (Uuid::new_v4(), principal.as_str(), json!({}), "1", false),
            (resolved.id, "ungranted-principal", json!({}), "1", false),
            (
                resolved.id,
                principal.as_str(),
                json!({"url":"http://169.254.169.254"}),
                "1",
                true,
            ),
            (resolved.id, principal.as_str(), json!({}), "unknown", false),
        ] {
            let result = executor
                .execute(tenant, actor, &capability, version, &input)
                .await;
            let rejected = if expected_invalid {
                matches!(result, Err(ExecutionError::InvalidData))
            } else {
                matches!(result, Err(ExecutionError::Forbidden))
            };
            if !rejected || calls.load(Ordering::SeqCst) != before {
                return Err("authorization/schema rejection called a destination".into());
            }
        }
        let output = executor
            .execute(resolved.id, &principal, &capability, "1", &json!({}))
            .await
            .map_err(|error| format!("{system}: {error}"))?;
        if calls.load(Ordering::SeqCst) != before + 1 {
            return Err("unexpected connector invocation count".into());
        }
        results.push(
            json!({"system":system,"result":output,"negative_checks":4,"live_invocations":1}),
        );
    }
    let audit = executor
        .audit
        .0
        .lock()
        .map_err(|_| "audit lock unavailable")?;
    if audit.len() != 40
        || audit
            .chunks_exact(2)
            .any(|pair| pair[0].0 != pair[1].0 || pair[0].1 != "started")
    {
        return Err("incomplete application audit".into());
    }
    println!(
        "{}",
        json!({"status":"passed","layer":"real ExecuteCapability; in-memory registry/grants/audit; live destination HTTP","systems":results,"audit_events":audit.len(),"invalid_key_denied":true,"messages_sent":0})
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("remote acceptance failed: {error}");
        std::process::exit(1);
    }
}
