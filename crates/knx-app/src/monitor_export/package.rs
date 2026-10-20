//! Bounded deterministic in-memory JSON and ZIP serialization.
use super::budget::MAX_OUTPUT;
use serde::Serialize;
use std::io::{self, Cursor, Seek, SeekFrom, Write};
#[cfg(test)]
mod tests;

pub(super) struct Bounded {
    cursor: Cursor<Vec<u8>>,
    limit: usize,
}
impl Bounded {
    fn new(limit: usize) -> Self {
        Self {
            cursor: Cursor::new(Vec::new()),
            limit,
        }
    }
}
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self
            .cursor
            .position()
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| io::Error::other("output limit exceeded"))?;
        if end > self.limit as u64 {
            return Err(io::Error::other("output limit exceeded (64 MiB)"));
        }
        self.cursor.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Seek for Bounded {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let old = self.cursor.position();
        let n = self.cursor.seek(position)?;
        if n > self.limit as u64 {
            self.cursor.set_position(old);
            return Err(io::Error::other("output limit exceeded"));
        }
        Ok(n)
    }
}
pub(super) fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut out = Bounded::new(MAX_OUTPUT);
    serde_json::to_writer_pretty(&mut out, value)
        .map_err(|e| format!("JSON output failed: {e}"))?;
    out.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(out.cursor.into_inner())
}
const README:&str="KNXBench monitoring-sparse-v1\n\nThis is a reduced current normalized snapshot, not a backup.\nIt is private project data and is not anonymized.\nOnly the selected installation is exported; other installations are outside\nselection scope, not counted as omissions. Counts are unique entities before\naddressless-device pruning. All objects of addressed devices are included,\nincluding inactive and unlinked objects; activity is not represented.\nObject/space/function/range identifiers are local to this native project.\nUnknown security and unknown individual flags are omitted with report issues.\nNo complete external schema compatibility or receiver acceptance is claimed.\nThere is no verified JSON/ZIP receiver ingress.\nParameters, notes, instructions, commissioning/recovery/history, keys/passwords,\nmanufacturer sources, opaque baggage and attachments are excluded.\nSee export-report.json for unavailable metadata and projection losses.\nproject_sha256 hashes the exact project.json bytes, including the final newline.\nProjection admission: 100000 entity records, 1000000 reference entries,\n32 MiB normalized input text/reference bytes, 64 hierarchy levels, conservative\n64 MiB generated-copy admission, and bounded 64 MiB JSON/report/ZIP writers.\nThese bounds do not cover an adapter's native/history loading or snapshot clone.\n";
pub(super) fn zip(project: &[u8], report: &[u8]) -> Result<Vec<u8>, String> {
    let mut z = zip::ZipWriter::new(Bounded::new(MAX_OUTPUT));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    for (name, bytes) in [
        ("project.json", project),
        ("export-report.json", report),
        ("README.txt", README.as_bytes()),
    ] {
        z.start_file(name, options)
            .map_err(|e| format!("ZIP output failed: {e}"))?;
        z.write_all(bytes)
            .map_err(|e| format!("ZIP output failed: {e}"))?;
    }
    Ok(z.finish()
        .map_err(|e| format!("ZIP output failed: {e}"))?
        .cursor
        .into_inner())
}
