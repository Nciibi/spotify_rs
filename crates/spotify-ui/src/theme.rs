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

/// Apply the Spotify look to egui's global style.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);

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
