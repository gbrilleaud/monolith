use crate::style_config::StylePreset;
use eframe::egui::{self, Color32, FontFamily, FontId, Margin, Rounding, Stroke};

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

pub fn hex_color(value: &str) -> Option<Color32> {
    let digits = value.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let rgb = u32::from_str_radix(digits, 16).ok()?;
    Some(Color32::from_rgb(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
    ))
}

/// Interpret the same canonical JSON tokens as Flutter without CSS/runtime code.
pub fn install_style(context: &egui::Context, preset: &StylePreset) {
    let color = |value: &str| hex_color(value).unwrap_or(Color32::GRAY);
    let background = color(&preset.colors.background);
    let surface = color(&preset.colors.surface);
    let raised = color(&preset.colors.surface_raised);
    let primary = color(&preset.colors.primary);
    let text = color(&preset.colors.text);
    let foreground_for = |fill: Color32| {
        if u16::from(fill.r()) + u16::from(fill.g()) + u16::from(fill.b()) > 400 {
            Color32::BLACK
        } else {
            Color32::WHITE
        }
    };
    let light =
        u16::from(background.r()) + u16::from(background.g()) + u16::from(background.b()) > 380;
    let mut visuals = if light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    visuals.panel_fill = background;
    visuals.window_fill = surface;
    visuals.faint_bg_color = raised;
    visuals.extreme_bg_color = background;
    visuals.widgets.noninteractive.bg_fill = surface;
    visuals.widgets.inactive.bg_fill = raised;
    visuals.widgets.hovered.bg_fill = color(&preset.colors.secondary);
    visuals.widgets.active.bg_fill = primary;
    visuals.widgets.open.bg_fill = raised;
    visuals.widgets.noninteractive.fg_stroke.color = color(&preset.colors.text_muted);
    visuals.widgets.inactive.fg_stroke.color = text;
    visuals.widgets.hovered.fg_stroke.color = foreground_for(color(&preset.colors.secondary));
    visuals.widgets.active.fg_stroke.color = foreground_for(primary);
    visuals.override_text_color = Some(text);
    visuals.selection.bg_fill = primary;
    visuals.selection.stroke.color = foreground_for(primary);
    visuals.hyperlink_color = primary;
    visuals.warn_fg_color = color(&preset.colors.secondary);
    visuals.error_fg_color = color(&preset.colors.error);
    visuals.window_stroke.color = color(&preset.colors.border);
    visuals.widgets.noninteractive.bg_stroke.color = color(&preset.colors.border);
    visuals.window_rounding = Rounding::same(preset.corner_radius);
    visuals.menu_rounding = Rounding::same(preset.corner_radius);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.rounding = Rounding::same(preset.corner_radius);
    }
    context.set_visuals(visuals);
    context.style_mut(|style| {
        let gap = preset.tile_spacing;
        style.spacing.item_spacing = egui::vec2(gap, gap);
        style.spacing.button_padding = egui::vec2(12.0 * preset.density, 7.0 * preset.density);
        style.spacing.window_margin = Margin::same(16.0 * preset.density);
        let family = if preset.font_family == "mono" {
            FontFamily::Monospace
        } else {
            FontFamily::Proportional
        };
        for (text_style, size) in [
            (egui::TextStyle::Heading, 24.0),
            (egui::TextStyle::Body, 15.0),
            (egui::TextStyle::Button, 15.0),
            (egui::TextStyle::Small, 12.0),
        ] {
            style
                .text_styles
                .insert(text_style, FontId::new(size, family.clone()));
        }
    });
}

pub fn card_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::none()
        .fill(ui.visuals().window_fill)
        .stroke(Stroke::new(1.0_f32, ui.visuals().window_stroke.color))
        .rounding(ui.visuals().window_rounding)
        .inner_margin(Margin::same(8.0))
}

pub fn muted(ui: &mut egui::Ui, text: impl AsRef<str>) {
    ui.colored_label(
        ui.visuals().widgets.noninteractive.fg_stroke.color,
        text.as_ref(),
    );
}
