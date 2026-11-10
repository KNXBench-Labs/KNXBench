//! Reads complete scalar product records and resolves legacy manufacturer-name placeholders.

use std::collections::BTreeSet;

use rusqlite::{types::ValueRef, Connection, OptionalExtension};

use crate::ProductDbError;

#[cfg(test)]
#[path = "product_details_tests.rs"]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDetailField {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDetailSection {
    pub kind: String,
    pub id: String,
    pub fields: Vec<ProductDetailField>,
}

fn meaningful_name<'a>(id: &str, name: Option<&'a str>) -> Option<&'a str> {
    name.filter(|name| {
        let value = name.trim();
        !value.is_empty()
            && !value.eq_ignore_ascii_case(id)
            && !id
                .strip_prefix("M-")
                .is_some_and(|digits| value.eq_ignore_ascii_case(digits))
    })
}

/// Keep a real installed name. Only missing/ID-shaped names consult retained masters;
/// conflicting declarations remain unresolved rather than inventing a winner or writing the DB.
pub fn manufacturer_name(
    conn: &Connection,
    id: &str,
    installed: Option<&str>,
) -> Result<Option<String>, ProductDbError> {
    if let Some(name) = meaningful_name(id, installed) {
        return Ok(Some(name.to_owned()));
    }
    if id.len() != 6
        || !id.starts_with("M-")
        || !id.as_bytes()[2..].iter().all(u8::is_ascii_hexdigit)
    {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT sha256, len, bytes FROM source_file s WHERE length(bytes) <= ?1 AND
         (EXISTS(SELECT 1 FROM package_member p WHERE p.source_sha256=s.sha256 AND p.role='Master')
          OR EXISTS(SELECT 1 FROM master_source m WHERE m.source_sha256=s.sha256)
          OR replace(s.source_path, char(92), '/') LIKE '%knx_master.xml') ORDER BY sha256",
    )?;
    let mut names = BTreeSet::new();
    for source in stmt.query_map([crate::master_provenance::MAX_MASTER_BYTES as i64], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Vec<u8>>(2)?,
        ))
    })? {
        let (sha, len, bytes) = source?;
        // Damaged/foreign XML is not name evidence. This uses the existing validated format,
        // not a repair parser, and never replays ingest writes.
        if len != bytes.len() as i64
            || crate::sha256_hex(&bytes) != sha
            || crate::xml::validate_complete_document("knx_master.xml", &bytes).is_err()
        {
            continue;
        }
        let mut reader = quick_xml::Reader::from_reader(bytes.as_slice());
        let mut stack = Vec::new();
        loop {
            let event = reader.read_event();
            let empty = matches!(&event, Ok(quick_xml::events::Event::Empty(_)));
            match event {
                Ok(quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e)) => {
                    let name = crate::xml::local_name(&e);
                    if stack == ["KNX", "MasterData", "Manufacturers"] && name == "Manufacturer" {
                        let attrs = crate::xml::attrs(&e, "knx_master.xml")?;
                        if attrs.get("Id") == Some(id) {
                            if let Some(name) = meaningful_name(id, attrs.get("Name")) {
                                names.insert(name.to_owned());
                            }
                        }
                    }
                    if !empty {
                        stack.push(name);
                    }
                }
                Ok(quick_xml::events::Event::End(_)) => {
                    stack.pop();
                }
                Ok(quick_xml::events::Event::Eof) | Err(_) => break,
                _ => {}
            }
        }
    }
    Ok((names.len() == 1)
        .then(|| names.into_iter().next())
        .flatten())
}

/// All non-NULL scalar columns from the normalized records supporting this exact pair.
/// Fixed table names only; IDs remain bound parameters. Source blobs and local paths never leave
/// this query. Additional attributes are explicitly file-scoped diagnostics, not device facts.
pub fn device_product_details(
    conn: &Connection,
    product: &str,
    program_link: &str,
) -> Result<Vec<ProductDetailSection>, ProductDbError> {
    let hardware: Option<String> = conn
        .query_row(
            "SELECT hardware_id FROM product WHERE id=?1",
            [product],
            |r| r.get(0),
        )
        .optional()?;
    let Some(hardware) = hardware else {
        return Ok(Vec::new());
    };
    let application: Option<Option<String>> = conn
        .query_row(
            "SELECT application_program_ref FROM hardware2program WHERE id=?1 AND hardware_id=?2",
            (program_link, &hardware),
            |r| r.get(0),
        )
        .optional()?;
    let manufacturer: String = conn.query_row(
        "SELECT manufacturer_id FROM product WHERE id=?1",
        [product],
        |r| r.get(0),
    )?;
    let mut sections = Vec::new();
    let mut sources = BTreeSet::new();
    for (kind, table, id) in [
        ("manufacturer", "manufacturer", manufacturer.as_str()),
        ("product", "product", product),
        ("hardware", "hardware", hardware.as_str()),
        ("programLink", "hardware2program", program_link),
        (
            "application",
            "application_program",
            application
                .as_ref()
                .and_then(Option::as_deref)
                .unwrap_or_default(),
        ),
    ] {
        if kind == "programLink" && application.is_none() {
            continue;
        }
        append_record(conn, kind, table, id, &mut sections, &mut sources)?;
    }
    let mut items = conn.prepare(
        "SELECT id, section_id FROM catalog_item WHERE product_ref_id=?1 AND
         (hardware2program_ref_id=?2 AND EXISTS(SELECT 1 FROM hardware2program WHERE id=?2 AND hardware_id=?3)
          OR ((hardware2program_ref_id IS NULL OR hardware2program_ref_id='') AND ?2='')) ORDER BY id",
    )?;
    let mut seen_sections = BTreeSet::new();
    for item in items.query_map((product, program_link, &hardware), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (item, mut section) = item?;
        append_record(
            conn,
            "catalogItem",
            "catalog_item",
            &item,
            &mut sections,
            &mut sources,
        )?;
        while seen_sections.insert(section.clone()) {
            append_record(
                conn,
                "catalogSection",
                "catalog_section",
                &section,
                &mut sections,
                &mut sources,
            )?;
            let parent: Option<Option<String>> = conn
                .query_row(
                    "SELECT parent_id FROM catalog_section WHERE id=?1",
                    [&section],
                    |r| r.get(0),
                )
                .optional()?;
            match parent.flatten() {
                Some(id) => section = id,
                None => break,
            }
        }
    }
    let mut packages = BTreeSet::new();
    for source in sources {
        let mut package_query = conn.prepare("SELECT DISTINCT package_sha256 FROM package_member WHERE source_sha256=?1 ORDER BY package_sha256")?;
        for package in package_query.query_map([&source], |r| r.get::<_, String>(0))? {
            packages.insert(package?);
        }
        let mut fields = vec![ProductDetailField {
            key: "source_sha256".into(),
            value: source.clone(),
        }];
        let mut stmt = conn.prepare("SELECT s.len, s.source_path, COALESCE(n.source_name,p.source_name), s.manufacturer_id FROM source_file s
            LEFT JOIN package_member m ON m.source_sha256=s.sha256 LEFT JOIN package p ON p.sha256=m.package_sha256
            LEFT JOIN package_source_name n ON n.package_sha256=p.sha256 WHERE s.sha256=?1")?;
        let mut names = BTreeSet::new();
        let mut len = None;
        let mut manufacturer = None;
        for row in stmt.query_map([&source], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })? {
            let (size, path, package, source_manufacturer) = row?;
            len = Some(size);
            manufacturer = source_manufacturer;
            if let Some(name) = crate::catalog_details::display_source_name(&path) {
                names.insert(name);
            }
            if let Some(name) = package
                .as_deref()
                .and_then(crate::catalog_details::display_source_name)
            {
                names.insert(name);
            }
        }
        if let Some(len) = len {
            fields.push(ProductDetailField {
                key: "source_bytes".into(),
                value: len.to_string(),
            });
        }
        if let Some(manufacturer) = manufacturer {
            fields.push(ProductDetailField {
                key: "manufacturer_id".into(),
                value: manufacturer,
            });
        }
        for name in names {
            fields.push(ProductDetailField {
                key: "source_name".into(),
                value: name,
            });
        }
        sections.push(ProductDetailSection {
            kind: "source".into(),
            id: source.clone(),
            fields,
        });
        let mut stmt = conn.prepare("SELECT program_id, xpath, kind, name, occurrences, sample FROM ingest_unknown WHERE source_sha256=?1 ORDER BY xpath, kind, name, id")?;
        for (index, row) in stmt
            .query_map([&source], |r| {
                let keys = [
                    "program_id",
                    "xpath",
                    "kind",
                    "name",
                    "occurrences",
                    "sample",
                ];
                scalar_fields(r, &keys)
            })?
            .enumerate()
        {
            sections.push(ProductDetailSection {
                kind: "additionalAttributes".into(),
                id: format!("{source}:{}", index + 1),
                fields: row?,
            });
        }
    }
    for package in packages {
        append_record(
            conn,
            "package",
            "package",
            &package,
            &mut sections,
            &mut BTreeSet::new(),
        )?;
    }
    Ok(sections)
}

fn scalar_fields(
    row: &rusqlite::Row<'_>,
    keys: &[&str],
) -> rusqlite::Result<Vec<ProductDetailField>> {
    let mut fields = Vec::new();
    for (i, &key) in keys.iter().enumerate() {
        let value = match row.get_ref(i)? {
            ValueRef::Text(_) => row.get::<_, String>(i)?,
            ValueRef::Integer(value) => value.to_string(),
            ValueRef::Real(value) => value.to_string(),
            ValueRef::Null | ValueRef::Blob(_) => continue,
        };
        fields.push(ProductDetailField {
            key: key.into(),
            value,
        });
    }
    Ok(fields)
}

fn append_record(
    conn: &Connection,
    kind: &str,
    table: &str,
    id: &str,
    sections: &mut Vec<ProductDetailSection>,
    sources: &mut BTreeSet<String>,
) -> Result<(), ProductDbError> {
    let id_column = if table == "package" { "sha256" } else { "id" };
    let mut stmt = conn.prepare(&format!("SELECT * FROM {table} WHERE {id_column}=?1"))?;
    let keys: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|key| (*key).to_owned())
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let fields = stmt
        .query_row([id], |row| scalar_fields(row, &keys))
        .optional()?;
    if let Some(mut fields) = fields {
        for field in &fields {
            if field.key == "source_sha256" {
                sources.insert(field.value.clone());
            }
        }
        // An internal ingestion-completeness marker, not declared product metadata.
        fields.retain(|field| {
            field.key != "write_authority_recorded"
                && !(table == "package" && field.key.ends_with("_count"))
        });
        if table == "package" {
            for field in &mut fields {
                if field.key == "source_name" {
                    field.value = crate::catalog_details::display_source_name(&field.value)
                        .unwrap_or_default();
                }
            }
        }
        sections.push(ProductDetailSection {
            kind: kind.into(),
            id: id.into(),
            fields,
        });
    }
    Ok(())
}
