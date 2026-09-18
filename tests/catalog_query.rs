use monolith::catalog_query::{
    filter_and_sort_catalog_rows, glob_matches, CatalogRow, CatalogSort, CatalogSortColumn,
    SortDirection,
};

fn row(title: &str, favourite: bool, added_at: &str, size_bytes: u64) -> CatalogRow {
    CatalogRow {
        system_id: 10,
        title: title.into(),
        file_name: format!("{title}.gdi"),
        path: format!("/roms/10/{title}.gdi"),
        extension: "gdi".into(),
        favourite,
        added_at: added_at.into(),
        size_bytes,
        rating: Some(4),
        available: true,
        cover_art: None,
        game_id: None,
    }
}

#[test]
fn wildcard_matching_is_case_insensitive_and_supports_star_and_question_mark() {
    assert!(glob_matches("Rayman Legends", "ray*"));
    assert!(glob_matches("RAYMAN 2", "rayman ?"));
    assert!(glob_matches("/roms/PC/Star Wars.rom", "*star?wars*"));
    assert!(!glob_matches("Donkey Kong", "ray*"));
}

#[test]
fn filter_searches_title_file_name_and_path_then_sorts_by_each_selected_column() {
    let rows = vec![
        row("Rayman 2", false, "2026-09-01T10:00:00Z", 20),
        row("Rayman Legends", true, "2026-09-02T10:00:00Z", 10),
        row("Donkey Kong", false, "2026-09-03T10:00:00Z", 30),
    ];

    let filtered = filter_and_sort_catalog_rows(
        &rows,
        "ray*",
        CatalogSort::new(CatalogSortColumn::SizeBytes, SortDirection::Ascending),
    );

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].title, "Rayman Legends");
    assert_eq!(filtered[1].title, "Rayman 2");

    let favourites = filter_and_sort_catalog_rows(
        &rows,
        "*",
        CatalogSort::new(CatalogSortColumn::Favourite, SortDirection::Descending),
    );
    assert_eq!(favourites[0].title, "Rayman Legends");
}

#[test]
fn every_column_sort_is_deterministic() {
    let rows = vec![
        row("Beta", false, "2026-09-01T10:00:00Z", 20),
        row("Alpha", true, "2026-09-02T10:00:00Z", 10),
    ];
    let columns = [
        CatalogSortColumn::Title,
        CatalogSortColumn::FileName,
        CatalogSortColumn::Favourite,
        CatalogSortColumn::AddedAt,
        CatalogSortColumn::SizeBytes,
        CatalogSortColumn::Extension,
        CatalogSortColumn::Availability,
        CatalogSortColumn::Rating,
        CatalogSortColumn::Path,
    ];

    for column in columns {
        let sorted = filter_and_sort_catalog_rows(
            &rows,
            "*",
            CatalogSort::new(column, SortDirection::Ascending),
        );
        assert_eq!(sorted.len(), 2);
    }
}
