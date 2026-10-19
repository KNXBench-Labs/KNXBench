//! Read-only catalogue provenance: placement, exact program and recorded source names.

use std::collections::{BTreeSet, HashMap};

use rusqlite::Connection;
use serde::Serialize;

use crate::query::CatalogItemRow;
use crate::ProductDbError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItemDetails {
    pub application_program_id: Option<String>,
    pub application_name: Option<String>,
    pub application_version: Option<String>,
    pub catalog_section_id: Option<String>,
    pub source_names: Vec<String>,
}

/// Batch the provenance query, rather than issuing queries for every listed item.
/// Package aliases describe the actual winning catalogue source, not whichever
/// package most recently supplied the same product or application ID.
pub(crate) fn attach_details(
    conn: &Connection,
    items: &mut [CatalogItemRow],
) -> Result<(), ProductDbError> {
    if items.is_empty() {
        return Ok(());
    }
    let indexes: HashMap<&str, usize> = items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.id.as_str(), i))
        .collect();
    let mut details: HashMap<usize, (CatalogItemDetails, BTreeSet<String>)> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT ci.id, ap.id, ap.name, ap.application_version, ci.section_id,
                COALESCE(psn.source_name, p.source_name), sf.source_path
         FROM catalog_item ci
         LEFT JOIN hardware2program hp ON hp.id = ci.hardware2program_ref_id
         LEFT JOIN application_program ap ON ap.id = hp.application_program_ref
         LEFT JOIN package_member pm ON pm.source_sha256 = ci.source_sha256
         LEFT JOIN package p ON p.sha256 = pm.package_sha256
         LEFT JOIN package_source_name psn ON psn.package_sha256 = p.sha256
         LEFT JOIN source_file sf ON sf.sha256 = ci.source_sha256",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let Some(&index) = indexes.get(id.as_str()) else {
            continue;
        };
        let (_, names) = details.entry(index).or_insert((
            CatalogItemDetails {
                application_program_id: row.get(1)?,
                application_name: row.get(2)?,
                application_version: row.get(3)?,
                catalog_section_id: row.get(4)?,
                source_names: Vec::new(),
            },
            BTreeSet::new(),
        ));
        let package_name: Option<String> = row.get(5)?;
        let source_path: Option<String> = row.get(6)?;
        if let Some(name) = package_name {
            names.insert(name);
        } else if let Some(path) = source_path {
            // Standalone XML ingests have no package name. Display only the
            // original member/file name, never an absolute ingestion path.
            if let Some(name) = path.rsplit(['/', '\\']).next().filter(|s| !s.is_empty()) {
                names.insert(name.to_owned());
            }
        }
    }
    drop(indexes);
    for (index, (mut detail, names)) in details {
        detail.source_names = names.into_iter().collect();
        items[index].details = Some(detail);
    }
    Ok(())
}
