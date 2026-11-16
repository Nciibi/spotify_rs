//! Spotify's colour palette and a few shared widget helpers.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Vec2};

pub const BLACK: Color32 = Color32::from_rgb(0, 0, 0);
pub const BASE: Color32 = Color32::from_rgb(18, 18, 18);
pub const RAISED: Color32 = Color32::from_rgb(24, 24, 24);
pub const HOVER: Color32 = Color32::from_rgb(40, 40, 40);
pub const PRESSED: Color32 = Color32::from_rgb(46, 46, 46);
pub const GREEN: Color32 = Color32::from_rgb(30, 215, 96);
pub const TEXT: Color32 = Color32::from_rgb(255, 255, 255);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(179, 179, 179);
pub const TEXT_DIM: Color32 = Color32::from_rgb(122, 122, 122);
pub const BORDER: Color32 = Color32::from_rgb(42, 42, 42);
/// The brighter green used for the primary play button.
pub const GREEN_BRIGHT: Color32 = Color32::from_rgb(40, 235, 110);
/// Top of the ambient gradient behind the home page.
pub const GRADIENT_TOP: Color32 = Color32::from_rgb(46, 78, 63);
/// Bottom of the ambient gradient (fades into the page background).
pub const GRADIENT_BOTTOM: Color32 = Color32::from_rgb(18, 18, 18);
/// Background of a hovered card.
pub const CARD_HOVER: Color32 = Color32::from_rgb(38, 38, 38);

/// Apply the Spotify look to egui's global style.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);

    // Spotify's own type face, extracted from the desktop client's `xpui.spa`
    // (`re/target/xpui/fonts`) and converted from WOFF2. Using it matters more
    // than any single colour: it is most of what makes the UI read as Spotify.
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "spotify".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/SpotifyMixUI-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "spotify".to_owned());
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|style| {
        style.visuals.window_fill = RAISED;
        style.visuals.extreme_bg_color = RAISED;
        style.visuals.faint_bg_color = RAISED;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.hyperlink_color = GREEN;
        style.visuals.selection.bg_fill = GREEN.gamma_multiply(0.35);
        style.visuals.selection.stroke = Stroke::new(1.0, GREEN);
        style.visuals.widgets.hovered.bg_fill = HOVER;
        style.visuals.widgets.active.bg_fill = PRESSED;
        style.visuals.widgets.inactive.bg_fill = RAISED;
        style.visuals.widgets.open.bg_fill = PRESSED;
        style.visuals.widgets.noninteractive.bg_fill = RAISED;
        style.visuals.widgets.noninteractive.weak_bg_fill = RAISED;
        style.visuals.window_corner_radius = CornerRadius::same(6);

        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(14.0, 7.0);
        style.spacing.menu_margin = egui::Margin::same(8);
        style.spacing.interact_size.y = 28.0;

        for (text_style, size) in [
            (TextStyle::Body, 14.0),
            (TextStyle::Button, 14.0),
            (TextStyle::Heading, 24.0),
            (TextStyle::Small, 11.0),
        ] {
            style.text_styles.insert(
                text_style,
                FontId {
                    size,
                    family: FontFamily::Proportional,
                },
            );
        }
    });
}

/// A rounded rectangle painted the way Spotify paints its cards.
pub fn card(painter: &egui::Painter, rect: egui::Rect, fill: Color32) {
    painter.rect_filled(rect, CornerRadius::same(8), fill);
}

/// Draw a circular green play button.
pub fn play_button(painter: &egui::Painter, center: egui::Pos2, radius: f32, hovered: bool) {
    let fill = if hovered {
        Color32::from_rgb(40, 235, 110)
    } else {
        GREEN
    };
    painter.circle_filled(center, radius, fill);
    // A right-pointing triangle.
    let s = radius * 0.55;
    let points = vec![
        egui::pos2(center.x - s * 0.6, center.y - s),
        egui::pos2(center.x - s * 0.6, center.y + s),
        egui::pos2(center.x + s * 0.8, center.y),
    ];
    painter.add(egui::Shape::convex_polygon(points, BLACK, Stroke::NONE));
}

/// Paint a vertical gradient, the way Spotify tints the top of the home page.
pub fn gradient(painter: &egui::Painter, rect: egui::Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&[
        egui::epaint::Vertex {
            pos: rect.left_top(),
            uv: egui::pos2(0.0, 0.0),
            color: top,
        },
        egui::epaint::Vertex {
            pos: rect.right_top(),
            uv: egui::pos2(1.0, 0.0),
            color: top,
        },
        egui::epaint::Vertex {
            pos: rect.left_bottom(),
            uv: egui::pos2(0.0, 1.0),
            color: bottom,
        },
        egui::epaint::Vertex {
            pos: rect.right_bottom(),
            uv: egui::pos2(1.0, 1.0),
            color: bottom,
        },
    ]);
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
    painter.add(egui::Shape::Mesh(std::sync::Arc::new(mesh)));
}

/// A rounded card surface. `hovered` lifts the fill slightly.
pub fn card_background(painter: &egui::Painter, rect: egui::Rect, hovered: bool) {
    painter.rect_filled(
        rect,
        CornerRadius::same(8),
        if hovered { CARD_HOVER } else { RAISED },
    );
}

/// Icons use Unicode glyphs that egui falls back to a system font for when
/// SpotifyMixUI does not carry them.
pub mod icon {
    pub const HOME: &str = "⌂";
    pub const SEARCH: &str = "⌕";
    pub const LIBRARY: &str = "♫";
    pub const PLAY: &str = "▶";
    pub const PAUSE: &str = "❚❚";
    pub const NEXT: &str = "⏭";
    pub const PREV: &str = "⏮";
    pub const HEART: &str = "♡";
    pub const SHUFFLE: &str = "⇄";
    pub const REPEAT: &str = "↻";
    pub const PLUS: &str = "+";
    pub const MORE: &str = "⋯";
    pub const VOLUME: &str = "◖";
    pub const EXPAND: &str = "⛶";
    pub const DOWNLOAD: &str = "↓";
}

/// A circular button with an icon glyph.
pub fn round_button(
    ui: &mut egui::Ui,
    glyph: &str,
    size: f32,
    filled: bool,
) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(glyph).color(if filled { BLACK } else { TEXT }).size(size))
            .fill(if filled { GREEN } else { HOVER })
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(50))
            .min_size(Vec2::new(32.0, 32.0)),
    )
}

/// A section heading used above rows and grids of cards.
pub fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .color(TEXT)
            .size(20.0)
            .strong(),
    );
}

/// The primary play/pause control in the player bar.
pub fn play_pause_button(
    ui: &mut egui::Ui,
    paused: bool,
    on_toggle: impl FnOnce(),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), egui::Sense::click());
    let hovered = response.hovered();
    play_button(ui.painter(), rect.center(), 16.0, hovered);
    if !paused {
        // Two bars over the green circle.
        let painter = ui.painter();
        let bar = egui::Stroke::new(3.5, BLACK);
        for offset in [-4.0, 4.0] {
            painter.line_segment(
                [
                    egui::pos2(rect.center().x + offset, rect.center().y - 6.0),
                    egui::pos2(rect.center().x + offset, rect.center().y + 6.0),
                ],
                bar,
            );
        }
    }
    if response.clicked() {
        on_toggle();
    }
    response
}
