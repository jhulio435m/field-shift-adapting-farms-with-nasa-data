use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use kusi_application::oauth::{OAuthError, OAuthProfile, OAuthProvider, OAuthTokens};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::Duration;

pub struct OfficialOAuthProvider {
    pub simulation: bool,
}
pub fn random_secret() -> Result<String, OAuthError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| OAuthError::Storage)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
fn secret(profile: &OAuthProfile) -> Result<String, OAuthError> {
    profile
        .client_secret_env
        .as_ref()
        .and_then(|k| std::env::var(k).ok())
        .filter(|v| !v.is_empty())
        .ok_or(OAuthError::Invalid)
}
pub fn validate_profile(profile: &OAuthProfile, simulation: bool) -> Result<(), OAuthError> {
    if !["facebook", "instagram", "tiktok", "simulated"].contains(&profile.provider.as_str())
        || profile.client_id.is_empty()
        || profile.client_id.len() > 200
        || profile.scopes.is_empty()
        || profile.scopes.len() > 20
        || profile.scopes.iter().any(|s| {
            s.is_empty()
                || s.len() > 100
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
        })
    {
        return Err(OAuthError::Invalid);
    }
    let url = reqwest::Url::parse(&profile.redirect_uri).map_err(|_| OAuthError::Invalid)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != format!("/v1/oauth/{}/callback", profile.provider)
    {
        return Err(OAuthError::Invalid);
    }
    if profile.provider == "simulated" {
        if !simulation {
            return Err(OAuthError::Invalid);
        }
    } else {
        secret(profile)?;
    }
    // This delivery implements confidential web clients; PKCE is used only by the verified simulator.
    // TikTok documents code_verifier as mandatory for mobile/desktop, not this web flow.
    if profile.pkce && profile.provider != "simulated" {
        return Err(OAuthError::Invalid);
    }
    if ["facebook", "instagram"].contains(&profile.provider.as_str()) {
        let version = profile
            .graph_version
            .as_deref()
            .ok_or(OAuthError::Invalid)?;
        if !version.starts_with('v')
            || version.len() < 4
            || version.len() > 10
            || !version[1..]
                .bytes()
                .all(|c| c.is_ascii_digit() || c == b'.')
        {
            return Err(OAuthError::Invalid);
        }
    }
    Ok(())
}
fn client() -> Result<reqwest::Client, OAuthError> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| OAuthError::Provider)
}
async fn response(request: reqwest::RequestBuilder) -> Result<Value, OAuthError> {
    let mut response = request.send().await.map_err(|_| OAuthError::Provider)?;
    if !response.status().is_success() {
        return Err(OAuthError::Provider);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| OAuthError::Provider)? {
        if bytes.len() + chunk.len() > 65536 {
            return Err(OAuthError::Provider);
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| OAuthError::Provider)?;
    if v.get("error").is_some_and(|v| !v.is_null()) {
        return Err(OAuthError::Provider);
    }
    Ok(v)
}
fn value_text(v: &Value, key: &str) -> Result<String, OAuthError> {
    v[key]
        .as_str()
        .map(str::to_owned)
        .or_else(|| v[key].as_u64().map(|n| n.to_string()))
        .filter(|s| !s.is_empty() && s.len() <= 16384)
        .ok_or(OAuthError::Provider)
}
fn duration(v: &Value, key: &str) -> Result<i64, OAuthError> {
    v[key]
        .as_i64()
        .filter(|n| *n > 0 && *n <= 63_072_000)
        .ok_or(OAuthError::Provider)
}
fn scopes(value: &Value) -> Vec<String> {
    let items = match value {
        Value::String(v) => v
            .split([',', ' '])
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect(),
        Value::Array(v) => v
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    let mut items: Vec<String> = items;
    items.sort();
    items.dedup();
    items
}
fn ensure_scopes(profile: &OAuthProfile, granted: &[String]) -> Result<(), OAuthError> {
    if profile.scopes.iter().all(|v| granted.contains(v)) {
        Ok(())
    } else {
        Err(OAuthError::Provider)
    }
}
fn parse_tiktok(profile: &OAuthProfile, v: &Value) -> Result<OAuthTokens, OAuthError> {
    let granted = scopes(&v["scope"]);
    ensure_scopes(profile, &granted)?;
    Ok(OAuthTokens {
        access_token: value_text(v, "access_token")?,
        refresh_token: Some(value_text(v, "refresh_token")?),
        expires_in: duration(v, "expires_in")?,
        refresh_expires_in: Some(duration(v, "refresh_expires_in")?),
        external_id: value_text(v, "open_id")?,
        scopes: granted,
    })
}
fn simulated_tokens(profile: &OAuthProfile, code: &str) -> OAuthTokens {
    OAuthTokens {
        access_token: format!("SIMULATED_ACCESS_{code}"),
        refresh_token: Some(format!("SIMULATED_REFRESH_{code}")),
        expires_in: 3600,
        refresh_expires_in: Some(86400),
        external_id: "simulated-social-account".into(),
        scopes: profile.scopes.clone(),
    }
}

#[async_trait::async_trait]
impl OAuthProvider for OfficialOAuthProvider {
    fn authorize_url(
        &self,
        p: &OAuthProfile,
        state: &str,
        challenge: Option<&str>,
    ) -> Result<String, OAuthError> {
        validate_profile(p, self.simulation)?;
        let base = match p.provider.as_str() {
            "facebook" => format!(
                "https://www.facebook.com/{}/dialog/oauth",
                p.graph_version.as_deref().ok_or(OAuthError::Invalid)?
            ),
            "instagram" => "https://www.instagram.com/oauth/authorize".into(),
            "tiktok" => "https://www.tiktok.com/v2/auth/authorize/".into(),
            "simulated" => "https://oauth.invalid/authorize".into(),
            _ => return Err(OAuthError::Invalid),
        };
        let mut url = reqwest::Url::parse(&base).map_err(|_| OAuthError::Invalid)?;
        url.query_pairs_mut()
            .append_pair(
                if p.provider == "tiktok" {
                    "client_key"
                } else {
                    "client_id"
                },
                &p.client_id,
            )
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &p.redirect_uri)
            .append_pair("scope", &p.scopes.join(","))
            .append_pair("state", state);
        if let Some(challenge) = challenge {
            url.query_pairs_mut()
                .append_pair("code_challenge", challenge)
                .append_pair("code_challenge_method", "S256");
        }
        Ok(url.into())
    }
    async fn exchange(
        &self,
        p: &OAuthProfile,
        code: &str,
        verifier: Option<&str>,
    ) -> Result<OAuthTokens, OAuthError> {
        validate_profile(p, self.simulation)?;
        if code.is_empty() || code.len() > 8192 {
            return Err(OAuthError::Invalid);
        }
        if p.provider == "simulated" {
            if code == "deny" || (p.pkce && verifier.is_none()) {
                return Err(OAuthError::Provider);
            }
            return Ok(simulated_tokens(p, code));
        }
        let secret = secret(p)?;
        let http = client()?;
        match p.provider.as_str() {
            "tiktok" => {
                let v = response(
                    http.post("https://open.tiktokapis.com/v2/oauth/token/")
                        .form(&[
                            ("client_key", p.client_id.as_str()),
                            ("client_secret", secret.as_str()),
                            ("code", code),
                            ("grant_type", "authorization_code"),
                            ("redirect_uri", p.redirect_uri.as_str()),
                        ]),
                )
                .await?;
                parse_tiktok(p, &v)
            }
            "instagram" => {
                let form = reqwest::multipart::Form::new()
                    .text("client_id", p.client_id.clone())
                    .text("client_secret", secret.clone())
                    .text("grant_type", "authorization_code")
                    .text("redirect_uri", p.redirect_uri.clone())
                    .text("code", code.to_owned());
                let short = response(
                    http.post("https://api.instagram.com/oauth/access_token")
                        .multipart(form),
                )
                .await?;
                let short = short
                    .get("data")
                    .and_then(Value::as_array)
                    .and_then(|v| v.first())
                    .unwrap_or(&short);
                let granted = scopes(&short["permissions"]);
                ensure_scopes(p, &granted)?;
                let short_token = value_text(short, "access_token")?;
                let long = response(
                    http.get("https://graph.instagram.com/access_token")
                        .query(&[
                            ("grant_type", "ig_exchange_token"),
                            ("client_secret", secret.as_str()),
                            ("access_token", short_token.as_str()),
                        ]),
                )
                .await?;
                Ok(OAuthTokens {
                    access_token: value_text(&long, "access_token")?,
                    refresh_token: None,
                    expires_in: duration(&long, "expires_in")?,
                    refresh_expires_in: None,
                    external_id: value_text(short, "user_id")?,
                    scopes: granted,
                })
            }
            "facebook" => {
                let base = format!(
                    "https://graph.facebook.com/{}",
                    p.graph_version.as_deref().ok_or(OAuthError::Invalid)?
                );
                let short = response(http.get(format!("{base}/oauth/access_token")).query(&[
                    ("client_id", p.client_id.as_str()),
                    ("client_secret", secret.as_str()),
                    ("redirect_uri", p.redirect_uri.as_str()),
                    ("code", code),
                ]))
                .await?;
                let short_token = value_text(&short, "access_token")?;
                let long = response(http.get(format!("{base}/oauth/access_token")).query(&[
                    ("grant_type", "fb_exchange_token"),
                    ("client_id", p.client_id.as_str()),
                    ("client_secret", secret.as_str()),
                    ("fb_exchange_token", short_token.as_str()),
                ]))
                .await?;
                let token = value_text(&long, "access_token")?;
                let me = response(
                    http.get(format!("{base}/me"))
                        .query(&[("fields", "id")])
                        .bearer_auth(&token),
                )
                .await?;
                let permissions = response(
                    http.get(format!("{base}/me/permissions"))
                        .bearer_auth(&token),
                )
                .await?;
                let granted: Vec<String> = permissions["data"]
                    .as_array()
                    .ok_or(OAuthError::Provider)?
                    .iter()
                    .filter(|v| v["status"] == "granted")
                    .filter_map(|v| v["permission"].as_str().map(str::to_owned))
                    .collect();
                ensure_scopes(p, &granted)?;
                Ok(OAuthTokens {
                    access_token: token,
                    refresh_token: None,
                    expires_in: duration(&long, "expires_in")?,
                    refresh_expires_in: None,
                    external_id: value_text(&me, "id")?,
                    scopes: granted,
                })
            }
            _ => Err(OAuthError::Invalid),
        }
    }
    async fn refresh(
        &self,
        p: &OAuthProfile,
        tokens: &OAuthTokens,
    ) -> Result<OAuthTokens, OAuthError> {
        validate_profile(p, self.simulation)?;
        match p.provider.as_str() {
            "simulated" => {
                let mut next = simulated_tokens(p, &random_secret()?);
                next.external_id = tokens.external_id.clone();
                Ok(next)
            }
            "tiktok" => {
                let secret = secret(p)?;
                let refresh = tokens
                    .refresh_token
                    .as_deref()
                    .ok_or(OAuthError::Conflict)?;
                let v = response(
                    client()?
                        .post("https://open.tiktokapis.com/v2/oauth/token/")
                        .form(&[
                            ("client_key", p.client_id.as_str()),
                            ("client_secret", secret.as_str()),
                            ("grant_type", "refresh_token"),
                            ("refresh_token", refresh),
                        ]),
                )
                .await?;
                let next = parse_tiktok(p, &v)?;
                if next.external_id != tokens.external_id {
                    return Err(OAuthError::Provider);
                }
                Ok(next)
            }
            "instagram" => {
                let v = response(
                    client()?
                        .get("https://graph.instagram.com/refresh_access_token")
                        .query(&[
                            ("grant_type", "ig_refresh_token"),
                            ("access_token", tokens.access_token.as_str()),
                        ]),
                )
                .await?;
                Ok(OAuthTokens {
                    access_token: value_text(&v, "access_token")?,
                    refresh_token: None,
                    expires_in: duration(&v, "expires_in")?,
                    refresh_expires_in: None,
                    external_id: tokens.external_id.clone(),
                    scopes: tokens.scopes.clone(),
                })
            }
            _ => Err(OAuthError::Conflict),
        }
    }
    async fn revoke(&self, p: &OAuthProfile, tokens: &OAuthTokens) -> Result<(), OAuthError> {
        validate_profile(p, self.simulation)?;
        match p.provider.as_str() {
            "simulated" => Ok(()),
            "tiktok" => {
                let secret = secret(p)?;
                response(
                    client()?
                        .post("https://open.tiktokapis.com/v2/oauth/revoke/")
                        .form(&[
                            ("client_key", p.client_id.as_str()),
                            ("client_secret", secret.as_str()),
                            ("token", tokens.access_token.as_str()),
                        ]),
                )
                .await?;
                Ok(())
            }
            "facebook" => {
                let url = format!(
                    "https://graph.facebook.com/{}/me/permissions",
                    p.graph_version.as_deref().ok_or(OAuthError::Invalid)?
                );
                let v = response(client()?.delete(url).bearer_auth(&tokens.access_token)).await?;
                if v == json!(true) || v["success"] == true {
                    Ok(())
                } else {
                    Err(OAuthError::Provider)
                }
            }
            // Meta's Instagram revocation contract could not be verified; disable local access and leave a visible pending revocation.
            "instagram" => Err(OAuthError::Provider),
            _ => Err(OAuthError::Invalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(provider: &str) -> OAuthProfile {
        OAuthProfile {
            provider: provider.into(),
            client_id: "fixture-client".into(),
            client_secret_env: Some("PATH".into()),
            redirect_uri: format!("https://app.example/v1/oauth/{provider}/callback"),
            scopes: vec!["fixture.scope".into()],
            graph_version: Some("v27.0".into()),
            pkce: false,
        }
    }
    #[test]
    fn pkce_matches_rfc7636_vector() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
    #[test]
    fn provider_authorization_urls_are_fixed_and_do_not_contain_client_secrets() {
        let provider = OfficialOAuthProvider { simulation: false };
        for (name, host, idkey) in [
            ("facebook", "www.facebook.com", "client_id"),
            ("instagram", "www.instagram.com", "client_id"),
            ("tiktok", "www.tiktok.com", "client_key"),
        ] {
            let url = reqwest::Url::parse(
                &provider
                    .authorize_url(&fixture(name), "fixture-state", None)
                    .expect("authorization URL"),
            )
            .expect("URL");
            assert_eq!(url.host_str(), Some(host));
            let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
            assert_eq!(
                params.get(idkey).map(|v| v.as_ref()),
                Some("fixture-client")
            );
            assert_eq!(
                params.get("state").map(|v| v.as_ref()),
                Some("fixture-state")
            );
            assert!(!params.contains_key("client_secret"));
        }
    }
    #[test]
    fn tiktok_requires_granted_scopes_and_rotatable_refresh_credentials() {
        let p = fixture("tiktok");
        let mut response = json!({"access_token":"fixture-access","refresh_token":"new-refresh","expires_in":7200,"refresh_expires_in":86400,"open_id":"fixture-user","scope":"fixture.scope"});
        let tokens = parse_tiktok(&p, &response).expect("token contract");
        assert_eq!(tokens.refresh_token.as_deref(), Some("new-refresh"));
        response["scope"] = json!("other.scope");
        assert!(parse_tiktok(&p, &response).is_err());
        response["scope"] = json!("fixture.scope");
        response["expires_in"] = json!(-1);
        assert!(parse_tiktok(&p, &response).is_err());
    }
    #[test]
    fn redirect_and_unsupported_pkce_are_rejected() {
        let mut p = fixture("tiktok");
        p.redirect_uri = "https://attacker.example/callback?tenant=123".into();
        assert!(validate_profile(&p, false).is_err());
        p = fixture("facebook");
        p.pkce = true;
        assert!(validate_profile(&p, false).is_err());
        assert!(validate_profile(&fixture("simulated"), false).is_err());
    }
}
