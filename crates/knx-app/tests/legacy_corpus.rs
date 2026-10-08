//! Real legacy files from the private corpus: pinned identities, counts and no embedded password.
//!
//! Run with `KNXBENCH_PRODUCT_CORPUS=<OriginalData/ProductDatabases>` and
//! `KNXBENCH_VD_PASSWORD_FILE=<file holding the password on its first line>`.
//! Without the password file every test here fails loudly instead of
//! passing vacuously. Only digests and counts are asserted; no manufacturer
//! text is printed.

use std::path::{Path, PathBuf};

use knx_app::legacy::{inspect_legacy_file, LegacyPassword};
use knx_productdb::legacy::{ExImContent, LegacyMemberKind};

fn corpus_root() -> PathBuf {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        });
    assert!(
        root.exists(),
        "SKIP: set KNXBENCH_PRODUCT_CORPUS; corpus root {} does not exist",
        root.display()
    );
    root
}

fn password_bytes() -> Vec<u8> {
    let path = std::env::var_os("KNXBENCH_VD_PASSWORD_FILE")
        .expect("SKIP: set KNXBENCH_VD_PASSWORD_FILE to the local legacy password file");
    let raw = std::fs::read(&path).expect("KNXBENCH_VD_PASSWORD_FILE unreadable");
    let line = raw.split(|&b| b == b'\n').next().unwrap_or_default();
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    assert!(
        !line.is_empty(),
        "KNXBENCH_VD_PASSWORD_FILE holds no password"
    );
    line.to_vec()
}

fn password() -> LegacyPassword {
    LegacyPassword::new(String::from_utf8(password_bytes()).expect("ASCII password"))
}

fn read(root: &Path, name: &str) -> Vec<u8> {
    let path = knx_testsupport::find_corpus_file(root, name)
        .unwrap_or_else(|| panic!("corpus fixture {name} unavailable under {}", root.display()));
    std::fs::read(&path).unwrap_or_else(|e| panic!("corpus fixture {name} unreadable: {e}"))
}

struct Expected {
    name: &'static str,
    kind: LegacyMemberKind,
    content: ExImContent,
    source_sha256: &'static str,
    payload_sha256: &'static str,
    version: &'static str,
    tables: usize,
    rows: usize,
    products: usize,
    masks: &'static [&'static str],
}

const EXPECTED: [Expected; 3] = [
    Expected {
        name: "EIBMARKT.VD3",
        kind: LegacyMemberKind::ProductDatabase,
        content: ExImContent::ProductDatabase,
        source_sha256: "6d4cc9d1eb62d6d10d182c03e271693ddaed1a762f3690158e50e67c3ec3b96f",
        payload_sha256: "a0202da31ac597d28363bfee01059aee9266ec06c59f2330bd8fb80ac87e10c8",
        version: "5.10",
        tables: 37,
        rows: 4214,
        products: 3,
        masks: &["MV-0020", "MV-0021"],
    },
    Expected {
        name: "Eibmarkt Motion Sensor N520_IRBM_N530_IRBM.vd4",
        kind: LegacyMemberKind::ProductDatabase,
        content: ExImContent::ProductDatabase,
        source_sha256: "f5d698d58769ba8c787f63aec1a751761720c8e29117b4590ecca37a6bf880b6",
        payload_sha256: "a3ff864b0a4b01da7b81c8574300c74547da755e026d326df0cb7099de14f487",
        version: "6.2",
        tables: 37,
        rows: 14734,
        products: 2,
        masks: &["MV-0701"],
    },
    Expected {
        name: "MDT_VD_VisuControl.pr5",
        kind: LegacyMemberKind::ProjectExport,
        content: ExImContent::ProjectExport,
        source_sha256: "2d93d5ac0e57bf56a81f7ff52b2189222d492499f3c342efd6a9dbf50db29a69",
        payload_sha256: "65b06b025ecf21b179eb02797da16b58f9972c6377ce479de1a93aa0011fffac",
        version: "6.3",
        tables: 16,
        rows: 12,
        products: 0,
        masks: &[],
    },
];

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS and KNXBENCH_VD_PASSWORD_FILE"]
fn the_real_legacy_files_inspect_with_their_pinned_identity_and_counts() {
    let root = corpus_root();
    let password = password();
    for expected in &EXPECTED {
        let bytes = read(&root, expected.name);
        let inspection = inspect_legacy_file(&bytes, Some(&password))
            .unwrap_or_else(|e| panic!("{}: {e}", expected.name));
        let label = expected.name;
        assert_eq!(inspection.member_kind, expected.kind, "{label}");
        assert_eq!(inspection.content, expected.content, "{label}");
        assert!(inspection.encrypted, "{label}");
        assert_eq!(inspection.source_sha256, expected.source_sha256, "{label}");
        assert_eq!(
            inspection.payload_sha256, expected.payload_sha256,
            "{label}"
        );
        assert_eq!(
            inspection.format_version.as_deref(),
            Some(expected.version),
            "{label}"
        );
        assert_eq!(inspection.tables.len(), expected.tables, "{label}");
        assert_eq!(inspection.total_rows(), expected.rows, "{label}");
        assert_eq!(inspection.products.len(), expected.products, "{label}");
        let mut masks: Vec<&str> = inspection
            .products
            .iter()
            .filter_map(|p| p.mask_version.as_deref())
            .collect();
        masks.sort_unstable();
        masks.dedup();
        assert_eq!(masks, expected.masks, "{label}");
        assert!(
            inspection.diagnostics.is_empty(),
            "{label}: {:?}",
            inspection.diagnostics
        );
        println!(
            "{label}: {} tables, {} rows, {} continuation lines, {} products",
            inspection.tables.len(),
            inspection.total_rows(),
            inspection.continuation_lines,
            inspection.products.len()
        );
    }
}

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS and KNXBENCH_VD_PASSWORD_FILE"]
fn no_tracked_file_contains_the_real_legacy_password() {
    let secret = password_bytes();
    let root = knx_testsupport::workspace_root();
    let listing = std::process::Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(&root)
        .output()
        .expect("git ls-files");
    assert!(listing.status.success(), "git ls-files failed");
    let mut scanned = 0usize;
    for name in listing.stdout.split(|&b| b == 0).filter(|n| !n.is_empty()) {
        let path = root.join(String::from_utf8_lossy(name).as_ref());
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        scanned += 1;
        assert!(
            !bytes.windows(secret.len()).any(|w| w == secret.as_slice()),
            "the legacy password occurs in {}",
            path.display()
        );
    }
    assert!(
        scanned > 1000,
        "only {scanned} files scanned; the scan is vacuous"
    );
}
