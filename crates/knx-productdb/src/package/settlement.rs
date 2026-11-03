//! Inventories all physical entries through production ZIP admission, including empty directories.
use super::*;
#[cfg(test)]
#[path = "settlement_review.rs"]
mod review;

/// Ordered, security-admitted physical member identity and unpacked bytes, not package_member rows.
#[derive(Debug, PartialEq, Eq)]
pub struct SettlementMember {
    pub local_ordinal: usize,
    pub raw_name: Vec<u8>,
    pub name: String,
    pub flags: u16,
    pub central_extra: Vec<u8>,
    pub local_extra: Vec<u8>,
    pub unix_mode: Option<u32>,
    pub directory: bool,
    pub bytes: Vec<u8>,
}

/// Read-only bounded comparison inventory; grants no provenance, signature or execution authority.
pub fn settlement_inventory(
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<Vec<SettlementMember>, PackageError> {
    if bytes.len() > MAX_PACKAGE_INPUT_BYTES as usize {
        return Err(PackageError::SizeLimit {
            path: "package input".into(),
        });
    }
    let directory = preflight_zip(bytes, Cancellation::Quiet)?;
    let physical = validate_central_directory(bytes, directory, Cancellation::Quiet)?;
    let mut local_order: Vec<_> = physical
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| (entry.local_offset, i))
        .collect();
    local_order.sort_unstable();
    let mut local_ordinals = vec![0; local_order.len()];
    // A central-directory reader can legitimately ignore prefixes, padding and orphan
    // local records. This stricter comparison cannot: account for every pre-directory
    // byte using only validated record ranges (including data descriptors), not magic
    // signature searches that would mistake indexed payload or comment bytes for files.
    let mut covered_end = 0;
    for (ordinal, (offset, index)) in local_order.into_iter().enumerate() {
        if offset != usize_to_u64(covered_end, "covered local-record extent")? {
            return Err(zip_error(
                "unaccounted physical bytes in settlement inventory",
            ));
        }
        covered_end = physical.entries[index].record_end;
        local_ordinals[index] = ordinal;
    }
    if covered_end != directory.start {
        return Err(zip_error(
            "unaccounted physical bytes before central directory",
        ));
    }
    let mut checked = open_checked_archive(bytes, limits, Cancellation::Quiet)?;
    let mut members = Vec::new();
    for (index, identity) in physical.entries.iter().enumerate() {
        let file = checked.archive.by_index_raw(index).map_err(zip_error)?;
        let raw_name = file.name_raw().to_vec();
        let name = file.name().map_err(zip_error)?.into_owned();
        let unix_mode = file.unix_mode();
        let directory = file.is_dir();
        if unix_mode.is_some_and(|m| {
            !matches!(m & 0o170000, 0 | 0o100000 | 0o040000)
                || (m & 0o170000 == 0o040000 && !directory)
                || (m & 0o170000 == 0o100000 && directory)
        }) {
            return Err(PackageError::UnsafeMember { path: name });
        }
        drop(file);
        let (actual, data) = read_checked_member(
            &mut checked.archive,
            index,
            limits.max_member_size,
            Cancellation::Quiet,
        )?;
        if name != actual || data.is_none() != directory {
            return Err(zip_error("member identity changed"));
        }
        members.push(SettlementMember {
            local_ordinal: local_ordinals[index],
            raw_name,
            name,
            flags: identity.flags,
            central_extra: bytes[identity.central_extra.clone()].to_vec(),
            local_extra: bytes[identity.local_extra.clone()].to_vec(),
            unix_mode,
            directory,
            bytes: data.unwrap_or_default(),
        });
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn settlement_inventory_refuses_unsafe_duplicate_encrypted_and_directory_data() {
        let make = |names: &[(&str, &[u8])]| {
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            for (name, data) in names {
                zip.start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap().into_inner()
        };
        let limits = PackageLimits {
            max_member_size: 1024,
            max_expanded_size: 2048,
        };
        assert!(settlement_inventory(&make(&[("../x", &[1])]), limits).is_err());
        assert!(settlement_inventory(&make(&[("empty/", &[1])]), limits).is_err());
        let mut duplicate = make(&[("a.bin", &[1]), ("b.bin", &[2])]);
        for offset in 0..duplicate.len() - 4 {
            if duplicate.get(offset..offset + 5) == Some(b"b.bin") {
                duplicate[offset] = b'a';
            }
        }
        assert!(settlement_inventory(&duplicate, limits).is_err());
        let mut encrypted = make(&[("a.bin", &[1])]);
        for offset in 0..encrypted.len() - 10 {
            let flag = if encrypted.get(offset..offset + 4) == Some(b"PK\x03\x04") {
                Some(offset + 6)
            } else if encrypted.get(offset..offset + 4) == Some(b"PK\x01\x02") {
                Some(offset + 8)
            } else {
                None
            };
            if let Some(flag) = flag {
                encrypted[flag] |= 1;
            }
        }
        assert!(settlement_inventory(&encrypted, limits).is_err());
    }
    #[test]
    fn settlement_inventory_preserves_order_empty_directories_and_all_bytes() {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.add_directory("empty/", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.start_file("x.bin", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&[0, 255, 1]).unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let members = settlement_inventory(
            &bytes,
            PackageLimits {
                max_member_size: 16,
                max_expanded_size: 16,
            },
        )
        .unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].raw_name, b"empty/");
        assert!(members[0].directory);
        assert!(members[0].bytes.is_empty());
        assert_eq!(members[1].name, "x.bin");
        assert!(!members[1].directory);
        assert_eq!(members[1].bytes, [0, 255, 1]);
        assert!(settlement_inventory(
            &bytes,
            PackageLimits {
                max_member_size: 2,
                max_expanded_size: 16
            }
        )
        .is_err());
    }
}
