use crate::models::{ScanIssue, ScanObservation, ScanReport, ScanRoot};
use anyhow::Result;
use std::{collections::BTreeSet, fs, path::Path, time::UNIX_EPOCH};

pub fn scan_root(root: &ScanRoot) -> Result<ScanReport> {
    root.validate()?;
    let root_path = Path::new(&root.path);
    let canonical_root = match root_path.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            return Ok(ScanReport {
                issues: vec![ScanIssue {
                    path: root_path.display().to_string(),
                    message: format!("racine de scan inaccessible : {error}"),
                }],
                ..ScanReport::default()
            });
        }
    };
    let accepted_extensions = root
        .extensions
        .iter()
        .map(|extension| {
            extension
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase()
        })
        .filter(|extension| !extension.is_empty())
        .collect::<BTreeSet<_>>();

    let mut report = ScanReport::default();
    let mut pending = vec![canonical_root];
    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                report.issues.push(ScanIssue {
                    path: directory.display().to_string(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        let mut paths = entries
            .filter_map(|entry| entry.map(|entry| entry.path()).ok())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    report.issues.push(ScanIssue {
                        path: path.display().to_string(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            report.visited += 1;
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase());
            let Some(extension) =
                extension.filter(|extension| accepted_extensions.contains(extension))
            else {
                report.ignored += 1;
                continue;
            };
            let canonical_path = path.canonicalize().unwrap_or(path.clone());
            report.observations.push(ScanObservation {
                system_id: root.system_id,
                path: canonical_path.display().to_string(),
                extension,
                size_bytes: metadata.len(),
                modified_at: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs() as i64),
            });
            report.accepted += 1;
        }
    }
    report
        .observations
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(report)
}
