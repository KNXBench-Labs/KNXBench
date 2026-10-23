//! Read-only source ownership for native catalogue portability.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use rusqlite::{Connection, OptionalExtension};

use crate::{identity::IdentityKind, ProductDbError};

fn refused(reason: impl Into<String>) -> ProductDbError {
    ProductDbError::Xml {
        source_path: "catalogue source portability".into(),
        cause: reason.into(),
    }
}

/// Exact retained bytes; callers keep payloads outside history snapshots.
#[derive(Debug, Clone)]
pub struct DependencySource {
    pub source_path: String,
    pub sha256: String,
    pub kind: String,
    pub bytes: Vec<u8>,
}

/// Current direct source owners. Missing historical mappings stay unknown.
pub fn device_source_owners(
    conn: &Connection,
    product: &str,
    hardware2program: &str,
) -> Result<Option<BTreeSet<String>>, ProductDbError> {
    Ok(owners(conn, product, hardware2program, false)?.map(|owners| owners.into_keys().collect()))
}

fn owners(
    conn: &Connection,
    product: &str,
    hardware2program: &str,
    verify: bool,
) -> Result<Option<BTreeMap<String, String>>, ProductDbError> {
    if hardware2program.is_empty() {
        let row: Option<(String, String, String)> = conn.query_row(
            "SELECT p.hardware_id,p.source_sha256,h.source_sha256 FROM product p JOIN hardware h ON h.id=p.hardware_id WHERE p.id=?1 AND h.has_application_program=0",
            [product], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional()?;
        return row
            .map(|(hardware, p, h)| {
                checked_owners(
                    conn,
                    vec![
                        (IdentityKind::Product, product.to_string(), p, "Hardware"),
                        (IdentityKind::Hardware, hardware, h, "Hardware"),
                    ],
                    verify,
                )
            })
            .transpose();
    }
    type OwnerRow = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    );
    let row: Option<OwnerRow> = conn
        .query_row(
            "SELECT p.hardware_id, hp.hardware_id, hp.application_program_ref,
                p.source_sha256, hp.source_sha256, h.source_sha256, ap.source_sha256, h.id
         FROM product p JOIN hardware2program hp ON hp.id=?2
         JOIN hardware h ON h.id=p.hardware_id
         JOIN application_program ap ON ap.id=hp.application_program_ref WHERE p.id=?1",
            [product, hardware2program],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )
        .optional()?;
    let Some((product_hardware, program_hardware, application, p, hp, h, ap, hardware)) = row
    else {
        return Ok(None);
    };
    if product_hardware != program_hardware {
        return Err(refused("product and Hardware2Program disagree on hardware"));
    }
    checked_owners(
        conn,
        vec![
            (IdentityKind::Product, product.to_string(), p, "Hardware"),
            (
                IdentityKind::Hardware2Program,
                hardware2program.to_string(),
                hp,
                "Hardware",
            ),
            (IdentityKind::Hardware, hardware, h, "Hardware"),
            (
                IdentityKind::ApplicationProgram,
                application,
                ap,
                "ApplicationProgram",
            ),
        ],
        verify,
    )
    .map(Some)
}

fn checked_owners(
    conn: &Connection,
    declarations: Vec<(IdentityKind, String, String, &str)>,
    verify: bool,
) -> Result<BTreeMap<String, String>, ProductDbError> {
    let mut sources = BTreeMap::new();
    for (kind, id, sha, role) in declarations {
        if !verify {
            sources.insert(sha, role.into());
            continue;
        }
        crate::identity::owned_source_identity(conn, kind, &id, &sha)?;
        let report = crate::identity_candidates(conn, kind, &id)?;
        if !report.unmeasured.is_empty()
            || report
                .candidates
                .iter()
                .any(|c| c.same_as_winner != Some(true))
        {
            return Err(refused(format!("ambiguous source declarations for {id}")));
        }
        sources.insert(sha, role.into());
    }
    Ok(sources)
}

/// Read an original, refusing oversized values before materializing a blob.
pub fn load_catalogue_original(
    conn: &Connection,
    sha256: &str,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, ProductDbError> {
    let length: Option<i64> = conn
        .query_row(
            "SELECT length(bytes) FROM package WHERE sha256=?1",
            [sha256],
            |r| r.get(0),
        )
        .optional()?;
    let Some(length) = length else {
        return Ok(None);
    };
    if length < 0 || length as u64 > max_bytes as u64 {
        return Err(refused(
            "original package exceeds native retained-value limit",
        ));
    }
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM package WHERE sha256=?1 AND length(bytes)<=?2",
            rusqlite::params![sha256, i64::try_from(max_bytes).unwrap_or(i64::MAX)],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| {
            refused("original package changed or exceeds native retained-value limit")
        })?;
    if crate::sha256_hex(&bytes) != sha256 {
        return Err(refused("original package hash mismatch"));
    }
    Ok(Some(bytes))
}

pub fn has_catalogue_original(conn: &Connection, sha256: &str) -> Result<bool, ProductDbError> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM package WHERE sha256=?1)",
        [sha256],
        |r| r.get(0),
    )?)
}

/// The unique recorded package owning all direct sources, its original and
/// exact direct/master/catalogue members. Unknown resources stay in the original.
pub fn catalogue_dependency_sources(
    conn: &Connection,
    product: &str,
    hardware2program: &str,
    max_bytes: usize,
    remaining_bytes: u64,
) -> Result<Vec<DependencySource>, ProductDbError> {
    let mut sources = owners(conn, product, hardware2program, true)?
        .ok_or_else(|| refused("required product/application mapping is unavailable"))?;
    let mut common: Option<BTreeSet<String>> = None;
    for sha in sources.keys() {
        let packages = conn.prepare("SELECT DISTINCT package_sha256 FROM package_member WHERE source_sha256=?1 ORDER BY package_sha256")?
            .query_map([sha], |r| r.get::<_, String>(0))?.collect::<Result<BTreeSet<_>, _>>()?;
        common = Some(match common {
            None => packages,
            Some(previous) => previous.intersection(&packages).cloned().collect(),
        });
    }
    let common = common.unwrap_or_default();
    if common.len() != 1 {
        return Err(refused(
            "required source package ownership is unavailable or ambiguous",
        ));
    }
    let package = common.into_iter().next().expect("one package");
    let original = load_catalogue_original(
        conn,
        &package,
        max_bytes.min(usize::try_from(remaining_bytes).unwrap_or(usize::MAX)),
    )?
    .ok_or_else(|| refused("required original package is unavailable"))?;
    let mut charged = original.len() as u64;
    let mut archive = zip::ZipArchive::new(Cursor::new(&original))
        .map_err(|_| refused("required original is not a readable package"))?;
    let catalogues = conn.prepare("SELECT DISTINCT source_sha256 FROM catalog_item WHERE product_ref_id=?1 AND (hardware2program_ref_id=?2 OR (?2='' AND hardware2program_ref_id IS NULL)) ORDER BY source_sha256")?
        .query_map([product, hardware2program], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
    for catalogue in catalogues {
        sources.insert(catalogue, "Catalog".into());
    }
    let members = conn.prepare("SELECT path, role, source_sha256, size FROM package_member WHERE package_sha256=?1 ORDER BY ordinal")?
        .query_map([&package], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?)))?
        .collect::<Result<Vec<_>,_>>()?;
    let mut out = Vec::new();
    let mut masters = 0;
    for (path, role, sha, expected_len) in members {
        if role != "Master" && !sources.contains_key(&sha) {
            continue;
        }
        if role == "Master" {
            masters += 1;
        }
        if sources.get(&sha).is_some_and(|expected| expected != &role) {
            return Err(refused(
                "required package member role disagrees with source owner",
            ));
        }
        let length = crate::blob::source_file_length(conn, &sha)?
            .ok_or_else(|| refused("required source member is unavailable"))?;
        if length as usize > max_bytes || expected_len != i64::from(length) {
            return Err(refused(
                "required source member length is invalid or exceeds native limit",
            ));
        }
        charged = charged
            .checked_add(u64::from(length))
            .ok_or_else(|| refused("source closure charge overflow"))?;
        if charged > remaining_bytes {
            return Err(refused("source closure exceeds expanded archive limit"));
        }
        let bytes = crate::load_source_file(conn, &sha)?
            .ok_or_else(|| refused("required source member is unavailable"))?;
        if crate::sha256_hex(&bytes) != sha {
            return Err(refused("required source member hash mismatch"));
        }
        let mut original_member = archive
            .by_name(&path)
            .map_err(|_| refused("required member is absent from the original package"))?;
        if original_member.size() != u64::from(length) {
            return Err(refused(
                "original package member length differs from recorded membership",
            ));
        }
        let mut original_bytes = Vec::new();
        (&mut original_member)
            .take(u64::from(length) + 1)
            .read_to_end(&mut original_bytes)
            .map_err(|_| refused("original package member could not be read"))?;
        if original_bytes != bytes {
            return Err(refused(
                "original package member differs from recorded source",
            ));
        }
        sources.remove(&sha);
        out.push(DependencySource {
            source_path: path,
            sha256: sha,
            kind: if role == "Master" {
                "MasterData".into()
            } else {
                "ManufacturerData".into()
            },
            bytes,
        });
    }
    if masters != 1 || !sources.is_empty() {
        return Err(refused(
            "required master or direct member is unavailable in the selected package",
        ));
    }
    drop(archive);
    out.insert(
        0,
        DependencySource {
            source_path: format!("catalogue-packages/{package}.knxprod"),
            sha256: package,
            kind: "CataloguePackage".into(),
            bytes: original,
        },
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_programless_product_acquires_its_hardware_master_and_exact_original() {
        let master = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Fictional"/></Manufacturers></MasterData></KNX>"#;
        let hardware = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" HasApplicationProgram="0"><Products><Product Id="M-0001_H-1_P-1" Text="Programless"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, bytes) in [
            ("knx_master.xml", master.as_slice()),
            ("M-0001/Hardware.xml", hardware.as_slice()),
        ] {
            zip.start_file(path, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        let original = zip.finish().unwrap().into_inner();
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        crate::install_package(&conn, "programless.knxprod", &original).unwrap();
        let before = conn.total_changes();
        let sources = catalogue_dependency_sources(
            &conn,
            "M-0001_H-1_P-1",
            "",
            knx_core::limits::MAX_RETAINED_BYTES_USIZE,
            knx_core::limits::MAX_EXPANDED_ARCHIVE_BYTES,
        )
        .unwrap();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0].kind, "CataloguePackage");
        assert_eq!(sources[0].bytes, original);
        assert!(sources
            .iter()
            .any(|s| s.kind == "MasterData" && s.bytes == master));
        assert!(sources
            .iter()
            .any(|s| s.source_path == "M-0001/Hardware.xml" && s.bytes == hardware));
        assert_eq!(conn.total_changes(), before);
    }
}
