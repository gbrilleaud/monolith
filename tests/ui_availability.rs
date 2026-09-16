use monolith::{models::LaunchAvailability, ui::availability_label};

#[test]
fn availability_label_reports_available_location_count() {
    assert_eq!(
        availability_label(&LaunchAvailability {
            available: true,
            location_count: 2,
            preferred_path: Some("/library/Rayman 2.chd".into()),
        }),
        "Disponible · 2 emplacements"
    );
}

#[test]
fn availability_label_reports_an_absent_game() {
    assert_eq!(
        availability_label(&LaunchAvailability::default()),
        "Absent de la bibliothèque"
    );
}
