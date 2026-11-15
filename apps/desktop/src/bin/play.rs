//! Headless playback test.
//!
//! Connects a librespot session with the stored access token, loads a track
//! and reports the player events it receives. Useful to verify that the
//! protocol path works without starting the GUI.

use std::time::Duration;

use spotify_core::api::ApiClient;
use spotify_core::auth;
use spotify_playback::Playback;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let Some(tokens) = auth::load_tokens()? else {
        eprintln!("no stored credentials; run spotify-rs-auth first");
        return Ok(());
    };

    // Pick a track: an explicit argument, otherwise the user's top tracks.
    let track_id = match std::env::args().nth(1) {
        Some(id) => id,
        None => {
            let client = ApiClient::new(tokens.clone(), spotify_core::api::client_id());
            match client.top_tracks(1).await {
                Ok(paging) => match paging.items.first().and_then(|t| t.id.clone()) {
                    Some(id) => id,
                    None => {
                        eprintln!("could not pick a track from the web api; pass one explicitly");
                        return Ok(());
                    }
                },
                Err(err) => {
                    eprintln!("web api unavailable ({err}); pass a track id explicitly");
                    return Ok(());
                }
            }
        }
    };

    println!("track: {track_id}");
    println!("connecting the playback session...");

    let mut playback = Playback::connect(&tokens.access_token).await?;
    println!("connected");

    playback.load_track(&track_id, true, 0)?;
    println!("loaded, playing for 15 seconds...");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        let event = tokio::time::timeout(remaining, playback.next_event()).await;
        match event {
            Ok(Some(event)) => {
                println!("event: {event:?}");
                let state = playback.state();
                println!(
                    "  state: title={:?} artist={:?} pos={}ms dur={}ms paused={}",
                    state.title, state.artist, state.position_ms, state.duration_ms, state.paused
                );
            }
            Ok(None) => {
                println!("event channel closed");
                break;
            }
            Err(_) => break,
        }
    }

    println!("done");
    Ok(())
}
