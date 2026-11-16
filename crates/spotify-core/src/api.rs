//! Async client for the public Spotify Web API.
//!
//! The client owns the OAuth token set and transparently refreshes an expired
//! access token before a request, retrying once if the server answers `401`.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::models::*;

/// How many times a rate-limited request is retried before giving up.
const MAX_RATE_LIMIT_RETRIES: u32 = 6;

/// The longest pause taken between retries.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Base URL of the versioned Web API.
pub const API_BASE: &str = "https://api.spotify.com/v1";

/// Endpoint used to exchange and refresh OAuth tokens.
pub const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";

/// The Spotify Desktop app's OAuth client id. Its registered redirect URI is
/// `http://127.0.0.1/login`, which is what [`crate::auth`] relies on.
pub const DESKTOP_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

/// The redirect URI registered for [`DESKTOP_CLIENT_ID`].
pub const DESKTOP_REDIRECT_URI: &str = "http://127.0.0.1/login";

/// Client id to authenticate with.
///
/// The desktop client id is shared by every third-party client, so Spotify
/// throttles it aggressively. Register your own app in the Spotify developer
/// dashboard and set `SPOTIFY_RS_CLIENT_ID` to get a dedicated quota.
pub fn client_id() -> String {
    std::env::var("SPOTIFY_RS_CLIENT_ID").unwrap_or_else(|_| DESKTOP_CLIENT_ID.to_string())
}

/// Client secret used to mint a client token.
///
/// Only needed for your own registered app; set `SPOTIFY_RS_CLIENT_SECRET`.
pub fn client_secret() -> Option<String> {
    std::env::var("SPOTIFY_RS_CLIENT_SECRET").ok()
}

/// Redirect URI to use during sign-in.
///
/// Must match a redirect URI registered for [`client_id`].
pub fn redirect_uri() -> String {
    std::env::var("SPOTIFY_RS_REDIRECT_URI").unwrap_or_else(|_| DESKTOP_REDIRECT_URI.to_string())
}

/// Endpoint that issues client tokens.
pub const CLIENT_TOKEN_URL: &str = "https://clienttoken.spotify.com/v1/clienttoken";

/// Scopes the Spotify Desktop app itself requests.
pub const SCOPES: &[&str] = &[
    "app-remote-control",
    "playlist-modify",
    "playlist-modify-private",
    "playlist-modify-public",
    "playlist-read",
    "playlist-read-collaborative",
    "playlist-read-private",
    "streaming",
    "ugc-image-upload",
    "user-follow-modify",
    "user-follow-read",
    "user-library-modify",
    "user-library-read",
    "user-modify",
    "user-modify-playback-state",
    "user-modify-private",
    "user-personalized",
    "user-read-birthdate",
    "user-read-currently-playing",
    "user-read-email",
    "user-read-play-history",
    "user-read-playback-position",
    "user-read-playback-state",
    "user-read-private",
    "user-read-recently-played",
    "user-top-read",
];

/// A stored OAuth token set.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix timestamp (seconds) at which the access token stops working.
    pub expires_at: u64,
    #[serde(default)]
    pub scope: Option<String>,
}

impl TokenSet {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Treat the token as expired slightly early to avoid racing the server.
        self.expires_at <= now + 30
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("spotify api returned {status}: {body}")]
    Status { status: StatusCode, body: String },
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("could not decode response: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("could not build request: {0}")]
    Url(#[from] url::ParseError),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("spotify kept rate limiting this request")]
    RateLimited,
}

/// A client for the Spotify Web API.
#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base: Arc<str>,
    client_id: Arc<str>,
    tokens: Arc<RwLock<TokenSet>>,
    client_token: Arc<RwLock<Option<String>>>,
}

impl ApiClient {
    pub fn new(tokens: TokenSet, client_id: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("spotify-rs/0.1.0")
            .build()
            .expect("failed to build http client");

        Self {
            http,
            base: Arc::from(API_BASE),
            client_id: Arc::from(client_id.into()),
            tokens: Arc::new(RwLock::new(tokens)),
            client_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Mint a client token, which lifts the Web API rate limit.
    ///
    /// Requires a client secret, so it only works for an app registered in the
    /// Spotify developer dashboard. The token is cached for the lifetime of the
    /// client.
    pub async fn client_token(&self) -> Result<String, ApiError> {
        if let Some(token) = self.client_token.read().await.clone() {
            return Ok(token);
        }

        let Some(secret) = client_secret() else {
            return Err(ApiError::Auth(
                "no client secret configured; set SPOTIFY_RS_CLIENT_SECRET".into(),
            ));
        };

        let body = serde_json::json!({
            "client_id": self.client_id.as_ref(),
            "client_secret": secret,
            "grant_type": "client_credentials",
        });

        let response = self.http.post(CLIENT_TOKEN_URL).json(&body).send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(ApiError::Auth(format!(
                "client token request rejected with {status}: {text}"
            )));
        }

        let value: Value = serde_json::from_str(&text)?;
        let token = value["client_token"]
            .as_str()
            .ok_or_else(|| ApiError::Auth("client token response was empty".into()))?
            .to_string();

        *self.client_token.write().await = Some(token.clone());
        Ok(token)
    }

    pub async fn tokens(&self) -> TokenSet {
        self.tokens.read().await.clone()
    }

    /// Replace the stored token set, e.g. after an out-of-band refresh.
    pub async fn set_tokens(&self, tokens: TokenSet) {
        *self.tokens.write().await = tokens;
    }

    async fn access_token(&self) -> Result<String, ApiError> {
        {
            let guard = self.tokens.read().await;
            if !guard.is_expired() {
                return Ok(guard.access_token.clone());
            }
        }
        self.refresh().await
    }

    /// Exchange the stored refresh token for a fresh access token.
    pub async fn refresh(&self) -> Result<String, ApiError> {
        let refresh_token = self.tokens.read().await.refresh_token.clone();
        if refresh_token.is_empty() {
            return Err(ApiError::Auth("no refresh token available".into()));
        }

        let form = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", self.client_id.as_ref()),
        ];

        let response = self.http.post(TOKEN_URL).form(&form).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ApiError::Auth(format!(
                "token refresh rejected with {status}: {body}"
            )));
        }

        // Spotify's token endpoint answers with a flat object.
        let value: Value = serde_json::from_str(&body)?;
        let access_token = value["access_token"]
            .as_str()
            .ok_or_else(|| ApiError::Auth("token response missing access_token".into()))?
            .to_string();
        let expires_in = value["expires_in"].as_u64().unwrap_or(3600);
        let new_refresh = value["refresh_token"]
            .as_str()
            .unwrap_or(&refresh_token)
            .to_string();
        let scope = value["scope"].as_str().map(str::to_string);
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            + expires_in;

        let mut guard = self.tokens.write().await;
        guard.access_token = access_token.clone();
        guard.refresh_token = new_refresh;
        guard.expires_at = expires_at;
        if scope.is_some() {
            guard.scope = scope;
        }

        Ok(access_token)
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<Value, ApiError> {
        let url = format!("{}{}", self.base, path);

        // First attempt; retry once after refreshing if the token was stale.
        for attempt in 0..2 {
            let token = self.access_token().await?;

            let mut delay = Duration::from_secs(2);
            let mut response = None;

            // The shared desktop client id hits Spotify's rate limit often, so
            // back off and try again instead of surfacing the error.
            for _ in 0..MAX_RATE_LIMIT_RETRIES {
                let mut request = self
                    .http
                    .request(method.clone(), &url)
                    .bearer_auth(&token)
                    .query(query);

                // A client token lifts the per-client rate limit.
                if let Some(client_token) = self.client_token.read().await.clone() {
                    request = request.header("client-token", client_token);
                }

                if let Some(body) = &body {
                    request = request.json(body);
                }

                let result = request.send().await?;
                let status = result.status();

                if status == StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = result
                        .headers()
                        .get("retry-after")
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse::<u64>().ok());

                    // Spotify often answers with `Retry-After: 86400` for the
                    // shared desktop client id. Sleeping here only hangs the
                    // UI; give up immediately and let the caller fall back to
                    // its offline/demo content.
                    if retry_after.is_some_and(|seconds| seconds > MAX_BACKOFF.as_secs()) {
                        tracing::warn!(path, ?retry_after, "rate limited for a long time, giving up");
                        return Err(ApiError::RateLimited);
                    }

                    let wait = retry_after.map(Duration::from_secs).unwrap_or(delay);
                    tracing::warn!(path, ?wait, "rate limited by spotify, backing off");
                    tokio::time::sleep(wait).await;
                    delay = (delay * 2).min(MAX_BACKOFF);
                    continue;
                }

                response = Some(result);
                break;
            }

            let Some(response) = response else {
                return Err(ApiError::RateLimited);
            };

            let status = response.status();

            if status == StatusCode::UNAUTHORIZED && attempt == 0 {
                self.refresh().await?;
                continue;
            }

            let text = response.text().await?;
            if !status.is_success() {
                return Err(ApiError::Status { status, body: text });
            }

            // Spotify answers `204 No Content` for some endpoints.
            if text.trim().is_empty() {
                return Ok(Value::Null);
            }

            let mut value: Value = serde_json::from_str(&text)?;
            crate::adblock::strip_ads(&mut value);
            return Ok(value);
        }

        unreachable!("retry loop always returns")
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, ApiError> {
        let value = self.send(Method::GET, path, query, None).await?;
        Ok(serde_json::from_value(value)?)
    }

    // ---- Library -------------------------------------------------------

    pub async fn current_user(&self) -> Result<UserProfile, ApiError> {
        self.get("/me", &[]).await
    }

    pub async fn my_playlists(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Paging<Playlist>, ApiError> {
        self.get(
            "/me/playlists",
            &[("limit", limit.to_string()), ("offset", offset.to_string())],
        )
        .await
    }

    pub async fn saved_tracks(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Paging<SavedTrack>, ApiError> {
        self.get(
            "/me/tracks",
            &[("limit", limit.to_string()), ("offset", offset.to_string())],
        )
        .await
    }

    pub async fn saved_albums(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Paging<SavedAlbum>, ApiError> {
        self.get(
            "/me/albums",
            &[("limit", limit.to_string()), ("offset", offset.to_string())],
        )
        .await
    }

    pub async fn followed_artists(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Paging<Artist>, ApiError> {
        self.get(
            "/me/following",
            &[
                ("type", "artist".to_string()),
                ("limit", limit.to_string()),
                ("offset", offset.to_string()),
            ],
        )
        .await
    }

    pub async fn recently_played(&self, limit: u32) -> Result<Paging<Value>, ApiError> {
        self.get("/me/player/recently-played", &[("limit", limit.to_string())])
            .await
    }

    pub async fn top_tracks(&self, limit: u32) -> Result<Paging<Track>, ApiError> {
        self.get("/me/top/tracks", &[("limit", limit.to_string())])
            .await
    }

    pub async fn top_artists(&self, limit: u32) -> Result<Paging<Artist>, ApiError> {
        self.get("/me/top/artists", &[("limit", limit.to_string())])
            .await
    }

    // ---- Entities ------------------------------------------------------

    pub async fn playlist(&self, id: &str) -> Result<Playlist, ApiError> {
        self.get(&format!("/playlists/{id}"), &[]).await
    }

    pub async fn playlist_tracks(
        &self,
        id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Paging<PlaylistItem>, ApiError> {
        self.get(
            &format!("/playlists/{id}/tracks"),
            &[("limit", limit.to_string()), ("offset", offset.to_string())],
        )
        .await
    }

    pub async fn album(&self, id: &str) -> Result<Album, ApiError> {
        self.get(&format!("/albums/{id}"), &[]).await
    }

    pub async fn artist(&self, id: &str) -> Result<Artist, ApiError> {
        self.get(&format!("/artists/{id}"), &[]).await
    }

    pub async fn track(&self, id: &str) -> Result<Track, ApiError> {
        self.get(&format!("/tracks/{id}"), &[]).await
    }

    // ---- Discovery -----------------------------------------------------

    pub async fn search(
        &self,
        query: &str,
        kinds: &str,
        limit: u32,
    ) -> Result<SearchResult, ApiError> {
        self.get(
            "/search",
            &[
                ("q", query.to_string()),
                ("type", kinds.to_string()),
                ("limit", limit.to_string()),
            ],
        )
        .await
    }

    pub async fn categories(&self, limit: u32) -> Result<CategoryList, ApiError> {
        self.get("/browse/categories", &[("limit", limit.to_string())])
            .await
    }

    pub async fn featured_playlists(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Paging<Playlist>, ApiError> {
        self.get(
            "/browse/featured-playlists",
            &[("limit", limit.to_string()), ("offset", offset.to_string())],
        )
        .await
    }

    pub async fn new_releases(&self, limit: u32) -> Result<Paging<Album>, ApiError> {
        self.get(
            "/browse/new-releases",
            &[("limit", limit.to_string())],
        )
        .await
    }

    pub async fn recommendations(
        &self,
        seed_artists: &[String],
        seed_tracks: &[String],
        limit: u32,
    ) -> Result<Recommendations, ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if !seed_artists.is_empty() {
            query.push(("seed_artists", seed_artists.join(",")));
        }
        if !seed_tracks.is_empty() {
            query.push(("seed_tracks", seed_tracks.join(",")));
        }
        query.push(("limit", limit.to_string()));
        self.get("/recommendations", &query).await
    }

    // ---- Playback / Connect --------------------------------------------

    pub async fn currently_playing(&self) -> Result<CurrentlyPlaying, ApiError> {
        self.get("/me/player/currently-playing", &[]).await
    }

    pub async fn devices(&self) -> Result<Paging<Device>, ApiError> {
        self.get("/me/player/devices", &[]).await
    }

    pub async fn play(&self, device_id: Option<&str>, uris: &[String]) -> Result<(), ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(id) = device_id {
            query.push(("device_id", id.to_string()));
        }
        let body = if uris.is_empty() {
            None
        } else {
            Some(serde_json::json!({ "uris": uris }))
        };
        let value = self.send(Method::PUT, "/me/player/play", &query, body).await?;
        Ok::<(), ApiError>(if value.is_null() { () } else { () })
    }

    pub async fn pause(&self, device_id: Option<&str>) -> Result<(), ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(id) = device_id {
            query.push(("device_id", id.to_string()));
        }
        self.send(Method::PUT, "/me/player/pause", &query, None).await?;
        Ok(())
    }

    pub async fn set_volume(&self, percent: u32, device_id: Option<&str>) -> Result<(), ApiError> {
        let mut query: Vec<(&str, String)> = vec![("volume_percent", percent.to_string())];
        if let Some(id) = device_id {
            query.push(("device_id", id.to_string()));
        }
        self.send(Method::PUT, "/me/player/volume", &query, None)
            .await?;
        Ok(())
    }

    pub async fn add_to_playlist(
        &self,
        playlist_id: &str,
        uris: &[String],
    ) -> Result<(), ApiError> {
        let body = serde_json::json!({ "uris": uris });
        self.send(
            Method::POST,
            &format!("/playlists/{playlist_id}/tracks"),
            &[],
            Some(body),
        )
        .await?;
        Ok(())
    }
}
