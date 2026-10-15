use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path, time::Duration};

const BUNDLED_CATALOG: &str = include_str!("../assets/theme-presets.json");
pub const MAX_CATALOG_BYTES: usize = 256 * 1024;

pub(crate) async fn read_style_response(mut response: reqwest::Response) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > MAX_CATALOG_BYTES.saturating_sub(bytes.len()) {
            bail!("style catalog exceeds 256 KiB");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleColors {
    pub background: String,
    pub surface: String,
    pub surface_raised: String,
    pub primary: String,
    pub secondary: String,
    pub text: String,
    pub text_muted: String,
    pub border: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StylePreset {
    pub id: String,
    pub name: String,
    pub colors: StyleColors,
    pub font_family: String,
    pub density: f32,
    pub tile_spacing: f32,
    pub corner_radius: f32,
    pub background_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleCatalog {
    pub presets: Vec<StylePreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleSnapshot {
    pub catalog: StyleCatalog,
    pub selected: String,
    pub resolved: StylePreset,
    pub cartridge_animation: bool,
    /// Copie du preset local, utilisée par les éditeurs sans relire `style.json`.
    pub custom: Option<StylePreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalStyle {
    #[serde(default = "default_selected")]
    selected: String,
    #[serde(default = "default_animation")]
    cartridge_animation: bool,
    #[serde(default)]
    custom: Option<StylePreset>,
}

fn default_selected() -> String {
    "dark".into()
}
fn default_animation() -> bool {
    true
}

impl Default for LocalStyle {
    fn default() -> Self {
        Self {
            selected: default_selected(),
            cartridge_animation: true,
            custom: None,
        }
    }
}

pub struct StyleStore;

impl StyleStore {
    /// Explicitly replace damaged local preferences while preserving the cached server catalog.
    pub fn reset(root: &Path) -> Result<StyleSnapshot> {
        let local = LocalStyle::default();
        write_json_atomic(&root.join("style.json"), &local)?;
        snapshot(root, local)
    }

    pub fn load(root: &Path) -> Result<StyleSnapshot> {
        let local = read_local(root)?;
        snapshot(root, local)
    }

    pub fn select(root: &Path, id: &str) -> Result<StyleSnapshot> {
        let mut local = read_local(root)?;
        let current = snapshot(root, local.clone())?;
        if !current.catalog.presets.iter().any(|preset| preset.id == id) {
            bail!("style inconnu: {id}");
        }
        local.selected = id.to_owned();
        write_json_atomic(&root.join("style.json"), &local)?;
        snapshot(root, local)
    }

    pub fn set_animation(root: &Path, enabled: bool) -> Result<StyleSnapshot> {
        let mut local = read_local(root)?;
        local.cartridge_animation = enabled;
        write_json_atomic(&root.join("style.json"), &local)?;
        snapshot(root, local)
    }

    pub fn save_custom_json(root: &Path, json: &str) -> Result<StyleSnapshot> {
        if json.len() > MAX_CATALOG_BYTES {
            bail!("style personnalisé trop volumineux");
        }
        let preset: StylePreset =
            serde_json::from_str(json).context("style personnalisé invalide")?;
        if preset.id != "custom" {
            bail!("l’identifiant du style personnalisé doit être custom");
        }
        validate_preset(&preset)?;
        let mut local = read_local(root)?;
        local.custom = Some(preset);
        local.selected = "custom".into();
        write_json_atomic(&root.join("style.json"), &local)?;
        snapshot(root, local)
    }

    pub fn refresh(root: &Path, backend_url: &str) -> Result<StyleSnapshot> {
        let base = backend_url.trim().trim_end_matches('/').to_owned();
        if !(base.starts_with("http://") || base.starts_with("https://")) {
            bail!("URL backend invalide");
        }
        let body = std::thread::spawn(move || -> Result<Vec<u8>> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async move {
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(3))
                    .build()?;
                let response = client
                    .get(format!("{base}/api/v1/styles"))
                    .send()
                    .await?
                    .error_for_status()?;
                read_style_response(response).await
            })
        })
        .join()
        .map_err(|_| anyhow::anyhow!("échec du rafraîchissement"))??;
        let catalog: StyleCatalog =
            serde_json::from_slice(&body).context("catalogue distant invalide")?;
        validate_catalog(&catalog)?;
        write_json_atomic(&root.join("style-catalog.json"), &catalog)?;
        Self::load(root)
    }
}

pub fn bundled_style_catalog() -> StyleCatalog {
    serde_json::from_str(BUNDLED_CATALOG).expect("catalogue de styles intégré valide")
}

pub fn parse_style_catalog(bytes: &[u8]) -> Result<StyleCatalog> {
    if bytes.len() > MAX_CATALOG_BYTES {
        bail!("catalogue trop volumineux");
    }
    let catalog = serde_json::from_slice(bytes).context("catalogue invalide")?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

pub fn validate_catalog(catalog: &StyleCatalog) -> Result<()> {
    const BUILTIN_IDS: [&str; 7] = [
        "light",
        "dark",
        "windows_xp",
        "pink",
        "back_to_future",
        "zelda",
        "futuristic",
    ];
    if !(BUILTIN_IDS.len()..=64).contains(&catalog.presets.len())
        || catalog
            .presets
            .iter()
            .take(BUILTIN_IDS.len())
            .map(|preset| preset.id.as_str())
            .ne(BUILTIN_IDS)
    {
        bail!("les sept styles intégrés sont requis");
    }
    let mut ids = HashSet::new();
    for preset in &catalog.presets {
        validate_preset(preset)?;
        if preset.id == "custom" || !ids.insert(&preset.id) {
            bail!("identifiant de style réservé ou dupliqué");
        }
    }
    Ok(())
}

fn validate_preset(preset: &StylePreset) -> Result<()> {
    if preset.id.is_empty()
        || preset.id.len() > 64
        || !preset
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        bail!("identifiant de style invalide");
    }
    if preset.name.trim().is_empty() || preset.name.len() > 128 {
        bail!("nom de style invalide");
    }
    if !matches!(preset.font_family.as_str(), "sans" | "serif" | "mono") {
        bail!("famille de police invalide");
    }
    if !preset.density.is_finite() || !(0.75..=1.4).contains(&preset.density) {
        bail!("densité invalide");
    }
    if !preset.tile_spacing.is_finite() || !(4.0..=24.0).contains(&preset.tile_spacing) {
        bail!("espacement invalide");
    }
    if !preset.corner_radius.is_finite() || !(0.0..=24.0).contains(&preset.corner_radius) {
        bail!("rayon invalide");
    }
    for color in [
        &preset.colors.background,
        &preset.colors.surface,
        &preset.colors.surface_raised,
        &preset.colors.primary,
        &preset.colors.secondary,
        &preset.colors.text,
        &preset.colors.text_muted,
        &preset.colors.border,
        &preset.colors.error,
    ] {
        if color.len() != 7
            || !color.starts_with('#')
            || !color[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            bail!("couleur invalide");
        }
    }
    if let Some(image) = &preset.background_image {
        let path = Path::new(image);
        let allowed = path.extension().and_then(|v| v.to_str()).is_some_and(|v| {
            matches!(
                v.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        });
        if image.is_empty()
            || image.contains(['\\', ':'])
            || path.is_absolute()
            || path.file_name().and_then(|v| v.to_str()) != Some(image)
            || !allowed
        {
            bail!("image de fond invalide");
        }
    }
    Ok(())
}

fn read_local(root: &Path) -> Result<LocalStyle> {
    let path = root.join("style.json");
    if !path.exists() {
        return Ok(LocalStyle::default());
    }
    let bytes = std::fs::read(&path)?;
    if bytes.len() > MAX_CATALOG_BYTES {
        bail!("configuration de style trop volumineuse");
    }
    let local: LocalStyle =
        serde_json::from_slice(&bytes).context("configuration de style invalide")?;
    if let Some(custom) = &local.custom {
        if custom.id != "custom" {
            bail!("style personnalisé invalide");
        }
        validate_preset(custom)?;
    }
    Ok(local)
}

fn snapshot(root: &Path, local: LocalStyle) -> Result<StyleSnapshot> {
    let cache = root.join("style-catalog.json");
    let mut catalog = if cache.exists() {
        std::fs::read(&cache)
            .ok()
            .and_then(|bytes| parse_style_catalog(&bytes).ok())
            .unwrap_or_else(bundled_style_catalog)
    } else {
        bundled_style_catalog()
    };
    validate_catalog(&catalog)?;
    let custom = local.custom.clone();
    if let Some(custom) = &local.custom {
        catalog.presets.push(custom.clone());
    }
    let selected = if catalog
        .presets
        .iter()
        .any(|preset| preset.id == local.selected)
    {
        local.selected
    } else {
        "dark".into()
    };
    let resolved = catalog
        .presets
        .iter()
        .find(|preset| preset.id == selected)
        .cloned()
        .or_else(|| {
            bundled_style_catalog()
                .presets
                .into_iter()
                .find(|preset| preset.id == "dark")
        })
        .context("style sombre absent")?;
    Ok(StyleSnapshot {
        catalog,
        selected,
        resolved,
        cartridge_animation: local.cartridge_animation,
        custom,
    })
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let partial = path.with_extension("json.partial");
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(&partial, bytes)?;
    if let Err(error) = std::fs::rename(&partial, path) {
        let _ = std::fs::remove_file(&partial);
        return Err(error.into());
    }
    Ok(())
}
