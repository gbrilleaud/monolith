use eframe::egui;
use monolith::{style_config::StyleStore, theme};

#[test]
fn egui_spacing_is_not_scaled_by_button_density() {
    let root = tempfile::tempdir().unwrap();
    let style = StyleStore::select(root.path(), "futuristic").unwrap();
    assert_ne!(style.resolved.density, 1.0);
    let context = egui::Context::default();
    theme::install_style(&context, &style.resolved);
    assert_eq!(
        context.style().spacing.item_spacing.x,
        style.resolved.tile_spacing
    );
}

#[test]
fn egui_light_preset_updates_palette_and_spacing() {
    let root = tempfile::tempdir().unwrap();
    let style = StyleStore::select(root.path(), "light").unwrap();
    let context = egui::Context::default();
    theme::install_style(&context, &style.resolved);
    assert_eq!(
        context.style().visuals.panel_fill,
        theme::hex_color(&style.resolved.colors.background).unwrap()
    );
    assert_eq!(
        context.style().visuals.hyperlink_color,
        theme::hex_color(&style.resolved.colors.primary).unwrap()
    );
    assert_eq!(
        context.style().spacing.item_spacing.x,
        style.resolved.tile_spacing
    );
}
