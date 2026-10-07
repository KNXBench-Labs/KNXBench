//! Read-only snapshots of the configured projects, and the product database.
//!
//! A project is loaded into memory through `knx_store::open_existing_read_only`
//! and its connection closed again, so nothing holds the file between calls.
//! Before each use the file's length and modification time are compared
//! with the snapshot's, and a changed file is reloaded: an agent always sees
//! the last *saved* state, never edits still unsaved in a running KNXBench
//! (ADR-0090).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use knx_core::Project;
use knx_projection::ProjectTree;
use knx_store::Connection;
use serde_json::{json, Value};

use crate::args::{Config, ProductDbChoice};

/// What a file looked like when it was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

impl FileStamp {
    fn of(path: &Path) -> Result<Self, String> {
        let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
        Ok(Self {
            len: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

/// One loaded project and its read model.
pub struct Snapshot {
    pub alias: String,
    pub project: Project,
    /// `knx_projection::build_project_tree(&project)`, the same read model
    /// the UI receives, built once per snapshot.
    pub tree: ProjectTree,
    /// The file's own schema version when it was older and the project was
    /// migrated in memory; `None` when the file was current.
    pub migrated_from: Option<i64>,
    file_name: String,
    stamp: FileStamp,
}

impl Snapshot {
    /// The `source` block of every response about this project.
    pub fn source_json(&self) -> Value {
        json!({
            "project": self.alias,
            "file": self.file_name,
            "state": "saved",
            "fileModified": self.stamp.modified.map(rfc3339),
            "migratedInMemoryFrom": self.migrated_from,
        })
    }
}

fn rfc3339(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

struct Slot {
    alias: String,
    path: PathBuf,
    cached: Mutex<Option<Arc<Snapshot>>>,
}

/// The read-only product database, when one is in use.
pub struct ProductDb {
    pub conn: Mutex<Connection>,
    pub migrated_from: Option<i64>,
}

/// Every configured project plus the product database.
pub struct Workspace {
    slots: Vec<Slot>,
    products: Option<ProductDb>,
    /// Why there is no product database, or `None` when there is one.
    products_absent: Option<String>,
}

impl Workspace {
    /// Loads every configured project and opens the product database, so a
    /// misconfiguration fails at startup rather than on an agent's first
    /// question.
    pub fn open(config: &Config) -> Result<Self, String> {
        let mut slots = Vec::with_capacity(config.projects.len());
        for project in &config.projects {
            let snapshot = load(&project.alias, &project.path)?;
            slots.push(Slot {
                alias: project.alias.clone(),
                path: project.path.clone(),
                cached: Mutex::new(Some(Arc::new(snapshot))),
            });
        }
        let (products, products_absent) = open_products(&config.product_db)?;
        Ok(Self {
            slots,
            products,
            products_absent,
        })
    }

    /// The configured aliases, in command-line order.
    pub fn aliases(&self) -> Vec<&str> {
        self.slots.iter().map(|slot| slot.alias.as_str()).collect()
    }

    /// The current snapshot of `alias`, reloaded first when the file
    /// changed since it was read. A file that can no longer be read is an
    /// error, never an old snapshot passed off as current.
    pub fn snapshot(&self, alias: &str) -> Result<Arc<Snapshot>, String> {
        let slot = self
            .slots
            .iter()
            .find(|slot| slot.alias == alias)
            .ok_or_else(|| {
                format!(
                    "unknown project {alias:?}; configured: {}",
                    self.aliases().join(", ")
                )
            })?;
        let mut cached = slot.cached.lock().expect("snapshot mutex poisoned");
        let current = FileStamp::of(&slot.path)
            .map_err(|e| format!("project {alias:?} can no longer be read: {e}"))?;
        if let Some(snapshot) = cached.as_ref() {
            if snapshot.stamp == current {
                return Ok(Arc::clone(snapshot));
            }
        }
        *cached = None;
        let fresh = Arc::new(load(alias, &slot.path)?);
        *cached = Some(Arc::clone(&fresh));
        Ok(fresh)
    }

    pub fn products(&self) -> Option<&ProductDb> {
        self.products.as_ref()
    }

    /// The `productDatabase` block of `project_summary`.
    pub fn products_json(&self) -> Value {
        match (&self.products, &self.products_absent) {
            (Some(db), _) => json!({
                "available": true,
                "migratedInMemoryFrom": db.migrated_from,
            }),
            (None, reason) => json!({
                "available": false,
                "reason": reason,
            }),
        }
    }
}

fn load(alias: &str, path: &Path) -> Result<Snapshot, String> {
    // Stamped before reading: a save racing this load changes the stamp
    // afterwards, so the next call reloads instead of keeping a mix.
    let stamp = FileStamp::of(path).map_err(|e| format!("project {alias:?}: {e}"))?;
    let store = knx_store::open_existing_read_only(path)
        .map_err(|e| format!("project {alias:?} cannot be opened read-only: {e}"))?;
    let project = knx_store::load_project(&store.conn)
        .map_err(|e| format!("project {alias:?} cannot be loaded: {e}"))?;
    let migrated_from = store.migrated_from;
    drop(store);
    let tree = knx_projection::build_project_tree(&project);
    Ok(Snapshot {
        alias: alias.to_string(),
        project,
        tree,
        migrated_from,
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        stamp,
    })
}

fn open_products(choice: &ProductDbChoice) -> Result<(Option<ProductDb>, Option<String>), String> {
    let path = match choice {
        ProductDbChoice::Disabled => {
            return Ok((None, Some("disabled with --no-product-db".into())));
        }
        ProductDbChoice::Path(path) => path.clone(),
        ProductDbChoice::Default => match knx_productdb::default_path() {
            Some(path) if path.exists() => path,
            _ => return Ok((None, Some("no product database is installed".into()))),
        },
    };
    let db = knx_productdb::open_read_only(&path)
        .map_err(|e| format!("product database cannot be opened read-only: {e}"))?;
    Ok((
        Some(ProductDb {
            conn: Mutex::new(db.conn),
            migrated_from: db.migrated_from,
        }),
        None,
    ))
}
