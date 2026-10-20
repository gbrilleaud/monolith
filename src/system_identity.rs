/// Identité stable déduite des métadonnées textuelles du catalogue.
///
/// Cette classification décrit uniquement un système : elle ne garantit pas
/// qu'un moteur soit installé, compatible ou prêt à lancer un jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemIdentity {
    MegaDrive,
    MasterSystem,
    GameGear,
    PlayStation2,
    Xbox360,
    Unknown,
}

impl SystemIdentity {
    /// Clé stable destinée aux associations avec les futurs moteurs partagés.
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::MegaDrive => "mega-drive",
            Self::MasterSystem => "master-system",
            Self::GameGear => "game-gear",
            Self::PlayStation2 => "playstation-2",
            Self::Xbox360 => "xbox-360",
            Self::Unknown => "unknown",
        }
    }
}

/// Déduit une identité du nom et/ou du slug réellement fournis par le catalogue.
///
/// Un nom et un slug reconnus mais contradictoires produisent `Unknown`. L'ancien
/// identifiant explicite `11` n'est conservé comme secours Mega Drive que lorsque
/// les deux champs textuels sont absents ou blancs. Aucun autre identifiant
/// numérique n'est interprété, car les identifiants SQLite ne sont pas stables.
pub fn detect_system_identity(
    catalog_name: Option<&str>,
    catalog_slug: Option<&str>,
    legacy_system_id: Option<i64>,
) -> SystemIdentity {
    let labels = [catalog_name, catalog_slug];
    let mut has_catalog_label = false;
    let mut detected = None;

    for label in labels.into_iter().flatten() {
        if label.trim().is_empty() {
            continue;
        }
        has_catalog_label = true;
        let Some(identity) = identity_for_label(label) else {
            continue;
        };
        if detected.is_some_and(|current| current != identity) {
            return SystemIdentity::Unknown;
        }
        detected = Some(identity);
    }

    if let Some(identity) = detected {
        identity
    } else if !has_catalog_label && legacy_system_id == Some(11) {
        SystemIdentity::MegaDrive
    } else {
        SystemIdentity::Unknown
    }
}

fn identity_for_label(label: &str) -> Option<SystemIdentity> {
    let normalized = normalize_label(label);
    match normalized.as_str() {
        "mega drive"
        | "megadrive"
        | "genesis"
        | "sega mega drive"
        | "sega megadrive"
        | "sega genesis"
        | "mega drive genesis"
        | "genesis mega drive"
        | "sega mega drive genesis"
        | "sega genesis mega drive" => Some(SystemIdentity::MegaDrive),
        "master system" | "mastersystem" | "sega master system" | "sms" => {
            Some(SystemIdentity::MasterSystem)
        }
        "game gear" | "gamegear" | "sega game gear" | "gg" => Some(SystemIdentity::GameGear),
        "playstation 2" | "playstation2" | "sony playstation 2" | "ps 2" | "ps2" => {
            Some(SystemIdentity::PlayStation2)
        }
        "xbox 360" | "xbox360" | "microsoft xbox 360" => Some(SystemIdentity::Xbox360),
        _ => None,
    }
}

fn normalize_label(label: &str) -> String {
    let mut normalized = String::with_capacity(label.len());
    let mut separator_pending = false;

    for character in label.chars() {
        if is_combining_mark(character) {
            continue;
        }
        if let Some(folded) = fold_latin_character(character) {
            if separator_pending && !normalized.is_empty() {
                normalized.push(' ');
            }
            separator_pending = false;
            normalized.push_str(folded);
        } else if character.is_ascii_alphanumeric() {
            if separator_pending && !normalized.is_empty() {
                normalized.push(' ');
            }
            separator_pending = false;
            normalized.push(character.to_ascii_lowercase());
        } else {
            separator_pending = true;
        }
    }

    normalized
}

fn is_combining_mark(character: char) -> bool {
    matches!(
        character,
        '\u{0300}'..='\u{036f}'
            | '\u{1ab0}'..='\u{1aff}'
            | '\u{1dc0}'..='\u{1dff}'
            | '\u{20d0}'..='\u{20ff}'
            | '\u{fe20}'..='\u{fe2f}'
    )
}

fn fold_latin_character(character: char) -> Option<&'static str> {
    match character.to_lowercase().next()? {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => Some("a"),
        'æ' => Some("ae"),
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => Some("c"),
        'ď' | 'đ' => Some("d"),
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => Some("e"),
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => Some("g"),
        'ĥ' | 'ħ' => Some("h"),
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => Some("i"),
        'ĵ' => Some("j"),
        'ķ' => Some("k"),
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => Some("l"),
        'ñ' | 'ń' | 'ņ' | 'ň' => Some("n"),
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => Some("o"),
        'œ' => Some("oe"),
        'ŕ' | 'ŗ' | 'ř' => Some("r"),
        'ś' | 'ŝ' | 'ş' | 'š' | 'ß' => Some("s"),
        'ţ' | 'ť' | 'ŧ' => Some("t"),
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => Some("u"),
        'ŵ' => Some("w"),
        'ý' | 'ÿ' | 'ŷ' => Some("y"),
        'ź' | 'ż' | 'ž' => Some("z"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_handles_precomposed_and_combining_accents() {
        assert_eq!(normalize_label(" SÉGA__Méga-Drive "), "sega mega drive");
        assert_eq!(normalize_label("Me\u{301}ga Drive"), "mega drive");
    }

    #[test]
    fn a_known_slug_can_resolve_an_unrecognized_display_name() {
        assert_eq!(
            detect_system_identity(Some("Console de salon"), Some("xbox-360"), None),
            SystemIdentity::Xbox360
        );
    }

    #[test]
    fn legacy_fallback_does_not_assign_other_numeric_ids() {
        assert_eq!(
            detect_system_identity(None, None, Some(2)),
            SystemIdentity::Unknown
        );
    }
}
