//! Headless sign-in helper.
//!
//! Uses the same device-authorization (QR pairing) flow as the desktop client:
//! Spotify shows a short code and a QR code on `spotify.com/pair`, the browser
//! is opened for the user, and we poll until the pairing is approved.

use spotify_core::api::ApiClient;
use spotify_core::auth;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Ok(Some(existing)) = auth::load_tokens() {
        let client = ApiClient::new(existing, spotify_core::api::client_id());
        match client.current_user().await {
            Ok(user) => {
                println!("already signed in as {}", user.name());
                println!("product: {}", user.product.unwrap_or_default());
                println!("country: {}", user.country.unwrap_or_default());
                return Ok(());
            }
            Err(err) => println!("stored token rejected: {err}"),
        }
    }

    let authorization = auth::start_device_flow().await?;

    println!();
    println!("  Spotify pairing code: {}", authorization.user_code);
    println!();
    println!("  Opening your browser. Scan the QR code or enter the code above at:");
    println!("    {}", authorization.browser_url());
    println!();
    open_browser(authorization.browser_url());

    let tokens = auth::await_device_token(&authorization, |message| println!("  {message}")).await?;
    auth::save_tokens(&tokens)?;

    let client = ApiClient::new(tokens, spotify_core::api::client_id());
    let user = client.current_user().await?;
    println!("signed in as {}", user.name());
    println!("product: {}", user.product.unwrap_or_default());

    let playlists = client.my_playlists(5, 0).await?;
    println!("playlists visible: {}", playlists.total);

    Ok(())
}

/// Hand the pairing URL to the system's default browser.
fn open_browser(url: &str) {
    #[cfg(windows)]
    {
        // `rundll32` hands the URL straight to the shell's protocol handler,
        // so query characters need no extra escaping.
        let _ = std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
