use log::{debug, info, warn};
use openidconnect::{IssuerUrl, core::{self, CoreIdTokenClaims, CoreProviderMetadata, CoreResponseType}, ClientId, ClientSecret, RedirectUrl, reqwest::async_http_client, AuthenticationFlow, CsrfToken, Nonce, Scope, AuthorizationCode, TokenResponse, OAuth2TokenResponse};
use rocket_airlock::{Airlock, Communicator, Hatch};
use rocket::{config::{Config, ConfigError, Value}, Route, http::{Cookie, Cookies, SameSite, uri::{Absolute, Uri}, ext::IntoOwned, Status}, response::{Debug, Redirect}, debug_, info_, log_, warn_, yansi::Paint, request::{Outcome, FromRequest}, Request};
use std::ops::{Deref, DerefMut};
use anyhow::{anyhow, Error};

pub struct CoreClient(core::CoreClient);

impl Deref for CoreClient {
    type Target = core::CoreClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CoreClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[rocket::async_trait]
impl Communicator for CoreClient {
    async fn from_config(config: &Config) -> Result<Self, ConfigError> {
        let name = OidcHatch::name().replace(" ", "").to_lowercase();
        let airlocks = config.get_table("airlock")?;
        let hatch = airlocks
            .get(&name)
            .ok_or_else(|| ConfigError::Missing(name.to_string()))?;

        let client_id = try_get_string(hatch, "client_id")?;
        let client_secret = try_get_string(hatch, "client_secret")?;

        let redirect_url = try_get_absolute_url(hatch, "redirect_url", &config.address, config.port)?;
        let discover_url = try_get_absolute_url(hatch, "discover_url", &config.address, config.port)?;

        let issuer_url = IssuerUrl::new(discover_url.to_string())
            .expect("Invalid issuer Url");

        let redirect_url = RedirectUrl::new(redirect_url.to_string())
            .expect("Invalid redirect Url");

        info_!("Fetching OpenID Connect discover manifest at: {}", Paint::new(discover_url.to_string()).underline());
        // Fetch OpenID Connect discovery document.
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
            .await
            .map_err(|e| ConfigError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, e), "OidcHatch.client"))?;

        info_!("Initializing OpenID Client");
        // Set up the config for the auth process.
        let client = core::CoreClient::from_provider_metadata(
                provider_metadata,
                ClientId::new(client_id.to_string()),
                Some(ClientSecret::new(client_secret.to_string())),
            )
            .set_redirect_uri(redirect_url);

        Ok(CoreClient(client))
    }
}

#[allow(dead_code)]
pub struct OidcHatch<'h> {
    pub(crate) discover_url: Absolute<'h>,
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) redirect_url: Absolute<'h>,
    pub(crate) client: Option<CoreClient>,
}

impl<'h> OidcHatch<'static> {
    pub fn authorize_url(&self) -> (Absolute<'static>, String, String) {
        info_!("Generating authorization Url from Manifest with random token and nonce.");
        // Generate the authorization URL to which we'll redirect the user.
        let (authorize_url, csrf_state, nonce) = self.comm()
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("profile".to_string()))
            .url();

        let authorize_url = Absolute::parse(authorize_url.as_ref())
            .expect("Valid Url");

        debug_!("Generated redirect authorization url: {}",
            Paint::new(format!("{}://{}",
                authorize_url.scheme(),
                authorize_url.authority().expect("Came from a valid Url"))
            ).underline()
        );
        (authorize_url.into_owned(), csrf_state.secret().to_string(), nonce.secret().to_string())
    }

    pub async fn exchange_token(&self, auth_response: &AuthenticationResponse) -> Result<ClaimResponse, Error>{
        let token_request = self.comm()
            .exchange_code(AuthorizationCode::new(auth_response.code.to_string()));

        let token_response = token_request
            .request_async(async_http_client)
            .await?;

        let claims = token_response.id_token()
            .ok_or_else(|| anyhow!("No ID token found. Authorization Server seems to only speak OAuth2"))?
            .claims(&self.comm().id_token_verifier(), &Nonce::new(auth_response.nonce.to_string()))?;

        Ok(ClaimResponse {
            access_token: token_response.access_token().secret().to_string(),
            claims: claims.to_owned()
        })
    }

    pub fn validate_access_token(&self, access_token: &str) -> bool {
        // Normally you would use self.comm() to communicate with the OpenID Provider and
        // validate the token incl. Session Management, as per https://openid.net/specs/openid-connect-session-1_0.html.
        // But that is currently not implemented in openidconnect-rs.
        true
    }
}

#[rocket::async_trait]
impl<'h> Hatch for OidcHatch<'static> {
    type Comm = CoreClient;

    fn comm(&self) -> &CoreClient {
        self.client.as_ref().expect("Communicator should have been connected")
    }

    fn connect_comm(&mut self, comm: Self::Comm) {
        self.client = Some(comm);
    }

    fn name() -> &'static str {
        "OpenID Connect"
    }

    fn routes() -> Vec<Route> {
        rocket::routes![login, login_callback]
    }

    async fn from_config(config: &Config) -> Result<OidcHatch<'static>, ConfigError> {
        let name = OidcHatch::name().replace(" ", "").to_lowercase();
        let airlocks = config.get_table("airlock")?;
        let hatch = airlocks
            .get(&name)
            .ok_or_else(|| ConfigError::Missing(name.to_string()))?;

        let redirect_url = try_get_absolute_url(hatch, "redirect_url", &config.address, config.port)?;
        let discover_url = try_get_absolute_url(hatch, "discover_url", &config.address, config.port)?;

        let client_id = try_get_string(hatch, "client_id")?;
        let client_secret = try_get_string(hatch, "client_secret")?;

        let oidc_hatch = OidcHatch {
                client_id,
                client_secret,
                discover_url,
                redirect_url,
                client: None
            };

        Ok(oidc_hatch)
    }
}

fn try_get_string(value: &Value, key: &str) -> Result<String, ConfigError> {
    let key_value = value
            .get(key)
            .ok_or_else(|| ConfigError::Missing(key.to_string()))?;

    Ok(key_value
        .as_str()
        .ok_or_else(|| ConfigError::BadType(key.to_string(), "string", key_value.type_str(), None))?
        .to_string()
    )
}

fn try_get_absolute_url<'h>(value: &Value, key: &str, address: &str, port: u16) -> Result<Absolute<'h>, ConfigError> {
    let redirect_url = try_get_string(value, key)?;
    match Uri::parse(&redirect_url) {
        Ok(Uri::Absolute(absolute)) => Ok(absolute.into_owned()),
        Ok(Uri::Origin(origin)) => Absolute::parse(&format!("http://{}:{}{}", address, port, &origin))
            .map_err(|e| ConfigError::BadType(format!("{} - Got: {}", e, &format!("{}:{}{}", address, port, &origin)), "", "Tried Origin", None))
            .map(|uri| uri.into_owned()),
        Ok(Uri::Authority(authority)) => Absolute::parse(&format!("{}", &authority))
            .map_err(|e| ConfigError::BadType(format!("{} - Got: {}", e, &format!("{}", &authority)), "", "Tried Authority", None))
            .map(|uri| uri.into_owned()),
        Ok(Uri::Asterisk) => Err(ConfigError::BadType(redirect_url.to_string(), "Expected 'Uri'", "Asterisk is not a valid redirect Url", None)),
        Err(e) => Err(ConfigError::BadType(format!("{} - Got: {}", e, redirect_url.to_string()), "", "", None)),
    }
}

#[rocket::get("/login", rank = 2)]
pub fn login(airlock: Airlock<OidcHatch<'static>>, mut cookies: Cookies<'_>) -> Redirect {
    let (authorize_url, csrf_state, nonce) = airlock.hatch.authorize_url();
    cookies.add_private(
        Cookie::build("oicd_state", csrf_state)
            .same_site(SameSite::Lax)
            .finish(),
    );
    cookies.add_private(
        Cookie::build("oicd_nonce", nonce)
            .same_site(SameSite::Lax)
            .finish(),
    );

    info_!("Redirecting to {}", Paint::new(&authorize_url).underline());
    Redirect::to(authorize_url)
}

#[rocket::get("/login")]
pub(crate) async fn login_callback(airlock: Airlock<OidcHatch<'static>>, auth_response: AuthenticationResponse, mut cookies: Cookies<'_>) -> Result<Redirect, Debug<Error>> {
    debug_!("[login_callback] Returned code: {}", &auth_response.code);

    // Is part of the OpenID Connect Session Management specification: https://openid.net/specs/openid-connect-session-1_0.html
    // TODO: impl session management
    let _ = auth_response.session_state;

    // Use the token to retrieve the user's information.
    let claim_resonse = airlock.hatch.exchange_token(&auth_response)
        .await?;

    // Set a private cookie with the user's name, and redirect to the home page.
    cookies.add_private(
        Cookie::build("username", claim_resonse.claims.preferred_username().unwrap().to_string())
            .same_site(SameSite::Lax)
            .finish(),
    );
    cookies.add_private(
        Cookie::build("oicd_access_token_hash", claim_resonse.access_token)
            .same_site(SameSite::Lax)
            .finish(),
    );

    Ok(Redirect::to("/"))
}

pub struct ClaimResponse {
    access_token: String,
    claims: CoreIdTokenClaims,
}

pub struct AuthenticationResponse {
    code: String,
    nonce: String,
    session_state: String
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for AuthenticationResponse {
    type Error = ();

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let code = request.get_query_value("code")
            .and_then(|code| code.ok());
        let state: Option<String> = request.get_query_value("state")
            .and_then(|state| state.ok());
        let session_state = request.get_query_value("session_state")
            .and_then(|session_state| session_state.ok());

        let auth_response = match (code, state, session_state) {
            (Some(code), Some(state), Some(session_state)) => {
                let mut cookies = request.cookies();

                let state_cookie = cookies.get_private("oicd_state");
                match state_cookie {
                    Some(stored_state) if stored_state.value().to_string() == state => {
                        cookies.remove(stored_state.clone());
                    },
                    other => {
                        if other.is_some() {
                            warn_!("The stored state differs from the state returned from the OpenID Provider.");
                        }
                        return Outcome::Failure((Status::BadRequest, ()))
                    }
                }

                let nonce_cookie = cookies.get_private("oicd_nonce");
                let nonce = match nonce_cookie {
                    Some(stored_nonce) => {
                        cookies.remove(stored_nonce.clone());
                        stored_nonce.value().to_string()
                    },
                    _ => {
                        warn_!("No nonce was stored for the current auth flow.");
                        return Outcome::Failure((Status::BadRequest, ()))
                    }
                };

                AuthenticationResponse {
                    code,
                    nonce,
                    session_state
                }
            },
            _ => {
                info_!("Either 'code', 'state' or 'session_state' was missing on the providers response.");
                return Outcome::Forward(());
            }
        };

        Outcome::Success(auth_response)
    }
}
