use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogViewMode {
    Tiles,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSortColumn {
    Title,
    FileName,
    Favourite,
    AddedAt,
    SizeBytes,
    Extension,
    Availability,
    Rating,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogSort {
    pub column: CatalogSortColumn,
    pub direction: SortDirection,
}

impl CatalogSort {
    pub const fn new(column: CatalogSortColumn, direction: SortDirection) -> Self {
        Self { column, direction }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRow {
    pub system_id: i64,
    pub title: String,
    pub file_name: String,
    pub path: String,
    pub extension: String,
    pub favourite: bool,
    pub added_at: String,
    pub size_bytes: u64,
    pub rating: Option<u8>,
    pub available: bool,
    pub cover_art: Option<String>,
    pub game_id: Option<i64>,
}

pub fn glob_matches(value: &str, pattern: &str) -> bool {
    let value = value.to_lowercase().chars().collect::<Vec<_>>();
    let pattern = pattern.to_lowercase().chars().collect::<Vec<_>>();
    let mut matches = vec![vec![false; pattern.len() + 1]; value.len() + 1];
    matches[0][0] = true;
    for index in 1..=pattern.len() {
        matches[0][index] = pattern[index - 1] == '*' && matches[0][index - 1];
    }
    for value_index in 1..=value.len() {
        for pattern_index in 1..=pattern.len() {
            matches[value_index][pattern_index] = match pattern[pattern_index - 1] {
                '*' => {
                    matches[value_index][pattern_index - 1]
                        || matches[value_index - 1][pattern_index]
                }
                '?' => matches[value_index - 1][pattern_index - 1],
                literal => {
                    literal == value[value_index - 1] && matches[value_index - 1][pattern_index - 1]
                }
            };
        }
    }
    matches[value.len()][pattern.len()]
}

pub fn filter_and_sort_catalog_rows(
    rows: &[CatalogRow],
    query: &str,
    sort: CatalogSort,
) -> Vec<CatalogRow> {
    let pattern = if query.trim().is_empty() {
        "*"
    } else {
        query.trim()
    };
    let mut filtered = rows
        .iter()
        .filter(|row| {
            glob_matches(&row.title, pattern)
                || glob_matches(&row.file_name, pattern)
                || glob_matches(&row.path, pattern)
        })
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| compare_rows(left, right, sort));
    filtered
}

fn compare_rows(left: &CatalogRow, right: &CatalogRow, sort: CatalogSort) -> Ordering {
    let primary = match sort.column {
        CatalogSortColumn::Title => left.title.cmp(&right.title),
        CatalogSortColumn::FileName => left.file_name.cmp(&right.file_name),
        CatalogSortColumn::Favourite => left.favourite.cmp(&right.favourite),
        CatalogSortColumn::AddedAt => left.added_at.cmp(&right.added_at),
        CatalogSortColumn::SizeBytes => left.size_bytes.cmp(&right.size_bytes),
        CatalogSortColumn::Extension => left.extension.cmp(&right.extension),
        CatalogSortColumn::Availability => left.available.cmp(&right.available),
        CatalogSortColumn::Rating => left.rating.cmp(&right.rating),
        CatalogSortColumn::Path => left.path.cmp(&right.path),
    };
    let ordered = if sort.direction == SortDirection::Descending {
        primary.reverse()
    } else {
        primary
    };
    ordered
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.path.cmp(&right.path))
}
