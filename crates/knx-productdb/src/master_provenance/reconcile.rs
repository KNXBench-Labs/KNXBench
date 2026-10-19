//! Byte-derived structure can be repaired; lost encounter outcomes cannot.
use super::*;

pub(super) fn reconcile(
    conn: &Connection,
    sha: &str,
    declarations: &[MasterDeclaration],
) -> Result<(), ProductDbError> {
    let history: Option<i64> = conn.query_row(
        "SELECT CASE WHEN definition_version<>1 THEN -1 WHEN history='observed' THEN 1 WHEN history='historical-unavailable' THEN 0 ELSE -1 END FROM master_source WHERE source_sha256=?1", [sha], |r| r.get(0),
    ).optional()?;
    let observed = match history {
        Some(1) => true,
        Some(0) | None => false,
        _ => return Err(error("invalid master source history during reconciliation")),
    };
    let ambiguous =
        || error("master replay cannot reconcile lost historical outcomes or winner keys");
    let mut rows = conn.prepare("SELECT ordinal,family,element,logical_id,missing_id,orphan_parent,observed_write FROM master_declaration WHERE source_sha256=?1 ORDER BY ordinal")?;
    let mut rows = rows.query([sha])?;
    let mut present = vec![false; declarations.len()];
    while let Some(row) = rows.next()? {
        let ordinal: u32 = row.get(0)?;
        let outcome: Option<i64> = row.get(6)?;
        let d = ordinal
            .checked_sub(1)
            .and_then(|n| declarations.get(n as usize));
        if let Some(d) = d {
            present[ordinal as usize - 1] = true;
            let same_identity = row.get_ref(1)?.as_str().map_err(|_| ambiguous())?
                == d.family.as_str()
                && row.get_ref(2)?.as_str().map_err(|_| ambiguous())? == d.element
                && match row.get_ref(3)? {
                    rusqlite::types::ValueRef::Null => d.logical_id.is_none(),
                    value => {
                        Some(value.as_str().map_err(|_| ambiguous())?) == d.logical_id.as_deref()
                    }
                }
                && row.get::<_, i64>(4)? == i64::from(d.missing_id)
                && row.get::<_, i64>(5)? == i64::from(d.orphan_parent);
            if outcome.is_some() && (!same_identity || !matches!(outcome, Some(0 | 1)))
                || observed && !d.missing_id && !d.orphan_parent && outcome.is_none()
                || !observed && outcome.is_some()
            {
                return Err(ambiguous());
            }
        } else {
            let referenced: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM master_winner WHERE source_sha256=?1 AND ordinal=?2)",
                params![sha, ordinal],
                |r| r.get(0),
            )?;
            if outcome.is_some() || referenced {
                return Err(ambiguous());
            }
            conn.execute(
                "DELETE FROM master_declaration WHERE source_sha256=?1 AND ordinal=?2",
                params![sha, ordinal],
            )?;
        }
    }
    if observed
        && declarations
            .iter()
            .zip(present)
            .any(|(d, present)| !present && !d.missing_id && !d.orphan_parent)
    {
        return Err(ambiguous());
    }
    let mut winners =
        conn.prepare("SELECT family,logical_id,ordinal FROM master_winner WHERE source_sha256=?1")?;
    let mut winners = winners.query([sha])?;
    while let Some(row) = winners.next()? {
        let ordinal: u32 = row.get(2)?;
        let d = ordinal
            .checked_sub(1)
            .and_then(|n| declarations.get(n as usize))
            .ok_or_else(ambiguous)?;
        let outcome: Option<i64> = conn.query_row("SELECT observed_write FROM master_declaration WHERE source_sha256=?1 AND ordinal=?2",params![sha,ordinal],|r|r.get(0)).optional()?.flatten();
        if row.get_ref(0)?.as_str().map_err(|_| ambiguous())? != d.family.as_str()
            || row.get_ref(1)?.as_str().map_err(|_| ambiguous())?
                != d.logical_id.as_deref().unwrap_or_default()
            || d.missing_id
            || d.orphan_parent
            || outcome != Some(1)
        {
            return Err(ambiguous());
        }
    }
    Ok(())
}
