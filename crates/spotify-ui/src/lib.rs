//! The egui front end of spotify-rs.

pub mod app;
pub mod theme;
pub mod worker;

use eframe::egui::ViewportBuilder;

/// Start the desktop application.
pub fn run() {
    let options = eframe::NativeOptions {
        // OpenGL keeps the memory footprint an order of magnitude below the
        // wgpu/Vulkan backend, which is wasted on a 2D interface.
        renderer: eframe::Renderer::Glow,
        viewport: ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([860.0, 560.0])
            .with_title("spotify-rs"),
        ..Default::default()
    };

    match eframe::run_native(
        "spotify-rs",
        options,
        Box::new(|cc| Ok(Box::new(app::SpotifyApp::new(cc)))),
    ) {
        Ok(()) => tracing::info!("window closed"),
        Err(err) => tracing::error!("could not start the window: {err}"),
    }
}
