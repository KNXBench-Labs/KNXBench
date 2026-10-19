//! One authoritative source inventory validation per distinct report source.
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct SourceInventoryCache {
    sources: BTreeMap<String, MasterSourceReport>,
    pub(crate) budget: EvidenceBudget,
}
impl SourceInventoryCache {
    fn load(&mut self, conn: &Connection, sha: &str) -> Result<(), ProductDbError> {
        if !self.sources.contains_key(sha) {
            self.budget.charge(sha.len())?;
            let source = master_source_report_with_budget(conn, sha, &mut self.budget)?
                .ok_or_else(|| error("diagnostic source header missing"))?;
            self.sources.insert(sha.to_owned(), source);
        }
        Ok(())
    }
    fn declaration(&self, sha: &str, ordinal: u32) -> Result<&MasterDeclaration, ProductDbError> {
        self.sources
            .get(sha)
            .and_then(|s| {
                ordinal
                    .checked_sub(1)
                    .and_then(|n| s.declarations.get(n as usize))
            })
            .filter(|d| d.ordinal == ordinal)
            .ok_or_else(|| error("diagnostic declaration missing"))
    }
}

pub(crate) fn validate_diagnostic(
    conn: &Connection,
    cache: &mut SourceInventoryCache,
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
    cache.load(conn, sha)?;
    let d = cache.declaration(sha, ordinal)?;
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
    let id_len = d.logical_id.as_ref().map_or(0, String::len);
    cache.budget.prefix(id_len)?;
    let d = cache.declaration(sha, ordinal)?;
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
        return if detail == prefix {
            Ok(())
        } else {
            Err(invalid())
        };
    }
    let suffix = detail.strip_prefix(&prefix).ok_or_else(invalid)?;
    if suffix == "; kept winner historical-unavailable" {
        return Ok(());
    }
    let (kept, n) = suffix
        .strip_prefix("; kept source ")
        .and_then(|s| s.split_once(" declaration "))
        .ok_or_else(invalid)?;
    let n: u32 = n.parse().map_err(|_| invalid())?;
    if !valid_sha(kept) || n == 0 {
        return Err(invalid());
    }
    cache.load(conn, kept)?;
    let d = cache.declaration(sha, ordinal)?;
    let winner = cache.declaration(kept, n)?;
    if winner.family != d.family
        || winner.logical_id != d.logical_id
        || winner.observed_write != Some(true)
    {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_cache_loads_each_inventory_once_and_fresh_reports_revalidate() {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let xml = b"<KNX><MasterData><SpaceUsage Id='A'/><SpaceUsage Id='A'/></MasterData></KNX>";
        let sha = crate::ingest_master_data(&conn, xml).unwrap().source_sha256;
        let mut cache = SourceInventoryCache::default();
        cache.load(&conn, &sha).unwrap();
        let end = cache.declaration(&sha, 1).unwrap().byte_end;
        // A load for another diagnostic of this source must use the already
        // admitted inventory, not rehash/reparse or query a declaration row.
        conn.execute(
            "UPDATE master_declaration SET byte_end=byte_end+1 WHERE ordinal=1",
            [],
        )
        .unwrap();
        cache.load(&conn, &sha).unwrap();
        assert_eq!(cache.sources.len(), 1);
        assert_eq!(cache.declaration(&sha, 1).unwrap().byte_end, end);
        assert!(SourceInventoryCache::default().load(&conn, &sha).is_err());
    }
}
