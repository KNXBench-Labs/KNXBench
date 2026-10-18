//! Classifies measured corpus installs without discarding resource refusals.
use knx_productdb::{Connection, PackageError};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Installed,
    Duplicate,
    StandardSizeRefusal,
}

pub(super) fn measure(
    conn: &Connection,
    name: &str,
    bytes: &[u8],
) -> Result<Outcome, PackageError> {
    let before = conn.total_changes();
    match knx_productdb::install_package(conn, name, bytes) {
        Ok(report) => Ok(if report.skipped {
            Outcome::Duplicate
        } else {
            Outcome::Installed
        }),
        Err(PackageError::SizeLimit { path })
            if expected_standard_refusal(name, bytes).as_deref() == Some(path.as_str()) =>
        {
            assert_eq!(
                conn.total_changes(),
                before,
                "size preflight must not mutate any database row"
            );
            Ok(Outcome::StandardSizeRefusal)
        }
        Err(error) => Err(error),
    }
}

// Independent declared-length observation, never decompression or a larger profile.
// Any other SizeLimit (metadata/structure/actual expansion) stays a failing test.
fn expected_standard_refusal(name: &str, bytes: &[u8]) -> Option<String> {
    if bytes.len() as u64 > knx_productdb::MAX_PACKAGE_INPUT_BYTES {
        return Some(name.to_owned());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let limits = knx_productdb::PackageLimits::STANDARD;
    let mut total = 0_u64;
    for i in 0..zip.len() {
        let entry = zip.by_index_raw(i).ok()?;
        total = total.checked_add(entry.size())?;
        if entry.size() > limits.max_member_size || total > limits.max_expanded_size {
            return Some(entry.name().to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    fn declaration(declared: u32) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("knx_master.xml", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"x").unwrap();
        let mut bytes = writer.finish().unwrap().into_inner();
        let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        bytes[22..26].copy_from_slice(&declared.to_le_bytes());
        bytes[central + 24..central + 28].copy_from_slice(&declared.to_le_bytes());
        bytes
    }

    fn oversized_declaration() -> Vec<u8> {
        declaration(
            u32::try_from(knx_productdb::PackageLimits::STANDARD.max_member_size + 1).unwrap(),
        )
    }

    #[test]
    fn independent_declared_limit_is_inclusive_not_a_blanket_error_escape() {
        let limit = u32::try_from(knx_productdb::PackageLimits::STANDARD.max_member_size).unwrap();
        assert_eq!(
            expected_standard_refusal("fixture", &declaration(limit)),
            None
        );
        assert_eq!(
            expected_standard_refusal("fixture", &declaration(limit - 1)),
            None
        );
        assert_eq!(
            expected_standard_refusal("fixture", &declaration(limit + 1)).as_deref(),
            Some("knx_master.xml")
        );
        assert_eq!(expected_standard_refusal("fixture", b"not a ZIP"), None);
    }

    #[test]
    fn standard_refusal_is_counted_without_expanding_or_changing_database() {
        let d = tempfile::tempdir().unwrap();
        let c = knx_productdb::open_and_migrate(&d.path().join("p.sqlite")).unwrap();
        let before = c.total_changes();
        assert!(matches!(
            knx_productdb::install_package(&c, "synthetic.knxprod", &oversized_declaration()),
            Err(PackageError::SizeLimit { .. })
        ));
        assert_eq!(
            measure(&c, "synthetic.knxprod", &oversized_declaration()).unwrap(),
            Outcome::StandardSizeRefusal
        );
        assert_eq!(c.total_changes(), before);
    }

    #[test]
    fn unrelated_metadata_size_refusal_still_fails_the_census() {
        // A classic ZIP cannot exceed the member-count bound (the 16-bit field
        // maximum), so cross the path-component budget instead: 40 members of
        // 32,001 distinct components each exceed 16 nodes per possible member.
        let deep = "a/".repeat(32_000);
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for i in 0..40 {
            writer
                .start_file(format!("{i}/{deep}f"), SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"x").unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();
        let d = tempfile::tempdir().unwrap();
        let c = knx_productdb::open_and_migrate(&d.path().join("p.sqlite")).unwrap();
        let before = c.total_changes();
        assert_eq!(expected_standard_refusal("fixture", &bytes), None);
        assert!(matches!(
            measure(&c, "fixture", &bytes),
            Err(PackageError::SizeLimit { .. })
        ));
        assert_eq!(c.total_changes(), before);
    }

    #[test]
    fn cumulative_declared_limit_is_also_inclusive() {
        fn fixture(sizes: &[u32]) -> Vec<u8> {
            let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
            for i in 0..sizes.len() {
                writer
                    .start_file(format!("entry-{i}"), SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(b"x").unwrap();
            }
            let mut bytes = writer.finish().unwrap().into_inner();
            let locals: Vec<_> = bytes
                .windows(4)
                .enumerate()
                .filter(|(_, b)| *b == b"PK\x03\x04")
                .map(|(i, _)| i)
                .collect();
            let centrals: Vec<_> = bytes
                .windows(4)
                .enumerate()
                .filter(|(_, b)| *b == b"PK\x01\x02")
                .map(|(i, _)| i)
                .collect();
            assert_eq!(locals.len(), sizes.len());
            assert_eq!(centrals.len(), sizes.len());
            for ((l, c), size) in locals.into_iter().zip(centrals).zip(sizes) {
                bytes[l + 22..l + 26].copy_from_slice(&size.to_le_bytes());
                bytes[c + 24..c + 28].copy_from_slice(&size.to_le_bytes());
            }
            bytes
        }
        // Members at the member limit, then the remainder, fill the total
        // exactly; one more byte crosses it.
        let limits = knx_productdb::PackageLimits::STANDARD;
        let member = u32::try_from(limits.max_member_size).unwrap();
        let full = limits.max_expanded_size / limits.max_member_size;
        let mut sizes = vec![member; usize::try_from(full).unwrap()];
        let remainder = limits.max_expanded_size % limits.max_member_size;
        if remainder > 0 {
            sizes.push(u32::try_from(remainder).unwrap());
        }
        assert_eq!(expected_standard_refusal("fixture", &fixture(&sizes)), None);
        sizes.push(1);
        assert_eq!(
            expected_standard_refusal("fixture", &fixture(&sizes)).as_deref(),
            Some(format!("entry-{}", sizes.len() - 1).as_str())
        );
    }

    #[test]
    fn malformed_or_unexplained_refusals_are_not_turned_into_success() {
        let d = tempfile::tempdir().unwrap();
        let c = knx_productdb::open_and_migrate(&d.path().join("p.sqlite")).unwrap();
        assert!(measure(&c, "synthetic.knxprod", b"not a ZIP").is_err());
    }
}
