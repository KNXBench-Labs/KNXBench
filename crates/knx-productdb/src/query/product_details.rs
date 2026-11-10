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

// Optional name discovery is deliberately smaller than ingest. Exhaustion refuses the entire
// fallback index: a prefix cannot prove uniqueness. No filename or membership filter is sound
// for older generic ingest, which retained content-classified masters under arbitrary names.
const MAX_NAME_SOURCES: usize = 4096;
const MAX_NAME_SOURCE_BYTES: usize = 8 * 1024 * 1024;
const MAX_NAME_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const MAX_NAME_XML_EVENTS: usize = 262_144;
const MAX_NAME_XML_DEPTH: usize = 128;
const MAX_NAME_XML_ATTRIBUTES: usize = 256;

type RetainedNames = std::collections::BTreeMap<String, Option<String>>;

/// Optional discovery refused; ordinary normalized product facts remain available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameDiscoveryRefusal {
    SourceCount,
    SourceBytes,
    XmlWork,
    Unreadable,
}

/// Lazy, complete retained-name index owned by one request/read snapshot only.
/// Construct a fresh cache for every catalogue read; never share across connections or writes.
#[derive(Default)]
pub struct ManufacturerNameCache {
    retained: Option<Result<RetainedNames, NameDiscoveryRefusal>>,
}

impl ManufacturerNameCache {
    pub fn refusal(&self) -> Option<NameDiscoveryRefusal> {
        self.retained
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .copied()
    }

    pub fn resolve(
        &mut self,
        conn: &Connection,
        id: &str,
        installed: Option<&str>,
    ) -> Option<String> {
        if let Some(name) = meaningful_name(id, installed) {
            return Some(name.to_owned());
        }
        if !valid_manufacturer_id(id) {
            return None;
        }
        self.retained
            .get_or_insert_with(|| discover_names(conn))
            .as_ref()
            .ok()
            .and_then(|names| names.get(id))
            .cloned()
            .flatten()
    }
}

fn valid_manufacturer_id(id: &str) -> bool {
    id.len() == 6 && id.starts_with("M-") && id.as_bytes()[2..].iter().all(u8::is_ascii_hexdigit)
}

/// Keep real installed names; bounded, complete retained evidence alone can supply a fallback.
/// Discovery failures are optional-name refusals, never failures of the product lookup itself.
pub fn manufacturer_name(
    conn: &Connection,
    id: &str,
    installed: Option<&str>,
) -> Result<Option<String>, ProductDbError> {
    Ok(ManufacturerNameCache::default().resolve(conn, id, installed))
}

fn discover_names(conn: &Connection) -> Result<RetainedNames, NameDiscoveryRefusal> {
    discover_names_sql(conn).map_err(|_| NameDiscoveryRefusal::Unreadable)?
}

fn discover_names_sql(
    conn: &Connection,
) -> rusqlite::Result<Result<RetainedNames, NameDiscoveryRefusal>> {
    // One bounded table pass; no sort, correlated membership scans or payload projection.
    // SQLite length(BLOB) reads the stored size without materializing the payload. Preflight
    // every retained source before fetching any bytes, even ones with unhelpful filenames.
    let mut stmt = conn.prepare("SELECT rowid, len, length(bytes) FROM source_file LIMIT ?1")?;
    let mut sources = Vec::new();
    let mut total = 0usize;
    for row in stmt.query_map([MAX_NAME_SOURCES as i64 + 1], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })? {
        let (rowid, recorded, size) = row?;
        let Ok(size) = usize::try_from(size) else {
            return Ok(Err(NameDiscoveryRefusal::SourceBytes));
        };
        if sources.len() == MAX_NAME_SOURCES {
            return Ok(Err(NameDiscoveryRefusal::SourceCount));
        }
        if size > MAX_NAME_SOURCE_BYTES || size > MAX_NAME_TOTAL_BYTES - total {
            return Ok(Err(NameDiscoveryRefusal::SourceBytes));
        }
        total += size;
        sources.push((rowid, recorded, size));
    }
    drop(stmt);
    let mut names = RetainedNames::new();
    let mut events_left = MAX_NAME_XML_EVENTS;
    let mut bytes_stmt = conn.prepare(
        "SELECT sha256, bytes FROM source_file WHERE rowid=?1 AND length(bytes)=?2 AND length(sha256)=64",
    )?;
    for (rowid, recorded, size) in sources {
        // Invalid recorded length/hash is not valid name evidence. Bound even malformed hashes
        // in SQL, before Rust copies them, and only fetch byte-admitted sources by rowid.
        if recorded != size as i64 {
            continue;
        }
        let source = bytes_stmt
            .query_row((rowid, size as i64), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .optional()?;
        let Some((sha, bytes)) = source else { continue };
        if crate::sha256_hex(&bytes) != sha {
            continue;
        }
        // Preflight parser work before complete validation's attribute checks. All events,
        // depth and attributes are bounded; exceeding any bound invalidates the entire index.
        if !xml_work_admitted(&bytes, &mut events_left) {
            return Ok(Err(NameDiscoveryRefusal::XmlWork));
        }
        if crate::xml::validate_complete_document("retained-source.xml", &bytes).is_err() {
            continue;
        }
        let mut reader = quick_xml::Reader::from_reader(bytes.as_slice());
        let mut stack = Vec::new();
        loop {
            let event = reader.read_event();
            let empty = matches!(&event, Ok(quick_xml::events::Event::Empty(_)));
            match event {
                Ok(quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e)) => {
                    let element = crate::xml::local_name(&e);
                    if stack == ["KNX", "MasterData", "Manufacturers"] && element == "Manufacturer"
                    {
                        let attrs = match crate::xml::attrs(&e, "retained-source.xml") {
                            Ok(attrs) => attrs,
                            Err(_) => return Ok(Err(NameDiscoveryRefusal::Unreadable)),
                        };
                        if let Some(id) = attrs.get("Id").filter(|id| valid_manufacturer_id(id)) {
                            if let Some(name) = meaningful_name(id, attrs.get("Name")) {
                                names
                                    .entry(id.to_owned())
                                    .and_modify(|old| {
                                        if old.as_deref() != Some(name) {
                                            *old = None;
                                        }
                                    })
                                    .or_insert_with(|| Some(name.to_owned()));
                            }
                        }
                    }
                    if !empty {
                        stack.push(element);
                    }
                }
                Ok(quick_xml::events::Event::End(_)) => {
                    stack.pop();
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => return Ok(Err(NameDiscoveryRefusal::Unreadable)),
                _ => {}
            }
        }
    }
    Ok(Ok(names))
}

fn xml_work_admitted(bytes: &[u8], events_left: &mut usize) -> bool {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut depth = 0usize;
    loop {
        let Some(remaining) = events_left.checked_sub(1) else {
            return false;
        };
        *events_left = remaining;
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                if depth > MAX_NAME_XML_DEPTH
                    || e.attributes()
                        .with_checks(false)
                        .take(MAX_NAME_XML_ATTRIBUTES + 1)
                        .count()
                        > MAX_NAME_XML_ATTRIBUTES
                {
                    return false;
                }
            }
            Ok(Event::Empty(e)) => {
                if depth + 1 > MAX_NAME_XML_DEPTH
                    || e.attributes()
                        .with_checks(false)
                        .take(MAX_NAME_XML_ATTRIBUTES + 1)
                        .count()
                        > MAX_NAME_XML_ATTRIBUTES
                {
                    return false;
                }
            }
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Eof) | Err(_) => return true,
            _ => {}
        }
    }
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
    let columns = if table == "package" {
        "sha256, source_name, scheme, size"
    } else {
        "*"
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT {columns} FROM {table} WHERE {id_column}=?1"
    ))?;
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
