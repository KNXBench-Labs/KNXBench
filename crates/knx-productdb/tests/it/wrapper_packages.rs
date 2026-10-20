//! Vendor wrapper ZIPs install every inner package together or none (census G3, ADR-0140).

use std::io::{Cursor, Write};

use knx_productdb::{
    apply_package_update, install_package, install_product_file, open_and_migrate,
    preview_package_update, sha256_hex, PackageError, PackageLimits, WrapperMemberKind,
};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;

fn program(version: u8) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-{version:02X}-0001" Name="Dimmer" ApplicationNumber="1" ApplicationVersion="{version}" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure" DefaultLanguage="en-US"><Static/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn hardware(version: u8) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-{version}" Name="Dimmer hardware"><Products><Product Id="M-0001_H-{version}_P-1" Text="Dimmer" OrderNumber="DIM-{version}"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-{version}_HP-0001-{version:02X}-0001" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-0001-{version:02X}-0001"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn zip_of(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        if let Some(directory) = name.strip_suffix('/') {
            zip.add_directory(directory, SimpleFileOptions::default())
                .unwrap();
        } else {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
    }
    zip.finish().unwrap().into_inner()
}

fn package(version: u8) -> Vec<u8> {
    let program = program(version);
    let hardware = hardware(version);
    zip_of(&[
        ("knx_master.xml", MASTER),
        ("M-0001/M-0001_A-0001.xml", program.as_bytes()),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
    ])
}

const MANUAL: &[u8] = b"%PDF-1.4 not really a manual";

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn stored_packages(conn: &Connection) -> Vec<(String, Vec<u8>)> {
    let mut statement = conn
        .prepare("SELECT sha256, bytes FROM package ORDER BY sha256")
        .unwrap();
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    rows
}

fn source_names(conn: &Connection, sha256: &str) -> Vec<String> {
    let mut statement = conn
        .prepare("SELECT source_name FROM package_source_name WHERE package_sha256 = ?1 ORDER BY source_name")
        .unwrap();
    let rows = statement
        .query_map([sha256], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();
    rows
}

fn fresh() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

#[test]
fn a_wrapper_installs_every_inner_package_byte_exact_and_names_the_rest() {
    let (_dir, conn) = fresh();
    let first = package(0x10);
    let second = package(0x11);
    let wrapper = zip_of(&[
        ("Dimmer/", b""),
        ("Dimmer/dimmer-v16.knxprod", &first),
        ("Dimmer/Manual.pdf", MANUAL),
        ("DIMMER-V17.KNXPROD", &second),
    ]);

    let installed =
        install_product_file(&conn, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap();

    assert_eq!(installed.sha256, sha256_hex(&wrapper));
    let paths: Vec<Option<&str>> = installed
        .packages
        .iter()
        .map(|package| package.path.as_deref())
        .collect();
    assert_eq!(
        paths,
        [
            Some("Dimmer/dimmer-v16.knxprod"),
            Some("DIMMER-V17.KNXPROD")
        ],
        "archive order, case-insensitive extension"
    );
    assert_eq!(installed.packages[0].report.sha256, sha256_hex(&first));
    assert_eq!(installed.packages[1].report.sha256, sha256_hex(&second));
    assert!(installed.packages.iter().all(|p| !p.report.skipped));
    let wrapper_contents = installed.wrapper.as_ref().expect("a wrapper");
    assert_eq!(
        wrapper_contents.other_members.len(),
        1,
        "{wrapper_contents:?}"
    );
    let manual = &wrapper_contents.other_members[0];
    assert_eq!(manual.path, "Dimmer/Manual.pdf");
    assert_eq!(manual.size, MANUAL.len() as u64);
    assert_eq!(manual.sha256, sha256_hex(MANUAL));
    assert_eq!(manual.kind, WrapperMemberKind::File);

    // The stored packages are exactly the inner bytes; the wrapper itself
    // is not a package.
    let mut expected = vec![
        (sha256_hex(&first), first.clone()),
        (sha256_hex(&second), second.clone()),
    ];
    expected.sort();
    assert_eq!(stored_packages(&conn), expected);
    assert_eq!(
        source_names(&conn, &sha256_hex(&first)),
        ["vendor.zip!/Dimmer/dimmer-v16.knxprod"]
    );
    assert_eq!(count(&conn, "SELECT count(*) FROM application_program"), 2);
}

#[test]
fn a_plain_package_takes_the_same_door_unchanged() {
    let (_dir, conn) = fresh();
    let bytes = package(0x10);
    let installed =
        install_product_file(&conn, "dimmer.knxprod", &bytes, PackageLimits::STANDARD).unwrap();
    assert!(installed.wrapper.is_none());
    assert_eq!(installed.packages.len(), 1);
    assert_eq!(installed.packages[0].path, None);
    assert_eq!(installed.packages[0].report.sha256, sha256_hex(&bytes));
    assert_eq!(installed.sha256, sha256_hex(&bytes));
    assert_eq!(source_names(&conn, &sha256_hex(&bytes)), ["dimmer.knxprod"]);
}

#[test]
fn install_package_names_a_wrapper_instead_of_a_missing_master() {
    let (_dir, conn) = fresh();
    let wrapper = zip_of(&[
        ("a.knxprod", &package(0x10)),
        ("b/b.knxprod", &package(0x11)),
        ("readme.txt", b"hello"),
    ]);
    let error = install_package(&conn, "vendor.knxprod", &wrapper).unwrap_err();
    match &error {
        PackageError::WrapperArchive { packages } => {
            assert_eq!(
                packages,
                &["a.knxprod".to_string(), "b/b.knxprod".to_string()]
            );
        }
        other => panic!("expected a wrapper refusal, got {other:?}"),
    }
    let message = error.to_string();
    assert!(message.contains("2 product package"), "{message}");
    assert!(message.contains("a.knxprod"), "{message}");
    assert!(!message.contains("missing knx_master.xml"), "{message}");
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 0);
}

#[test]
fn a_zip_without_master_or_inner_package_names_the_absent_product_package() {
    let (_dir, conn) = fresh();
    let seed = package(0x10);
    install_product_file(&conn, "seed.knxprod", &seed, PackageLimits::STANDARD).unwrap();
    let before = stored_packages(&conn);
    let sources = count(&conn, "SELECT count(*) FROM source_file");
    for members in [
        vec![("Manual.pdf", MANUAL)],
        vec![("installer.exe", b"MZ synthetic installer".as_slice())],
        vec![("legacy.vd1", b"not an inner product package".as_slice())],
        vec![("nested.zip", seed.as_slice())],
    ] {
        let bytes = zip_of(&members);
        let error = match install_product_file(&conn, "vendor.zip", &bytes, PackageLimits::STANDARD)
        {
            Err(error) => error,
            Ok(_) => panic!("a ZIP without a product must refuse"),
        };
        assert!(matches!(error, PackageError::MissingMaster), "{error}");
        assert_eq!(
            error.to_string(),
            "ZIP is not a standalone product package: missing knx_master.xml file",
        );
        assert_eq!(stored_packages(&conn), before);
        assert_eq!(count(&conn, "SELECT count(*) FROM source_file"), sources);
        assert_eq!(count(&conn, "SELECT count(*) FROM application_program"), 1);
    }
    // Empty archives keep the structural refusal ahead of content diagnosis.
    let error =
        match install_product_file(&conn, "empty.zip", &zip_of(&[]), PackageLimits::STANDARD) {
            Err(error) => error,
            Ok(_) => panic!("empty ZIP must refuse"),
        };
    assert!(matches!(error, PackageError::InvalidZip { .. }), "{error}");
    assert_eq!(stored_packages(&conn), before);
}

#[test]
fn a_master_named_directory_does_not_deny_a_present_inner_product() {
    let (_dir, conn) = fresh();
    let product = package(0x10);
    install_product_file(&conn, "seed.knxprod", &product, PackageLimits::STANDARD).unwrap();
    let before = stored_packages(&conn);
    let sources = count(&conn, "SELECT count(*) FROM source_file");
    for members in [
        vec![
            ("knx_master.xml/", b"".as_slice()),
            ("good.knxprod", product.as_slice()),
        ],
        vec![("knx_master.xml/", b"".as_slice())],
    ] {
        let bytes = zip_of(&members);
        // Both public APIs retain their existing refusal; finding a package
        // inside does not make the outer master-named directory a master file.
        let direct = install_package(&conn, "vendor.zip", &bytes).unwrap_err();
        let wrapper_aware =
            install_product_file(&conn, "vendor.zip", &bytes, PackageLimits::STANDARD).unwrap_err();
        for error in [direct, wrapper_aware] {
            assert!(matches!(error, PackageError::MissingMaster), "{error}");
            assert_eq!(
                error.to_string(),
                "ZIP is not a standalone product package: missing knx_master.xml file",
            );
        }
        assert_eq!(stored_packages(&conn), before);
        assert_eq!(count(&conn, "SELECT count(*) FROM source_file"), sources);
    }
}

#[test]
fn one_bad_inner_package_refuses_the_whole_wrapper() {
    let (_dir, conn) = fresh();
    let wrapper = zip_of(&[
        ("good.knxprod", &package(0x10)),
        ("broken.knxprod", b"this is not a ZIP at all"),
    ]);
    let error =
        install_product_file(&conn, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap_err();
    match &error {
        PackageError::WrapperMember { path, error } => {
            assert_eq!(path, "broken.knxprod");
            assert!(
                matches!(**error, PackageError::InvalidZip { .. }),
                "{error:?}"
            );
        }
        other => panic!("expected an inner refusal, got {other:?}"),
    }
    assert!(error.to_string().contains("broken.knxprod"), "{error}");
    assert_eq!(
        count(&conn, "SELECT count(*) FROM package"),
        0,
        "the good package is rolled back with the bad one"
    );
    assert_eq!(count(&conn, "SELECT count(*) FROM source_file"), 0);
    assert_eq!(count(&conn, "SELECT count(*) FROM application_program"), 0);
}

#[test]
fn a_wrapper_inside_a_wrapper_is_refused_by_name() {
    let (_dir, conn) = fresh();
    let inner = zip_of(&[("deep.knxprod", &package(0x10))]);
    let wrapper = zip_of(&[("inner.knxprod", &inner)]);
    let error =
        install_product_file(&conn, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap_err();
    match &error {
        PackageError::WrapperMember { path, error } => {
            assert_eq!(path, "inner.knxprod");
            assert!(
                matches!(**error, PackageError::WrapperArchive { .. }),
                "{error:?}"
            );
        }
        other => panic!("expected a nested-wrapper refusal, got {other:?}"),
    }
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 0);
}

#[test]
fn wrapper_member_paths_pass_the_package_path_checks() {
    let (_dir, conn) = fresh();
    let wrapper = zip_of(&[
        ("good.knxprod", &package(0x10)),
        ("docs\\Manual.pdf", MANUAL),
    ]);
    let error =
        install_product_file(&conn, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap_err();
    assert!(
        matches!(&error, PackageError::UnsafeMember { path } if path.contains("Manual.pdf")),
        "{error:?}"
    );
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 0);
}

#[test]
fn inner_packages_share_one_expansion_budget() {
    let first = package(0x10);
    let second = package(0x11);
    let expanded = |bytes: &[u8]| -> u64 {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index_raw(i).unwrap().size())
            .sum()
    };
    let (one, two) = (expanded(&first), expanded(&second));
    let wrapper = zip_of(&[("a.knxprod", &first), ("b.knxprod", &second)]);
    let wrapper_layer = (first.len() + second.len()) as u64;
    // Room for the wrapper layer and either inner package, not for both.
    let budget = one.max(two).max(wrapper_layer) + 1;
    assert!(budget < one + two, "fixture must not fit twice");
    let limits = PackageLimits {
        max_member_size: PackageLimits::STANDARD.max_member_size,
        max_expanded_size: budget,
    };

    let (_dir, conn) = fresh();
    let error = install_product_file(&conn, "vendor.zip", &wrapper, limits).unwrap_err();
    match &error {
        PackageError::WrapperMember { path, error } => {
            assert_eq!(path, "b.knxprod");
            // Refused by the inner package's own pre-read check (a member path
            // inside it), not after expanding it.
            assert!(
                matches!(&**error, PackageError::SizeLimit { path } if path.starts_with("M-0001/")),
                "{error:?}"
            );
        }
        other => panic!("expected the shared budget to refuse, got {other:?}"),
    }
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 0);

    let roomy = PackageLimits {
        max_member_size: limits.max_member_size,
        max_expanded_size: one + two,
    };
    let (_dir, conn) = fresh();
    install_product_file(&conn, "vendor.zip", &wrapper, roomy).unwrap();
}

#[test]
fn a_package_twice_in_one_wrapper_is_stored_once_under_both_names() {
    let (_dir, conn) = fresh();
    let bytes = package(0x10);
    let wrapper = zip_of(&[("a/dimmer.knxprod", &bytes), ("b/dimmer.knxprod", &bytes)]);
    let installed =
        install_product_file(&conn, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap();
    assert_eq!(installed.packages.len(), 2);
    assert!(!installed.packages[0].report.skipped);
    assert!(installed.packages[1].report.skipped);
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 1);
    assert_eq!(
        source_names(&conn, &sha256_hex(&bytes)),
        [
            "vendor.zip!/a/dimmer.knxprod",
            "vendor.zip!/b/dimmer.knxprod"
        ]
    );
}

#[test]
fn a_previewed_wrapper_update_lists_every_inner_package_and_installs_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    drop(open_and_migrate(&path).unwrap());
    let before = std::fs::read(&path).unwrap();
    let first = package(0x10);
    let second = package(0x11);
    let wrapper = zip_of(&[
        ("dimmer-v16.knxprod", &first),
        ("Manual.pdf", MANUAL),
        ("dimmer-v17.knxprod", &second),
    ]);

    let preview =
        preview_package_update(&path, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "a preview never writes"
    );
    assert_eq!(preview.package_sha256, sha256_hex(&wrapper));
    assert!(!preview.package_already_installed);
    let ids: Vec<&str> = preview.programs.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["M-0001_A-0001-10-0001", "M-0001_A-0001-11-0001"]);
    let contents = preview.wrapper.as_ref().expect("wrapper preview");
    let inner: Vec<(&str, &str, bool)> = contents
        .packages
        .iter()
        .map(|p| (p.path.as_str(), p.sha256.as_str(), p.already_installed))
        .collect();
    assert_eq!(
        inner,
        [
            ("dimmer-v16.knxprod", sha256_hex(&first).as_str(), false),
            ("dimmer-v17.knxprod", sha256_hex(&second).as_str(), false),
        ]
    );
    assert_eq!(contents.other_members.len(), 1);
    assert_eq!(contents.other_members[0].path, "Manual.pdf");

    let applied = apply_package_update(
        &path,
        "vendor.zip",
        &wrapper,
        PackageLimits::STANDARD,
        &preview.token,
    )
    .unwrap();
    assert!(applied.backup.exists());
    assert_eq!(applied.installed.packages.len(), 2);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 2);

    let again =
        preview_package_update(&path, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap();
    assert!(
        again.package_already_installed,
        "every inner package is known"
    );
}

#[test]
fn a_plain_package_preview_has_no_wrapper_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    drop(open_and_migrate(&path).unwrap());
    let preview = preview_package_update(
        &path,
        "dimmer.knxprod",
        &package(0x10),
        PackageLimits::STANDARD,
    )
    .unwrap();
    assert!(preview.wrapper.is_none());
}

mod wp16 {
    use super::*;
    use crate::standalone_packages::wp16::{
        descriptor, named_invalid, named_open_error, preserved_refusal, product, raw_zip,
        RawMember, CRC_CAUSE, DIRECTORY, EMPTY, STREAM_CAUSE, TRAILING,
    };

    fn seed(conn: &Connection) {
        install_package(conn, "existing.knxprod", &package(0x10)).unwrap();
    }

    fn outer(member: RawMember<'_>) -> Vec<u8> {
        raw_zip(&[RawMember::stored(b"good.knxprod", &package(0x11)), member])
    }

    fn install(
        conn: &Connection,
        bytes: &[u8],
    ) -> Result<knx_productdb::ProductFileInstall, PackageError> {
        install_product_file(conn, "vendor.zip", bytes, PackageLimits::STANDARD)
    }

    fn inner_error(bytes: &[u8], populated: bool) -> PackageError {
        let wrapper = zip_of(&[
            ("first-new.knxprod", &package(0x11)),
            ("broken.knxprod", bytes),
        ]);
        let error = preserved_refusal(
            |conn| {
                if populated {
                    seed(conn);
                }
                assert_eq!(
                    count(
                        conn,
                        "SELECT count(*) FROM application_program WHERE application_version = 17"
                    ),
                    0,
                    "first member must be a new install, not an idempotent skip"
                );
            },
            |conn| install(conn, &wrapper),
        );
        match error {
            PackageError::WrapperMember { path, error } => {
                assert_eq!(path, "broken.knxprod");
                *error
            }
            other => panic!("expected a refusal inside broken.knxprod, got {other}"),
        }
    }

    #[test]
    fn standalone_entry_point_refuses_a_trailing_directory_stream() {
        let bytes = product(RawMember::directory(DIRECTORY, TRAILING));
        let error = preserved_refusal(seed, |conn| install(conn, &bytes));
        named_invalid(error, "M-0001/Baggages/empty/", &["trailing"]);
    }

    #[test]
    fn outer_trailing_directory_refuses_on_a_fresh_database() {
        let bytes = outer(RawMember::directory(b"docs/empty/", TRAILING));
        let error = preserved_refusal(|_| {}, |conn| install(conn, &bytes));
        named_invalid(error, "docs/empty/", &["trailing"]);
    }

    #[test]
    fn outer_trailing_directory_preserves_a_populated_database() {
        let bytes = outer(RawMember::directory(b"docs/empty/", TRAILING));
        let error = preserved_refusal(seed, |conn| install(conn, &bytes));
        named_invalid(error, "docs/empty/", &["trailing"]);
    }

    #[test]
    fn inner_trailing_directory_rolls_back_an_earlier_new_package_on_a_fresh_database() {
        let bytes = product(RawMember::directory(DIRECTORY, TRAILING));
        named_invalid(
            inner_error(&bytes, false),
            "M-0001/Baggages/empty/",
            &["trailing"],
        );
    }

    #[test]
    fn inner_trailing_directory_rolls_back_an_earlier_new_package_on_a_populated_database() {
        let bytes = product(RawMember::directory(DIRECTORY, TRAILING));
        named_invalid(
            inner_error(&bytes, true),
            "M-0001/Baggages/empty/",
            &["trailing"],
        );
    }

    #[test]
    fn outer_directory_crc_error_names_the_leaf_without_mutation() {
        let bytes = outer(RawMember {
            crc: 1,
            ..RawMember::stored(b"docs/empty/", &[])
        });
        let error = preserved_refusal(seed, |conn| install(conn, &bytes));
        named_invalid(error, "docs/empty/", CRC_CAUSE);
    }

    #[test]
    fn inner_directory_crc_error_names_both_layers_and_rolls_back() {
        let bytes = product(RawMember {
            crc: 1,
            ..RawMember::directory(DIRECTORY, EMPTY)
        });
        named_invalid(
            inner_error(&bytes, true),
            "M-0001/Baggages/empty/",
            CRC_CAUSE,
        );
    }

    #[test]
    fn outer_directory_stream_error_names_the_cp437_decoded_leaf() {
        let bytes = outer(RawMember::directory(b"docs/Gr\x81sse/", &[0xff]));
        let error = preserved_refusal(seed, |conn| install(conn, &bytes));
        named_invalid(error, "docs/Grüsse/", STREAM_CAUSE);
    }

    #[test]
    fn inner_directory_stream_error_names_both_layers_and_rolls_back() {
        let bytes = product(RawMember::directory(DIRECTORY, &[0x03]));
        named_invalid(
            inner_error(&bytes, true),
            "M-0001/Baggages/empty/",
            STREAM_CAUSE,
        );
    }

    #[test]
    fn inner_stream_refusal_rolls_back_an_earlier_new_package_on_a_fresh_database() {
        let bytes = product(RawMember::directory(DIRECTORY, &[0xff]));
        named_invalid(
            inner_error(&bytes, false),
            "M-0001/Baggages/empty/",
            STREAM_CAUSE,
        );
    }

    #[test]
    fn outer_member_open_error_names_the_leaf_without_mutation() {
        let bytes = outer(RawMember {
            method: 12,
            ..RawMember::stored(b"docs/codec.bin", &[])
        });
        named_open_error(
            preserved_refusal(seed, |conn| install(conn, &bytes)),
            "docs/codec.bin",
        );
    }

    #[test]
    fn inner_member_open_error_names_both_layers_and_rolls_back() {
        let bytes = product(RawMember {
            method: 12,
            ..RawMember::stored(b"M-0001/Baggages/codec.bin", &[])
        });
        named_open_error(inner_error(&bytes, true), "M-0001/Baggages/codec.bin");
    }

    #[test]
    fn outer_valid_signed_descriptor_directory_installs_the_new_package() {
        let bytes = descriptor(outer(RawMember::directory(b"docs/empty/", EMPTY)), true);
        let (_dir, conn) = fresh();
        let result = install(&conn, &bytes).unwrap();
        assert!(result.wrapper.is_some());
        assert_eq!(result.packages.len(), 1);
        assert_eq!(result.packages[0].path.as_deref(), Some("good.knxprod"));
        assert_eq!(
            stored_packages(&conn),
            [(sha256_hex(&package(0x11)), package(0x11))]
        );
    }

    #[test]
    fn inner_valid_unsigned_descriptor_directory_keeps_exact_package_bytes() {
        let bytes = descriptor(product(RawMember::directory(DIRECTORY, EMPTY)), false);
        let wrapper = zip_of(&[("good.knxprod", &bytes)]);
        let (_dir, conn) = fresh();
        let result = install(&conn, &wrapper).unwrap();
        assert!(result.wrapper.is_some());
        assert_eq!(result.packages.len(), 1);
        assert_eq!(stored_packages(&conn), [(sha256_hex(&bytes), bytes)]);
    }

    // CHARACTERIZATION only: outer wrapper bytes are not persisted by design,
    // unlike original inner package bytes. Future admission policy is separate.
    mod file_stream_characterization {
        use super::*;
        use crate::standalone_packages::wp16::file_stream_characterization as c;

        #[test]
        fn standalone_entry_currently_accepts_nonzero_trailer_and_retains_archive() {
            let bytes = c::package(&[c::A, b"\xa5"].concat());
            c::accepted(&bytes, |conn| {
                let mut result = install(conn, &bytes).unwrap();
                assert!(result.wrapper.is_none());
                assert_eq!(result.packages.len(), 1);
                result.packages.remove(0).report
            });
        }

        #[test]
        fn inner_package_currently_accepts_second_stream_with_signed_descriptor() {
            let bytes = descriptor(c::package(&[c::A, c::B].concat()), true);
            let wrapper = zip_of(&[("good.knxprod", &bytes)]);
            c::accepted(&bytes, |conn| {
                let mut result = install(conn, &wrapper).unwrap();
                assert_eq!(result.sha256, sha256_hex(&wrapper));
                assert!(result.wrapper.is_some());
                assert_eq!(result.packages.len(), 1);
                assert_eq!(result.packages[0].path.as_deref(), Some("good.knxprod"));
                result.packages.remove(0).report
            });
        }

        // A single literal stored DEFLATE block, independent of ZIP's writer.
        fn stored_stream(plain: &[u8]) -> Vec<u8> {
            let len: u16 = plain.len().try_into().unwrap();
            let mut stream = vec![1];
            stream.extend_from_slice(&len.to_le_bytes());
            stream.extend_from_slice(&(!len).to_le_bytes());
            stream.extend_from_slice(plain);
            stream
        }

        fn outer_leaf(trailer: &[u8], signed: bool) {
            let inner = c::package(c::A);
            let stream = [stored_stream(&inner), trailer.to_vec()].concat();
            let wrapper = descriptor(
                raw_zip(&[c::member(b"good.knxprod", &stream, &inner)]),
                signed,
            );
            c::accepted(&inner, |conn| {
                let mut result = install(conn, &wrapper).unwrap();
                assert_eq!(result.sha256, sha256_hex(&wrapper));
                assert!(result.wrapper.as_ref().unwrap().other_members.is_empty());
                assert_eq!(result.packages.len(), 1);
                assert_eq!(result.packages[0].path.as_deref(), Some("good.knxprod"));
                assert_eq!(
                    conn.query_row(
                        "SELECT count(*) FROM package WHERE sha256 = ?1",
                        [sha256_hex(&wrapper)],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                    0,
                    "outer archive (including its compressed trailer) is not retained"
                );
                result.packages.remove(0).report
            });
        }

        #[test]
        fn outer_package_leaf_currently_accepts_nonzero_trailer_with_signed_descriptor() {
            outer_leaf(b"\xa5", true);
        }

        #[test]
        fn outer_package_leaf_currently_accepts_zero_trailer_with_unsigned_descriptor() {
            outer_leaf(b"\x00", false);
        }

        #[test]
        fn outer_other_member_currently_accepts_second_stream_and_reports_first_output_only() {
            let inner = c::package(c::A);
            let stream = [c::B, c::A].concat();
            let wrapper = raw_zip(&[
                RawMember::stored(b"good.knxprod", &inner),
                c::member(b"docs/ordinary.bin", &stream, b"B"),
            ]);
            c::accepted(&inner, |conn| {
                let mut result = install(conn, &wrapper).unwrap();
                let others = &result.wrapper.as_ref().unwrap().other_members;
                assert_eq!(others.len(), 1);
                assert_eq!(others[0].path, "docs/ordinary.bin");
                assert_eq!(others[0].size, 1);
                assert_eq!(others[0].sha256, sha256_hex(b"B"));
                assert_eq!(others[0].kind, WrapperMemberKind::File);
                assert_eq!(
                    conn.query_row(
                        "SELECT count(*) FROM source_file WHERE sha256 = ?1",
                        [sha256_hex(b"B")],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                    0,
                    "outer non-package members are reported, not retained"
                );
                result.packages.remove(0).report
            });
        }

        fn outer_refusal(stream: &[u8], plain: &[u8], reason: &[&str]) {
            let bytes = outer(c::member(b"docs/broken.bin", stream, plain));
            for populated in [false, true] {
                named_invalid(
                    preserved_refusal(
                        |conn| {
                            if populated {
                                seed(conn);
                            }
                        },
                        |conn| install(conn, &bytes),
                    ),
                    "docs/broken.bin",
                    reason,
                );
            }
        }

        #[test]
        fn outer_malformed_stream_still_refuses_without_database_mutation() {
            outer_refusal(&[0xff], b"A", STREAM_CAUSE);
        }

        #[test]
        fn outer_wrong_crc_still_refuses_without_database_mutation() {
            outer_refusal(c::A, b"B", CRC_CAUSE);
        }

        #[test]
        fn inner_malformed_stream_still_names_both_layers_without_database_mutation() {
            for populated in [false, true] {
                named_invalid(
                    inner_error(&c::package(&[0xff]), populated),
                    c::PATH,
                    STREAM_CAUSE,
                );
            }
        }
    }
}

/// RFC 1740 AppleDouble header: magic 0x00051607, version 0x00020000.
const APPLE_DOUBLE: &[u8] = b"\x00\x05\x16\x07\x00\x02\x00\x00Mac OS X        ";

#[test]
fn macos_metadata_named_like_a_package_is_reported_not_installed() {
    let (_dir, conn) = fresh();
    let bytes = package(0x10);
    let wrapper = zip_of(&[
        ("dimmer.knxprod", &bytes),
        ("__MACOSX/._dimmer.knxprod", APPLE_DOUBLE),
    ]);
    let installed =
        install_product_file(&conn, "vendor.zip", &wrapper, PackageLimits::STANDARD).unwrap();
    assert_eq!(installed.packages.len(), 1);
    let others = &installed.wrapper.as_ref().unwrap().other_members;
    assert_eq!(others.len(), 1);
    assert_eq!(others[0].path, "__MACOSX/._dimmer.knxprod");
    assert_eq!(others[0].kind, WrapperMemberKind::AppleDouble);
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 1);
}

#[test]
fn appledouble_is_recognised_by_content_not_by_name() {
    let (_dir, conn) = fresh();
    // Named like macOS metadata, but a real package: installed.
    let real = zip_of(&[("__MACOSX/._dimmer.knxprod", &package(0x10))]);
    let installed =
        install_product_file(&conn, "vendor.zip", &real, PackageLimits::STANDARD).unwrap();
    assert_eq!(installed.packages.len(), 1);
    // Named like a package, AppleDouble bytes, nothing else: no package at all.
    let (_dir, conn) = fresh();
    let only_metadata = zip_of(&[("dimmer.knxprod", APPLE_DOUBLE)]);
    let error = install_product_file(&conn, "vendor.zip", &only_metadata, PackageLimits::STANDARD)
        .unwrap_err();
    assert!(matches!(error, PackageError::MissingMaster), "{error:?}");
    assert_eq!(
        error.to_string(),
        "ZIP is not a standalone product package: missing knx_master.xml file",
    );
    assert_eq!(count(&conn, "SELECT count(*) FROM package"), 0);
}
