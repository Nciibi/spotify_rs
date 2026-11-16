//! The background worker: owns the Web API client and the playback session,
//! and translates UI commands into network and audio work.

use spotify_core::api::ApiClient;
use spotify_core::auth::{self, LoginFlow};
use spotify_core::models::Track;
use spotify_playback::Playback;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::app::{Command, Event};

/// Commands routed to the playback task.
enum PlaybackCommand {
    Load(String),
    TogglePause,
    Seek(u32),
    Volume(u8),
}

pub async fn run(mut commands: UnboundedReceiver<Command>, events: UnboundedSender<Event>) {
    let mut api: Option<ApiClient> = None;
    let mut playback_tx: Option<UnboundedSender<PlaybackCommand>> = None;

    // Try to resume a previous session straight away.
    if let Ok(Some(tokens)) = auth::load_tokens() {
        let client = ApiClient::new(tokens.clone(), spotify_core::api::client_id());
        // A client token lifts the web api rate limit, but it needs a client
        // secret, so it is best effort.
        match client.client_token().await {
            Ok(_) => tracing::info!("client token acquired"),
            Err(err) => tracing::debug!("no client token available: {err}"),
        }
        match client.refresh().await {
            Ok(_) => {
                let fresh = client.tokens().await;
                let _ = auth::save_tokens(&fresh);
                let _ = events.send(Event::LoggedIn);
                playback_tx = Some(spawn_playback(fresh.access_token.clone(), &events));
                api = Some(client);
                load_home(api.as_ref().unwrap(), &events).await;
            }
            Err(err) => {
                tracing::warn!("could not refresh the stored token: {err}");
                let _ = events.send(Event::NeedsLogin(String::new()));
            }
        }
    } else {
        let _ = events.send(Event::NeedsLogin(String::new()));
    }

    while let Some(command) = commands.recv().await {
        match command {
            Command::Login => {
                start_login(&events);
            }
            Command::Logout => {
                let _ = auth::clear_tokens();
                api = None;
                playback_tx = None;
                let _ = events.send(Event::LoggedOut);
            }
            Command::LoadHome => {
                if let Some(client) = &api {
                    load_home(client, &events).await;
                }
            }
            Command::LoadLibrary => {
                if let Some(client) = &api {
                    load_library(client, &events).await;
                }
            }
            Command::LoadPlaylist(id) => {
                if let Some(client) = &api {
                    load_playlist(client, id, &events).await;
                }
            }
            Command::Search(query) => {
                if let Some(client) = &api {
                    match client.search(&query, "track,album,artist", 30).await {
                        Ok(result) => {
                            let _ = events.send(Event::Search(result));
                        }
                        Err(err) => {
                            let _ = events.send(Event::Error(err.to_string()));
                        }
                    }
                }
            }
            Command::PlayTrack(id) => {
                if let Some(tx) = &playback_tx {
                    let _ = tx.send(PlaybackCommand::Load(id));
                }
            }
            Command::TogglePause => {
                if let Some(tx) = &playback_tx {
                    let _ = tx.send(PlaybackCommand::TogglePause);
                }
            }
            Command::Seek(position) => {
                if let Some(tx) = &playback_tx {
                    let _ = tx.send(PlaybackCommand::Seek(position));
                }
            }
            Command::SetVolume(percent) => {
                if let Some(tx) = &playback_tx {
                    let _ = tx.send(PlaybackCommand::Volume(percent));
                }
            }
            Command::FetchImage(url) => {
                fetch_image(url, &events).await;
            }
        }
    }
}

/// Kick off Spotify's device-authorization (QR pairing) flow in the background.
fn start_login(events: &UnboundedSender<Event>) {
    let events = events.clone();
    tokio::spawn(async move {
        let authorization = match auth::start_device_flow().await {
            Ok(authorization) => authorization,
            Err(err) => {
                let _ = events.send(Event::Error(err.to_string()));
                return;
            }
        };

        let message = format!(
            "Code: {}\n{}",
            authorization.user_code,
            authorization.browser_url()
        );
        let _ = events.send(Event::NeedsLogin(message));
        open_browser(authorization.browser_url());

        match auth::await_device_token(&authorization, |_| {}).await {
            Ok(tokens) => {
                let _ = auth::save_tokens(&tokens);
                let client = ApiClient::new(tokens.clone(), spotify_core::api::client_id());
                let _ = events.send(Event::LoggedIn);
                let _ = events.send(Event::NowPlaying(Box::new(Default::default())));
                load_home(&client, &events).await;
            }
            Err(err) => {
                let _ = events.send(Event::Error(err.to_string()));
            }
        }
    });
}

/// Run the librespot player on its own task and report its state.
fn spawn_playback(
    access_token: String,
    events: &UnboundedSender<Event>,
) -> UnboundedSender<PlaybackCommand> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let events = events.clone();

    tokio::spawn(async move {
        let mut playback = match Playback::connect(&access_token).await {
            Ok(playback) => playback,
            Err(err) => {
                tracing::warn!("playback could not start: {err}");
                let _ = events.send(Event::Error(format!("playback unavailable: {err}")));
                return;
            }
        };

        let _ = events.send(Event::NowPlaying(Box::new(playback.state().clone())));

        loop {
            tokio::select! {
                command = rx.recv() => {
                    let Some(command) = command else { break };
                    match command {
                        PlaybackCommand::Load(id) => {
                            if let Err(err) = playback.load_track(&id, true, 0) {
                                let _ = events.send(Event::Error(err.to_string()));
                            }
                        }
                        PlaybackCommand::TogglePause => playback.toggle_pause(),
                        PlaybackCommand::Seek(position) => playback.seek(position),
                        PlaybackCommand::Volume(percent) => playback.set_volume(percent),
                    }
                    let _ = events.send(Event::NowPlaying(Box::new(playback.state().clone())));
                }
                event = playback.next_event() => {
                    if event.is_none() {
                        break;
                    }
                    let _ = events.send(Event::NowPlaying(Box::new(playback.state().clone())));
                }
            }
        }
    });

    tx
}

async fn load_home(client: &ApiClient, events: &UnboundedSender<Event>) {
    let playlists = client.my_playlists(50, 0).await.map(|p| p.items);
    let new_releases = client.new_releases(12).await.map(|p| p.items);
    let top_tracks = client.top_tracks(10).await.map(|p| p.items);

    match (playlists, new_releases, top_tracks) {
        (Ok(playlists), Ok(new_releases), Ok(top_tracks)) => {
            let _ = events.send(Event::Home {
                playlists,
                new_releases,
                top_tracks,
            });
        }
        (Err(err), _, _) => {
            let _ = events.send(Event::Error(err.to_string()));
        }
        _ => {}
    }
}

async fn load_library(client: &ApiClient, events: &UnboundedSender<Event>) {
    let playlists = client.my_playlists(100, 0).await.map(|p| p.items);
    let saved = client.saved_tracks(100, 0).await.map(|p| p.items);

    match (playlists, saved) {
        (Ok(playlists), Ok(saved)) => {
            let saved_tracks: Vec<Track> =
                saved.into_iter().filter_map(|item| item.track).collect();
            let _ = events.send(Event::Library {
                playlists,
                saved_tracks,
            });
        }
        (Err(err), _) | (_, Err(err)) => {
            let _ = events.send(Event::Error(err.to_string()));
        }
    }
}

async fn load_playlist(client: &ApiClient, id: String, events: &UnboundedSender<Event>) {
    match client.playlist(&id).await {
        Ok(playlist) => {
            let name = playlist.name.clone();
            match client.playlist_tracks(&id, 100, 0).await {
                Ok(paging) => {
                    let tracks: Vec<Track> =
                        paging.items.into_iter().filter_map(|item| item.track).collect();
                    let _ = events.send(Event::PlaylistTracks { id, name, tracks });
                }
                Err(err) => {
                    let _ = events.send(Event::Error(err.to_string()));
                }
            }
        }
        Err(err) => {
            let _ = events.send(Event::Error(err.to_string()));
        }
    }
}

/// Download and decode one piece of artwork.
async fn fetch_image(url: String, events: &UnboundedSender<Event>) {
    let response = reqwest::get(&url).await;
    let Ok(response) = response else {
        return;
    };
    let Ok(bytes) = response.bytes().await else {
        return;
    };

    let Ok(image) = image::load_from_memory(&bytes) else {
        return;
    };
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let _ = events.send(Event::Image {
        url,
        rgba: rgba.into_raw(),
        size,
    });
}
