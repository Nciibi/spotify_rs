//! Data models mirroring the shapes returned by the Spotify Web API.
//!
//! Every field that Spotify may omit is `Option` or defaulted, so a partial
//! response never fails to deserialize.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Image {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub width: Option<u32>,
}

impl Image {
    /// The largest available image, which Spotify orders first.
    pub fn largest(&self) -> Option<&Image> {
        Some(self)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalUrls {
    #[serde(default)]
    pub spotify: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Followers {
    #[serde(default)]
    pub href: Option<String>,
    #[serde(default)]
    pub total: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Artist {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub popularity: Option<u32>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub external_urls: ExternalUrls,
}

impl Artist {
    pub fn image(&self) -> Option<&Image> {
        self.images.first()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimplifiedAlbum {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub album_type: String,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub total_tracks: u32,
}

impl SimplifiedAlbum {
    pub fn image(&self) -> Option<&Image> {
        self.images.first()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Track {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub duration_ms: u32,
    #[serde(default)]
    pub explicit: bool,
    #[serde(default)]
    pub popularity: u32,
    #[serde(default)]
    pub track_number: u32,
    #[serde(default)]
    pub disc_number: u32,
    #[serde(default)]
    pub is_local: bool,
    #[serde(default)]
    pub preview_url: Option<String>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub album: SimplifiedAlbum,
}

impl Track {
    pub fn id(&self) -> &str {
        self.id.as_deref().unwrap_or_default()
    }

    pub fn image(&self) -> Option<&Image> {
        self.album.image()
    }

    /// `Artist A, Artist B` — how Spotify renders the artist line.
    pub fn artist_line(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Album {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub album_type: String,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub total_tracks: u32,
    #[serde(default)]
    pub popularity: u32,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub tracks: Paging<SimplifiedTrack>,
}

impl Album {
    pub fn image(&self) -> Option<&Image> {
        self.images.first()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimplifiedTrack {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub duration_ms: u32,
    #[serde(default)]
    pub track_number: u32,
    #[serde(default)]
    pub disc_number: u32,
    #[serde(default)]
    pub explicit: bool,
    #[serde(default)]
    pub artists: Vec<Artist>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Playlist {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub collaborative: bool,
    #[serde(default)]
    pub public: Option<bool>,
    #[serde(default)]
    pub owner: Option<Owner>,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub followers: Followers,
    #[serde(default)]
    pub tracks: Paging<PlaylistItem>,
    #[serde(default)]
    pub external_urls: ExternalUrls,
}

impl Playlist {
    pub fn image(&self) -> Option<&Image> {
        self.images.first()
    }

    pub fn owner_name(&self) -> &str {
        self.owner
            .as_ref()
            .and_then(|owner| owner.display_name.as_deref())
            .unwrap_or("")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Owner {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub external_urls: ExternalUrls,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaylistItem {
    #[serde(default)]
    pub added_at: Option<String>,
    #[serde(default)]
    pub track: Option<Track>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedTrack {
    #[serde(default)]
    pub added_at: Option<String>,
    #[serde(default)]
    pub track: Option<Track>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedAlbum {
    #[serde(default)]
    pub added_at: Option<String>,
    #[serde(default)]
    pub album: Option<Album>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub external_urls: ExternalUrls,
}

impl UserProfile {
    pub fn name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.id)
    }
}

/// A page of results, mirroring Spotify's `Paging` object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Paging<T> {
    #[serde(default)]
    pub href: Option<String>,
    #[serde(default)]
    pub items: Vec<T>,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub previous: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResult {
    #[serde(default)]
    pub tracks: Paging<Track>,
    #[serde(default)]
    pub artists: Paging<Artist>,
    #[serde(default)]
    pub albums: Paging<Album>,
    #[serde(default)]
    pub playlists: Paging<Playlist>,
    #[serde(default)]
    pub shows: Paging<serde_json::Value>,
    #[serde(default)]
    pub episodes: Paging<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Recommendations {
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub seeds: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Category {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icons: Vec<Image>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryList {
    #[serde(default)]
    pub categories: Vec<Category>,
}

/// A snapshot of what Spotify Connect is currently playing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentlyPlaying {
    #[serde(default)]
    pub is_playing: bool,
    #[serde(default)]
    pub item: Option<Track>,
    #[serde(default)]
    pub progress_ms: Option<u32>,
    #[serde(default)]
    pub device: Option<Device>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub volume_percent: Option<u32>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_private_session: bool,
}
