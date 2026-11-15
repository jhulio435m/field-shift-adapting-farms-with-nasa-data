//! Bounded synthetic probe. Provider transport is deliberately unavailable in local mode.
use kusi_application::connection_settings::ConnectionKind;
pub fn credentials_present(kind: ConnectionKind, fields: &[String]) -> bool {
    let has = |key: &str| fields.iter().any(|field| field == key);
    match kind {
        ConnectionKind::Plane => has("api_key"),
        ConnectionKind::Frappe => has("api_key") && has("api_secret"),
        ConnectionKind::Telegram => has("bot_token") && has("webhook_secret"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_credentials_are_not_ready() {
        assert!(!credentials_present(
            ConnectionKind::Frappe,
            &["api_key".into()]
        ));
        assert!(credentials_present(
            ConnectionKind::Frappe,
            &["api_key".into(), "api_secret".into()]
        ));
    }
}

/// Trusted runtime policy, never deserialized from a browser or model request.
pub enum ProbeNetworkPolicy {
    LocalBlocked,
    ProviderAllowlist(Vec<String>),
}
/// Write-only values. Deliberately no Debug/Serialize implementations.
pub struct ProviderCredentials {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub bot_token: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProbeError {
    LocalBlocked,
    InvalidDestination,
    InvalidCredentials,
    InsufficientScope,
    Timeout,
    Unavailable,
    InvalidResponse,
}
impl ProbeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::LocalBlocked => "local_external_blocked",
            Self::InvalidDestination => "invalid_destination",
            Self::InvalidCredentials => "invalid_credentials",
            Self::InsufficientScope => "insufficient_scope",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::InvalidResponse => "invalid_response",
        }
    }
}
pub struct ProviderIdentity {
    pub identity_digest: String,
    pub scope_summary: &'static str,
}
fn public_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && !ip.is_unspecified()
                && !ip.is_broadcast()
                && a != 0
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 192 && (b == 0 || b == 2))
                && !(a == 198 && (b == 18 || b == 19 || b == 51 && c == 100))
                && !(a == 203 && b == 0 && c == 113)
        }
        std::net::IpAddr::V6(ip) => {
            let s = ip.segments();
            s[0] & 0xe000 == 0x2000 && !(s[0] == 0x2001 && s[1] == 0xdb8)
        }
    }
}
fn identity(
    kind: ConnectionKind,
    value: &serde_json::Value,
) -> Result<ProviderIdentity, ProbeError> {
    let subject = match kind {
        ConnectionKind::Plane => value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 255)
            .map(str::to_owned),
        ConnectionKind::Frappe => value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 255 && *s != "Guest")
            .map(str::to_owned),
        ConnectionKind::Telegram => {
            if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true)
                && value
                    .pointer("/result/is_bot")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
            {
                value
                    .pointer("/result/id")
                    .and_then(serde_json::Value::as_i64)
                    .filter(|n| *n > 0)
                    .map(|n| n.to_string())
            } else {
                None
            }
        }
    }
    .ok_or(ProbeError::InvalidResponse)?;
    Ok(ProviderIdentity {
        identity_digest: kusi_application::hash_api_key(&format!("{}:{subject}", kind.name())),
        scope_summary: "identity_only_resource_scope_not_checked",
    })
}
/// Provider-only transport. The local HTTP use case rejects provider mode before reaching this function.
/// All DNS answers must be public; the client is pinned to those answers and redirects/proxies are disabled.
pub async fn verify_provider(
    policy: &ProbeNetworkPolicy,
    kind: ConnectionKind,
    base_url: Option<&str>,
    credentials: &ProviderCredentials,
) -> Result<ProviderIdentity, ProbeError> {
    let ProbeNetworkPolicy::ProviderAllowlist(allowlist) = policy else {
        return Err(ProbeError::LocalBlocked);
    };
    let work = async {
        let mut url = url::Url::parse(if kind == ConnectionKind::Telegram {
            "https://api.telegram.org"
        } else {
            base_url.ok_or(ProbeError::InvalidDestination)?
        })
        .map_err(|_| ProbeError::InvalidDestination)?;
        let host = url
            .host_str()
            .ok_or(ProbeError::InvalidDestination)?
            .to_owned();
        if url.scheme() != "https"
            || url.port_or_known_default() != Some(443)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || !allowlist.contains(&host)
        {
            return Err(ProbeError::InvalidDestination);
        }
        let (header, value) = match kind {
            ConnectionKind::Plane => {
                url.set_path("/api/v2/users/me/");
                (
                    "X-Api-Key",
                    credentials
                        .api_key
                        .as_deref()
                        .ok_or(ProbeError::InvalidCredentials)?
                        .to_owned(),
                )
            }
            ConnectionKind::Frappe => {
                url.set_path("/api/method/frappe.auth.get_logged_user");
                (
                    "Authorization",
                    format!(
                        "token {}:{}",
                        credentials
                            .api_key
                            .as_deref()
                            .ok_or(ProbeError::InvalidCredentials)?,
                        credentials
                            .api_secret
                            .as_deref()
                            .ok_or(ProbeError::InvalidCredentials)?
                    ),
                )
            }
            ConnectionKind::Telegram => {
                let token = credentials
                    .bot_token
                    .as_deref()
                    .ok_or(ProbeError::InvalidCredentials)?;
                if token.is_empty()
                    || !token
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b":_-".contains(&b))
                {
                    return Err(ProbeError::InvalidCredentials);
                }
                url.set_path(&format!("/bot{token}/getMe"));
                ("Accept", "application/json".into())
            }
        };
        let header_value = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| ProbeError::InvalidCredentials)?;
        let addresses: Vec<_> = tokio::net::lookup_host((host.as_str(), 443))
            .await
            .map_err(|_| ProbeError::Unavailable)?
            .collect();
        if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
            return Err(ProbeError::InvalidDestination);
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .resolve_to_addrs(&host, &addresses)
            .build()
            .map_err(|_| ProbeError::Unavailable)?;
        let mut response = client
            .get(url)
            .header(header, header_value)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ProbeError::Timeout
                } else {
                    ProbeError::Unavailable
                }
            })?;
        match response.status().as_u16() {
            401 => return Err(ProbeError::InvalidCredentials),
            403 => return Err(ProbeError::InsufficientScope),
            200 => {}
            _ => return Err(ProbeError::Unavailable),
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ProbeError::Unavailable)?
        {
            if bytes.len() + chunk.len() > 65536 {
                return Err(ProbeError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        identity(
            kind,
            &serde_json::from_slice(&bytes).map_err(|_| ProbeError::InvalidResponse)?,
        )
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), work)
        .await
        .map_err(|_| ProbeError::Timeout)?
}
#[cfg(test)]
mod provider_tests {
    use super::*;
    #[tokio::test]
    async fn local_policy_blocks_before_parsing_credentials_or_dns() {
        let result = verify_provider(
            &ProbeNetworkPolicy::LocalBlocked,
            ConnectionKind::Plane,
            Some("invalid"),
            &ProviderCredentials {
                api_key: None,
                api_secret: None,
                bot_token: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ProbeError::LocalBlocked)));
    }
    #[test]
    fn reject_private_and_reserved_dns_answers() {
        for raw in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "203.0.113.1",
            "::1",
            "::ffff:8.8.8.8",
            "2001:db8::1",
        ] {
            assert!(!public_ip(raw.parse().expect("IP")), "{raw}");
        }
        assert!(public_ip("8.8.8.8".parse().expect("IP")));
    }
    #[test]
    fn identity_parser_does_not_infer_resource_permissions() {
        assert!(identity(ConnectionKind::Plane, &serde_json::json!({"html":"login"})).is_err());
        assert!(
            identity(
                ConnectionKind::Telegram,
                &serde_json::json!({"ok":true,"result":{"id":1,"is_bot":false}})
            )
            .is_err()
        );
        let parsed = identity(
            ConnectionKind::Frappe,
            &serde_json::json!({"message":"synthetic@example.invalid"}),
        )
        .expect("identity");
        assert_eq!(
            parsed.scope_summary,
            "identity_only_resource_scope_not_checked"
        );
        assert!(!parsed.identity_digest.contains('@'));
    }
}
