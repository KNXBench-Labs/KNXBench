//! Read-only catalogue provenance: placement, exact program and recorded source names.

use std::collections::{BTreeSet, HashMap};

use rusqlite::Connection;
use serde::Serialize;

use crate::query::CatalogItemRow;
use crate::{ProductDbError, WRAPPER_SOURCE_SEPARATOR};

/// IDs per statement: far below SQLite's bound parameter limit, so one
/// listing of a large catalogue costs a few indexed statements, not one each.
pub(crate) const ID_CHUNK: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItemDetails {
    pub application_program_id: Option<String>,
    pub application_name: Option<String>,
    pub application_version: Option<String>,
    pub catalog_section_id: Option<String>,
    pub source_names: Vec<String>,
}

/// `?,?,…` for `count` bound parameters.
pub(crate) fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(",")
}

fn file_name(path: &str) -> Option<&str> {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
}

/// The recorded source name without any directory: a package installed from
/// `/home/alice/Downloads/x.knxprod` shows `x.knxprod`, an inner package of a
/// vendor ZIP shows `vendor.zip!/inner.knxprod`. Local paths never leave the
/// server through this field.
fn display_source_name(recorded: &str) -> Option<String> {
    match recorded.split_once(WRAPPER_SOURCE_SEPARATOR) {
        Some((outer, inner)) => Some(format!(
            "{}{WRAPPER_SOURCE_SEPARATOR}{}",
            file_name(outer)?,
            file_name(inner)?
        )),
        None => file_name(recorded).map(str::to_owned),
    }
}

/// Attach provenance to exactly the given rows, reading only those rows'
/// placements. Package aliases describe the actual winning catalogue source,
/// not whichever package most recently supplied the same product or program.
pub(crate) fn attach_details(
    conn: &Connection,
    items: &mut [CatalogItemRow],
) -> Result<(), ProductDbError> {
    let indexes: HashMap<String, usize> = items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.id.clone(), i))
        .collect();
    let ids: Vec<&str> = indexes.keys().map(String::as_str).collect();
    let mut details: HashMap<usize, (CatalogItemDetails, BTreeSet<String>)> = HashMap::new();
    for chunk in ids.chunks(ID_CHUNK) {
        let mut stmt = conn.prepare(&format!(
            "SELECT ci.id, ap.id, ap.name, ap.application_version, ci.section_id,
                    COALESCE(psn.source_name, p.source_name), sf.source_path
             FROM catalog_item ci
             LEFT JOIN hardware2program hp ON hp.id = ci.hardware2program_ref_id
             LEFT JOIN application_program ap ON ap.id = hp.application_program_ref
             LEFT JOIN package_member pm ON pm.source_sha256 = ci.source_sha256
             LEFT JOIN package p ON p.sha256 = pm.package_sha256
             LEFT JOIN package_source_name psn ON psn.package_sha256 = p.sha256
             LEFT JOIN source_file sf ON sf.sha256 = ci.source_sha256
             WHERE ci.id IN ({})",
            placeholders(chunk.len())
        ))?;
        let mut rows = stmt.query(rusqlite::params_from_iter(chunk.iter()))?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let Some(&index) = indexes.get(&id) else {
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
            // Standalone XML ingests have no package name: the file itself.
            if let Some(name) = package_name
                .or(source_path)
                .as_deref()
                .and_then(display_source_name)
            {
                names.insert(name);
            }
        }
    }
    for (index, (mut detail, names)) in details {
        detail.source_names = names.into_iter().collect();
        items[index].details = Some(detail);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::display_source_name;

    #[test]
    fn source_names_lose_every_directory_part() {
        for (recorded, shown) in [
            ("dimmer.knxprod", Some("dimmer.knxprod")),
            (
                "/home/alice/Downloads/dimmer.knxprod",
                Some("dimmer.knxprod"),
            ),
            ("C:\\Users\\alice\\dimmer.knxprod", Some("dimmer.knxprod")),
            (
                "/srv/in/vendor.zip!/sub/dir/inner.knxprod",
                Some("vendor.zip!/inner.knxprod"),
            ),
            ("M-0001/M-0001_A-0001.xml", Some("M-0001_A-0001.xml")),
            ("/srv/in/", None),
        ] {
            assert_eq!(
                display_source_name(recorded).as_deref(),
                shown,
                "{recorded}"
            );
        }
    }
}
