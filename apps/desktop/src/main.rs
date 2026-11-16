//! The spotify-rs desktop client.
//!
//! `spotify-rs` is a native egui application by default: its own windows,
//! widgets and renderer, with no browser engine. `--web` opts into the
//! alternative `spotify-host` front end, which embeds the Spotify web client
//! through WebView2.

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();

    if !args.iter().any(|a| a == "--web") {
        tracing::info!("starting the native front end");
        spotify_ui::run();
        return;
    }

    let url = args
        .iter()
        .find_map(|a| a.strip_prefix("--url="))
        .unwrap_or("https://open.spotify.com/")
        .to_string();

    let mut options = spotify_host::HostOptions {
        url,
        bootstrap: String::new(),
        devtools: args.iter().any(|a| a == "--devtools"),
        ad_block: !args.iter().any(|a| a == "--no-ad-block"),
        backdrop: spotify_host::backdrop::Backdrop::Mica,
    };

    if let Some(backdrop) = args.iter().find_map(|a| a.strip_prefix("--backdrop=")) {
        options.backdrop = match backdrop {
            "acrylic" => spotify_host::backdrop::Backdrop::Acrylic,
            "none" => spotify_host::backdrop::Backdrop::None,
            _ => spotify_host::backdrop::Backdrop::Mica,
        };
    }

    spotify_host::run(options);
}
