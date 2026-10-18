//! Read-only, snapshot-labelled storage metrics for the Overview.
use std::{fs, io, path::Path};

use axum::{extract::State, Json};
use serde::Serialize;

use crate::SharedState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFileMetric {
    state: &'static str,
    project_incarnation: Option<u64>,
    snapshot_revision: Option<u64>,
    bytes: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogueCounts {
    manufacturers: u64,
    products: u64,
    application_programs: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogueMetric {
    state: &'static str,
    bytes: Option<u64>,
    counts: Option<CatalogueCounts>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusMetrics {
    project: ProjectFileMetric,
    catalogue: CatalogueMetric,
}

fn file_bytes(path: &Path) -> io::Result<u64> {
    Ok(fs::metadata(path)?.len())
}

/// SQLite sidecars belong to the active database's on-disk footprint. Metadata
/// reads never open, checkpoint, vacuum or write to the database.
fn database_bytes(path: &Path) -> io::Result<u64> {
    let mut total = file_bytes(path)?;
    for suffix in ["-wal", "-shm"] {
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        match fs::metadata(Path::new(&name)) {
            Ok(metadata) => total = total.saturating_add(metadata.len()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(total)
}

pub(crate) async fn current(State(state): State<SharedState>) -> Json<StatusMetrics> {
    // The short metadata read shares the lock that publishes a replacement or
    // save. Thus the measured path belongs to the revision in this response.
    let project = {
        let open = state.project.lock().expect("project poisoned");
        if open.is_none() {
            ProjectFileMetric {
                state: "unavailable",
                project_incarnation: None,
                snapshot_revision: None,
                bytes: None,
            }
        } else {
            let incarnation = state
                .project_incarnation
                .load(std::sync::atomic::Ordering::Relaxed);
            let revision = state
                .project_revision
                .load(std::sync::atomic::Ordering::Relaxed);
            let path = state.store_path.lock().expect("store path poisoned");
            match path.as_ref() {
                None => ProjectFileMetric {
                    state: "notSavedYet",
                    project_incarnation: Some(incarnation),
                    snapshot_revision: Some(revision),
                    bytes: None,
                },
                Some(path) => match file_bytes(path) {
                    Ok(bytes) => ProjectFileMetric {
                        state: "available",
                        project_incarnation: Some(incarnation),
                        snapshot_revision: Some(revision),
                        bytes: Some(bytes),
                    },
                    Err(_) => ProjectFileMetric {
                        state: "error",
                        project_incarnation: Some(incarnation),
                        snapshot_revision: Some(revision),
                        bytes: None,
                    },
                },
            }
        }
    };
    let catalogue = match state.product_db.as_ref() {
        None => CatalogueMetric {
            state: "unavailable",
            bytes: None,
            counts: None,
        },
        Some(db) => {
            let db = db.lock().expect("product DB poisoned");
            let measurement = db
                .path()
                .filter(|path| !path.is_empty())
                .and_then(|path| database_bytes(Path::new(path)).ok());
            let counts = knx_productdb::query::catalogue_counts(&db).ok();
            match (measurement, counts) {
                (Some(bytes), Some(counts)) => CatalogueMetric {
                    state: "available",
                    bytes: Some(bytes),
                    counts: Some(CatalogueCounts {
                        manufacturers: counts.manufacturers,
                        products: counts.products,
                        application_programs: counts.application_programs,
                    }),
                },
                _ => CatalogueMetric {
                    state: "error",
                    bytes: None,
                    counts: None,
                },
            }
        }
    };
    Json(StatusMetrics { project, catalogue })
}

#[cfg(test)]
mod tests {
    use super::database_bytes;

    #[test]
    fn database_bytes_include_present_sidecars_without_modifying_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("active.sqlite");
        std::fs::write(&path, [0; 1_000_000]).unwrap();
        std::fs::write(dir.path().join("active.sqlite-wal"), [0; 200_000]).unwrap();
        std::fs::write(dir.path().join("active.sqlite-shm"), [0; 8_000]).unwrap();
        std::fs::write(dir.path().join("source.knxprod"), [0; 50_000]).unwrap();
        assert_eq!(database_bytes(&path).unwrap(), 1_208_000);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 1_000_000);
    }
}
