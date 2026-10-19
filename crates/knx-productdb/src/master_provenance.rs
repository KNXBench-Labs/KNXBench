//! Master declaration addresses and observed winners, separate from product identity.
//! See ADR-0147. Replay never writes normalized values or historical reports.
use crate::{sha256_hex, ProductDbError};
use rusqlite::{params, Connection, OptionalExtension};

pub const MASTER_PROVENANCE_VERSION: i64 = 1;
pub const MAX_MASTER_DECLARATIONS: usize = 1_000_000;
pub const MAX_MASTER_DEPTH: usize = 4_096;
pub const MAX_MASTER_BYTES: usize = knx_core::limits::MAX_RETAINED_BYTES as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterKind {
    DatapointType,
    FunctionType,
    FunctionPoint,
    SpaceUsage,
}
impl MasterKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DatapointType => "datapoint_type",
            Self::FunctionType => "function_type",
            Self::FunctionPoint => "function_point",
            Self::SpaceUsage => "space_usage",
        }
    }
    pub(crate) fn from_element(name: &str) -> Option<Self> {
        match name {
            "DatapointType" | "DatapointSubtype" => Some(Self::DatapointType),
            "FunctionType" => Some(Self::FunctionType),
            "FunctionPoint" => Some(Self::FunctionPoint),
            "SpaceUsage" => Some(Self::SpaceUsage),
            _ => None,
        }
    }
}
pub(crate) fn error(cause: impl Into<String>) -> ProductDbError {
    ProductDbError::Xml {
        source_path: "knx_master.xml".into(),
        cause: cause.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterDeclaration {
    pub source_sha256: String,
    pub ordinal: u32,
    pub family: MasterKind,
    pub element: String,
    pub logical_id: Option<String>,
    pub xml_path: String,
    pub byte_start: u32,
    pub byte_end: u32,
    pub main_number: Option<i64>,
    pub function_type_id: Option<String>,
    pub missing_id: bool,
    pub orphan_parent: bool,
    /// None means unobserved history or malformed (no eligible insert).
    pub observed_write: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterSourceReport {
    pub source_sha256: String,
    pub history: String,
    pub status: String,
    pub reason: Option<String>,
    pub declarations: Vec<MasterDeclaration>,
    pub retained_source_available: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterWinner {
    pub source_sha256: String,
    pub ordinal: u32,
    pub retained_source_available: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterIdentityReport {
    pub family: MasterKind,
    pub logical_id: String,
    pub normalized_present: bool,
    /// None for an absent row OR a historical winner; use normalized_present.
    pub winner: Option<MasterWinner>,
    pub candidates: Vec<MasterDeclaration>,
    pub unavailable_sources: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterDiagnostic {
    pub kind: crate::InstallDiagnosticKind,
    pub xml_path: String,
    pub detail: String,
}

pub(crate) fn atomic<T>(
    conn: &Connection,
    name: &str,
    f: impl FnOnce() -> Result<T, ProductDbError>,
) -> Result<T, ProductDbError> {
    conn.execute_batch(&format!("SAVEPOINT {name}"))?;
    let result = f().and_then(|value| {
        conn.execute_batch(&format!("RELEASE {name}"))?;
        Ok(value)
    });
    if result.is_err() && !conn.is_autocommit() {
        conn.execute_batch(&format!("ROLLBACK TO {name}; RELEASE {name}"))?;
    }
    result
}

pub(crate) fn create_schema(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch("CREATE TABLE master_source (
        source_sha256 TEXT PRIMARY KEY,
        definition_version INTEGER NOT NULL CHECK(definition_version=1),
        history TEXT NOT NULL CHECK(history IN ('observed','historical-unavailable')),
        status TEXT NOT NULL CHECK(status IN ('complete','failed','unexamined')),
        reason TEXT,
        CHECK((status='complete')=(reason IS NULL))
    ) STRICT;
    CREATE TABLE master_declaration (
        source_sha256 TEXT NOT NULL REFERENCES master_source(source_sha256),
        ordinal INTEGER NOT NULL CHECK(ordinal>0),
        family TEXT NOT NULL CHECK(family IN ('datapoint_type','function_type','function_point','space_usage')),
        element TEXT NOT NULL CHECK(element IN ('DatapointType','DatapointSubtype','FunctionType','FunctionPoint','SpaceUsage')),
        logical_id TEXT, xml_path TEXT NOT NULL,
        byte_start INTEGER NOT NULL CHECK(byte_start>=0),
        byte_end INTEGER NOT NULL CHECK(byte_end>byte_start),
        main_number INTEGER, function_type_id TEXT,
        missing_id INTEGER NOT NULL CHECK(missing_id IN (0,1)),
        orphan_parent INTEGER NOT NULL CHECK(orphan_parent IN (0,1)),
        observed_write INTEGER CHECK(observed_write IN (0,1)),
        CHECK(missing_id=(logical_id IS NULL)),
        CHECK(observed_write IS NULL OR (missing_id=0 AND orphan_parent=0)),
        PRIMARY KEY(source_sha256,ordinal)
    ) STRICT;
    CREATE INDEX master_declaration_identity ON master_declaration(family,logical_id,source_sha256,ordinal);
    CREATE TABLE master_winner (
        family TEXT NOT NULL CHECK(family IN ('datapoint_type','function_type','function_point','space_usage')),
        logical_id TEXT NOT NULL,
        source_sha256 TEXT, ordinal INTEGER,
        CHECK((source_sha256 IS NULL)=(ordinal IS NULL)),
        FOREIGN KEY(source_sha256,ordinal) REFERENCES master_declaration(source_sha256,ordinal),
        PRIMARY KEY(family,logical_id)
    ) STRICT;")?;
    seed_historical_winners(conn)
}
fn seed_historical_winners(conn: &Connection) -> Result<(), ProductDbError> {
    for kind in [
        MasterKind::DatapointType,
        MasterKind::FunctionType,
        MasterKind::FunctionPoint,
        MasterKind::SpaceUsage,
    ] {
        conn.execute(&format!("INSERT INTO master_winner(family,logical_id) SELECT ?1,id FROM {} WHERE true ON CONFLICT DO NOTHING",kind.as_str()),[kind.as_str()])?;
    }
    Ok(())
}

pub(crate) fn retain(conn: &Connection, bytes: &[u8]) -> Result<String, ProductDbError> {
    if bytes.len() > MAX_MASTER_BYTES {
        return Err(error("master exceeds byte budget"));
    }
    let sha = sha256_hex(bytes);
    let saved: Option<(i64, Vec<u8>)> = conn
        .query_row(
            "SELECT len,bytes FROM source_file WHERE sha256=?1",
            [&sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((len, saved)) = saved {
        if len != bytes.len() as i64 || saved != bytes {
            return Err(error(
                "retained master identity does not match bytes/length",
            ));
        }
    } else {
        conn.execute("INSERT INTO source_file(sha256,source_path,len,bytes) VALUES(?1,'knx_master.xml',?2,?3)",params![sha,bytes.len() as i64,bytes])?;
    }
    Ok(sha)
}
fn source_status(
    conn: &Connection,
    sha: &str,
    history: &str,
    status: &str,
    reason: Option<&str>,
) -> Result<(), ProductDbError> {
    conn.execute("INSERT INTO master_source(source_sha256,definition_version,history,status,reason) VALUES(?1,1,?2,?3,?4)
        ON CONFLICT(source_sha256) DO UPDATE SET status=excluded.status,reason=excluded.reason,history=CASE WHEN excluded.history='observed' THEN 'observed' ELSE master_source.history END",params![sha,history,status,reason])?;
    Ok(())
}
pub(crate) fn persist(
    conn: &Connection,
    sha: &str,
    declarations: &[MasterDeclaration],
    observed: bool,
) -> Result<(), ProductDbError> {
    source_status(
        conn,
        sha,
        if observed {
            "observed"
        } else {
            "historical-unavailable"
        },
        "complete",
        None,
    )?;
    for d in declarations {
        conn.execute("INSERT INTO master_declaration(source_sha256,ordinal,family,element,logical_id,xml_path,byte_start,byte_end,main_number,function_type_id,missing_id,orphan_parent,observed_write)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(source_sha256,ordinal) DO UPDATE SET observed_write=coalesce(master_declaration.observed_write,excluded.observed_write)",
            params![sha,d.ordinal,d.family.as_str(),d.element,d.logical_id,d.xml_path,d.byte_start,d.byte_end,d.main_number,d.function_type_id,d.missing_id,d.orphan_parent,if observed {d.observed_write} else {None}])?;
        if observed && d.observed_write == Some(true) {
            conn.execute("INSERT INTO master_winner(family,logical_id,source_sha256,ordinal) VALUES(?1,?2,?3,?4)
                ON CONFLICT(family,logical_id) DO UPDATE SET source_sha256=excluded.source_sha256,ordinal=excluded.ordinal",params![d.family.as_str(),d.logical_id,sha,d.ordinal])?;
        }
    }
    Ok(())
}
pub(crate) fn diagnostics(
    conn: &Connection,
    declarations: &[MasterDeclaration],
) -> Result<Vec<MasterDiagnostic>, ProductDbError> {
    let mut out = Vec::new();
    for d in declarations {
        let malformed = d.missing_id || d.orphan_parent;
        if !malformed && d.observed_write != Some(false) {
            continue;
        }
        let kind = if malformed {
            crate::InstallDiagnosticKind::MalformedMasterDeclaration
        } else {
            crate::InstallDiagnosticKind::MasterIdentityCollision
        };
        let reason = match (d.missing_id, d.orphan_parent) {
            (true, true) => "missing Id; orphan parent",
            (true, false) => "missing Id",
            (false, true) => "orphan parent",
            _ => "identity collision",
        };
        let kept = if malformed {
            String::new()
        } else {
            let winner:Option<(Option<String>,Option<u32>)>=conn.query_row("SELECT source_sha256,ordinal FROM master_winner WHERE family=?1 AND logical_id=?2",params![d.family.as_str(),d.logical_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            match winner {
                Some((Some(sha), Some(n))) => format!("; kept source {sha} declaration {n}"),
                _ => "; kept winner historical-unavailable".into(),
            }
        };
        out.push(MasterDiagnostic {
            kind,
            xml_path: format!("{}[@master-ordinal={}]", d.xml_path, d.ordinal),
            detail: format!(
                "{} {:?}: {}; source {} declaration {} bytes [{}..{}){}",
                d.family.as_str(),
                d.logical_id,
                reason,
                d.source_sha256,
                d.ordinal,
                d.byte_start,
                d.byte_end,
                kept
            ),
        });
    }
    Ok(out)
}

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
    let header:Option<(i64,String,String,Option<String>)>=conn.query_row("SELECT definition_version,history,status,reason FROM master_source WHERE source_sha256=?1",[sha],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let Some((version, history, status, reason)) = header else {
        return Ok(None);
    };
    if version != MASTER_PROVENANCE_VERSION
        || !["observed", "historical-unavailable"].contains(&history.as_str())
        || !["complete", "failed", "unexamined"].contains(&status.as_str())
        || (status == "complete") != reason.is_none()
    {
        return Err(error("invalid master source header"));
    }
    let declarations: Vec<MasterDeclaration> = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM master_declaration WHERE source_sha256=?1 ORDER BY ordinal"
        ))?
        .query_map([sha], declaration)?
        .collect::<Result<Vec<_>, _>>()?;
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
    if let Some(bytes) = bytes.filter(|_| status == "complete") {
        // Reuse the authoritative parser, including unusual carried contexts;
        // never trust persisted offsets or introduce another declaration scanner.
        let actual = crate::parse::master::parse_master(conn, &bytes, false)?.declarations;
        if actual.len() != declarations.len()
            || actual.iter().zip(&declarations).any(|(actual, stored)| {
                let mut stored = stored.clone();
                stored.observed_write = None;
                actual != &stored
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
    let normalized_present: bool = conn.query_row(
        &format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id=?1)",
            family.as_str()
        ),
        [id],
        |r| r.get(0),
    )?;
    let mut winner=conn.query_row("SELECT source_sha256,ordinal FROM master_winner WHERE family=?1 AND logical_id=?2 AND source_sha256 IS NOT NULL",params![family.as_str(),id],|r|Ok(MasterWinner {source_sha256:r.get(0)?,ordinal:r.get(1)?,retained_source_available:false})).optional()?;
    let mut candidates = Vec::new();
    let mut availability = std::collections::BTreeMap::new();
    let sources=conn.prepare("SELECT DISTINCT source_sha256 FROM master_declaration WHERE family=?1 AND logical_id=?2 ORDER BY source_sha256")?.query_map(params![family.as_str(),id],|r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
    // One authenticated source read/parse per distinct source, not per candidate.
    for sha in sources {
        let source = master_source_report(conn, &sha)?
            .ok_or_else(|| error("candidate source header is missing"))?;
        availability.insert(sha, source.retained_source_available);
        candidates.extend(
            source
                .declarations
                .into_iter()
                .filter(|d| d.family == family && d.logical_id.as_deref() == Some(id)),
        );
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MasterReplayReport {
    pub master_sources: u32,
    pub failed_sources: u32,
    pub unexamined_sources: u32,
    /// Current byte-determined declarations, not historical encounters/writes.
    pub declarations: u32,
}

/// Rebuild current declaration evidence atomically. Source failures are named;
/// SQL failures (including final RELEASE) roll back this entire operation.
pub fn rederive_master_provenance(conn: &Connection) -> Result<MasterReplayReport, ProductDbError> {
    atomic(conn, "master_provenance_replay", || replay(conn))
}

fn replay(conn: &Connection) -> Result<MasterReplayReport, ProductDbError> {
    let mut report = MasterReplayReport::default();
    seed_historical_winners(conn)?;
    let mut missing = conn.prepare("SELECT source_sha256 FROM master_source WHERE NOT EXISTS(SELECT 1 FROM source_file s WHERE s.sha256=master_source.source_sha256)
        UNION SELECT source_sha256 FROM package_member m WHERE role='Master' AND NOT EXISTS(SELECT 1 FROM source_file s WHERE s.sha256=m.source_sha256) ORDER BY 1")?;
    for sha in missing.query_map([], |r| r.get::<_, String>(0))? {
        let sha = sha?;
        source_status(
            conn,
            &sha,
            "historical-unavailable",
            "failed",
            Some("retained master source is missing"),
        )?;
        report.master_sources += 1;
        report.failed_sources += 1;
    }
    let mut sources=conn.prepare("SELECT sha256,len,length(bytes),
        EXISTS(SELECT 1 FROM package_member m WHERE m.source_sha256=s.sha256 AND role='Master')
        OR EXISTS(SELECT 1 FROM master_source p WHERE p.source_sha256=s.sha256 AND status<>'unexamined')
        FROM source_file s ORDER BY sha256")?;
    let mut rows = sources.query([])?;
    while let Some(row) = rows.next()? {
        let sha: String = row.get(0)?;
        let declared: i64 = row.get(1)?;
        let actual: i64 = row.get(2)?;
        let known: bool = row.get(3)?;
        if actual < 0 || actual > MAX_MASTER_BYTES as i64 {
            source_status(
                conn,
                &sha,
                "historical-unavailable",
                if known { "failed" } else { "unexamined" },
                Some("source exceeds master inspection byte budget"),
            )?;
            if known {
                report.master_sources += 1;
                report.failed_sources += 1;
            } else {
                report.unexamined_sources += 1;
            }
            continue;
        }
        let bytes: Vec<u8> = conn.query_row(
            "SELECT bytes FROM source_file WHERE sha256=?1",
            [&sha],
            |r| r.get(0),
        )?;
        // Classifying damaged bytes cannot establish the original source kind.
        let classified = crate::ingest::classify(&bytes) == crate::FileKind::MasterData;
        if declared != actual || sha256_hex(&bytes) != sha {
            let is_master = known || classified;
            source_status(
                conn,
                &sha,
                "historical-unavailable",
                if is_master { "failed" } else { "unexamined" },
                Some("source length/SHA-256 identity is invalid"),
            )?;
            if is_master {
                report.master_sources += 1;
                report.failed_sources += 1;
            } else {
                report.unexamined_sources += 1;
            }
            continue;
        }
        if !known && !classified {
            conn.execute(
                "DELETE FROM master_source WHERE source_sha256=?1 AND status='unexamined'",
                [&sha],
            )?;
            continue;
        }
        report.master_sources += 1;
        let derived = crate::xml::validate_complete_document("knx_master.xml", &bytes)
            .and_then(|()| crate::parse::master::parse_master(conn, &bytes, false));
        match derived {
            Ok(derived) => {
                report.declarations = report
                    .declarations
                    .checked_add(derived.declarations.len() as u32)
                    .ok_or_else(|| error("master declaration count overflow"))?;
                persist(conn, &sha, &derived.declarations, false)?;
            }
            Err(ProductDbError::Xml { cause, .. }) => {
                source_status(conn, &sha, "historical-unavailable", "failed", Some(&cause))?;
                report.failed_sources += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(report)
}

fn valid_sha(sha: &str) -> bool {
    sha.len() == 64
        && sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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

pub(crate) fn validate_diagnostic(
    conn: &Connection,
    sha: &str,
    kind: crate::InstallDiagnosticKind,
    path: &str,
    detail: &str,
) -> Result<(), ProductDbError> {
    let invalid = || error("invalid master declaration diagnostic");
    let (xml_path, ordinal) = path
        .strip_suffix(']')
        .and_then(|p| p.rsplit_once("[@master-ordinal="))
        .ok_or_else(invalid)?;
    let ordinal: u32 = ordinal.parse().map_err(|_| invalid())?;
    let d = conn
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM master_declaration WHERE source_sha256=?1 AND ordinal=?2"
            ),
            params![sha, ordinal],
            declaration,
        )
        .optional()?
        .ok_or_else(invalid)?;
    if d.xml_path != xml_path {
        return Err(invalid());
    }
    let malformed = d.missing_id || d.orphan_parent;
    if (kind == crate::InstallDiagnosticKind::MalformedMasterDeclaration) != malformed {
        return Err(invalid());
    }
    let reason = match (d.missing_id, d.orphan_parent) {
        (true, true) => "missing Id; orphan parent",
        (true, false) => "missing Id",
        (false, true) => "orphan parent",
        _ => "identity collision",
    };
    let prefix = format!(
        "{} {:?}: {}; source {} declaration {} bytes [{}..{})",
        d.family.as_str(),
        d.logical_id,
        reason,
        d.source_sha256,
        d.ordinal,
        d.byte_start,
        d.byte_end
    );
    if malformed {
        if detail != prefix {
            return Err(invalid());
        }
    } else {
        let suffix = detail.strip_prefix(&prefix).ok_or_else(invalid)?;
        if suffix != "; kept winner historical-unavailable" {
            let (kept, n) = suffix
                .strip_prefix("; kept source ")
                .and_then(|s| s.split_once(" declaration "))
                .ok_or_else(invalid)?;
            let n: u32 = n.parse().map_err(|_| invalid())?;
            if !valid_sha(kept) || n == 0 {
                return Err(invalid());
            }
            let matching:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM master_declaration WHERE source_sha256=?1 AND ordinal=?2 AND family=?3 AND logical_id=?4 AND observed_write=1)",params![kept,n,d.family.as_str(),d.logical_id],|r|r.get(0))?;
            if !matching {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
