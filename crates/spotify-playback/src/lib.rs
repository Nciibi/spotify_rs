//! Native audio playback, built on the librespot protocol client.
//!
//! The [`Playback`] handle owns a librespot session and player, and surfaces
//! player events so the UI can mirror the current transport state.

use std::sync::Arc;
use std::time::Duration;

use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_core::spotify_id::SpotifyId;
use librespot_core::SpotifyUri;
use librespot_metadata::audio::item::UniqueFields;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, Bitrate, PlayerConfig};
use librespot_playback::mixer::{Mixer, MixerConfig};
use librespot_playback::player::{Player, PlayerEvent, PlayerEventChannel};

#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("could not connect to spotify: {0}")]
    Connect(String),
    #[error("no audio backend is available")]
    NoAudioBackend,
    #[error("invalid track id `{0}`")]
    InvalidTrackId(String),
}

/// The transport state the UI renders.
#[derive(Debug, Clone, Default)]
pub struct NowPlaying {
    pub track_id: Option<String>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub position_ms: u32,
    pub duration_ms: u32,
    pub paused: bool,
    pub volume_percent: u8,
}

/// A live Spotify playback session.
pub struct Playback {
    player: Arc<Player>,
    mixer: Arc<dyn Mixer>,
    events: PlayerEventChannel,
    state: NowPlaying,
}

impl Playback {
    /// Connect to Spotify and prepare an audio pipeline.
    ///
    /// `access_token` must carry the `streaming` scope.
    pub async fn connect(access_token: &str) -> Result<Self, PlaybackError> {
        let backend = audio_backend::find(None).ok_or(PlaybackError::NoAudioBackend)?;

        let session_config = SessionConfig::default();
        let credentials = Credentials::with_access_token(access_token);

        tracing::info!("connecting to spotify");
        let session = Session::new(session_config, None);
        session
            .connect(credentials, false)
            .await
            .map_err(|e| PlaybackError::Connect(e.to_string()))?;

        let mixer_config = MixerConfig::default();
        let mixer = librespot_playback::mixer::find(None)
            .ok_or(PlaybackError::NoAudioBackend)?
            (mixer_config)
            .map_err(|e| PlaybackError::Connect(e.to_string()))?;

        let soft_volume = mixer.get_soft_volume();

        let mut player_config = PlayerConfig {
            bitrate: Bitrate::Bitrate320,
            gapless: true,
            ..PlayerConfig::default()
        };
        // Ask the player to report its position so the UI can draw a progress bar.
        player_config.position_update_interval = Some(Duration::from_millis(500));

        let audio_format = AudioFormat::default();
        let player = Player::new(player_config, session, soft_volume, move || {
            backend(None, audio_format)
        });

        let events = player.get_player_event_channel();

        Ok(Self {
            player,
            mixer,
            events,
            state: NowPlaying::default(),
        })
    }

    /// Load a track by its base62 id and start playing it.
    pub fn load_track(&mut self, track_id: &str, autoplay: bool, position_ms: u32) -> Result<(), PlaybackError> {
        let id = SpotifyId::from_base62(track_id)
            .map_err(|_| PlaybackError::InvalidTrackId(track_id.to_string()))?;
        let uri = SpotifyUri::Track { id };

        self.state.track_id = Some(track_id.to_string());
        self.state.position_ms = position_ms;
        self.player.load(uri, autoplay, position_ms);
        Ok(())
    }

    pub fn play(&mut self) {
        self.player.play();
        self.state.paused = false;
    }

    pub fn pause(&mut self) {
        self.player.pause();
        self.state.paused = true;
    }

    pub fn toggle_pause(&mut self) {
        if self.state.paused {
            self.play();
        } else {
            self.pause();
        }
    }

    pub fn seek(&mut self, position_ms: u32) {
        self.player.seek(position_ms);
        self.state.position_ms = position_ms;
    }

    /// Set the volume, where `percent` is in `0..=100`.
    pub fn set_volume(&mut self, percent: u8) {
        let percent = percent.min(100);
        // The soft mixer works in 16-bit attenuation units.
        let attenuation = u16::from(percent) * 100;
        self.mixer.set_volume(attenuation);
        self.state.volume_percent = percent;
    }

    pub fn state(&self) -> &NowPlaying {
        &self.state
    }

    /// Wait for the next player event and fold it into the mirrored state.
    pub async fn next_event(&mut self) -> Option<PlayerEvent> {
        let event = self.events.recv().await?;

        match &event {
            PlayerEvent::TrackChanged { audio_item } => {
                self.state.track_id = Some(audio_item.track_id.to_id());
                self.state.title = audio_item.name.clone();
                self.state.duration_ms = audio_item.duration_ms;
                if let UniqueFields::Track { artists, album, .. } = &audio_item.unique_fields {
                    self.state.artist = artists
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.state.album = album.clone();
                }
            }
            PlayerEvent::PositionChanged { position_ms, .. } => {
                self.state.position_ms = *position_ms;
            }
            PlayerEvent::PositionCorrection { position_ms, .. } => {
                self.state.position_ms = *position_ms;
            }
            PlayerEvent::Seeked { position_ms, .. } => {
                self.state.position_ms = *position_ms;
            }
            PlayerEvent::VolumeChanged { volume } => {
                self.state.volume_percent = (*volume / 100).min(100) as u8;
            }
            PlayerEvent::EndOfTrack { .. } => {
                self.state.position_ms = 0;
            }
            _ => {}
        }

        Some(event)
    }
}
