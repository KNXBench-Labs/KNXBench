//! Conservative logical output admission, independent of corpus or machine size.
use super::{error, MasterDeclaration};
use crate::ProductDbError;

pub const MAX_MASTER_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;
const RECORD_BYTES: usize = 256;

pub(crate) struct EvidenceBudget {
    used: usize,
}
impl Default for EvidenceBudget {
    fn default() -> Self {
        Self { used: 1024 }
    }
}
impl EvidenceBudget {
    pub(crate) fn charge(&mut self, bytes: usize) -> Result<(), ProductDbError> {
        self.used = self
            .used
            .checked_add(bytes)
            .filter(|n| *n <= MAX_MASTER_EVIDENCE_BYTES)
            .ok_or_else(|| error("master evidence byte budget exceeded"))?;
        Ok(())
    }
    pub(crate) fn record(&mut self) -> Result<(), ProductDbError> {
        self.charge(RECORD_BYTES)
    }
    // Ingest/replay reserve a second inventory before publishing complete evidence.
    // Persisted reads instead charge the stored and authoritative inventories directly.
    pub(crate) fn reserve_validation(&mut self) -> Result<(), ProductDbError> {
        self.charge(self.used)
    }
    pub(crate) fn prefix(&mut self, id_len: usize) -> Result<(), ProductDbError> {
        self.charge(
            id_len
                .checked_mul(10)
                .and_then(|n| n.checked_add(512))
                .ok_or_else(|| error("master evidence byte budget exceeded"))?,
        )
    }
    pub(crate) fn diagnostic(&mut self, d: &MasterDeclaration) -> Result<(), ProductDbError> {
        // Two copies cover the emitted diagnostic and its persisted-read validation.
        for _ in 0..2 {
            self.record()?;
            self.charge(d.xml_path.len())?;
            self.charge(32)?; // ordinal suffix
            self.prefix(d.logical_id.as_ref().map_or(0, String::len))?;
        }
        Ok(())
    }
    pub(crate) fn stored_diagnostic(
        &mut self,
        archive: &str,
        path: &str,
        detail: &str,
    ) -> Result<(), ProductDbError> {
        self.record()?;
        self.charge(archive.len())?;
        self.charge(path.len())?;
        self.charge(detail.len())
    }
}

pub(crate) fn admit_report(conn: &rusqlite::Connection, sha: &str) -> Result<(), ProductDbError> {
    // SQLite measures UTF-8 byte lengths without copying the strings into Rust.
    let bytes: i64 = conn.query_row("SELECT coalesce(sum(256+length(CAST(archive_path AS BLOB))+length(CAST(xml_path AS BLOB))+length(CAST(detail AS BLOB))),0) FROM package_install_diagnostic WHERE package_sha256=?1 AND kind IN ('master-identity-collision','malformed-master-declaration')", [sha], |r|r.get(0))?;
    EvidenceBudget::default()
        .charge(usize::try_from(bytes).map_err(|_| error("master evidence byte budget exceeded"))?)
}
