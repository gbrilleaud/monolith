use monolith::flutter_api::{
    save_custom_style, select_style, set_cartridge_animation, style_snapshot_json,
};

#[test]
fn flutter_style_api_persists_selection_animation_and_custom_palette() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().to_string_lossy().into_owned();
    let initial: serde_json::Value =
        serde_json::from_str(&style_snapshot_json(data.clone()).unwrap()).unwrap();
    assert_eq!(initial["selected"], "dark");

    let chosen: serde_json::Value =
        serde_json::from_str(&select_style(data.clone(), "pink".into()).unwrap()).unwrap();
    assert_eq!(chosen["resolved"]["id"], "pink");

    let disabled: serde_json::Value =
        serde_json::from_str(&set_cartridge_animation(data.clone(), false).unwrap()).unwrap();
    assert_eq!(disabled["cartridge_animation"], false);
    assert_eq!(disabled["selected"], "pink");

    let mut custom = disabled["resolved"].clone();
    custom["id"] = serde_json::json!("custom");
    custom["name"] = serde_json::json!("Ma palette");
    custom["colors"]["primary"] = serde_json::json!("#123456");
    let saved: serde_json::Value =
        serde_json::from_str(&save_custom_style(data.clone(), custom.to_string()).unwrap())
            .unwrap();
    assert_eq!(saved["selected"], "custom");
    assert_eq!(saved["resolved"]["colors"]["primary"], "#123456");
    assert_eq!(saved["cartridge_animation"], false);
    let restored: serde_json::Value =
        serde_json::from_str(&style_snapshot_json(data).unwrap()).unwrap();
    assert_eq!(restored, saved);
}
