//! The spotify-rs desktop client.
//!
//! Two front ends share one engine:
//!
//! * `spotify-host` renders the Spotify web client in a WebView2 window,
//!   which is the same interface the official client shows.
//! * `spotify-ui` is a fully native egui client for people who want no
//!   browser engine at all.
//!
//! `--native` selects the egui front end, `--web` (the default) the web one.

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--native") {
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
