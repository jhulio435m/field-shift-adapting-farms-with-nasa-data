//! HTTPS transport pinned to a validated DNS resolution. No redirects or proxy inheritance.
use serde_json::Value;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("destination rejected")]
    Destination,
    #[error("connector unavailable")]
    Unavailable,
    #[error("invalid connector response")]
    Response,
}

/// Only explicitly configured hosts can be contacted. Caller input never selects a URL.
async fn execute_json(
    url: &str,
    allowed_hosts: &[String],
    input: &Value,
    timeout_ms: u32,
) -> Result<Value, ConnectorError> {
    let url = reqwest::Url::parse(url).map_err(|_| ConnectorError::Destination)?;
    let host = url.host_str().ok_or(ConnectorError::Destination)?;
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !allowed_hosts.iter().any(|allowed| allowed == host)
        || !(100..=30000).contains(&timeout_ms)
    {
        return Err(ConnectorError::Destination);
    }
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, 443))
        .await
        .map_err(|_| ConnectorError::Destination)?
        .collect();
    if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
        return Err(ConnectorError::Destination);
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, &addresses)
        .timeout(Duration::from_millis(u64::from(timeout_ms)))
        .build()
        .map_err(|_| ConnectorError::Unavailable)?;
    let mut response = client
        .post(url)
        .json(input)
        .send()
        .await
        .map_err(|_| ConnectorError::Unavailable)?;
    if !response.status().is_success() {
        return Err(ConnectorError::Response);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ConnectorError::Response)?
    {
        if body.len() + chunk.len() > 1_048_576 {
            return Err(ConnectorError::Response);
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| ConnectorError::Response)
}

pub(crate) fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && !ip.is_unspecified()
                && !(a == 192 && b == 0 && c == 0)
                && !(a == 192 && b == 88 && c == 99)
                && a != 0
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 198 && (b == 18 || b == 19))
        }
        // IPv6 is rejected until an equally strict destination policy is implemented.
        IpAddr::V6(_) => false,
    }
}

/// Immutable server-owned destinations. Never populate this map from an execution request.
pub struct HttpConnector {
    pub destinations: std::collections::HashMap<String, String>,
    pub allowed_hosts: Vec<String>,
}

#[async_trait::async_trait]
impl kusi_application::integrations::ConnectorExecutor for HttpConnector {
    async fn execute(
        &self,
        adapter: &str,
        input: &Value,
        timeout_ms: u32,
    ) -> Result<Value, kusi_application::integrations::ExecutionError> {
        use kusi_application::integrations::ExecutionError;
        let destination = self
            .destinations
            .get(adapter)
            .ok_or(ExecutionError::Forbidden)?;
        tokio::time::timeout(
            Duration::from_millis(u64::from(timeout_ms)),
            execute_json(destination, &self.allowed_hosts, input, timeout_ms),
        )
        .await
        .map_err(|_| ExecutionError::Unavailable)?
        .map_err(|error| match error {
            ConnectorError::Destination => ExecutionError::Forbidden,
            ConnectorError::Response => ExecutionError::InvalidData,
            ConnectorError::Unavailable => ExecutionError::Unavailable,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_private_metadata_and_special_networks() {
        for address in [
            "127.0.0.1",
            "169.254.169.254",
            "10.0.0.1",
            "192.168.1.1",
            "100.64.0.1",
            "198.18.0.1",
            "224.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!public_ip(address.parse().expect("valid test address")));
        }
        assert!(public_ip("8.8.8.8".parse().expect("valid test address")));
    }
}
