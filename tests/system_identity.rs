use monolith::system_identity::{detect_system_identity, SystemIdentity};

#[test]
fn detects_catalog_aliases_despite_accents_case_and_spacing() {
    let cases = [
        ("  SÉGA   Méga-Drive  ", SystemIdentity::MegaDrive),
        ("GENESIS", SystemIdentity::MegaDrive),
        ("sega_master-system", SystemIdentity::MasterSystem),
        ("Game   Gear", SystemIdentity::GameGear),
        ("Sony PlayStation 2", SystemIdentity::PlayStation2),
        ("MICROSOFT-XBOX_360", SystemIdentity::Xbox360),
    ];

    for (name, expected) in cases {
        assert_eq!(
            detect_system_identity(Some(name), None, None),
            expected,
            "alias non reconnu : {name}"
        );
    }
}

#[test]
fn detects_identity_from_catalog_slug() {
    assert_eq!(
        detect_system_identity(None, Some("sega-mega-drive-genesis"), None),
        SystemIdentity::MegaDrive
    );
    assert_eq!(
        detect_system_identity(None, Some("playstation_2"), None),
        SystemIdentity::PlayStation2
    );
}

#[test]
fn conflicting_or_ambiguous_catalog_labels_are_unknown() {
    let cases = [
        (Some("PlayStation"), None),
        (Some("Xbox"), None),
        (Some("PlayStation 20"), None),
        (Some("Xbox 360 Controller"), None),
        (Some("Master System / Game Gear"), None),
        (Some("Mega Drive"), Some("playstation-2")),
    ];

    for (name, slug) in cases {
        assert_eq!(
            detect_system_identity(name, slug, Some(11)),
            SystemIdentity::Unknown,
            "libellé ambigu reconnu à tort : name={name:?}, slug={slug:?}"
        );
    }
}

#[test]
fn legacy_id_11_is_used_only_when_catalog_identity_is_unavailable() {
    assert_eq!(
        detect_system_identity(None, None, Some(11)),
        SystemIdentity::MegaDrive
    );
    assert_eq!(
        detect_system_identity(Some("  "), Some(""), Some(11)),
        SystemIdentity::MegaDrive
    );
    assert_eq!(
        detect_system_identity(None, None, Some(360)),
        SystemIdentity::Unknown
    );
    assert_eq!(
        detect_system_identity(Some("Système inconnu"), None, Some(11)),
        SystemIdentity::Unknown
    );
}

#[test]
fn canonical_keys_are_stable_and_unknown_is_explicit() {
    assert_eq!(SystemIdentity::MegaDrive.as_key(), "mega-drive");
    assert_eq!(SystemIdentity::MasterSystem.as_key(), "master-system");
    assert_eq!(SystemIdentity::GameGear.as_key(), "game-gear");
    assert_eq!(SystemIdentity::PlayStation2.as_key(), "playstation-2");
    assert_eq!(SystemIdentity::Xbox360.as_key(), "xbox-360");
    assert_eq!(SystemIdentity::Unknown.as_key(), "unknown");
}
