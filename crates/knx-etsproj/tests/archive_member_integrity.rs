//! Archive members keep their identity and a memory budget, or the import is refused.
//
// AR18 independent review, findings F2 and F3 (2026-10-06): duplicate or
// case-colliding member names used to be collapsed or substituted without a
// report, and a member could inflate far past the size its headers declare.
// Every case here must end in a named `ContainerError`, never a silent import.

use std::io::{Cursor, Write};

use knx_etsproj::{import_knxproj_bytes, Container, ContainerError, ImportFailure};

const INSTALLATION: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" CompletionStatus="Undefined">
        <Topology />
        <GroupAddresses><GroupRanges /></GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

const PROJECT_INFO: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001"><ProjectInformation Name="Fictional" /></Project>
</KNX>"#;

fn zip_with(entries: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(method);
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A minimal `.knxproj` plus `extra` members, all stored uncompressed.
fn knxproj_with(extra: &[(&str, &[u8])]) -> Vec<u8> {
    let mut entries: Vec<(&str, &[u8])> = vec![
        ("P-0001.signature", b"x"),
        ("P-0001/0.xml", INSTALLATION),
        ("P-0001/Project.xml", PROJECT_INFO),
    ];
    entries.extend_from_slice(extra);
    zip_with(&entries, zip::CompressionMethod::Stored)
}

fn u16_at(bytes: &[u8], at: usize) -> usize {
    u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

/// Walks the central directory of an archive without a comment and calls
/// `patch` with (archive, central record offset, local header offset, name).
fn for_each_member(bytes: &mut [u8], mut patch: impl FnMut(&mut [u8], usize, usize, &[u8])) {
    let eocd = bytes.len() - 22;
    assert_eq!(
        &bytes[eocd..eocd + 4],
        b"PK\x05\x06",
        "fixture has no comment"
    );
    let count = u16_at(bytes, eocd + 10);
    let mut at = u32_at(bytes, eocd + 16);
    for _ in 0..count {
        assert_eq!(&bytes[at..at + 4], b"PK\x01\x02");
        let name_len = u16_at(bytes, at + 28);
        let record = 46 + name_len + u16_at(bytes, at + 30) + u16_at(bytes, at + 32);
        let local = u32_at(bytes, at + 42);
        let name = bytes[at + 46..at + 46 + name_len].to_vec();
        patch(bytes, at, local, &name);
        at += record;
    }
}

/// Renames `from` to the same-length `to` in both the local header and the
/// central record, so the archive stays structurally valid.
fn rename_member(bytes: &mut [u8], from: &str, to: &str) {
    assert_eq!(from.len(), to.len());
    let mut hits = 0;
    for_each_member(bytes, |b, central, local, name| {
        if name == from.as_bytes() {
            b[central + 46..central + 46 + to.len()].copy_from_slice(to.as_bytes());
            b[local + 30..local + 30 + to.len()].copy_from_slice(to.as_bytes());
            hits += 1;
        }
    });
    assert_eq!(hits, 1, "{from} renamed once");
}

/// Sets the declared uncompressed size of `member` in both headers.
fn declare_size(bytes: &mut [u8], member: &str, size: u32) {
    let mut hits = 0;
    for_each_member(bytes, |b, central, local, name| {
        if name == member.as_bytes() {
            b[central + 24..central + 28].copy_from_slice(&size.to_le_bytes());
            b[local + 22..local + 26].copy_from_slice(&size.to_le_bytes());
            hits += 1;
        }
    });
    assert_eq!(hits, 1, "{member} patched once");
}

fn refused_as_duplicate(bytes: Vec<u8>) -> String {
    match import_knxproj_bytes(bytes, "fixture.knxproj") {
        Err(ImportFailure::Container(ContainerError::DuplicateEntry { path })) => path,
        other => panic!("expected DuplicateEntry, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn the_fixture_itself_imports_cleanly() {
    let bytes = knxproj_with(&[("P-0001/BinaryData/a.dat", b"lower-case-bytes")]);
    assert!(import_knxproj_bytes(bytes, "fixture.knxproj").is_ok());
}

#[test]
fn two_members_differing_only_in_case_are_refused_by_name() {
    let bytes = knxproj_with(&[
        ("P-0001/BinaryData/a.dat", b"lower-case-bytes"),
        ("P-0001/BinaryData/A.DAT", b"UPPER-CASE-BYTES"),
    ]);
    assert_eq!(refused_as_duplicate(bytes), "P-0001/BinaryData/A.DAT");
}

#[test]
fn an_exact_duplicate_member_name_is_refused_not_collapsed() {
    let mut bytes = knxproj_with(&[
        ("P-0001/BinaryData/x.dat", b"first copy"),
        ("P-0001/BinaryData/y.dat", b"second copy"),
    ]);
    rename_member(
        &mut bytes,
        "P-0001/BinaryData/y.dat",
        "P-0001/BinaryData/x.dat",
    );
    assert_eq!(refused_as_duplicate(bytes), "P-0001/BinaryData/x.dat");
}

#[test]
fn an_exact_duplicate_inside_a_protected_projects_payload_is_refused_too() {
    // The nested `P-0001.zip` of a password-protected project goes through
    // the same `zip` reader, which would collapse the two copies as well.
    // Its members are stored unencrypted here: the nested-payload loop reads
    // a plain member the same way, so no cipher is needed to reach it.
    let mut inner = zip_with(
        &[
            ("P-0001/0.xml", INSTALLATION),
            ("P-0001/Project.xml", PROJECT_INFO),
            ("P-0001/BinaryData/x.dat", b"first copy"),
            ("P-0001/BinaryData/y.dat", b"second copy"),
        ],
        zip::CompressionMethod::Stored,
    );
    rename_member(
        &mut inner,
        "P-0001/BinaryData/y.dat",
        "P-0001/BinaryData/x.dat",
    );
    let outer = zip_with(
        &[("P-0001.signature", b"x"), ("P-0001.zip", &inner)],
        zip::CompressionMethod::Stored,
    );
    match Container::open_with_password(outer, "fictional") {
        Err(ContainerError::DuplicateEntry { path }) => assert_eq!(path, "P-0001/BinaryData/x.dat"),
        other => panic!("expected DuplicateEntry, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_case_variant_of_the_topology_document_is_refused() {
    let bytes = knxproj_with(&[("P-0001/0.XML", b"<KNX/>")]);
    assert_eq!(refused_as_duplicate(bytes), "P-0001/0.XML");
}

#[test]
fn a_member_that_inflates_past_its_declared_size_is_refused() {
    let zeros = vec![0u8; 1024 * 1024];
    let mut bytes = zip_with(
        &[
            ("P-0001.signature", b"x"),
            ("P-0001/0.xml", INSTALLATION),
            ("P-0001/Project.xml", PROJECT_INFO),
            ("P-0001/BinaryData/bomb.dat", &zeros),
        ],
        zip::CompressionMethod::Deflated,
    );
    declare_size(&mut bytes, "P-0001/BinaryData/bomb.dat", 100);
    match import_knxproj_bytes(bytes, "bomb.knxproj") {
        Err(ImportFailure::Container(ContainerError::Read { path, cause })) => {
            assert_eq!(path, "P-0001/BinaryData/bomb.dat");
            assert!(
                cause.contains("more than the declared 100 bytes"),
                "{cause}"
            );
        }
        other => panic!("expected a Read refusal, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn members_declaring_more_than_the_total_budget_are_refused_at_open() {
    let member_size = 60 * 1024 * 1024;
    let member_count = knx_etsproj::MAX_ARCHIVE_UNCOMPRESSED / member_size + 1;
    let names: Vec<String> = (0..member_count)
        .map(|i| format!("P-0001/BinaryData/{i}.dat"))
        .collect();
    let extra: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"y"[..])).collect();
    let mut bytes = knxproj_with(&extra);
    // Each filler stays under the per-member limit; together they exceed
    // the archive budget without allocating the declared payloads.
    for name in &names {
        declare_size(&mut bytes, name, u32::try_from(member_size).unwrap());
    }
    match Container::open(bytes) {
        Err(ContainerError::TooLarge { total, limit }) => {
            assert!(total > limit, "{total} > {limit}");
            assert_eq!(limit, knx_etsproj::MAX_ARCHIVE_UNCOMPRESSED);
        }
        other => panic!("expected TooLarge, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn the_archive_budget_is_1024_mib() {
    assert_eq!(knx_etsproj::MAX_ARCHIVE_UNCOMPRESSED, 1024 * 1024 * 1024);
}

#[test]
fn an_archive_above_the_previous_512_mib_budget_still_opens() {
    let names: Vec<String> = (0..11)
        .map(|i| format!("P-0001/BinaryData/{i}.dat"))
        .collect();
    let extra: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"y"[..])).collect();
    let mut bytes = knxproj_with(&extra);
    // 660 MiB of declared payload, no giant allocation or member read.
    // This proves container admission only, not a successful full import.
    for name in &names {
        declare_size(&mut bytes, name, 60 * 1024 * 1024);
    }
    assert!(Container::open(bytes).is_ok());
}

#[test]
fn one_byte_above_1024_mib_is_refused_with_the_exact_totals() {
    let name = "P-0001/BinaryData/fill.dat";
    let mut bytes = knxproj_with(&[(name, b"y")]);
    let real: u64 = Container::open(bytes.clone())
        .unwrap()
        .entries()
        .iter()
        .filter(|e| e.path != name)
        .map(|e| e.size)
        .sum();
    let limit = 1024 * 1024 * 1024;
    let total = limit + 1;
    declare_size(&mut bytes, name, u32::try_from(total - real).unwrap());
    assert!(matches!(
        Container::open(bytes),
        Err(ContainerError::TooLarge { total: actual_total, limit: actual_limit })
            if actual_total == total && actual_limit == limit
    ));
}

#[test]
fn members_just_inside_the_total_budget_still_open() {
    let mut bytes = knxproj_with(&[("P-0001/BinaryData/fill.dat", b"y")]);
    // Exactly the budget across all members: the three real ones keep their
    // sizes, the filler declares the rest (and is never read here).
    let real: u64 = Container::open(bytes.clone())
        .unwrap()
        .entries()
        .iter()
        .filter(|e| e.path != "P-0001/BinaryData/fill.dat")
        .map(|e| e.size)
        .sum();
    let fill = knx_etsproj::MAX_ARCHIVE_UNCOMPRESSED - real;
    declare_size(
        &mut bytes,
        "P-0001/BinaryData/fill.dat",
        u32::try_from(fill).unwrap(),
    );
    assert!(Container::open(bytes).is_ok());
}

#[test]
fn the_wrong_password_message_is_one_clean_line() {
    // AR18 review M4: a stray `\\` once put a backslash, a line break and
    // indentation into this message on the CLI and in the session log.
    let text = ContainerError::WrongPassword {
        nested_entry: "P-0001.zip".into(),
    }
    .to_string();
    assert!(
        !text.contains('\\') && !text.contains('\n') && !text.contains("  "),
        "{text}"
    );
    assert!(text.ends_with("a damaged encrypted entry is reported the same way"));
}
