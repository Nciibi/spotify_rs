//! Headless sign-in helper.
//!
//! Runs the PKCE flow, stores the resulting tokens and prints a short summary
//! so the credentials can be verified without starting the GUI.

use spotify_core::api::ApiClient;
use spotify_core::auth::{self, LoginFlow};

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

    let flow = LoginFlow::start().await?;
    println!("open this URL to authorise the client:\n\n  {}\n", flow.auth_url);
    flow.open_browser();

    let tokens = flow.complete().await?;
    auth::save_tokens(&tokens)?;

    let client = ApiClient::new(tokens, spotify_core::api::client_id());
    let user = client.current_user().await?;
    println!("signed in as {}", user.name());
    println!("product: {}", user.product.unwrap_or_default());

    let playlists = client.my_playlists(5, 0).await?;
    println!("playlists visible: {}", playlists.total);

    Ok(())
}
