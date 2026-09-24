//! Local-only regression for the observed product packages whose ZIP
//! member names use the historical, unflagged CP437 encoding.
//! Manufacturer bytes and filenames remain outside the repository and output.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const EXPECTED_LEGACY_PACKAGE_COUNT: usize = 11;
const EXPECTED_LEGACY_CORPUS_COMMITMENT: &str =
    "190fc09ab111d4f70e3cd7a32baa1ae8f19a39a382c87fdab79d06084c5bff5d";

fn corpus_root() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .expect("set KNXBENCH_PRODUCT_CORPUS to run the private corpus regression")
}

fn corpus_files(root: &Path) -> Vec<PathBuf> {
    fn visit(path: &Path, files: &mut Vec<PathBuf>) {
        let Ok(metadata) = fs::metadata(path) else {
            panic!("corpus entry is unreadable");
        };
        if metadata.is_file() {
            files.push(path.to_path_buf());
            return;
        }
        let mut children = fs::read_dir(path)
            .expect("corpus directory is unreadable")
            .map(|entry| entry.expect("corpus directory entry is unreadable").path())
            .collect::<Vec<_>>();
        children.sort();
        for child in children {
            visit(&child, files);
        }
    }

    let mut files = Vec::new();
    visit(root, &mut files);
    files
}

fn contains_legacy_member_name(bytes: &[u8]) -> bool {
    let Some(eocd) = bytes.windows(4).rposition(|value| value == b"PK\x05\x06") else {
        return false;
    };
    let Some(record) = bytes.get(eocd..eocd + 22) else {
        return false;
    };
    let count = u16::from_le_bytes([record[10], record[11]]) as usize;
    let mut offset = u32::from_le_bytes(record[16..20].try_into().unwrap()) as usize;
    for _ in 0..count {
        let Some(header) = bytes.get(offset..offset + 46) else {
            return false;
        };
        if header.get(..4) != Some(b"PK\x01\x02") {
            return false;
        }
        let flags = u16::from_le_bytes([header[8], header[9]]);
        let name_len = u16::from_le_bytes([header[28], header[29]]) as usize;
        let extra_len = u16::from_le_bytes([header[30], header[31]]) as usize;
        let comment_len = u16::from_le_bytes([header[32], header[33]]) as usize;
        let Some(name) = bytes.get(offset + 46..offset + 46 + name_len) else {
            return false;
        };
        if flags & (1 << 11) == 0 && std::str::from_utf8(name).is_err() {
            return true;
        }
        offset += 46 + name_len + extra_len + comment_len;
    }
    false
}

fn verify_package(bytes: &[u8], ordinal: usize) -> Option<String> {
    if !contains_legacy_member_name(bytes) {
        return None;
    }
    let sha256 = knx_productdb::sha256_hex(bytes);
    let dir = tempfile::tempdir().expect("temporary database directory");
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite"))
        .expect("temporary product database");
    if let Err(error) =
        knx_productdb::install_package(&conn, &format!("legacy-corpus-{ordinal}.knxprod"), bytes)
    {
        let kind = match error {
            knx_productdb::PackageError::InvalidZip { cause }
                if cause == "member name has invalid flagged UTF-8" =>
            {
                "invalid flagged UTF-8"
            }
            knx_productdb::PackageError::InvalidZip { .. } => "other invalid ZIP",
            knx_productdb::PackageError::UnsafeMember { .. } => "unsafe member",
            knx_productdb::PackageError::DuplicateMember { .. } => "duplicate member",
            knx_productdb::PackageError::SizeLimit { .. } => "size limit",
            knx_productdb::PackageError::Encrypted { .. } => "encrypted member",
            knx_productdb::PackageError::MissingMaster => "missing master",
            knx_productdb::PackageError::UnsupportedNamespace { .. } => "unsupported namespace",
            knx_productdb::PackageError::ProjectArchive => "project archive",
            knx_productdb::PackageError::MissingManufacturerData => "missing manufacturer data",
            knx_productdb::PackageError::LegacyVd2 { .. } => "legacy container",
            knx_productdb::PackageError::Database(knx_productdb::ProductDbError::Xml {
                ..
            }) => "XML",
            knx_productdb::PackageError::Database(knx_productdb::ProductDbError::Sqlite(_)) => {
                "SQLite"
            }
            knx_productdb::PackageError::Database(
                knx_productdb::ProductDbError::FutureVersion { .. },
            ) => "future database",
        };
        panic!("legacy-name corpus package {ordinal} failed as {kind}");
    }
    Some(sha256)
}

#[test]
#[ignore = "requires explicit KNXBENCH_PRODUCT_CORPUS access"]
fn all_observed_legacy_name_packages_install_without_weakening_the_boundary() {
    let root = corpus_root();
    assert!(root.exists(), "configured product corpus is unavailable");

    let mut legacy_packages = Vec::new();
    let mut ordinal = 0;
    for path in corpus_files(&root) {
        let bytes = fs::read(&path).expect("corpus file is unreadable");
        if path
            .extension()
            .is_some_and(|extension| extension == "knxprod")
        {
            ordinal += 1;
            legacy_packages.extend(verify_package(&bytes, ordinal));
            continue;
        }
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        {
            continue;
        }

        let Ok(mut bundle) = zip::ZipArchive::new(Cursor::new(&bytes)) else {
            continue;
        };
        for index in 0..bundle.len() {
            let mut member = bundle.by_index(index).expect("bundle member is unreadable");
            if !member.name().to_ascii_lowercase().ends_with(".knxprod") {
                continue;
            }
            let mut package = Vec::new();
            member
                .read_to_end(&mut package)
                .expect("nested product package is unreadable");
            ordinal += 1;
            legacy_packages.extend(verify_package(&package, ordinal));
        }
    }

    legacy_packages.sort();
    assert_eq!(
        legacy_packages.len(),
        EXPECTED_LEGACY_PACKAGE_COUNT,
        "legacy-name corpus package count changed"
    );
    let commitment = knx_productdb::sha256_hex(legacy_packages.join("\n").as_bytes());
    assert_eq!(
        commitment, EXPECTED_LEGACY_CORPUS_COMMITMENT,
        "legacy-name corpus aggregate identity changed"
    );
}
