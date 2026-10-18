//! Raw ZIP directory, read before `zip::ZipArchive` normalises it.
//!
//! `ZipArchive` keeps one entry per file name: a second central-directory
//! entry with the same name replaces the first, and `len()`/`by_index()`
//! show only the survivor. The diagnostic reader therefore walks the
//! end-of-central-directory record and the central file headers itself
//! (APPNOTE.TXT 4.3.12 and 4.3.16), bounded and without extracting
//! anything, and refuses duplicate names and ZIP64. The fixed archive
//! format carries no ZIP comment, so the end record must be the last 22
//! octets: no second, competing end record can hide in a comment. The
//! reader then requires `ZipArchive` to report the same directory start and,
//! entry by entry, the same name, central header and local header offsets,
//! so both parsers are bound to one directory. Shared by formats 1 and 2.

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

/// The end-of-central-directory record: exactly the last 22 octets, with
/// no comment.
fn end_record(bytes: &[u8]) -> Option<usize> {
    let at = bytes.len().checked_sub(EOCD_LEN)?;
    (&bytes[at..at + 4] == EOCD_SIGNATURE && u16_at(bytes, at + 20) == Some(0)).then_some(at)
}

/// One central-directory entry as stored.
#[derive(Debug)]
pub(super) struct Entry<'a> {
    pub(super) name: &'a [u8],
    /// Where this central file header starts.
    pub(super) central_offset: u64,
    /// The local file header offset recorded in it (APPNOTE 4.4.16).
    pub(super) local_offset: u64,
}

/// The central directory the end record points to.
#[derive(Debug)]
pub(super) struct Directory<'a> {
    pub(super) start: u64,
    pub(super) entries: Vec<Entry<'a>>,
}

/// Every central-directory entry, in directory order. Refuses an end
/// record that is not exactly the last 22 octets (a comment, trailing
/// bytes), a missing or ZIP64 directory, more than [`MAX_ENTRIES`] entries,
/// a directory that does not exactly fill its declared extent, and repeated
/// names.
pub(super) fn entries(bytes: &[u8]) -> Result<Directory<'_>, String> {
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
    let mut entries: Vec<Entry> = Vec::with_capacity(usize::from(total));
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
        let local_offset = u32_at(bytes, at + 42).ok_or_else(invalid)?;
        if local_offset == u32::MAX {
            return Err("diagnostics-zip64-refused".into());
        }
        let name_start = at + CENTRAL_LEN;
        let next = name_start + name + extra + comment;
        if next > stop {
            return Err(invalid());
        }
        let name = &bytes[name_start..name_start + name];
        if entries.iter().any(|entry| entry.name == name) {
            return Err("diagnostics-duplicate-member".into());
        }
        entries.push(Entry {
            name,
            central_offset: at as u64,
            local_offset: u64::from(local_offset),
        });
        at = next;
    }
    if at != stop {
        return Err(invalid());
    }
    Ok(Directory {
        start: u64::from(offset),
        entries,
    })
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
        let names: Vec<&[u8]> = entries(&bytes)
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names, [&b"a.json"[..], &b"b.json"[..]]);
        let many: Vec<String> = (0..=MAX_ENTRIES).map(|i| format!("{i}.json")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        assert_eq!(
            entries(&archive(&many)).unwrap_err(),
            "diagnostics-member-count"
        );
    }

    #[test]
    fn entries_carry_the_offsets_zip_archive_reports() {
        let bytes = archive(&["a.json", "b.json", "c.json"]);
        let directory = entries(&bytes).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(directory.start, zip.central_directory_start());
        assert_eq!(directory.entries.len(), zip.len());
        for (i, entry) in directory.entries.iter().enumerate() {
            let file = zip.by_index(i).unwrap();
            assert_eq!(entry.name, file.name().as_bytes());
            assert_eq!(entry.central_offset, file.central_header_start());
            assert_eq!(entry.local_offset, file.header_start());
        }
    }

    #[test]
    fn an_end_record_not_exactly_at_the_end_is_refused() {
        let bytes = archive(&["a.json"]);
        let mut commented = bytes.clone();
        let end = commented.len() - EOCD_LEN;
        commented[end + 20..end + 22].copy_from_slice(&1u16.to_le_bytes());
        commented.push(b'!');
        assert_eq!(entries(&commented).unwrap_err(), "diagnostics-invalid-zip");
        let mut padded = bytes.clone();
        padded.push(0);
        assert_eq!(entries(&padded).unwrap_err(), "diagnostics-invalid-zip");
    }

    #[test]
    fn truncated_or_relocated_directories_are_refused() {
        let bytes = archive(&["a.json"]);
        for cut in [1, EOCD_LEN, bytes.len() / 2] {
            assert!(entries(&bytes[..bytes.len() - cut]).is_err());
        }
        let end = end_record(&bytes).unwrap();
        let mut moved = bytes.clone();
        moved[end + 16] ^= 1;
        assert!(entries(&moved).is_err());
        let mut zip64 = bytes.clone();
        zip64[end + 10..end + 12].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(entries(&zip64).unwrap_err(), "diagnostics-zip64-refused");
    }
}
