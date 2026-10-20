//! Bounded targeted evidence and per-member report reconciliation.
use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use super::{failure, record_failure, source_bytes, target};
use crate::report::{UnknownConstruct, UnknownKind};
use crate::ProductDbError;

pub(crate) type UnknownKey = (String, String, String);
pub(crate) fn key(row: &UnknownConstruct) -> UnknownKey {
    (
        row.xpath.clone(),
        row.kind.as_str().into(),
        row.name.clone(),
    )
}

/// Fixed repair working-set allowance, not a KNX or source-format limit.
/// One input is parsed at a time; the retained cross-source target inventory
/// and each package rewrite each have their own monotonic allowance. Refusal
/// preserves the affected source/report instead of truncating its evidence.
const MAX_REPAIR_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;
#[derive(Default, Clone)]
pub(crate) struct RepairBudget(usize);
impl RepairBudget {
    pub(crate) fn charge(&mut self, bytes: usize) -> Result<(), ProductDbError> {
        self.0 = self
            .0
            .checked_add(bytes)
            .filter(|n| *n <= MAX_REPAIR_EVIDENCE_BYTES)
            .ok_or_else(|| {
                failure(
                    "promotion repair evidence",
                    "16 MiB evidence working-set allowance exceeded",
                )
            })?;
        Ok(())
    }
    pub(super) fn row(&mut self, row: &UnknownConstruct) -> Result<(), ProductDbError> {
        // Reserve both inventory and key/map copies before keeping either.
        for bytes in [
            512,
            row.xpath.len(),
            row.name.len(),
            row.sample.as_ref().map_or(0, String::len),
        ] {
            self.charge(bytes)?;
            self.charge(bytes)?;
        }
        Ok(())
    }
    fn keys<'a>(
        &mut self,
        keys: impl Iterator<Item = &'a UnknownKey>,
    ) -> Result<(), ProductDbError> {
        for (path, kind, name) in keys {
            for bytes in [256, path.len(), kind.len(), name.len()] {
                self.charge(bytes)?;
            }
        }
        Ok(())
    }
    pub(super) fn evidence(
        &mut self,
        sha: &str,
        evidence: &Evidence,
    ) -> Result<(), ProductDbError> {
        self.charge(256 + sha.len())?;
        for row in &evidence.ordinary {
            self.row(row)?;
        }
        if let Some(extended) = &evidence.extended {
            for row in extended {
                self.row(row)?;
            }
        }
        self.keys(evidence.original_keys.iter())
    }
}

pub(super) struct Evidence {
    pub(super) ordinary: Vec<UnknownConstruct>,
    /// Identical modes share one vector rather than duplicating every source.
    pub(super) extended: Option<Vec<UnknownConstruct>>,
    pub(super) original_keys: BTreeSet<UnknownKey>,
}
impl Evidence {
    pub(super) fn for_scheme(&self, extended: bool) -> &[UnknownConstruct] {
        if extended {
            self.extended.as_deref().unwrap_or(&self.ordinary)
        } else {
            &self.ordinary
        }
    }
}

/// Only exact keys touched by program sources may change. Other sources with
/// the same suffix are not a promotion domain. If they share an exact key,
/// their existing source-bound contribution participates in member order too.
pub(crate) struct ReportRepair {
    pub(crate) affected: BTreeSet<UnknownKey>,
    pub(crate) candidate_members: BTreeMap<UnknownKey, u64>,
    /// These non-program contributions were never retired from the package
    /// counter, even if retirement deleted their shared aggregate report row.
    pub(crate) preserved_members: u64,
    pub(crate) replacements: Vec<UnknownConstruct>,
}

const SOURCE_FILTER: &str = "source_sha256=?1 AND ((kind='Attribute' AND name IN ('TextParameterRefId','Access','Offset','BitOffset','ObjectIndex','ObjectType','PropertyId','Occurrence')) OR (kind='Element' AND name='Property'))";

pub(super) fn admit_source_keys(
    conn: &Connection,
    sha: &str,
    budget: &mut RepairBudget,
) -> Result<(), ProductDbError> {
    let size:i64=conn.query_row(&format!("SELECT coalesce(sum(256+length(CAST(xpath AS BLOB))+length(CAST(kind AS BLOB))+length(CAST(name AS BLOB))),0) FROM ingest_unknown WHERE {SOURCE_FILTER}"),[sha],|r|r.get(0))?;
    budget.charge(usize::try_from(size).map_err(|_| failure(sha, "invalid stored key size"))?)
}

pub(super) fn source_target_keys(
    conn: &Connection,
    sha: &str,
) -> Result<BTreeSet<UnknownKey>, ProductDbError> {
    source_target_keys_in(conn, sha, &mut RepairBudget::default())
}
fn source_target_keys_in(
    conn: &Connection,
    sha: &str,
    budget: &mut RepairBudget,
) -> Result<BTreeSet<UnknownKey>, ProductDbError> {
    admit_source_keys(conn, sha, budget)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT xpath,kind,name FROM ingest_unknown WHERE {SOURCE_FILTER} ORDER BY id"
    ))?;
    let mut out = BTreeSet::new();
    for row in stmt.query_map([sha], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (path, kind, name) = row?;
        if target(&path, &kind, &name) {
            out.insert((path, kind, name));
        }
    }
    Ok(out)
}

pub(super) fn source_targets(
    conn: &Connection,
    sha: &str,
) -> Result<Vec<UnknownConstruct>, ProductDbError> {
    // Bound stored string allocations before decoding/copying them. The SQL
    // prefilter includes only the promotion vocabulary; paths remain parser-owned.
    let filter="source_sha256=?1 AND ((kind='Attribute' AND name IN ('TextParameterRefId','Access','Offset','BitOffset','ObjectIndex','ObjectType','PropertyId','Occurrence')) OR (kind='Element' AND name='Property'))";
    let size:i64=conn.query_row(&format!("SELECT coalesce(sum(512+length(CAST(xpath AS BLOB))+length(CAST(name AS BLOB))+coalesce(length(CAST(sample AS BLOB)),0)),0) FROM ingest_unknown WHERE {filter}"),[sha],|r|r.get(0))?;
    let mut budget = RepairBudget::default();
    budget
        .charge(usize::try_from(size).map_err(|_| failure(sha, "invalid stored evidence size"))?)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT xpath,kind,name,occurrences,sample FROM ingest_unknown WHERE {filter} ORDER BY id"
    ))?;
    let mut out = Vec::new();
    for row in stmt.query_map([sha], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<String>>(4)?,
        ))
    })? {
        let (xpath, kind, name, count, sample) = row?;
        if !target(&xpath, &kind, &name) {
            continue;
        }
        let occurrences = u32::try_from(count)
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| failure(sha, "invalid target occurrence count"))?;
        out.push(UnknownConstruct {
            xpath,
            kind: if kind == "Attribute" {
                UnknownKind::Attribute
            } else {
                UnknownKind::Element
            },
            name,
            occurrences,
            sample,
        });
    }
    Ok(out)
}

struct Member {
    sha: String,
    path: String,
    role: String,
}
impl Member {
    fn semantic(&self) -> bool {
        matches!(
            self.role.as_str(),
            "Master" | "Catalog" | "Hardware" | "ApplicationProgram" | "Baggages"
        )
    }
}
fn matches_key(row: &UnknownConstruct, k: &UnknownKey) -> bool {
    row.xpath == k.0 && row.kind.as_str() == k.1 && row.name == k.2
}
fn admitted_key(
    row: &UnknownConstruct,
    budget: &mut RepairBudget,
) -> Result<UnknownKey, ProductDbError> {
    budget.charge(256 + row.xpath.len() + row.kind.as_str().len() + row.name.len())?;
    Ok(key(row))
}
fn member_candidates(
    evidence: &Evidence,
    extended: bool,
    budget: &mut RepairBudget,
) -> Result<BTreeSet<UnknownKey>, ProductDbError> {
    let mut keys = BTreeSet::new();
    for k in &evidence.original_keys {
        budget.keys(std::iter::once(k))?;
        keys.insert(k.clone());
    }
    for row in evidence.for_scheme(extended) {
        keys.insert(admitted_key(row, budget)?);
    }
    Ok(keys)
}

/// Source bytes authenticate only the source, not arbitrary global ledger rows.
/// Replay the actual admitted non-program role, then require exact ledger
/// agreement for the affected keys. Never heal unrelated history from replay.
fn verified_nonprogram_rows(
    conn: &Connection,
    member: &Member,
    extended: bool,
    affected: &BTreeSet<UnknownKey>,
    budget: &RepairBudget,
) -> Result<Vec<UnknownConstruct>, ProductDbError> {
    if !member.semantic() {
        return Ok(Vec::new());
    }
    let bytes = source_bytes(conn, &member.sha, &member.path)?;
    let expected = match member.role.as_str() {
        "Master" => crate::FileKind::MasterData,
        "Catalog" => crate::FileKind::Catalog,
        "Hardware" => crate::FileKind::Hardware,
        "Baggages" => crate::FileKind::Baggages,
        _ => return Err(failure(&member.path, "unsupported non-program role")),
    };
    super::preparse_admission(conn, &member.sha, &member.path, &bytes, budget)?;
    if crate::ingest::classify(&bytes) != expected {
        return Err(failure(
            &member.path,
            "non-program source no longer matches member role",
        ));
    }
    crate::xml::validate_complete_document(&member.path, &bytes)?;
    let scratch = crate::migration::promotion_scratch()?;
    let mut rows = if member.role == "Master" {
        let mut unknown = crate::parse::master::ingest_master_data(&scratch, &bytes)?.unknown;
        if extended {
            crate::parse::scheme_evidence::reconcile_package_unknowns(
                &bytes,
                &member.path,
                &mut unknown,
            )?;
        }
        unknown
    } else {
        crate::ingest::ingest_file_in_transaction(&scratch, &member.path, &bytes, true, extended)?
            .unknown_constructs
    };
    rows.retain(|r| affected.iter().any(|k| matches_key(r, k)));
    let mut original = source_targets(conn, &member.sha)?;
    original.retain(|r| affected.iter().any(|k| matches_key(r, k)));
    let order = |a: &UnknownConstruct, b: &UnknownConstruct| {
        a.xpath
            .cmp(&b.xpath)
            .then(a.kind.as_str().cmp(b.kind.as_str()))
            .then(a.name.cmp(&b.name))
    };
    rows.sort_by(order);
    original.sort_by(order);
    if rows != original {
        return Err(failure(
            &member.path,
            "non-program target ledger is not complete source-derived evidence",
        ));
    }
    Ok(rows)
}

fn assemble(
    conn: &Connection,
    members: &[Member],
    extended: bool,
    repaired: &BTreeMap<String, Evidence>,
) -> Result<ReportRepair, ProductDbError> {
    let mut budget = RepairBudget::default();
    let mut affected = BTreeSet::new();
    for member in members.iter().filter(|m| m.role == "ApplicationProgram") {
        if let Some(evidence) = repaired.get(&member.sha) {
            // Each owned key is admitted before creating or copying it.
            for candidate in member_candidates(evidence, extended, &mut budget)? {
                affected.insert(candidate);
            }
        }
    }
    let mut candidate_members = BTreeMap::new();
    // Old program retirement could delete a merged key even when the program
    // no longer contributes it at all. Include independently verified semantic
    // non-program targets, but never keys from opaque members.
    for member in members
        .iter()
        .filter(|m| m.semantic() && m.role != "ApplicationProgram")
    {
        for k in source_target_keys_in(conn, &member.sha, &mut budget)? {
            affected.insert(k);
        }
    }
    let mut preserved_members = 0_u64;
    let mut replacements = Vec::new();
    for member in members {
        if !member.semantic() {
            continue;
        }
        let rows = if member.role == "ApplicationProgram" {
            let evidence = repaired
                .get(&member.sha)
                .ok_or_else(|| failure(&member.path, "program member was not repaired"))?;
            for candidate in member_candidates(evidence, extended, &mut budget)? {
                // The candidate map owns another key: reserve before copying.
                budget.keys(std::iter::once(&candidate))?;
                let count = candidate_members.entry(candidate).or_insert(0_u64);
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| failure(&member.sha, "member counter overflow"))?;
            }
            for row in evidence.for_scheme(extended) {
                budget.row(row)?;
            }
            evidence.for_scheme(extended).to_vec()
        } else {
            let rows = verified_nonprogram_rows(conn, member, extended, &affected, &budget)?;
            for row in &rows {
                budget.row(row)?;
            }
            preserved_members = preserved_members
                .checked_add(rows.len() as u64)
                .ok_or_else(|| failure(&member.sha, "preserved member counter overflow"))?;
            rows
        };
        replacements.extend(
            rows.into_iter()
                .filter(|row| affected.iter().any(|k| matches_key(row, k))),
        );
    }
    Ok(ReportRepair {
        affected,
        candidate_members,
        preserved_members,
        replacements,
    })
}

pub(super) fn repair_reports(
    conn: &Connection,
    repaired: &BTreeMap<String, Evidence>,
    failed: &BTreeSet<String>,
) -> Result<(), ProductDbError> {
    let packages = conn
        .prepare("SELECT sha256,scheme FROM package ORDER BY sha256")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (package, scheme) in packages {
        let members = conn
            .prepare(
                "SELECT source_sha256,path,role FROM package_member WHERE package_sha256=?1 ORDER BY ordinal",
            )?
            .query_map([&package], |r| Ok(Member { sha:r.get(0)?, path:r.get(1)?, role:r.get(2)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        if members
            .iter()
            .any(|m| m.role == "ApplicationProgram" && failed.contains(&m.sha))
        {
            crate::package::mark_report_unavailable(conn, &package)?;
            continue;
        }
        if !members
            .iter()
            .any(|m| m.role == "ApplicationProgram" && repaired.contains_key(&m.sha))
        {
            continue;
        }
        let measured:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM package_install_report WHERE package_sha256=?1 AND status='measured')",[&package],|r|r.get(0))?;
        if !measured {
            continue;
        }
        conn.execute_batch("SAVEPOINT promotion_report")?;
        let result =
            assemble(conn, &members, matches!(scheme, 21 | 23), repaired).and_then(|repair| {
                crate::package::reconcile_promotion_report(conn, &package, repair)?
                    .map_err(|e| failure(&package, e.0))
            });
        match result {
            Ok(()) => conn.execute_batch("RELEASE promotion_report")?,
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(error) => {
                conn.execute_batch("ROLLBACK TO promotion_report; RELEASE promotion_report")?;
                record_failure(conn, &package, "promotion_report_backfill", &error)?;
                crate::package::mark_report_unavailable(conn, &package)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_budget_admits_exact_boundary_and_refuses_growth_and_overflow() {
        let mut budget = RepairBudget::default();
        budget.charge(MAX_REPAIR_EVIDENCE_BYTES).unwrap();
        assert!(budget.charge(1).is_err());
        assert!(RepairBudget(1).charge(usize::MAX).is_err());
    }
}
