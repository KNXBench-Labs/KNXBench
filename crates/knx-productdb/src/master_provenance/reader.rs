//! Bounded source/identity queries and authoritative retained-byte validation.
use super::*;

fn declaration(row: &rusqlite::Row<'_>) -> Result<MasterDeclaration, rusqlite::Error> {
    let element: String = row.get(3)?;
    let family = MasterKind::from_element(&element).ok_or(rusqlite::Error::InvalidQuery)?;
    let stored: String = row.get(2)?;
    let bool_value = |n: i64| match n {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(rusqlite::Error::InvalidQuery),
    };
    let missing_id = bool_value(row.get(10)?)?;
    let orphan_parent = bool_value(row.get(11)?)?;
    let observed_write = row.get::<_, Option<i64>>(12)?.map(bool_value).transpose()?;
    let d = MasterDeclaration {
        source_sha256: row.get(0)?,
        ordinal: row.get(1)?,
        family,
        element,
        logical_id: row.get(4)?,
        xml_path: row.get(5)?,
        byte_start: row.get(6)?,
        byte_end: row.get(7)?,
        main_number: row.get(8)?,
        function_type_id: row.get(9)?,
        missing_id,
        orphan_parent,
        observed_write,
    };
    let expected_orphan = (d.element == "DatapointSubtype" && d.main_number.is_none())
        || (d.element == "FunctionPoint" && d.function_type_id.is_none());
    let path_valid = d.xml_path.starts_with('/')
        && !d.xml_path.contains(['\\', '\0'])
        && !d.xml_path[1..]
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
        && d.xml_path.rsplit('/').next() == Some(d.element.as_str());
    if stored != family.as_str()
        || d.ordinal == 0
        || d.ordinal as usize > MAX_MASTER_DECLARATIONS
        || d.byte_start >= d.byte_end
        || d.byte_end as usize > MAX_MASTER_BYTES
        || !path_valid
        || !valid_sha(&d.source_sha256)
        || d.logical_id.is_none() != d.missing_id
        || d.orphan_parent != expected_orphan
        || (d.element == "DatapointType" && d.main_number.is_none())
        || (family != MasterKind::DatapointType && d.main_number.is_some())
        || (d.element != "FunctionPoint" && d.function_type_id.is_some())
        || (d.observed_write.is_some() && (d.missing_id || d.orphan_parent))
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(d)
}
const COLUMNS:&str="source_sha256,ordinal,family,element,logical_id,xml_path,byte_start,byte_end,main_number,function_type_id,missing_id,orphan_parent,observed_write";
pub fn master_source_report(
    conn: &Connection,
    sha: &str,
) -> Result<Option<MasterSourceReport>, ProductDbError> {
    master_source_report_with_budget(conn, sha, &mut EvidenceBudget::default())
}
pub(super) fn master_source_report_with_budget(
    conn: &Connection,
    sha: &str,
    budget: &mut EvidenceBudget,
) -> Result<Option<MasterSourceReport>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT definition_version,history,status,reason FROM master_source WHERE source_sha256=?1",
    )?;
    let mut rows = stmt.query([sha])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    budget.charge(sha.len())?;
    for i in 1..=3 {
        if let rusqlite::types::ValueRef::Text(bytes) = row.get_ref(i)? {
            budget.charge(bytes.len())?;
        }
    }
    let (version, history, status, reason) = (
        row.get::<_, i64>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
    );
    if version != MASTER_PROVENANCE_VERSION
        || !["observed", "historical-unavailable"].contains(&history.as_str())
        || !["complete", "failed", "unexamined"].contains(&status.as_str())
        || (status == "complete") != reason.is_none()
    {
        return Err(error("invalid master source header"));
    }
    let mut declarations = Vec::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM master_declaration WHERE source_sha256=?1 ORDER BY ordinal"
    ))?;
    let mut rows = stmt.query([sha])?;
    while let Some(row) = rows.next()? {
        if declarations.len() >= MAX_MASTER_DECLARATIONS {
            return Err(error("master declaration budget exceeded"));
        }
        budget.record()?;
        for i in [0, 2, 3, 4, 5, 9] {
            if let rusqlite::types::ValueRef::Text(bytes) = row.get_ref(i)? {
                budget.charge(bytes.len())?;
            }
        }
        declarations.push(declaration(row)?);
    }
    if declarations.iter().enumerate().any(|(i, d)| {
        d.ordinal as usize != i + 1
            || (history == "historical-unavailable" && d.observed_write.is_some())
            || (history == "observed"
                && !d.missing_id
                && !d.orphan_parent
                && d.observed_write.is_none())
    }) || reason.as_ref().is_some_and(|s| s.is_empty())
        || (status == "complete" && !valid_sha(sha))
    {
        return Err(error("inconsistent master source declarations"));
    }
    let bytes = verified_source(conn, sha)?;
    let retained_source_available = bytes.is_some();
    if let Some(bytes) = bytes.filter(|_| status == "complete" || !declarations.is_empty()) {
        // Reuse the authoritative parser, including unusual carried contexts;
        // never trust persisted offsets or introduce another declaration scanner.
        let mut actual = crate::parse::master::parse_master_with_budget(
            conn,
            &bytes,
            false,
            budget,
            crate::operation::Cancellation::Quiet,
        )?
        .declarations;
        if actual.len() != declarations.len()
            || actual
                .iter_mut()
                .zip(&declarations)
                .any(|(actual, stored)| {
                    actual.observed_write = stored.observed_write;
                    actual != stored
                })
        {
            return Err(error("master declarations disagree with retained bytes"));
        }
    }
    Ok(Some(MasterSourceReport {
        source_sha256: sha.into(),
        history,
        status,
        reason,
        declarations,
        retained_source_available,
    }))
}
pub fn master_identity(
    conn: &Connection,
    family: MasterKind,
    id: &str,
) -> Result<MasterIdentityReport, ProductDbError> {
    let mut budget = EvidenceBudget::default();
    budget.charge(id.len())?;
    let normalized_present: bool = conn.query_row(
        &format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id=?1)",
            family.as_str()
        ),
        [id],
        |r| r.get(0),
    )?;
    let mut winner = match winner_marker(conn, family, id)? {
        WinnerMarker::Observed(winner) if normalized_present => Some(winner),
        WinnerMarker::Historical if normalized_present => None,
        WinnerMarker::Absent if !normalized_present => None,
        _ => return Err(error("missing or invalid master winner marker")),
    };
    let mut candidates = Vec::new();
    let mut availability = std::collections::BTreeMap::new();
    let mut stmt = conn.prepare("SELECT DISTINCT source_sha256 FROM master_declaration WHERE family=?1 AND logical_id=?2 ORDER BY source_sha256")?;
    let mut rows = stmt.query(params![family.as_str(), id])?;
    let mut sources = Vec::new();
    while let Some(row) = rows.next()? {
        budget.charge(
            row.get_ref(0)?
                .as_str()
                .map_err(|_| error("invalid source hash type"))?
                .len(),
        )?;
        sources.push(row.get::<_, String>(0)?);
    }
    // One authenticated source read/parse per distinct source, not per candidate.
    for sha in sources {
        let source = master_source_report_with_budget(conn, &sha, &mut budget)?
            .ok_or_else(|| error("candidate source header is missing"))?;
        availability.insert(sha, source.retained_source_available);
        candidates.extend(
            source
                .declarations
                .into_iter()
                .filter(|d| d.family == family && d.logical_id.as_deref() == Some(id)),
        );
    }
    for (sha, available) in &availability {
        if !available {
            budget.charge(sha.len())?;
        }
    }
    let unavailable_sources = availability
        .iter()
        .filter(|(_, available)| !**available)
        .map(|(sha, _)| sha.clone())
        .collect();
    if let Some(winner) = &mut winner {
        let d = candidates
            .iter()
            .find(|d| d.source_sha256 == winner.source_sha256 && d.ordinal == winner.ordinal)
            .ok_or_else(|| error("winner declaration is missing or disagrees with its identity"))?;
        if !normalized_present || d.observed_write != Some(true) {
            return Err(error("winner disagrees with its declaration"));
        }
        winner.retained_source_available = *availability
            .get(&winner.source_sha256)
            .ok_or_else(|| error("winner source is missing"))?;
    }
    Ok(MasterIdentityReport {
        family,
        logical_id: id.into(),
        normalized_present,
        winner,
        candidates,
        unavailable_sources,
    })
}

fn verified_source(conn: &Connection, sha: &str) -> Result<Option<Vec<u8>>, ProductDbError> {
    let header: Option<(i64, i64)> = conn
        .query_row(
            "SELECT len,length(bytes) FROM source_file WHERE sha256=?1",
            [sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((len, actual)) = header else {
        return Ok(None);
    };
    if len != actual || !(0..=MAX_MASTER_BYTES as i64).contains(&actual) {
        return Ok(None);
    }
    let bytes: Vec<u8> = conn.query_row(
        "SELECT bytes FROM source_file WHERE sha256=?1",
        [sha],
        |r| r.get(0),
    )?;
    Ok((sha256_hex(&bytes) == sha).then_some(bytes))
}
