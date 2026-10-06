//! Members are judged by their decoded names, and a protected payload is counted before unpacking.
//
// AR18 re-check (2026-10-06), findings N1 and N2. N1: two records whose raw
// names differ but which the `zip` reader *decodes* to one name (an Info-ZIP
// Unicode Path field, or CP437 against UTF-8) got past the raw-byte check
// and let one member stand in for the other — even for `0.xml`. N2: the
// 512 MiB budget was checked only after a protected project's nested
// payload had been unpacked in full.
//
// The archives are written by hand below, so names, flags, extra fields and
// declared sizes can be set independently of each other.

use knx_etsproj::{import_knxproj_bytes, Container, ContainerError, ImportFailure};
use knx_secure::zipcrypto::crc32;

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

const UTF8_FLAG: u16 = 0x0800;

/// One stored member: raw name bytes, general-purpose flags, extra field,
/// data, and the uncompressed size the headers declare (normally the
/// data's length).
struct Member {
    name: Vec<u8>,
    flags: u16,
    extra: Vec<u8>,
    data: Vec<u8>,
    declared: u32,
    /// The local header's name, when it should differ from the central one.
    local_name: Option<Vec<u8>>,
    /// Point this central record at an earlier member's local record
    /// instead of writing one of its own.
    alias_of: Option<usize>,
}

fn member(name: &str, data: &[u8]) -> Member {
    Member {
        name: name.as_bytes().to_vec(),
        flags: 0,
        extra: Vec::new(),
        data: data.to_vec(),
        declared: data.len() as u32,
        local_name: None,
        alias_of: None,
    }
}

/// An Info-ZIP Unicode Path extra field (0x7075) that renames `raw` to
/// `decoded` for any reader that honours it.
fn unicode_path(raw: &[u8], decoded: &str) -> Vec<u8> {
    let mut body = vec![1u8];
    body.extend_from_slice(&crc32(raw).to_le_bytes());
    body.extend_from_slice(decoded.as_bytes());
    let mut field = Vec::new();
    field.extend_from_slice(&0x7075u16.to_le_bytes());
    field.extend_from_slice(&(body.len() as u16).to_le_bytes());
    field.extend_from_slice(&body);
    field
}

/// A stored (method 0) ZIP with exactly these members, in this order.
fn zip(members: &[Member]) -> Vec<u8> {
    zip_after(&[], members)
}

/// [`zip`] behind `prefix` bytes (a self-extractor stub, say); offsets are
/// written relative to the archive, as such tools do.
fn zip_after(prefix: &[u8], members: &[Member]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    let mut offsets = Vec::new();
    for m in members {
        let crc = crc32(&m.data);
        let size = m.data.len() as u32;
        if let Some(earlier) = m.alias_of {
            let offset: u32 = offsets[earlier];
            offsets.push(offset);
            central_record(&mut central, m, crc, size, offset);
            continue;
        }
        let offset = out.len() as u32;
        offsets.push(offset);
        let local_name = m.local_name.as_ref().unwrap_or(&m.name);
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&m.flags.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // stored
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0x21u16.to_le_bytes()); // date 1980-01-01
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&m.declared.to_le_bytes());
        out.extend_from_slice(&(local_name.len() as u16).to_le_bytes());
        out.extend_from_slice(&(m.extra.len() as u16).to_le_bytes());
        out.extend_from_slice(local_name);
        out.extend_from_slice(&m.extra);
        out.extend_from_slice(&m.data);
        central_record(&mut central, m, crc, size, offset);
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(members.len() as u16).to_le_bytes());
    out.extend_from_slice(&(members.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    let mut whole = prefix.to_vec();
    whole.extend_from_slice(&out);
    whole
}

fn central_record(central: &mut Vec<u8>, m: &Member, crc: u32, size: u32, offset: u32) {
    {
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&m.flags.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0x21u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&m.declared.to_le_bytes());
        central.extend_from_slice(&(m.name.len() as u16).to_le_bytes());
        central.extend_from_slice(&(m.extra.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        central.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(&m.name);
        central.extend_from_slice(&m.extra);
    }
}

/// A minimal project plus `extra` members.
fn project_with(extra: Vec<Member>) -> Vec<u8> {
    let mut members = vec![
        member("P-0001.signature", b"x"),
        member("P-0001/0.xml", INSTALLATION),
        member("P-0001/Project.xml", PROJECT_INFO),
    ];
    members.extend(extra);
    zip(&members)
}

/// A protected-layout project: signature plus a nested `P-0001.zip` whose
/// members are stored plain (the nested loop reads those without a cipher).
fn protected_with(inner: Vec<Member>) -> Vec<u8> {
    let mut members = vec![
        member("P-0001/0.xml", INSTALLATION),
        member("P-0001/Project.xml", PROJECT_INFO),
    ];
    members.extend(inner);
    let payload = zip(&members);
    zip(&[
        member("P-0001.signature", b"x"),
        member("P-0001.zip", &payload),
    ])
}

fn refused_for_identity(result: Result<knx_etsproj::ImportOutcome, ImportFailure>) {
    match result {
        Err(ImportFailure::Container(
            ContainerError::DuplicateEntry { .. }
            | ContainerError::NameCollision { .. }
            | ContainerError::InconsistentRecord { .. },
        )) => {}
        other => panic!("expected an identity refusal, got {:?}", other.map(|_| ())),
    }
}

fn container_refused_for_identity(result: Result<Container, ContainerError>) {
    match result {
        Err(
            ContainerError::DuplicateEntry { .. }
            | ContainerError::NameCollision { .. }
            | ContainerError::InconsistentRecord { .. },
        ) => {}
        other => panic!("expected an identity refusal, got {:?}", other.map(|_| ())),
    }
}

fn upath_member(raw: &str, decoded: &str, data: &[u8]) -> Member {
    Member {
        extra: unicode_path(raw.as_bytes(), decoded),
        ..member(raw, data)
    }
}

#[test]
fn the_hand_written_fixture_imports_cleanly() {
    let bytes = project_with(vec![member("P-0001/BinaryData/a.dat", b"bytes")]);
    assert!(import_knxproj_bytes(bytes, "fixture.knxproj").is_ok());
}

#[test]
fn two_members_renamed_to_one_name_by_unicode_path_fields_are_refused() {
    let bytes = project_with(vec![
        upath_member(
            "P-0001/BinaryData/p1.dat",
            "P-0001/BinaryData/dup.dat",
            b"first-upath",
        ),
        upath_member(
            "P-0001/BinaryData/p2.dat",
            "P-0001/BinaryData/dup.dat",
            b"second-upath",
        ),
    ]);
    refused_for_identity(import_knxproj_bytes(bytes, "fixture.knxproj"));
}

#[test]
fn a_member_renamed_onto_the_topology_document_is_refused() {
    let forged = INSTALLATION.to_vec();
    let bytes = project_with(vec![upath_member("P-0001/zz.xml", "P-0001/0.xml", &forged)]);
    refused_for_identity(import_knxproj_bytes(bytes, "fixture.knxproj"));
}

#[test]
fn a_cp437_name_and_its_utf8_twin_are_refused() {
    let cp437 = Member {
        name: b"P-0001/BinaryData/\x82.dat".to_vec(),
        ..member("", b"cp437-bytes")
    };
    let utf8 = Member {
        flags: UTF8_FLAG,
        ..member("P-0001/BinaryData/\u{e9}.dat", b"utf8-bytes")
    };
    refused_for_identity(import_knxproj_bytes(
        project_with(vec![cp437, utf8]),
        "fixture.knxproj",
    ));
}

#[test]
fn two_distinct_non_ascii_names_still_import() {
    let a = Member {
        flags: UTF8_FLAG,
        ..member("P-0001/BinaryData/\u{e9}.dat", b"e-acute")
    };
    let b = Member {
        flags: UTF8_FLAG,
        ..member("P-0001/BinaryData/\u{fc}.dat", b"u-umlaut")
    };
    let outcome = import_knxproj_bytes(project_with(vec![a, b]), "fixture.knxproj").unwrap();
    for (path, bytes) in [
        ("P-0001/BinaryData/\u{e9}.dat", &b"e-acute"[..]),
        ("P-0001/BinaryData/\u{fc}.dat", &b"u-umlaut"[..]),
    ] {
        let entry = outcome
            .opaque
            .iter()
            .find(|entry| entry.source_path == path)
            .unwrap_or_else(|| panic!("{path} retained"));
        assert_eq!(entry.bytes, bytes, "{path}");
    }
}

#[test]
fn unicode_path_twins_inside_a_protected_payload_are_refused() {
    let bytes = protected_with(vec![
        upath_member(
            "P-0001/BinaryData/p1.dat",
            "P-0001/BinaryData/dup.dat",
            b"first-upath",
        ),
        upath_member(
            "P-0001/BinaryData/p2.dat",
            "P-0001/BinaryData/dup.dat",
            b"second-upath",
        ),
    ]);
    container_refused_for_identity(Container::open_with_password(bytes, "fictional"));
}

#[test]
fn a_protected_payload_declaring_more_than_the_budget_is_refused_before_unpacking() {
    // Nine members that each *declare* 60 MiB but hold a few bytes: the
    // refusal must come from the declared total, before the first member is
    // read — reading would fail differently ("decompressed to N bytes").
    let inner = (0..9)
        .map(|i| Member {
            declared: 60 * 1024 * 1024,
            ..member(&format!("P-0001/BinaryData/{i}.dat"), b"tiny")
        })
        .collect();
    match Container::open_with_password(protected_with(inner), "fictional") {
        Err(ContainerError::TooLarge { total, limit }) => {
            assert!(total > limit, "{total} > {limit}");
        }
        other => panic!("expected TooLarge, got {:?}", other.map(|_| ())),
    }
}

// AR18 re-check round 2, N7: a decoded name that ends in `/` made a record
// a "directory", and directories were skipped together with their bytes.

#[test]
fn the_real_topology_turned_into_a_directory_and_a_forgery_into_0_xml_is_refused() {
    let real = Member {
        extra: unicode_path(b"P-0001/0.xml", "P-0001/0.xml/"),
        ..member("P-0001/0.xml", INSTALLATION)
    };
    let forged = upath_member("P-0001/zz.xml", "P-0001/0.xml", INSTALLATION);
    let bytes = zip(&[
        member("P-0001.signature", b"x"),
        real,
        member("P-0001/Project.xml", PROJECT_INFO),
        forged,
    ]);
    refused_for_identity(import_knxproj_bytes(bytes, "fixture.knxproj"));
}

#[test]
fn the_same_inside_a_protected_payload_is_refused() {
    let real = Member {
        extra: unicode_path(b"P-0001/0.xml", "P-0001/0.xml/"),
        ..member("P-0001/0.xml", INSTALLATION)
    };
    let forged = upath_member("P-0001/zz.xml", "P-0001/0.xml", INSTALLATION);
    let payload = zip(&[real, member("P-0001/Project.xml", PROJECT_INFO), forged]);
    let bytes = zip(&[
        member("P-0001.signature", b"x"),
        member("P-0001.zip", &payload),
    ]);
    container_refused_for_identity(Container::open_with_password(bytes, "fictional"));
}

#[test]
fn a_directory_record_that_carries_bytes_is_refused() {
    let bytes = project_with(vec![member("P-0001/BinaryData/hidden/", &[7u8; 4500])]);
    refused_for_identity(import_knxproj_bytes(bytes, "fixture.knxproj"));
}

#[test]
fn a_directory_named_like_a_file_is_refused() {
    let bytes = project_with(vec![
        member("P-0001/BinaryData/x.dat/", b""),
        member("P-0001/BinaryData/x.dat", b"file"),
    ]);
    refused_for_identity(import_knxproj_bytes(bytes, "fixture.knxproj"));
}

#[test]
fn empty_directory_records_still_import() {
    // ETS6 writes them; they carry nothing.
    let bytes = project_with(vec![
        member("P-0001/BinaryData/", b""),
        member("P-0001/BinaryData/a.dat", b"bytes"),
    ]);
    assert!(import_knxproj_bytes(bytes, "fixture.knxproj").is_ok());
}

// N9: KNXBench follows the central directory, but an archive whose local
// headers disagree with it means two readers see two different projects.

#[test]
fn a_local_header_name_that_differs_from_the_central_one_is_refused() {
    let lying = Member {
        local_name: Some(b"P-0001/evil.xml".to_vec()),
        ..member("P-0001/0.xml", INSTALLATION)
    };
    let bytes = zip(&[
        member("P-0001.signature", b"x"),
        lying,
        member("P-0001/Project.xml", PROJECT_INFO),
    ]);
    refused_for_identity(import_knxproj_bytes(bytes, "fixture.knxproj"));
}

#[test]
fn two_central_records_sharing_one_local_record_are_refused() {
    let shared = Member {
        alias_of: Some(3),
        ..member("P-0001/BinaryData/b.dat", b"bytes")
    };
    let bytes = project_with(vec![member("P-0001/BinaryData/a.dat", b"bytes"), shared]);
    refused_for_identity(import_knxproj_bytes(bytes, "fixture.knxproj"));
}

#[test]
fn an_archive_behind_a_prefix_still_imports() {
    let bytes = zip_after(
        &[0x4d; 4096],
        &[
            member("P-0001.signature", b"x"),
            member("P-0001/0.xml", INSTALLATION),
            member("P-0001/Project.xml", PROJECT_INFO),
        ],
    );
    assert!(import_knxproj_bytes(bytes, "fixture.knxproj").is_ok());
}
