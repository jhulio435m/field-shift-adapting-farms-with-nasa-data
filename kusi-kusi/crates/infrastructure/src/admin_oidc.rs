use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreJwsSigningAlgorithm, CoreProviderMetadata},
};
use std::time::Duration;

type ConfiguredClient = CoreClient<
    openidconnect::EndpointSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointMaybeSet,
    openidconnect::EndpointMaybeSet,
>;

pub struct OidcLoginRequest {
    pub url: String,
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
}

pub struct VerifiedOidcIdentity {
    pub issuer: String,
    pub subject: String,
}

pub struct OidcProvider {
    issuer: String,
    client: ConfiguredClient,
    http: openidconnect::reqwest::Client,
}

impl OidcProvider {
    pub async fn load(local_mode: bool) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let keys = [
            "KUSI_ADMIN_OIDC_ISSUER",
            "KUSI_ADMIN_OIDC_CLIENT_ID",
            "KUSI_ADMIN_OIDC_CLIENT_SECRET",
            "KUSI_ADMIN_OIDC_REDIRECT_URI",
        ];
        let configured = keys.iter().filter(|key| std::env::var(key).is_ok()).count();
        if local_mode && configured > 0 {
            return Err("OIDC must remain disabled in local mode".into());
        }
        if configured == 0 {
            return Ok(None);
        }
        if configured != keys.len() {
            return Err("OIDC configuration is incomplete".into());
        }
        let issuer = std::env::var("KUSI_ADMIN_OIDC_ISSUER")?;
        let client_id = std::env::var("KUSI_ADMIN_OIDC_CLIENT_ID")?;
        let client_secret = std::env::var("KUSI_ADMIN_OIDC_CLIENT_SECRET")?;
        let redirect_uri = std::env::var("KUSI_ADMIN_OIDC_REDIRECT_URI")?;
        validate_https_or_loopback(&issuer)?;
        validate_https_or_loopback(&redirect_uri)?;
        let issuer_url = IssuerUrl::new(issuer.clone())?;
        let http = openidconnect::reqwest::ClientBuilder::new()
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(8))
            .build()?;
        let metadata = CoreProviderMetadata::discover_async(issuer_url, &http).await?;
        if !metadata
            .id_token_signing_alg_values_supported()
            .contains(&CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256)
        {
            return Err("OIDC provider must support RS256 ID token signing".into());
        }
        let metadata = metadata.set_id_token_signing_alg_values_supported(vec![
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        ]);
        let client = CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri)?);
        Ok(Some(Self {
            issuer,
            client,
            http,
        }))
    }

    pub fn begin(&self) -> OidcLoginRequest {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_owned()))
            .set_pkce_challenge(challenge)
            .url();
        OidcLoginRequest {
            url: url.to_string(),
            state: state.secret().to_owned(),
            nonce: nonce.secret().to_owned(),
            code_verifier: verifier.secret().to_owned(),
        }
    }

    pub async fn complete(
        &self,
        code: String,
        code_verifier: String,
        nonce: String,
    ) -> Result<VerifiedOidcIdentity, Box<dyn std::error::Error>> {
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code))?
            .set_pkce_verifier(PkceCodeVerifier::new(code_verifier))
            .request_async(&self.http)
            .await?;
        let id_token = token_response
            .id_token()
            .ok_or("OIDC token response has no ID token")?;
        let claims = id_token.claims(&self.client.id_token_verifier(), &Nonce::new(nonce))?;
        let subject = claims.subject().as_str().to_owned();
        if subject.is_empty() || subject.len() > 255 {
            return Err("OIDC subject is outside the supported length".into());
        }
        Ok(VerifiedOidcIdentity {
            issuer: self.issuer.clone(),
            subject,
        })
    }
}

fn validate_https_or_loopback(raw: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = url::Url::parse(raw)?;
    let host = parsed.host_str().unwrap_or_default();
    let loopback = host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1";
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || parsed.query().is_some()
        || !(parsed.scheme() == "https" || (parsed.scheme() == "http" && loopback))
    {
        return Err("OIDC issuer and callback require HTTPS except on loopback".into());
    }
    Ok(())
}
