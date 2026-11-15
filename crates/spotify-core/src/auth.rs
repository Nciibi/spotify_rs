//! OAuth 2.0 Authorization Code flow with PKCE.
//!
//! The flow runs a throwaway HTTP listener on `127.0.0.1`, opens Spotify's
//! authorize page in the user's browser, and exchanges the returned code for a
//! token set that is persisted to disk for later sessions.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::api::{ApiError, TokenSet, SCOPES, TOKEN_URL};

/// Default client id: the one the Spotify Desktop app uses.
///
/// Override it with `SPOTIFY_RS_CLIENT_ID` to use your own registered app.
pub fn client_id() -> String {
    crate::api::client_id()
}

/// Redirect URI to use during sign-in.
///
/// Override it with `SPOTIFY_RS_REDIRECT_URI`.
pub fn redirect_uri() -> String {
    crate::api::redirect_uri()
}

/// Redirect URI that pins an arbitrary loopback port.
pub const LOOPBACK_REDIRECT_URI: &str = "http://127.0.0.1:{port}/login";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("could not bind the local callback listener: {0}")]
    Bind(#[source] std::io::Error),
    #[error("io error while handling the oauth callback: {0}")]
    Io(#[from] std::io::Error),
    #[error("oauth exchange failed: {0}")]
    Exchange(String),
    #[error("no stored credentials; please sign in first")]
    NotAuthenticated,
    #[error("could not read or write the credential file: {0}")]
    Storage(String),
}

/// Where the token set is persisted between runs.
pub fn token_path() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(|v| PathBuf::from(v).join("spotify-rs"))
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("spotify-rs"))
            })
            .unwrap_or_else(|| PathBuf::from("."))
    };
    base.join("spotify-rs").join("tokens.json")
}

/// Load a previously stored token set, if any.
pub fn load_tokens() -> Result<Option<TokenSet>, AuthError> {
    let path = token_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let tokens: TokenSet =
                serde_json::from_str(&raw).map_err(|e| AuthError::Storage(e.to_string()))?;
            Ok(Some(tokens))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AuthError::Storage(err.to_string())),
    }
}

/// Persist a token set so the next launch does not require signing in again.
pub fn save_tokens(tokens: &TokenSet) -> Result<(), AuthError> {
    let path = token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AuthError::Storage(e.to_string()))?;
    }
    let raw = serde_json::to_string_pretty(tokens)
        .map_err(|e| AuthError::Storage(e.to_string()))?;
    std::fs::write(&path, raw).map_err(|e| AuthError::Storage(e.to_string()))?;
    Ok(())
}

/// Remove any stored credentials.
pub fn clear_tokens() -> Result<(), AuthError> {
    let path = token_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AuthError::Storage(err.to_string())),
    }
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    for byte in buf.iter_mut() {
        *byte = rand::random::<u8>();
    }
    buf
}

/// The PKCE verifier/challenge pair.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> Self {
        let verifier = URL_SAFE_NO_PAD.encode(random_bytes::<64>());
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self { verifier, challenge }
    }
}

/// An in-flight authorization request.
pub struct LoginFlow {
    /// The URL the user must open to grant access.
    pub auth_url: String,
    /// The redirect URI that was registered for this request.
    pub redirect_uri: String,
    listener: TcpListener,
    pkce: Pkce,
    client_id: String,
}

impl LoginFlow {
    /// Start the flow: bind the callback listener and build the authorize URL.
    ///
    /// The redirect URI registered for the desktop client id is
    /// `http://127.0.0.1/login`, so port 80 is preferred. When that port is
    /// unavailable the flow falls back to an ephemeral port, which Spotify also
    /// accepts for this client id.
    pub async fn start() -> Result<Self, AuthError> {
        let client_id = client_id();
        let redirect = redirect_uri();

        match Self::start_with(&client_id, &redirect).await {
            Ok(flow) => Ok(flow),
            Err(_) => Self::start_with(&client_id, LOOPBACK_REDIRECT_URI).await,
        }
    }

    /// Start the flow against an explicit client id and redirect URI.
    ///
    /// `redirect_uri` may contain `{port}`, which is replaced with the bound
    /// port.
    pub async fn start_with(
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<Self, AuthError> {
        // A `{port}` placeholder means "bind any free port".
        let (listener, redirect_uri) = if redirect_uri.contains("{port}") {
            let listener = TcpListener::bind("127.0.0.1:0").await.map_err(AuthError::Bind)?;
            let port = listener.local_addr().map_err(AuthError::Bind)?.port();
            (listener, redirect_uri.replace("{port}", &port.to_string()))
        } else {
            // The URI pins the port, so bind exactly that one.
            let port = redirect_uri
                .rsplit(':')
                .next()
                .and_then(|tail| tail.split('/').next())
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(80);
            let listener = TcpListener::bind(("127.0.0.1", port))
                .await
                .map_err(AuthError::Bind)?;
            (listener, redirect_uri.to_string())
        };

        let pkce = Pkce::new();
        let scope = SCOPES.join(" ");

        let auth_url = format!(
            "https://accounts.spotify.com/authorize\
             ?response_type=code\
             &client_id={client_id}\
             &scope={scope}\
             &redirect_uri={redirect_uri}\
             &code_challenge_method=S256\
             &code_challenge={challenge}\
             &state={state}",
            scope = urlencode(&scope),
            redirect_uri = urlencode(&redirect_uri),
            challenge = urlencode(&pkce.challenge),
            state = urlencode(&random_bytes::<8>().iter().map(|b| format!("{b:02x}")).collect::<String>()),
        );

        Ok(Self {
            auth_url,
            redirect_uri,
            listener,
            pkce,
            client_id: client_id.to_string(),
        })
    }

    /// Open the authorize page in the user's default browser.
    pub fn open_browser(&self) {
        // `rundll32` hands the URL straight to the shell's protocol handler, so
        // the `&` characters in the query string need no extra escaping.
        let _ = std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", &self.auth_url])
            .spawn();
    }

    /// Wait for the browser to redirect back, then exchange the code for tokens.
    pub async fn complete(mut self) -> Result<TokenSet, AuthError> {
        let code = self.wait_for_code().await?;
        self.exchange(code).await
    }

    async fn wait_for_code(&mut self) -> Result<String, AuthError> {
        let (mut stream, _) = self.listener.accept().await?;

        let mut buffer = vec![0u8; 8192];
        let read = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..read]).to_string();

        let target = request
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_string();

        let code = query_param(&target, "code").ok_or_else(|| {
            AuthError::Exchange(format!(
                "the callback did not carry an authorization code ({target})"
            ))
        })?;

        let body = "<!doctype html><html><head><meta charset=\"utf-8\">\
            <title>spotify-rs</title></head><body style=\"font-family:system-ui;\
            background:#121212;color:#eee;display:grid;place-items:center;height:100vh\">\
            <div style=\"text-align:center\"><h1>Signed in</h1>\
            <p>You can close this tab and return to spotify-rs.</p></div></body></html>";

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\n\
             content-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.flush().await;

        Ok(code)
    }

    async fn exchange(self, code: String) -> Result<TokenSet, AuthError> {
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("client_id", self.client_id.as_str()),
            ("code_verifier", self.pkce.verifier.as_str()),
        ];

        let http = reqwest::Client::new();
        let response = http
            .post(TOKEN_URL)
            .form(&form)
            .send()
            .await
            .map_err(|e| AuthError::Exchange(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AuthError::Exchange(e.to_string()))?;

        if !status.is_success() {
            return Err(AuthError::Exchange(format!(
                "spotify answered {status}: {body}"
            )));
        }

        let value: Value =
            serde_json::from_str(&body).map_err(|e| AuthError::Exchange(e.to_string()))?;

        let access_token = value["access_token"]
            .as_str()
            .ok_or_else(|| AuthError::Exchange("no access_token in response".into()))?
            .to_string();
        let refresh_token = value["refresh_token"].as_str().unwrap_or("").to_string();
        let expires_in = value["expires_in"].as_u64().unwrap_or(3600);
        let scope = value["scope"].as_str().map(str::to_string);

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            + expires_in;

        Ok(TokenSet {
            access_token,
            refresh_token,
            expires_at,
            scope,
        })
    }
}

/// Read the stored token set, refreshing it when it has expired.
pub async fn tokens_or_refresh(
    client: &crate::api::ApiClient,
) -> Result<TokenSet, ApiError> {
    let stored = load_tokens().map_err(|e| ApiError::Auth(e.to_string()))?;
    let Some(mut tokens) = stored else {
        return Err(ApiError::Auth("not signed in".into()));
    };

    if tokens.is_expired() && !tokens.refresh_token.is_empty() {
        client.set_tokens(tokens.clone()).await;
        client.refresh().await?;
        tokens = client.tokens().await;
        let _ = save_tokens(&tokens);
    }

    Ok(tokens)
}

/// Minimal percent-encoding for query string values.
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Pull a single query parameter out of a URL or request target.
fn query_param(target: &str, key: &str) -> Option<String> {
    let query = target.split('?').nth(1)?;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            return Some(percent_decode(value));
        }
    }
    None
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_query_parameter() {
        assert_eq!(
            query_param("/login?code=abc123&state=xyz", "code"),
            Some("abc123".to_string())
        );
        assert_eq!(query_param("/login?state=xyz", "code"), None);
    }

    #[test]
    fn encodes_scopes() {
        assert_eq!(urlencode("a b"), "a%20b");
        assert_eq!(urlencode("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn pkce_challenge_is_stable() {
        let pkce = Pkce::new();
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(pkce.verifier.as_bytes()));
        assert_eq!(pkce.challenge, expected);
    }
}
