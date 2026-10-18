//! Raw ZIP entry names, read before `zip::ZipArchive` normalises them.
//!
//! `ZipArchive` keeps one entry per file name: a second central-directory
//! entry with the same name replaces the first, and `len()`/`by_index()`
//! show only the survivor. The diagnostic reader therefore walks the
//! end-of-central-directory record and the central file headers itself
//! (APPNOTE.TXT 4.3.12 and 4.3.16), bounded and without extracting
//! anything, and refuses duplicate names and ZIP64. Shared by formats 1
//! and 2.

const EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";
const EOCD_LEN: usize = 22;
const ZIP64_LOCATOR_SIGNATURE: &[u8; 4] = b"PK\x06\x07";
const ZIP64_LOCATOR_LEN: usize = 20;
const CENTRAL_SIGNATURE: &[u8; 4] = b"PK\x01\x02";
const CENTRAL_LEN: usize = 46;
/// Bounds the walk; a diagnostic archive has four or five members.
const MAX_ENTRIES: u16 = 64;

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The end-of-central-directory record: the last signature whose comment
/// ends exactly at the end of the archive.
fn end_record(bytes: &[u8]) -> Option<usize> {
    let last = bytes.len().checked_sub(EOCD_LEN)?;
    let first = last.saturating_sub(usize::from(u16::MAX));
    (first..=last).rev().find(|&at| {
        &bytes[at..at + 4] == EOCD_SIGNATURE
            && u16_at(bytes, at + 20).map(usize::from) == Some(bytes.len() - at - EOCD_LEN)
    })
}

/// Every central-directory entry name, in directory order. Refuses a
/// missing or ZIP64 directory, more than [`MAX_ENTRIES`] entries, a
/// directory that does not exactly fill its declared extent, and repeated
/// names.
pub(super) fn entry_names(bytes: &[u8]) -> Result<Vec<&[u8]>, String> {
    let invalid = || "diagnostics-invalid-zip".to_string();
    let end = end_record(bytes).ok_or_else(invalid)?;
    let field = |offset| u16_at(bytes, end + offset).ok_or_else(invalid);
    let (disk, directory_disk, on_disk, total) = (field(4)?, field(6)?, field(8)?, field(10)?);
    let size = u32_at(bytes, end + 12).ok_or_else(invalid)?;
    let offset = u32_at(bytes, end + 16).ok_or_else(invalid)?;
    let zip64_locator = end
        .checked_sub(ZIP64_LOCATOR_LEN)
        .is_some_and(|at| &bytes[at..at + 4] == ZIP64_LOCATOR_SIGNATURE);
    if zip64_locator
        || [disk, directory_disk, on_disk, total].contains(&u16::MAX)
        || [size, offset].contains(&u32::MAX)
    {
        return Err("diagnostics-zip64-refused".into());
    }
    if disk != 0 || directory_disk != 0 || on_disk != total {
        return Err(invalid());
    }
    if total > MAX_ENTRIES {
        return Err("diagnostics-member-count".into());
    }
    let start = usize::try_from(offset).map_err(|_| invalid())?;
    let stop = start
        .checked_add(usize::try_from(size).map_err(|_| invalid())?)
        .filter(|&stop| stop <= end)
        .ok_or_else(invalid)?;
    let mut names: Vec<&[u8]> = Vec::with_capacity(usize::from(total));
    let mut at = start;
    for _ in 0..total {
        if at
            .checked_add(CENTRAL_LEN)
            .is_none_or(|header| header > stop)
            || &bytes[at..at + 4] != CENTRAL_SIGNATURE
        {
            return Err(invalid());
        }
        let length = |offset| {
            u16_at(bytes, at + offset)
                .map(usize::from)
                .ok_or_else(invalid)
        };
        let (name, extra, comment) = (length(28)?, length(30)?, length(32)?);
        let name_start = at + CENTRAL_LEN;
        let next = name_start + name + extra + comment;
        if next > stop {
            return Err(invalid());
        }
        let name = &bytes[name_start..name_start + name];
        if names.contains(&name) {
            return Err("diagnostics-duplicate-member".into());
        }
        names.push(name);
        at = next;
    }
    if at != stop {
        return Err(invalid());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn archive(names: &[&str]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(vec![]));
        for name in names {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"{}").unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn names_come_back_in_directory_order() {
        let bytes = archive(&["a.json", "b.json"]);
        assert_eq!(
            entry_names(&bytes).unwrap(),
            [&b"a.json"[..], &b"b.json"[..]]
        );
        let many: Vec<String> = (0..=MAX_ENTRIES).map(|i| format!("{i}.json")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        assert_eq!(
            entry_names(&archive(&many)).unwrap_err(),
            "diagnostics-member-count"
        );
    }

    #[test]
    fn truncated_or_relocated_directories_are_refused() {
        let bytes = archive(&["a.json"]);
        for cut in [1, EOCD_LEN, bytes.len() / 2] {
            assert!(entry_names(&bytes[..bytes.len() - cut]).is_err());
        }
        let end = end_record(&bytes).unwrap();
        let mut moved = bytes.clone();
        moved[end + 16] ^= 1;
        assert!(entry_names(&moved).is_err());
        let mut zip64 = bytes.clone();
        zip64[end + 10..end + 12].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            entry_names(&zip64).unwrap_err(),
            "diagnostics-zip64-refused"
        );
    }
}
