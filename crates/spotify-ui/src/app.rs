//! The desktop application shell: sidebar, content pane and now-playing bar.

use std::collections::{HashMap, HashSet};

use egui::{Color32, Layout, RichText, Sense, Stroke, TextStyle, Vec2};
use spotify_core::models::{Album, Playlist, SearchResult, Track};
use spotify_playback::NowPlaying;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::theme;

/// Which page the content pane shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    Home,
    Search,
    Library,
    Playlist(String),
}

/// Commands the UI sends to the background worker.
pub enum Command {
    Login,
    Logout,
    LoadHome,
    LoadLibrary,
    LoadPlaylist(String),
    Search(String),
    PlayTrack(String),
    TogglePause,
    Seek(u32),
    SetVolume(u8),
    FetchImage(String),
}

/// Events the background worker sends back to the UI.
pub enum Event {
    Ready,
    NeedsLogin(String),
    LoggedIn,
    LoggedOut,
    Home {
        playlists: Vec<Playlist>,
        new_releases: Vec<Album>,
        top_tracks: Vec<Track>,
    },
    Library {
        playlists: Vec<Playlist>,
        saved_tracks: Vec<Track>,
    },
    PlaylistTracks {
        id: String,
        name: String,
        tracks: Vec<Track>,
    },
    Search(SearchResult),
    NowPlaying(Box<NowPlaying>),
    Image {
        url: String,
        rgba: Vec<u8>,
        size: [usize; 2],
    },
    Error(String),
}

/// The egui application state.
pub struct SpotifyApp {
    cmd_tx: UnboundedSender<Command>,
    events: UnboundedReceiver<Event>,

    page: Page,
    status: String,
    error: Option<String>,

    user_name: Option<String>,
    playlists: Vec<Playlist>,
    new_releases: Vec<Album>,
    top_tracks: Vec<Track>,
    saved_tracks: Vec<Track>,
    library_loaded: bool,

    search_query: String,
    search_results: Option<SearchResult>,

    now_playing: NowPlaying,
    play_queue: Vec<Track>,
    queue_index: usize,

    textures: HashMap<String, egui::TextureHandle>,
    inflight_images: HashSet<String>,
    image_failures: HashSet<String>,

    logged_in: bool,
    /// Pairing code and URL while the device-authorization flow is in flight.
    pairing: Option<String>,
}

impl SpotifyApp {
    /// Spawn the background worker and build the UI state.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
        let (evt_tx, evt_rx) = unbounded_channel::<Event>();

        // The worker owns the API client and the playback session; it reports
        // back through the event channel the UI reads from.
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(err) => {
                    tracing::error!("could not start the tokio runtime: {err}");
                    return;
                }
            };
            runtime.block_on(crate::worker::run(cmd_rx, evt_tx));
        });

        Self {
            cmd_tx,
            events: evt_rx,
            page: Page::Home,
            status: "Starting up".into(),
            error: None,
            user_name: None,
            playlists: Vec::new(),
            new_releases: Vec::new(),
            top_tracks: Vec::new(),
            saved_tracks: Vec::new(),
            library_loaded: false,
            search_query: String::new(),
            search_results: None,
            now_playing: NowPlaying::default(),
            play_queue: Vec::new(),
            queue_index: 0,
            textures: HashMap::new(),
            inflight_images: HashSet::new(),
            image_failures: HashSet::new(),
            logged_in: false,
            pairing: None,
        }
    }

    fn send(&self, command: Command) {
        let _ = self.cmd_tx.send(command);
    }

    /// Ask the worker for artwork that is not cached yet.
    fn ensure_image(&mut self, url: &str) -> Option<egui::TextureHandle> {
        if url.is_empty() {
            return None;
        }
        if let Some(texture) = self.textures.get(url) {
            return Some(texture.clone());
        }
        if !self.inflight_images.contains(url) && !self.image_failures.contains(url) {
            self.inflight_images.insert(url.to_string());
            self.send(Command::FetchImage(url.to_string()));
        }
        None
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Ready => {
                    self.logged_in = true;
                    self.status = "Signed in".into();
                }
                Event::NeedsLogin(message) => {
                    self.logged_in = false;
                    let waiting = !message.is_empty();
                    self.pairing = waiting.then_some(message);
                    self.status = if waiting {
                        "Pairing — approve in your browser (QR code)".into()
                    } else {
                        "Sign in required".into()
                    };
                }
                Event::LoggedIn => {
                    self.logged_in = true;
                    self.pairing = None;
                    self.status = "Signed in".into();
                }
                Event::LoggedOut => {
                    self.logged_in = false;
                    self.user_name = None;
                    self.playlists.clear();
                    self.status = "Signed out".into();
                }
                Event::Home {
                    playlists,
                    new_releases,
                    top_tracks,
                } => {
                    self.playlists = playlists;
                    self.new_releases = new_releases;
                    self.top_tracks = top_tracks;
                    self.status = "Home loaded".into();
                }
                Event::Library {
                    playlists,
                    saved_tracks,
                } => {
                    self.playlists = playlists;
                    self.saved_tracks = saved_tracks;
                    self.library_loaded = true;
                    self.status = "Library loaded".into();
                }
                Event::PlaylistTracks { id, name, tracks } => {
                    self.page = Page::Playlist(id);
                    self.play_queue = tracks;
                    self.queue_index = 0;
                    self.status = format!("Loaded {name}");
                }
                Event::Search(result) => {
                    self.search_results = Some(result);
                    self.status = "Search finished".into();
                }
                Event::NowPlaying(state) => {
                    self.now_playing = *state;
                }
                Event::Image { url, rgba, size } => {
                    self.inflight_images.remove(&url);
                    let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                    let texture = ctx.load_texture(url.clone(), image, egui::TextureOptions::default());
                    self.textures.insert(url, texture);
                }
                Event::Error(message) => {
                    self.inflight_images.clear();
                    self.error = Some(message);
                }
            }
        }
    }

    // ---- Layout --------------------------------------------------------

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(
            RichText::new("spotify-rs")
                .color(theme::GREEN)
                .size(19.0)
                .strong(),
        );
        ui.add_space(12.0);

        for (label, page) in [
            ("Home", Page::Home),
            ("Search", Page::Search),
            ("Your Library", Page::Library),
        ] {
            let selected = self.page == page;
            let color = if selected {
                theme::TEXT
            } else {
                theme::TEXT_MUTED
            };
            let clicked = ui
                .add(
                    egui::Button::new(RichText::new(label).color(color).size(15.0))
                        .fill(if selected {
                            theme::HOVER
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::NONE)
                        .corner_radius(egui::CornerRadius::same(4)),
                )
                .clicked();
            if clicked {
                self.page = page.clone();
                match page {
                    Page::Home => self.send(Command::LoadHome),
                    Page::Library => {
                        if !self.library_loaded {
                            self.send(Command::LoadLibrary);
                        }
                    }
                    Page::Search => {}
                    Page::Playlist(_) => {}
                }
            }
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new("PLAYLISTS").color(theme::TEXT_DIM).size(11.0));

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.playlists.is_empty() {
                    ui.label(RichText::new("No playlists yet").color(theme::TEXT_DIM).size(13.0));
                }
                for playlist in &self.playlists {
                    let label = RichText::new(&playlist.name).color(theme::TEXT_MUTED).size(13.0);
                    let clicked = ui
                        .add(
                            egui::Button::new(label)
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .corner_radius(egui::CornerRadius::same(4)),
                        )
                        .clicked();
                    if clicked {
                        self.send(Command::LoadPlaylist(playlist.id.clone()));
                    }
                }
            });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let arrow = |ui: &mut egui::Ui, glyph: &str| {
                ui.add(
                    egui::Button::new(RichText::new(glyph).color(theme::TEXT_DIM).size(18.0))
                        .fill(theme::HOVER)
                        .stroke(Stroke::NONE)
                        .corner_radius(egui::CornerRadius::same(50))
                        .min_size(Vec2::new(30.0, 30.0)),
                );
            };

            arrow(ui, "<");
            arrow(ui, ">");
            ui.add_space(12.0);

            let edit = egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("What do you want to play?")
                .desired_width(320.0)
                .background_color(theme::HOVER)
                .text_color(theme::TEXT)
                .font(TextStyle::Body);
            let response = ui.add(edit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let query = self.search_query.trim().to_string();
                if !query.is_empty() {
                    self.page = Page::Search;
                    self.send(Command::Search(query));
                }
            }
        });
    }

    fn artwork(&mut self, ui: &mut egui::Ui, url: &str, size: f32) -> Option<egui::TextureHandle> {
        match self.ensure_image(url) {
            Some(texture) => {
                ui.add(
                    egui::Image::new(&texture)
                        .fit_to_exact_size(Vec2::new(size, size))
                        .corner_radius(egui::CornerRadius::same(4)),
                );
                Some(texture)
            }
            None => {
                // Placeholder until the artwork arrives.
                let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(4), theme::HOVER);
                None
            }
        }
    }

    fn track_row(&mut self, ui: &mut egui::Ui, index: usize, track: &Track, queue: &[Track]) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("{index}"))
                    .color(theme::TEXT_DIM)
                    .size(13.0),
            );
            let url = track.image().map(|image| image.url.clone()).unwrap_or_default();
            self.artwork(ui, &url, 40.0);
            ui.vertical(|ui| {
                let title = RichText::new(&track.name).color(theme::TEXT).size(14.0);
                let clicked = ui.add(egui::Label::new(title).sense(Sense::click())).clicked();
                if clicked {
                    self.play_queue = queue.to_vec();
                    self.queue_index = index;
                    self.send(Command::PlayTrack(track.id().to_string()));
                }
                ui.label(
                    RichText::new(track.artist_line())
                        .color(theme::TEXT_MUTED)
                        .size(12.0),
                );
            });
        });
        ui.separator();
    }

    fn home_view(&mut self, ui: &mut egui::Ui) {
        // A tinted header, like the gradient the desktop client paints behind
        // the greeting on its home page.
        let (banner, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 116.0), Sense::hover());
        theme::gradient(ui.painter(), banner, theme::GRADIENT_TOP, theme::GRADIENT_BOTTOM);
        ui.allocate_ui_at_rect(banner, |ui| {
            ui.vertical(|ui| {
                ui.add_space(38.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(RichText::new("Good evening").color(theme::TEXT).size(32.0).strong());
                });
            });
        });

        let albums = self.new_releases.clone();
        if !albums.is_empty() {
            ui.add_space(6.0);
            theme::section_title(ui, "New releases");
            ui.add_space(8.0);
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for album in &albums {
                        let url = album.image().map(|image| image.url.clone()).unwrap_or_default();
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(168.0, 214.0), Sense::click());
                        theme::card_background(ui.painter(), rect, response.hovered());
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.vertical(|ui| {
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(9.0);
                                    self.artwork(ui, &url, 150.0);
                                });
                                ui.horizontal(|ui| {
                                    ui.add_space(9.0);
                                    ui.label(RichText::new(&album.name).color(theme::TEXT).size(13.0).strong());
                                });
                                ui.horizontal(|ui| {
                                    ui.add_space(9.0);
                                    ui.label(
                                        RichText::new(album.album_type.replace('_', " "))
                                            .color(theme::TEXT_MUTED)
                                            .size(11.0),
                                    );
                                });
                            });
                        });
                        ui.add_space(10.0);
                    }
                });
            });
            ui.add_space(16.0);
        }

        let tracks = self.top_tracks.clone();
        if !tracks.is_empty() {
            ui.label(RichText::new("Your top tracks").color(theme::TEXT).size(20.0).strong());
            ui.add_space(8.0);
            for (index, track) in tracks.iter().enumerate() {
                self.track_row(ui, index + 1, track, &tracks);
            }
        }
    }

    fn library_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.label(RichText::new("Your Library").color(theme::TEXT).size(28.0).strong());
        ui.add_space(12.0);

        let tracks = self.saved_tracks.clone();
        if !tracks.is_empty() {
            ui.label(
                RichText::new("Liked songs")
                    .color(theme::TEXT)
                    .size(20.0)
                    .strong(),
            );
            ui.add_space(8.0);
            for (index, track) in tracks.iter().enumerate() {
                self.track_row(ui, index + 1, track, &tracks);
            }
        } else {
            ui.label(
                RichText::new("Loading your library...").color(theme::TEXT_DIM).size(13.0),
            );
        }
    }

    fn search_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);

        let Some(result) = self.search_results.clone() else {
            ui.label(
                RichText::new("Search for songs, artists and albums")
                    .color(theme::TEXT_DIM)
                    .size(14.0),
            );
            return;
        };

        if !result.tracks.items.is_empty() {
            ui.label(RichText::new("Songs").color(theme::TEXT).size(20.0).strong());
            ui.add_space(8.0);
            let tracks = result.tracks.items.clone();
            for (index, track) in tracks.iter().enumerate() {
                self.track_row(ui, index + 1, track, &tracks);
            }
        }

        if !result.albums.items.is_empty() {
            ui.add_space(16.0);
            ui.label(RichText::new("Albums").color(theme::TEXT).size(20.0).strong());
            ui.add_space(8.0);
            let albums = result.albums.items.clone();
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for album in &albums {
                        let url = album.image().map(|image| image.url.clone()).unwrap_or_default();
                        ui.vertical(|ui| {
                            ui.set_width(150.0);
                            self.artwork(ui, &url, 150.0);
                            ui.label(
                                RichText::new(&album.name)
                                    .color(theme::TEXT)
                                    .size(13.0)
                                    .strong(),
                            );
                        });
                        ui.add_space(12.0);
                    }
                });
            });
        }

        if !result.artists.items.is_empty() {
            ui.add_space(16.0);
            ui.label(RichText::new("Artists").color(theme::TEXT).size(20.0).strong());
            ui.add_space(8.0);
            let artists = result.artists.items.clone();
            for artist in &artists {
                ui.horizontal(|ui| {
                    let url = artist.image().map(|image| image.url.clone()).unwrap_or_default();
                    self.artwork(ui, &url, 40.0);
                    ui.label(RichText::new(&artist.name).color(theme::TEXT).size(14.0));
                });
            }
        }
    }

    fn playlist_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        let title = match &self.page {
            Page::Playlist(id) => self
                .playlists
                .iter()
                .find(|playlist| &playlist.id == id)
                .map(|playlist| playlist.name.clone())
                .unwrap_or_else(|| "Playlist".into()),
            _ => "Playlist".into(),
        };
        ui.label(RichText::new(title).color(theme::TEXT).size(28.0).strong());
        ui.add_space(12.0);

        let tracks = self.play_queue.clone();
        if tracks.is_empty() {
            ui.label(RichText::new("Loading tracks...").color(theme::TEXT_DIM).size(13.0));
            return;
        }
        for (index, track) in tracks.iter().enumerate() {
            self.track_row(ui, index + 1, track, &tracks);
        }
    }

    fn now_playing_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(12.0);

            let art_url = self
                .play_queue
                .get(self.queue_index)
                .and_then(|track| track.image().map(|image| image.url.clone()))
                .unwrap_or_default();
            self.artwork(ui, &art_url, 56.0);

            ui.vertical(|ui| {
                ui.set_width(200.0);
                ui.label(
                    RichText::new(&self.now_playing.title)
                        .color(theme::TEXT)
                        .size(13.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(&self.now_playing.artist)
                        .color(theme::TEXT_MUTED)
                        .size(11.0),
                );
            });

            ui.add_space(16.0);

            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    let button = |ui: &mut egui::Ui, glyph: &str, size: f32| {
                        ui.add(
                            egui::Button::new(RichText::new(glyph).color(theme::TEXT).size(size))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                        )
                    };
                    button(ui, "|<<", 13.0);
                    button(ui, "<", 20.0);
                    let paused = self.now_playing.paused;
                    theme::play_pause_button(ui, paused, || self.send(Command::TogglePause));
                    button(ui, ">", 20.0);
                    button(ui, ">>|", 13.0);
                });

                ui.add_space(4.0);
                let duration = self.now_playing.duration_ms.max(1);
                let ratio = (self.now_playing.position_ms as f32 / duration as f32).clamp(0.0, 1.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(360.0, 4.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(2), theme::HOVER);
                let filled = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.min.x + rect.width() * ratio, rect.max.y),
                );
                ui.painter()
                    .rect_filled(filled, egui::CornerRadius::same(2), theme::TEXT);
            });

            ui.add_space(16.0);

            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!("{}%", self.now_playing.volume_percent))
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                );
                let mut volume = self.now_playing.volume_percent as f32;
                let slider = egui::Slider::new(&mut volume, 0.0..=100.0).show_value(false);
                let changed = ui.add_sized([90.0, 18.0], slider).changed();
                if changed {
                    self.send(Command::SetVolume(volume.round() as u8));
                }
            });

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(name) = &self.user_name {
                    ui.label(RichText::new(name).color(theme::TEXT_MUTED).size(12.0));
                }
                ui.add_space(12.0);
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new(&self.status).color(theme::TEXT_DIM).size(11.0));
            if let Some(error) = &self.error {
                ui.label(
                    RichText::new(error)
                        .color(Color32::from_rgb(255, 100, 100))
                        .size(11.0),
                );
            }
            if !self.logged_in {
                ui.label(RichText::new("Sign in").color(theme::GREEN).size(11.0).strong());
                let clicked = ui
                    .add(
                        egui::Button::new(
                            RichText::new("Sign in with Spotify")
                                .color(theme::BLACK)
                                .size(12.0),
                        )
                        .fill(theme::GREEN)
                        .stroke(Stroke::NONE),
                    )
                    .clicked();
                if clicked {
                    self.send(Command::Login);
                }
            }
        });
    }
}

impl eframe::App for SpotifyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        tracing::debug!("frame");
        self.drain_events(ui.ctx());

        // Poll for player events without blocking the UI thread.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));

        egui::containers::Panel::bottom("now-playing")
            .frame(egui::Frame::default().fill(theme::BLACK))
            .exact_size(88.0)
            .resizable(false)
            .show(ui, |ui| self.now_playing_bar(ui));

        egui::containers::Panel::left("sidebar")
            .frame(egui::Frame::default().fill(theme::BLACK))
            .exact_size(220.0)
            .resizable(false)
            .show(ui, |ui| self.sidebar(ui));

        egui::containers::Panel::top("top-bar")
            .frame(egui::Frame::default().fill(theme::BASE))
            .exact_size(60.0)
            .resizable(false)
            .show(ui, |ui| self.top_bar(ui));

        egui::containers::CentralPanel::default()
            .frame(egui::Frame::default().fill(theme::BASE))
            .show(ui, |ui| {
                if let Some(message) = self.pairing.clone() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(90.0);
                        ui.label(
                            RichText::new("Pair with Spotify")
                                .color(theme::TEXT)
                                .size(28.0)
                                .strong(),
                        );
                        ui.add_space(18.0);
                        ui.label(
                            RichText::new(&message)
                                .color(theme::TEXT_MUTED)
                                .size(15.0),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(
                                "Your browser shows a QR code — scan it with the Spotify \
                                 mobile app, or enter the code there.",
                            )
                            .color(theme::TEXT_DIM)
                            .size(12.0),
                        );
                    });
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| match self.page.clone() {
                    Page::Home => self.home_view(ui),
                    Page::Search => self.search_view(ui),
                    Page::Library => self.library_view(ui),
                    Page::Playlist(_) => self.playlist_view(ui),
                });
            });

        egui::containers::Panel::top("status")
            .frame(egui::Frame::default().fill(theme::BLACK))
            .exact_size(24.0)
            .resizable(false)
            .show(ui, |ui| self.status_bar(ui));
    }
}
