use eframe::egui::{self, Color32, FontId, Margin, Rounding, Stroke};

pub const ACCENT: Color32 = Color32::from_rgb(93, 196, 152);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(123, 222, 177);
pub const SURFACE: Color32 = Color32::from_rgb(24, 28, 35);
pub const SURFACE_RAISED: Color32 = Color32::from_rgb(34, 40, 49);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(163, 173, 187);

pub fn install(context: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(17, 20, 26);
    visuals.window_fill = SURFACE;
    visuals.faint_bg_color = Color32::from_rgb(30, 35, 44);
    visuals.extreme_bg_color = Color32::from_rgb(11, 13, 17);
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.inactive.bg_fill = SURFACE_RAISED;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(52, 62, 72);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    visuals.selection.bg_fill = Color32::from_rgb(42, 110, 88);
    visuals.hyperlink_color = ACCENT_HOVER;
    visuals.window_rounding = Rounding::same(10.0);
    visuals.menu_rounding = Rounding::same(8.0);
    visuals.widgets.inactive.rounding = Rounding::same(7.0);
    visuals.widgets.hovered.rounding = Rounding::same(7.0);
    visuals.widgets.active.rounding = Rounding::same(7.0);
    context.set_visuals(visuals);
    context.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.window_margin = Margin::same(16.0);
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(24.0));
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.0));
    });
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(54, 63, 75)))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(8.0))
}

pub fn muted(ui: &mut egui::Ui, text: impl AsRef<str>) {
    ui.colored_label(TEXT_MUTED, text.as_ref());
}
