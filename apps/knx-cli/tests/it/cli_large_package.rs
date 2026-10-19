//! `knx products ingest --allow-large-package` opts into the large package profile (KL-151).
//!
//! Since ADR-0142 the standard profile admits members up to the storage
//! ceiling and 4 GiB in total; the opt-in raises only the total. Fixtures are
//! synthetic: a 65 MiB member of zeros that compresses to a few KiB, and a
//! package whose headers declare more than 4 GiB without containing it. No
//! bus, private corpus or manufacturer data is used.

use std::io::{Cursor, Write};
use std::path::Path;
use std::process::{Command, Output};

use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1"><Products><Product Id="M-0001_H-1_P-1" Text="Synthetic switch" OrderNumber="SYNTHETIC-1"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn package_with_a_65_mib_member(path: &Path) {
    let payload = vec![0_u8; 65 * 1024 * 1024];
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/large.bin", payload.as_slice()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    std::fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}

/// Five baggage members whose local and central headers each declare
/// 1,000,000,000 bytes: 5 GB in total, above the standard 4 GiB.
fn package_declaring_more_than_4_gib(path: &Path) {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer.start_file("knx_master.xml", options).unwrap();
    writer.write_all(MASTER).unwrap();
    writer.start_file("M-0001/Hardware.xml", options).unwrap();
    writer.write_all(HARDWARE).unwrap();
    for index in 0..5 {
        writer
            .start_file(format!("M-0001/Baggages/declared-{index}.bin"), options)
            .unwrap();
        writer.write_all(b"x").unwrap();
    }
    let mut bytes = writer.finish().unwrap().into_inner();
    let declared = 1_000_000_000_u32.to_le_bytes();
    let headers: Vec<(usize, usize)> = {
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        (0..5)
            .map(|index| {
                let member = archive
                    .by_name(&format!("M-0001/Baggages/declared-{index}.bin"))
                    .unwrap();
                (
                    usize::try_from(member.header_start()).unwrap(),
                    usize::try_from(member.central_header_start()).unwrap(),
                )
            })
            .collect()
    };
    for (local, central) in headers {
        bytes[local + 22..local + 26].copy_from_slice(&declared);
        bytes[central + 24..central + 28].copy_from_slice(&declared);
    }
    std::fs::write(path, bytes).unwrap();
}

fn ingest(args: &[&std::ffi::OsStr], data_home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["products", "ingest"])
        .args(args)
        .env("XDG_DATA_HOME", data_home)
        .output()
        .expect("run the real CLI against a synthetic input")
}

#[test]
fn the_standard_profile_refuses_and_names_the_opt_in() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bundle.knxprod");
    let database = dir.path().join("products.sqlite");
    package_declaring_more_than_4_gib(&input);
    let output = ingest(
        &[
            input.as_os_str(),
            "--product-db".as_ref(),
            database.as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("product ZIP size limit exceeded: M-0001/Baggages/declared-4.bin"),
        "{stderr}"
    );
    assert!(stderr.contains("beyond 4 GiB"), "{stderr}");
    assert!(stderr.contains("--allow-large-package"), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn a_65_mib_member_installs_with_and_without_the_opt_in() {
    for opt_in in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("bundle.knxprod");
        let database = dir.path().join("products.sqlite");
        package_with_a_65_mib_member(&input);
        let mut args = vec![input.as_os_str()];
        if opt_in {
            args.push("--allow-large-package".as_ref());
        }
        args.extend(["--product-db".as_ref(), database.as_os_str()]);
        let output = ingest(&args, &dir.path().join("data"));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("package installed: scheme 11, 3 member(s)"),
            "{stdout}"
        );
    }
}

#[test]
fn the_opt_in_is_refused_for_a_project_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxproj");
    std::fs::write(&input, b"not read").unwrap();
    let database = dir.path().join("products.sqlite");
    let output = ingest(
        &[
            input.as_os_str(),
            "--allow-large-package".as_ref(),
            "--product-db".as_ref(),
            database.as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains(
            "--allow-large-package applies to .knxprod product packages and vendor ZIPs only"
        ),
        "{stderr}"
    );
    assert!(!database.exists(), "nothing is opened for a refused option");
}

/// The census case (ADR-0140): a vendor ZIP around a large inner package.
/// The wrapper layer is small; the inner package is not.
fn vendor_zip_around(dir: &Path, inner_package: fn(&Path)) -> std::path::PathBuf {
    let inner = dir.join("inner.knxprod");
    inner_package(&inner);
    let inner = std::fs::read(&inner).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("Database/complete.knxprod", inner.as_slice()),
        ("Database/Readme.pdf", b"%PDF-1.4 readme".as_slice()),
    ] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    let path = dir.join("vendor-database.zip");
    std::fs::write(&path, writer.finish().unwrap().into_inner()).unwrap();
    path
}

#[test]
fn a_vendor_zip_names_the_opt_in_beyond_4_gib_and_installs_a_65_mib_inner_package() {
    let dir = tempfile::tempdir().unwrap();
    let declared = dir.path().join("declared");
    std::fs::create_dir(&declared).unwrap();
    let oversized = vendor_zip_around(&declared, package_declaring_more_than_4_gib);
    let refused = ingest(
        &[
            oversized.as_os_str(),
            "--product-db".as_ref(),
            dir.path().join("refused.sqlite").as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert_eq!(refused.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("inner product package Database/complete.knxprod: product ZIP size limit exceeded: M-0001/Baggages/declared-4.bin"),
        "{stderr}"
    );
    assert!(stderr.contains("--allow-large-package"), "{stderr}");

    // The standard profile installs the 65 MiB inner package (ADR-0142).
    let input = vendor_zip_around(dir.path(), package_with_a_65_mib_member);
    let database = dir.path().join("products.sqlite");
    let installed = ingest(
        &[
            input.as_os_str(),
            "--product-db".as_ref(),
            database.as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stdout = String::from_utf8_lossy(&installed.stdout);
    assert_eq!(
        installed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    assert!(
        stdout.contains(
            "vendor ZIP: 1 product package(s) installed together, 1 other member(s) not installed"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "inner package Database/complete.knxprod: package installed: scheme 11, 3 member(s)"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("not installed: Database/Readme.pdf (15 bytes, sha256 "),
        "{stdout}"
    );
}
