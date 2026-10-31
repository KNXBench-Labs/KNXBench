//! Master declaration addresses and observed winners, separate from product identity.
//!
//! See ADR-0147. Replay never writes normalized values or historical reports.
mod bom_ranges;
pub(crate) mod budget;
pub(crate) use bom_ranges::{correct_bom_ranges, migrate_v30_to_v31};
mod reconcile;
use budget::EvidenceBudget;
pub use budget::MAX_MASTER_EVIDENCE_BYTES;
mod diagnostic_reader;
mod reader;
use crate::{sha256_hex, ProductDbError};
use reader::master_source_report_with_budget;
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

fn validate_winner_markers(conn: &Connection) -> Result<(), ProductDbError> {
    for kind in [
        MasterKind::DatapointType,
        MasterKind::FunctionType,
        MasterKind::FunctionPoint,
        MasterKind::SpaceUsage,
    ] {
        let mut stmt = conn.prepare(&format!("SELECT id FROM {}", kind.as_str()))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id = row
                .get_ref(0)?
                .as_str()
                .map_err(|_| error("invalid master identity"))?;
            if matches!(winner_marker(conn, kind, id)?, WinnerMarker::Absent) {
                return Err(error("master winner marker missing during replay"));
            }
        }
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
    reconcile::reconcile(conn, sha, declarations)?;
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
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(source_sha256,ordinal) DO UPDATE SET family=excluded.family,element=excluded.element,logical_id=excluded.logical_id,xml_path=excluded.xml_path,byte_start=excluded.byte_start,byte_end=excluded.byte_end,main_number=excluded.main_number,function_type_id=excluded.function_type_id,missing_id=excluded.missing_id,orphan_parent=excluded.orphan_parent,observed_write=coalesce(master_declaration.observed_write,excluded.observed_write)",
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
    budget: &mut EvidenceBudget,
) -> Result<Vec<MasterDiagnostic>, ProductDbError> {
    let mut out = Vec::new();
    for d in declarations {
        let malformed = d.missing_id || d.orphan_parent;
        if !malformed && d.observed_write != Some(false) {
            continue;
        }
        budget.diagnostic(d)?;
        let kind = if malformed {
            crate::InstallDiagnosticKind::MalformedMasterDeclaration
        } else {
            crate::InstallDiagnosticKind::MasterIdentityCollision
        };
        let kept = if malformed {
            String::new()
        } else {
            match winner_marker(
                conn,
                d.family,
                d.logical_id
                    .as_deref()
                    .ok_or_else(|| error("collision identity missing"))?,
            )? {
                WinnerMarker::Observed(winner) => format!(
                    "; kept source {} declaration {}",
                    winner.source_sha256, winner.ordinal
                ),
                WinnerMarker::Historical => "; kept winner historical-unavailable".into(),
                WinnerMarker::Absent => return Err(error("master winner marker missing")),
            }
        };
        out.push(MasterDiagnostic {
            kind,
            xml_path: format!("{}[@master-ordinal={}]", d.xml_path, d.ordinal),
            detail: format!("{}{kept}", diagnostic_prefix(d)),
        });
    }
    Ok(out)
}

/// The declaration part of a collision or malformed-declaration diagnostic, range included.
/// Writer, reader and the v31 range move share it so the three can never drift apart.
pub(crate) fn diagnostic_prefix(d: &MasterDeclaration) -> String {
    let reason = match (d.missing_id, d.orphan_parent) {
        (true, true) => "missing Id; orphan parent",
        (true, false) => "missing Id",
        (false, true) => "orphan parent",
        _ => "identity collision",
    };
    format!(
        "{} {:?}: {}; source {} declaration {} bytes [{}..{})",
        d.family.as_str(),
        d.logical_id,
        reason,
        d.source_sha256,
        d.ordinal,
        d.byte_start,
        d.byte_end
    )
}

pub use reader::{master_identity, master_source_report};

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
    // Only schema creation may seed explicit historical-unavailable markers.
    // On an existing v27 database, an absent marker is lost evidence, not history.
    validate_winner_markers(conn)?;
    let mut report = MasterReplayReport::default();
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
        let derived =
            crate::xml::validate_complete_document("knx_master.xml", &bytes).and_then(|()| {
                crate::parse::master::parse_master(
                    conn,
                    &bytes,
                    false,
                    crate::operation::Cancellation::Quiet,
                )
            });
        match derived {
            Ok(derived) => {
                report.declarations = report
                    .declarations
                    .checked_add(derived.declarations.len() as u32)
                    .ok_or_else(|| error("master declaration count overflow"))?;
                persist(conn, &sha, &derived.declarations, false)?;
            }
            Err(e @ ProductDbError::Xml { .. })
                if e.to_string().contains("evidence byte budget") =>
            {
                return Err(e)
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

enum WinnerMarker {
    Absent,
    Historical,
    Observed(MasterWinner),
}
fn winner_marker(
    conn: &Connection,
    family: MasterKind,
    id: &str,
) -> Result<WinnerMarker, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT source_sha256,ordinal FROM master_winner WHERE family=?1 AND logical_id=?2",
    )?;
    let mut rows = stmt.query(params![family.as_str(), id])?;
    let Some(row) = rows.next()? else {
        return Ok(WinnerMarker::Absent);
    };
    let ordinal: Option<u32> = row.get(1)?;
    match (row.get_ref(0)?, ordinal) {
        (rusqlite::types::ValueRef::Null, None) => Ok(WinnerMarker::Historical),
        (rusqlite::types::ValueRef::Text(bytes), Some(ordinal)) => {
            // Validate the bounded hash before copying persisted text.
            let sha =
                std::str::from_utf8(bytes).map_err(|_| error("invalid winner source hash"))?;
            if !valid_sha(sha) || ordinal == 0 || ordinal as usize > MAX_MASTER_DECLARATIONS {
                return Err(error("invalid master winner marker"));
            }
            Ok(WinnerMarker::Observed(MasterWinner {
                source_sha256: sha.into(),
                ordinal,
                retained_source_available: false,
            }))
        }
        _ => Err(error("invalid master winner marker pair")),
    }
}

fn valid_sha(sha: &str) -> bool {
    sha.len() == 64
        && sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) use diagnostic_reader::{validate_diagnostic, SourceInventoryCache};
