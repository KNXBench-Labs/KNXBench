//! ADR-0148: narrowly correct released promotions, not install history.
//!
//! Scratch parser writes are discarded. Only named columns and unknown keys
//! cross back to the caller's transaction; no normalized catalogue is rebuilt.
use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, types::Value, Connection, OptionalExtension};

use crate::report::UnknownConstruct;
use crate::{FileKind, ProductDbError};

pub(crate) mod reports;
use reports::{repair_reports, Evidence, RepairBudget};

/// The union of the v23-v25 retirement keys, not a new XML recognizer.
/// Authoritative parsers decide which occurrences actually remain unsupported.
pub(crate) fn target(path: &str, kind: &str, name: &str) -> bool {
    let host = path.rsplit('/').next().unwrap_or_default();
    match (kind, name) {
        ("Attribute", "TextParameterRefId") => matches!(
            host,
            "Channel" | "ParameterBlock" | "ParameterSeparator" | "ComObjectRef"
        ),
        ("Attribute", "Access") => matches!(host, "ParameterBlock" | "ParameterSeparator"),
        ("Attribute", "Offset" | "BitOffset") if host == "Parameter" => true,
        ("Element", "Property") => matches!(host, "Parameter" | "Union"),
        (
            "Attribute",
            "ObjectIndex" | "ObjectType" | "PropertyId" | "Offset" | "BitOffset" | "Occurrence",
        ) => path.ends_with("/Parameter/Property") || path.ends_with("/Union/Property"),
        _ => false,
    }
}

fn failure(path: &str, cause: impl ToString) -> ProductDbError {
    ProductDbError::Xml {
        source_path: path.into(),
        cause: format!("v28 promotion repair: {}", cause.to_string()),
    }
}

fn record_failure(
    conn: &Connection,
    sha: &str,
    path: &str,
    error: &ProductDbError,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO ingest_unknown(source_sha256,program_id,xpath,kind,name,occurrences,sample)
        VALUES(?1,NULL,?2,'PromotionRepairError','promotion_repair',1,?3) ON CONFLICT DO NOTHING",
        params![sha, path, error.to_string()],
    )?;
    Ok(())
}

/// Admit actual length before materializing; declared length and hash before
/// even classification. Missing known ownership remains a named failure.
fn source_bytes(conn: &Connection, sha: &str, path: &str) -> Result<Vec<u8>, ProductDbError> {
    let lengths: Option<(i64, i64)> = conn
        .query_row(
            "SELECT length(bytes),len FROM source_file WHERE sha256=?1",
            [sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((actual, declared)) = lengths else {
        return Err(failure(path, "missing retained program source"));
    };
    if actual < 0 || actual as u64 > knx_core::limits::MAX_RETAINED_BYTES {
        return Err(failure(path, "retained program exceeds source size limit"));
    }
    if declared != actual {
        return Err(failure(path, "retained source length mismatch"));
    }
    let bytes: Vec<u8> = conn.query_row(
        "SELECT bytes FROM source_file WHERE sha256=?1",
        [sha],
        |r| r.get(0),
    )?;
    if crate::sha256_hex(&bytes) != sha {
        return Err(failure(path, "retained source SHA-256 mismatch"));
    }
    Ok(bytes)
}

/// Borrowed lexical admission, not a second semantic recognizer. Every raw
/// element/attribute participates in the existing parser's item/path/value
/// work ceilings (including documents for which scheme reconciliation is a
/// no-op). Source bytes have the separate retained-source ceiling. This is a
/// work bound, not a claim about SQLite heap or process RSS.
///
/// Potential retirement keys are deduplicated lexically, with the largest raw
/// sample length. We do not decide whether a construct is modelled: even known
/// attributes reserve space. Raw XML values bound normalized values; canonical
/// Dynamic paths are covered by the longest parser-owned Dynamic prefix.
/// Reserve both possible scheme inventories and their key/map copies BEFORE
/// the semantic parsers, normalization or sample copies can run.
fn preparse_admission(
    conn: &Connection,
    sha: &str,
    path: &str,
    bytes: &[u8],
    retained: &RepairBudget,
) -> Result<(), ProductDbError> {
    use crate::parse::scheme_evidence::{
        MAX_EVIDENCE_BYTES, MAX_EVIDENCE_DEPTH, MAX_EVIDENCE_ITEMS,
    };
    use quick_xml::events::Event;
    let refuse = || {
        failure(
            path,
            "preparse evidence admission: 16 MiB evidence working-set allowance exceeded",
        )
    };
    let mut reader = quick_xml::NsReader::from_reader(bytes);
    let mut parents: Vec<String> = Vec::new();
    let mut raw_lengths: Vec<usize> = Vec::new();
    let mut candidates: BTreeMap<(String, &'static str, String), (usize, usize)> = BTreeMap::new();
    let mut scan_items = 0_usize;
    let mut scan_bytes = 0_usize;
    let mut scratch_keys = RepairBudget::default();
    let mut reservation = retained.clone();
    reservation.charge(256 + sha.len()).map_err(|_| refuse())?;
    // Stored keys are admitted without decoding samples or materializing rows.
    reports::admit_source_keys(conn, sha, &mut reservation).map_err(|e| match e {
        ProductDbError::Sqlite(_) => e,
        _ => refuse(),
    })?;
    let mut modes = 1;
    loop {
        let (namespace, event) = reader.read_resolved_event().map_err(|e| failure(path, e))?;
        let element = match &event {
            Event::Start(e) | Event::Empty(e) => e,
            Event::End(_) => {
                parents.pop();
                raw_lengths.pop();
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        if parents.len() >= MAX_EVIDENCE_DEPTH {
            return Err(failure(path, "preparse XML nesting exceeds evidence limit"));
        }
        let raw_name = element.name().into_inner();
        let local = raw_name.rsplit(':').next().unwrap_or(raw_name);
        let path_len = raw_lengths
            .iter()
            .try_fold(1_usize + raw_name.len(), |n, len| n.checked_add(1 + len))
            .ok_or_else(refuse)?;
        let namespace_len = match namespace {
            quick_xml::name::ResolveResult::Bound(ns) => ns.as_ref().len(),
            _ => 0,
        };
        let expanded_len = namespace_len
            .checked_add(local.len())
            .and_then(|n| n.checked_add(2))
            .ok_or_else(refuse)?;
        let mut item_bytes = path_len.checked_add(expanded_len).ok_or_else(refuse)?;
        if parents.is_empty()
            && matches!(namespace, quick_xml::name::ResolveResult::Bound(ns) if matches!(ns.as_ref(),"http://knx.org/xml/project/21"|"http://knx.org/xml/project/23"))
        {
            modes = 2;
        }
        let mut attributes = 0_usize;
        for attr in element.attributes() {
            let attr = attr.map_err(|e| failure(path, e))?;
            attributes = attributes.checked_add(1).ok_or_else(refuse)?;
            item_bytes = item_bytes
                .checked_add(path_len)
                .and_then(|n| n.checked_add(attr.key.as_ref().len()))
                .and_then(|n| n.checked_add(attr.value.len()))
                .ok_or_else(refuse)?;
            let (ns, attr_local) = reader.resolver().resolve_attribute(attr.key);
            let attr_expanded = match ns {
                quick_xml::name::ResolveResult::Bound(ns) => ns
                    .as_ref()
                    .len()
                    .checked_add(attr_local.as_ref().len())
                    .and_then(|n| n.checked_add(2))
                    .ok_or_else(refuse)?,
                _ => attr.key.as_ref().len(),
            };
            item_bytes = item_bytes
                .checked_add(expanded_len)
                .and_then(|n| n.checked_add(attr_expanded))
                .ok_or_else(refuse)?;
        }
        scan_items = scan_items
            .checked_add(1 + attributes)
            .filter(|n| *n <= MAX_EVIDENCE_ITEMS)
            .ok_or_else(|| failure(path, "preparse XML evidence item limit exceeded"))?;
        scan_bytes = scan_bytes
            .checked_add(item_bytes)
            .filter(|n| *n <= MAX_EVIDENCE_BYTES)
            .ok_or_else(|| failure(path, "preparse XML evidence byte limit exceeded"))?;
        // The small temporary path is independently admitted before joining.
        let mut temporary = RepairBudget::default();
        temporary.charge(path_len + 256).map_err(|_| refuse())?;
        let parent = format!("/{}", parents.join("/"));
        let current = if parents.is_empty() {
            format!("/{local}")
        } else {
            format!("{parent}/{local}")
        };
        let mut admit = |xpath: &str,
                         kind: &'static str,
                         name: &str,
                         sample: usize|
         -> Result<(), ProductDbError> {
            if !target(xpath, kind, name) {
                return Ok(());
            }
            // The dynamic parser deliberately flattens nested paths. Use its
            // longest canonical prefix even for a short/noncanonical input.
            let canonical = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic/".len() + local.len();
            let length = path_len.max(xpath.len()).max(canonical);
            let old = candidates
                .iter()
                .find(|((p, k, n), _)| p == xpath && *k == kind && n == name)
                .map(|(_, v)| *v);
            let estimate = |len: usize, sample: usize| {
                512_usize
                    .checked_add(len)
                    .and_then(|n| n.checked_add(name.len()))
                    .and_then(|n| n.checked_add(sample))
                    .and_then(|n| n.checked_mul(2 * modes))
                    .ok_or_else(refuse)
            };
            let previous = old
                .map(|(len, s)| estimate(len, s))
                .transpose()?
                .unwrap_or(0);
            let next = estimate(length, sample.max(old.map_or(0, |v| v.1)))?;
            reservation
                .charge(next.saturating_sub(previous))
                .map_err(|_| refuse())?;
            if old.is_none() {
                scratch_keys
                    .charge(256 + xpath.len() + kind.len() + name.len())
                    .map_err(|_| refuse())?;
                candidates.insert((xpath.into(), kind, name.into()), (length, sample));
            } else if let Some(value) = candidates
                .iter_mut()
                .find(|((p, k, n), _)| p == xpath && *k == kind && n == name)
                .map(|(_, v)| v)
            {
                value.0 = value.0.max(length);
                value.1 = value.1.max(sample);
            }
            Ok(())
        };
        admit(&parent, "Element", local, 0)?;
        for attr in element.attributes() {
            let attr = attr.map_err(|e| failure(path, e))?;
            let name = attr.key.into_inner();
            // Static parsers can alias a qualified attribute to its local key.
            admit(
                &current,
                "Attribute",
                name.rsplit(':').next().unwrap_or(name),
                attr.value.len(),
            )?;
        }
        if matches!(event, Event::Start(_)) {
            parents.push(local.into());
            raw_lengths.push(raw_name.len());
        }
    }
    Ok(())
}

fn derive(
    conn: &Connection,
    sha: &str,
    path: &str,
    bytes: &[u8],
) -> Result<(Connection, Evidence), ProductDbError> {
    crate::xml::validate_complete_document(path, bytes)?;
    let scratch = crate::migration::promotion_scratch()?;
    // A fresh empty parser alone would admit cross-file losing Dynamic trees.
    // Seed real foreign owners; never infer a winner from captions/hash order.
    let mut owners = conn.prepare(
        "SELECT id,manufacturer_id,source_sha256 FROM application_program WHERE source_sha256<>?1",
    )?;
    for row in owners.query_map([sha], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, manufacturer, owner) = row?;
        scratch.execute(
            "INSERT INTO application_program(id,manufacturer_id,source_sha256) VALUES(?1,?2,?3)",
            params![id, manufacturer, owner],
        )?;
    }
    let mut unknown = crate::parse::program::ingest_program(&scratch, sha, path, bytes)?.unknown;
    unknown.extend(crate::dynamic::parse::parse_dynamic_trees(&scratch, sha, path, bytes)?.unknown);
    // Never clone the complete parser inventory. The lexical reservation is
    // made before parsing; this second check admits each actual target copy.
    unknown.retain(|u| target(&u.xpath, u.kind.as_str(), &u.name));
    let mut copy_budget = RepairBudget::default();
    for row in &unknown {
        copy_budget.row(row)?;
    }
    let mut extended = unknown.clone();
    crate::parse::scheme_evidence::reconcile_targeted_unknowns(bytes, path, &mut unknown)?;
    crate::parse::scheme_evidence::reconcile_package_unknowns(bytes, path, &mut extended)?;
    unknown.retain(|u| target(&u.xpath, u.kind.as_str(), &u.name));
    extended.retain(|u| target(&u.xpath, u.kind.as_str(), &u.name));
    let extended = (extended != unknown).then_some(extended);
    let original_keys = reports::source_target_keys(conn, sha)?;
    Ok((
        scratch,
        Evidence {
            ordinary: unknown,
            extended,
            original_keys,
        },
    ))
}

const COLUMNS: &[(&str, &[&str])] = &[
    ("com_object_ref", &["text_parameter_ref_id"]),
    (
        "parameter",
        &[
            "own_offset",
            "own_bit_offset",
            "property_object_index",
            "property_object_type",
            "property_id",
            "property_offset",
            "property_bit_offset",
            "property_occurrence",
        ],
    ),
    (
        "parameter_type_enum",
        &["binary_value", "name", "icon", "picture_alignment"],
    ),
];

fn correct_columns(
    conn: &Connection,
    scratch: &Connection,
    sha: &str,
    path: &str,
) -> Result<(), ProductDbError> {
    let mut owners = conn.prepare("SELECT id FROM application_program WHERE source_sha256=?1")?;
    for id in owners.query_map([sha], |r| r.get::<_, String>(0))? {
        let id = id?;
        let exists: bool = scratch.query_row(
            "SELECT EXISTS(SELECT 1 FROM application_program WHERE id=?1 AND source_sha256=?2)",
            params![id, sha],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(failure(
                path,
                format!("owned program {id:?} has no authoritative declaration"),
            ));
        }
    }
    for (table, columns) in COLUMNS {
        let select = format!(
            "SELECT {} FROM {table} WHERE program_id=?1 AND id=?2",
            columns.join(",")
        );
        let assignments = columns
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{c}=?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let update = format!(
            "UPDATE {table} SET {assignments} WHERE program_id=?{} AND id=?{}",
            columns.len() + 1,
            columns.len() + 2
        );
        let mut rows = conn.prepare(&format!("SELECT program_id,id FROM {table} WHERE program_id IN (SELECT id FROM application_program WHERE source_sha256=?1)"))?;
        for row in rows.query_map([sha], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (program, id) = row?;
            let values: Option<Vec<Value>> = scratch
                .query_row(&select, params![program, id], |r| {
                    (0..columns.len()).map(|i| r.get(i)).collect()
                })
                .optional()?;
            let Some(mut values) = values else {
                return Err(failure(
                    path,
                    format!(
                        "owned {table} row {program:?}/{id:?} has no authoritative declaration"
                    ),
                ));
            };
            values.push(Value::Text(program));
            values.push(Value::Text(id));
            conn.execute(&update, rusqlite::params_from_iter(values))?;
        }
    }
    Ok(())
}

fn replace_evidence(
    conn: &Connection,
    sha: &str,
    evidence: &[UnknownConstruct],
) -> Result<(), ProductDbError> {
    for old in reports::source_targets(conn, sha)? {
        if !evidence
            .iter()
            .any(|u| u.xpath == old.xpath && u.kind == old.kind && u.name == old.name)
        {
            conn.execute("DELETE FROM ingest_unknown WHERE source_sha256=?1 AND xpath=?2 AND kind=?3 AND name=?4",
                params![sha,old.xpath,old.kind.as_str(),old.name])?;
        }
    }
    for row in evidence {
        conn.execute("UPDATE ingest_unknown SET occurrences=?5,sample=?6 WHERE source_sha256=?1 AND program_id IS NULL AND xpath=?2 AND kind=?3 AND name=?4",
            params![sha,row.xpath,row.kind.as_str(),row.name,row.occurrences,row.sample])?;
    }
    crate::report::insert_unknown(conn, sha, evidence)?;
    Ok(())
}

pub(crate) fn repair(conn: &Connection) -> Result<(), ProductDbError> {
    let sources = conn
        .prepare(
            "SELECT sha256 FROM source_parse_evidence
        UNION SELECT source_sha256 FROM application_program
        UNION SELECT source_sha256 FROM package_member WHERE role='ApplicationProgram'
        ORDER BY 1",
        )?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut repaired = BTreeMap::new();
    let mut budget = RepairBudget::default();
    let mut failed = BTreeSet::new();
    for sha in sources {
        let legacy: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM legacy_source WHERE payload_sha256=?1)",
            [&sha],
            |r| r.get(0),
        )?;
        if legacy {
            continue;
        }
        let known: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM application_program WHERE source_sha256=?1)
            OR EXISTS(SELECT 1 FROM package_member WHERE source_sha256=?1 AND role='ApplicationProgram')", [&sha], |r|r.get(0))?;
        let path: String = conn
            .query_row(
                "SELECT source_path FROM source_file WHERE sha256=?1",
                [&sha],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or_else(|| "missing program source".into());
        conn.execute_batch("SAVEPOINT promotion_source")?;
        let mut tentative = budget.clone();
        let result = source_bytes(conn,&sha,&path).and_then(|bytes| {
            if bytes.iter().find(|b|!b.is_ascii_whitespace())==Some(&b'<') || bytes.starts_with(&[0xEF,0xBB,0xBF]) {
                preparse_admission(conn,&sha,&path,&bytes,&budget)?;
            }
            if crate::ingest::classify(&bytes) != FileKind::ApplicationProgram {
                return if known { Err(failure(&path,"known program source no longer classifies as ApplicationProgram")) } else { Ok(None) };
            }
            let (scratch,evidence) = derive(conn,&sha,&path,&bytes)?;
            tentative.evidence(&sha, &evidence)?;
            correct_columns(conn,&scratch,&sha,&path)?;
            let extended: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM package_member m JOIN package p ON p.sha256=m.package_sha256 WHERE m.source_sha256=?1 AND m.role='ApplicationProgram' AND p.scheme IN (21,23))", [&sha], |r|r.get(0))?;
            replace_evidence(conn,&sha,evidence.for_scheme(extended))?;
            Ok(Some(evidence))
        });
        // The source savepoint encloses every durable effect; SQLite failures
        // propagate to the migration transaction rather than being hidden.
        match result {
            Ok(Some(evidence)) => {
                conn.execute_batch("RELEASE promotion_source")?;
                budget = tentative;
                repaired.insert(sha, evidence);
            }
            Ok(None) => {
                conn.execute_batch("RELEASE promotion_source")?;
            }
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(error) => {
                conn.execute_batch("ROLLBACK TO promotion_source; RELEASE promotion_source")?;
                record_failure(conn, &sha, &path, &error)?;
                failed.insert(sha);
            }
        }
    }
    repair_reports(conn, &repaired, &failed)
}
