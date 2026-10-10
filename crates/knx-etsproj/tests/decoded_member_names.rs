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

use std::io::Write;

use flate2::write::DeflateEncoder;
use flate2::Compression;
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

/// What one header (local or central) or a data descriptor declares about
/// a member.
#[derive(Clone)]
struct Fields {
    flags: u16,
    method: u16,
    crc: u32,
    compressed: u32,
    size: u32,
    extra: Vec<u8>,
}

/// A data descriptor (general-purpose bit 3) written after the member's data.
struct Descriptor {
    signature: bool,
    /// Edits the values the descriptor carries (normally the true ones).
    edit: fn(&mut Fields),
    /// Keep the true values in the local header too, as Info-ZIP's
    /// `zip -e` does, instead of the zeros APPNOTE 4.4.4 asks for.
    keep_local: bool,
}

/// One member: raw name bytes, general-purpose flags, extra field, data,
/// and the uncompressed size the headers declare (normally the data's
/// length). Every header value can be made to lie on its own.
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
    /// 0 stored, 8 deflated (`data` is then the raw deflate stream).
    method: u16,
    /// The content's CRC-32 when `data` is compressed.
    crc: Option<u32>,
    /// Edits the local header only, after the writer filled it in.
    local: fn(&mut Fields),
    /// Edits the central record only.
    central: fn(&mut Fields),
    descriptor: Option<Descriptor>,
    /// Declare both sizes as 0xFFFFFFFF and carry them in zip64 fields.
    zip64: bool,
    /// Bytes written after the record that belong to no record.
    after: Vec<u8>,
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
        method: 0,
        crc: None,
        local: |_| {},
        central: |_| {},
        descriptor: None,
        zip64: false,
        after: Vec::new(),
    }
}

/// `content` deflated, with its true CRC-32 and size.
fn deflated(name: &str, content: &[u8]) -> Member {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content).unwrap();
    Member {
        method: 8,
        crc: Some(crc32(content)),
        declared: content.len() as u32,
        ..member(name, &encoder.finish().unwrap())
    }
}

/// `m` written with bit 3: zeros in the local header, a descriptor after
/// the data.
fn with_descriptor(m: Member, signature: bool) -> Member {
    Member {
        flags: m.flags | 0x0008,
        descriptor: Some(Descriptor {
            signature,
            edit: |_| {},
            keep_local: false,
        }),
        ..m
    }
}

fn hide_sizes(fields: &mut Fields) {
    fields.crc = 0;
    fields.compressed = 0;
    fields.size = 0;
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

/// Moves both sizes into a zip64 extra field (uncompressed, then
/// compressed) and declares them as 0xFFFFFFFF.
fn widen(fields: &mut Fields) {
    fields.extra.extend_from_slice(&0x0001u16.to_le_bytes());
    fields.extra.extend_from_slice(&16u16.to_le_bytes());
    fields
        .extra
        .extend_from_slice(&u64::from(fields.size).to_le_bytes());
    fields
        .extra
        .extend_from_slice(&u64::from(fields.compressed).to_le_bytes());
    fields.size = u32::MAX;
    fields.compressed = u32::MAX;
}

/// A ZIP with exactly these members, in this order.
fn zip(members: &[Member]) -> Vec<u8> {
    build(&[], members, false)
}

/// [`zip`] behind `prefix` bytes (a self-extractor stub, say); offsets are
/// written relative to the archive, as such tools do.
fn zip_after(prefix: &[u8], members: &[Member]) -> Vec<u8> {
    build(prefix, members, false)
}

/// [`zip`] ending in a zip64 end record and locator, with the classic end
/// record's counts, size and offset all set to their zip64 placeholders.
fn zip64_end(members: &[Member]) -> Vec<u8> {
    build(&[], members, true)
}

fn build(prefix: &[u8], members: &[Member], zip64_end: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    let mut offsets = Vec::new();
    for m in members {
        let truth = Fields {
            flags: m.flags,
            method: m.method,
            crc: m.crc.unwrap_or_else(|| crc32(&m.data)),
            compressed: m.data.len() as u32,
            size: m.declared,
            extra: m.extra.clone(),
        };
        let mut cen = truth.clone();
        if m.zip64 {
            widen(&mut cen);
        }
        (m.central)(&mut cen);
        if let Some(earlier) = m.alias_of {
            let offset: u32 = offsets[earlier];
            offsets.push(offset);
            central_record(&mut central, &m.name, &cen, offset);
            continue;
        }
        let offset = out.len() as u32;
        offsets.push(offset);
        let mut local = truth.clone();
        if m.descriptor.as_ref().is_some_and(|d| !d.keep_local) {
            hide_sizes(&mut local);
        }
        if m.zip64 {
            widen(&mut local);
        }
        (m.local)(&mut local);
        let local_name = m.local_name.as_ref().unwrap_or(&m.name);
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&local.flags.to_le_bytes());
        out.extend_from_slice(&local.method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0x21u16.to_le_bytes()); // date 1980-01-01
        out.extend_from_slice(&local.crc.to_le_bytes());
        out.extend_from_slice(&local.compressed.to_le_bytes());
        out.extend_from_slice(&local.size.to_le_bytes());
        out.extend_from_slice(&(local_name.len() as u16).to_le_bytes());
        out.extend_from_slice(&(local.extra.len() as u16).to_le_bytes());
        out.extend_from_slice(local_name);
        out.extend_from_slice(&local.extra);
        out.extend_from_slice(&m.data);
        if let Some(descriptor) = &m.descriptor {
            let mut values = truth.clone();
            (descriptor.edit)(&mut values);
            if descriptor.signature {
                out.extend_from_slice(&0x0807_4b50u32.to_le_bytes());
            }
            out.extend_from_slice(&values.crc.to_le_bytes());
            if m.zip64 {
                out.extend_from_slice(&u64::from(values.compressed).to_le_bytes());
                out.extend_from_slice(&u64::from(values.size).to_le_bytes());
            } else {
                out.extend_from_slice(&values.compressed.to_le_bytes());
                out.extend_from_slice(&values.size.to_le_bytes());
            }
        }
        out.extend_from_slice(&m.after);
        central_record(&mut central, &m.name, &cen, offset);
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    let count = members.len() as u16;
    let (count16, size32, start32) = if zip64_end {
        let record = out.len() as u64;
        out.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        out.extend_from_slice(&44u64.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&u64::from(count).to_le_bytes());
        out.extend_from_slice(&u64::from(count).to_le_bytes());
        out.extend_from_slice(&(central.len() as u64).to_le_bytes());
        out.extend_from_slice(&u64::from(start).to_le_bytes());
        out.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&record.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        (u16::MAX, u32::MAX, u32::MAX)
    } else {
        (count, central.len() as u32, start)
    };
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count16.to_le_bytes());
    out.extend_from_slice(&count16.to_le_bytes());
    out.extend_from_slice(&size32.to_le_bytes());
    out.extend_from_slice(&start32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    let mut whole = prefix.to_vec();
    whole.extend_from_slice(&out);
    whole
}

fn central_record(central: &mut Vec<u8>, name: &[u8], f: &Fields, offset: u32) {
    central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    central.extend_from_slice(&20u16.to_le_bytes());
    central.extend_from_slice(&20u16.to_le_bytes());
    central.extend_from_slice(&f.flags.to_le_bytes());
    central.extend_from_slice(&f.method.to_le_bytes());
    central.extend_from_slice(&0u16.to_le_bytes());
    central.extend_from_slice(&0x21u16.to_le_bytes());
    central.extend_from_slice(&f.crc.to_le_bytes());
    central.extend_from_slice(&f.compressed.to_le_bytes());
    central.extend_from_slice(&f.size.to_le_bytes());
    central.extend_from_slice(&(name.len() as u16).to_le_bytes());
    central.extend_from_slice(&(f.extra.len() as u16).to_le_bytes());
    central.extend_from_slice(&0u16.to_le_bytes()); // comment
    central.extend_from_slice(&0u16.to_le_bytes()); // disk
    central.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
    central.extend_from_slice(&0u32.to_le_bytes()); // external attributes
    central.extend_from_slice(&offset.to_le_bytes());
    central.extend_from_slice(name);
    central.extend_from_slice(&f.extra);
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
fn invalid_declared_utf8_is_refused_before_extraction() {
    let invalid = Member {
        flags: UTF8_FLAG,
        name: b"P-0001/BinaryData/\xff.dat".to_vec(),
        ..member("", b"synthetic")
    };
    assert!(Container::open(project_with(vec![invalid])).is_err());
}

#[test]
fn a_cp437_member_is_read_by_its_validated_identity() {
    let entry = Member {
        name: b"P-0001/BinaryData/\x82.dat".to_vec(),
        ..member("", b"synthetic-legacy-payload")
    };
    let bytes = project_with(vec![entry]);
    let mut container = Container::open(bytes.clone()).unwrap();
    assert_eq!(
        container.read("p-0001/binarydata/é.dat").unwrap(),
        b"synthetic-legacy-payload"
    );
    let imported = import_knxproj_bytes(bytes, "synthetic.knxproj").unwrap();
    let retained = imported
        .opaque
        .iter()
        .find(|e| e.source_path == "P-0001/BinaryData/é.dat")
        .unwrap();
    assert_eq!(retained.bytes, b"synthetic-legacy-payload");
}

#[test]
fn a_cp437_member_in_a_nested_payload_keeps_its_bytes() {
    let entry = Member {
        name: b"P-0001/BinaryData/\x82.dat".to_vec(),
        ..member("", b"synthetic-nested-payload")
    };
    let mut container =
        Container::open_with_password(protected_with(vec![entry]), "fictional").unwrap();
    assert_eq!(
        container.read("P-0001/BinaryData/é.dat").unwrap(),
        b"synthetic-nested-payload"
    );
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

// AR18 re-check round 3, N11: the `zip` reader takes every size from the
// central record and reads the local header only to skip its name and
// extra field. A central record that says "0 bytes" therefore hid the
// local bytes, and with a Unicode Path the real `0.xml` vanished again.
// Every local header must now agree with its central record, and the
// records must tile the archive up to the central directory.

const SIZES: &str = "(sizes/CRC)";
const METADATA: &str = "(flags/method)";
const DESCRIPTOR: &str = "data descriptor";
const UNICODE: &str = "Unicode Path";
const LAYOUT: &str = "next record";
const CARRIES: &str = "carries data";
const CLASH: &str = "same name as a file";

fn refused_because(result: Result<knx_etsproj::ImportOutcome, ImportFailure>, why: &str) {
    match result {
        Err(ImportFailure::Container(ContainerError::InconsistentRecord { reason, .. }))
            if reason.contains(why) => {}
        other => panic!(
            "expected an inconsistent record ({why}), got {:?}",
            other.map(|_| ())
        ),
    }
}

fn container_refused_because(result: Result<Container, ContainerError>, why: &str) {
    match result {
        Err(ContainerError::InconsistentRecord { reason, .. }) if reason.contains(why) => {}
        other => panic!(
            "expected an inconsistent record ({why}), got {:?}",
            other.map(|_| ())
        ),
    }
}

fn imports(bytes: Vec<u8>) -> knx_etsproj::ImportOutcome {
    match import_knxproj_bytes(bytes, "fixture.knxproj") {
        Ok(outcome) => outcome,
        Err(e) => panic!("expected the archive to import, got {e:?}"),
    }
}

/// The real topology, deflated, renamed by a Unicode Path in both headers
/// to `target`, with a central record that claims no bytes.
fn hidden_topology(target: &str, method: u16) -> Member {
    let real = if method == 8 {
        deflated("P-0001/0.xml", INSTALLATION)
    } else {
        member("P-0001/0.xml", INSTALLATION)
    };
    Member {
        extra: unicode_path(b"P-0001/0.xml", target),
        central: hide_sizes,
        ..real
    }
}

fn forged_topology() -> Member {
    let forged = String::from_utf8(INSTALLATION.to_vec())
        .unwrap()
        .replace("Name=\"\"", "Name=\"FORGED\"");
    upath_member("P-0001/zz.xml", "P-0001/0.xml", forged.as_bytes())
}

fn outer(real: Member) -> Vec<u8> {
    zip(&[
        member("P-0001.signature", b"x"),
        real,
        member("P-0001/Project.xml", PROJECT_INFO),
        forged_topology(),
    ])
}

fn nested(real: Member) -> Vec<u8> {
    let payload = zip(&[
        real,
        member("P-0001/Project.xml", PROJECT_INFO),
        forged_topology(),
    ]);
    zip(&[
        member("P-0001.signature", b"x"),
        member("P-0001.zip", &payload),
    ])
}

#[test]
fn the_real_topology_hidden_as_an_empty_directory_is_refused() {
    refused_because(
        import_knxproj_bytes(outer(hidden_topology("P-0001/old/", 8)), "f.knxproj"),
        SIZES,
    );
}

#[test]
fn the_real_topology_hidden_as_a_backslash_directory_is_refused() {
    refused_because(
        import_knxproj_bytes(outer(hidden_topology("P-0001/0.xml\\", 8)), "f.knxproj"),
        SIZES,
    );
}

#[test]
fn the_real_topology_hidden_as_an_empty_stored_file_is_refused() {
    let real = hidden_topology("P-0001/BinaryData/old.bin", 0);
    refused_because(import_knxproj_bytes(outer(real), "f.knxproj"), SIZES);
}

#[test]
fn the_hidden_topology_inside_a_protected_payload_is_refused() {
    container_refused_because(
        Container::open_with_password(nested(hidden_topology("P-0001/old/", 8)), "fictional"),
        SIZES,
    );
    container_refused_because(
        Container::open_with_password(
            nested(hidden_topology("P-0001/BinaryData/old.bin", 0)),
            "fictional",
        ),
        SIZES,
    );
}

#[test]
fn the_hidden_topology_with_a_data_descriptor_is_refused() {
    // Bit 3 puts zeros in the local header, so the lie moves to the
    // descriptor: KNXBench looks for it where the central size ends.
    for signature in [true, false] {
        let real = with_descriptor(hidden_topology("P-0001/old/", 8), signature);
        refused_because(import_knxproj_bytes(outer(real), "f.knxproj"), DESCRIPTOR);
        let real = with_descriptor(hidden_topology("P-0001/old/", 8), signature);
        container_refused_because(
            Container::open_with_password(nested(real), "fictional"),
            DESCRIPTOR,
        );
    }
}

#[test]
fn a_record_that_ends_early_and_leaves_bytes_behind_is_refused() {
    // Central sizes 0 and bit 3: twelve zero bytes pass for an unsigned
    // descriptor (CRC 0, sizes 0); what follows them belongs to no record.
    let mut data = vec![0u8; 12];
    data.extend_from_slice(&[7u8; 4500]);
    let directory = Member {
        flags: 0x0008,
        crc: Some(0),
        central: hide_sizes,
        local: hide_sizes,
        ..member("P-0001/BinaryData/hidden/", &data)
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
        LAYOUT,
    );
}

#[test]
fn bytes_between_two_records_are_refused() {
    let first = Member {
        after: vec![9u8; 64],
        ..member("P-0001/BinaryData/a.dat", b"bytes")
    };
    let bytes = project_with(vec![first, member("P-0001/BinaryData/b.dat", b"more")]);
    refused_because(import_knxproj_bytes(bytes, "f.knxproj"), LAYOUT);
}

#[test]
fn bytes_between_the_last_record_and_the_central_directory_are_refused() {
    let last = Member {
        after: vec![9u8; 64],
        ..member("P-0001/BinaryData/a.dat", b"bytes")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![last]), "f.knxproj"),
        LAYOUT,
    );
}

#[test]
fn a_directory_whose_central_record_hides_its_local_bytes_is_refused() {
    for nested_layer in [false, true] {
        let directory = Member {
            central: hide_sizes,
            ..member("P-0001/BinaryData/hidden/", &[7u8; 4500])
        };
        if nested_layer {
            container_refused_because(
                Container::open_with_password(protected_with(vec![directory]), "fictional"),
                SIZES,
            );
        } else {
            refused_because(
                import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
                SIZES,
            );
        }
    }
}

#[test]
fn a_member_whose_central_record_says_stored_empty_is_refused() {
    let plain = Member {
        central: hide_sizes,
        ..member("P-0001/BinaryData/a.dat", &[5u8; 3200])
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![plain]), "f.knxproj"),
        SIZES,
    );
}

#[test]
fn a_local_crc_that_differs_from_the_central_one_is_refused() {
    let m = Member {
        local: |f| f.crc ^= 1,
        ..member("P-0001/BinaryData/a.dat", b"bytes")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
        SIZES,
    );
}

#[test]
fn a_local_compressed_size_that_differs_from_the_central_one_is_refused() {
    let m = Member {
        local: |f| f.compressed += 1,
        ..deflated("P-0001/BinaryData/a.dat", b"bytes bytes bytes")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
        SIZES,
    );
}

#[test]
fn a_local_uncompressed_size_that_differs_from_the_central_one_is_refused() {
    let m = Member {
        local: |f| f.size += 1,
        ..deflated("P-0001/BinaryData/a.dat", b"bytes bytes bytes")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
        SIZES,
    );
}

#[test]
fn a_local_method_that_differs_from_the_central_one_is_refused() {
    let m = Member {
        local: |f| f.method = 8,
        ..member("P-0001/BinaryData/a.dat", b"bytes")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
        METADATA,
    );
}

#[test]
fn local_flags_that_differ_from_the_central_ones_are_refused() {
    let m = Member {
        local: |f| f.flags |= 0x0800,
        ..member("P-0001/BinaryData/a.dat", b"bytes")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
        METADATA,
    );
}

#[test]
fn a_local_bit_3_without_the_central_one_is_refused() {
    let m = Member {
        central: |f| f.flags &= !0x0008,
        ..with_descriptor(member("P-0001/BinaryData/a.dat", b"bytes"), true)
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
        METADATA,
    );
}

#[test]
fn local_values_beside_a_descriptor_that_are_not_the_central_ones_are_refused() {
    // With bit 3 the local header carries zeros (APPNOTE) or the true
    // values (Info-ZIP); anything else is a second story.
    let m = Member {
        local: |f| f.size = 99,
        ..with_descriptor(member("P-0001/BinaryData/a.dat", b"bytes"), true)
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
        SIZES,
    );
}

#[test]
fn true_local_values_beside_a_descriptor_import() {
    // Info-ZIP's `zip -e` layout (the ZipCrypto fixtures were made with it).
    let info_zip = |m: Member| Member {
        descriptor: Some(Descriptor {
            signature: true,
            edit: |_| {},
            keep_local: true,
        }),
        ..with_descriptor(m, true)
    };
    imports(zip(&every_member(info_zip)));
}

#[test]
fn a_descriptor_that_differs_from_the_central_record_is_refused() {
    for edit in [
        (|f: &mut Fields| f.crc ^= 1) as fn(&mut Fields),
        |f| f.compressed += 1,
        |f| f.size += 1,
    ] {
        for signature in [true, false] {
            let m = with_descriptor(member("P-0001/BinaryData/a.dat", b"bytes"), signature);
            let m = Member {
                descriptor: Some(Descriptor {
                    signature,
                    edit,
                    keep_local: false,
                }),
                ..m
            };
            refused_because(
                import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
                DESCRIPTOR,
            );
        }
    }
}

#[test]
fn a_local_unicode_path_that_differs_from_the_central_one_is_refused() {
    let only_local = Member {
        local: |f| f.extra = unicode_path(b"P-0001/BinaryData/a.dat", "P-0001/x.dat"),
        ..member("P-0001/BinaryData/a.dat", b"bytes")
    };
    let only_central = Member {
        central: |f| f.extra = unicode_path(b"P-0001/BinaryData/a.dat", "P-0001/x.dat"),
        ..member("P-0001/BinaryData/a.dat", b"bytes")
    };
    let different = Member {
        local: |f| f.extra = unicode_path(b"P-0001/BinaryData/a.dat", "P-0001/y.dat"),
        ..upath_member("P-0001/BinaryData/a.dat", "P-0001/x.dat", b"bytes")
    };
    for m in [only_local, only_central, different] {
        refused_because(
            import_knxproj_bytes(project_with(vec![m]), "f.knxproj"),
            UNICODE,
        );
    }
}

#[test]
fn a_backslash_directory_named_like_a_file_is_refused() {
    // `zip` treats a trailing `\` as a directory, like a trailing `/`.
    let bytes = project_with(vec![
        member("P-0001/BinaryData/x.dat\\", b""),
        member("P-0001/BinaryData/x.dat", b"file"),
    ]);
    refused_because(import_knxproj_bytes(bytes, "f.knxproj"), CLASH);
}

// N12: Java's `ZipOutputStream` and `jar` write an empty directory as a
// deflated empty stream (two bytes). It carries nothing and imports; a
// directory whose stream inflates to bytes, or hides bytes behind the end
// of its stream, still carries data.

const EMPTY_DEFLATE: [u8; 2] = [0x03, 0x00];

#[test]
fn an_empty_directory_written_deflated_imports() {
    let directory = Member {
        method: 8,
        crc: Some(0),
        declared: 0,
        ..member("P-0001/BinaryData/", &EMPTY_DEFLATE)
    };
    imports(project_with(vec![
        directory,
        member("P-0001/BinaryData/a.dat", b"bytes"),
    ]));
}

#[test]
fn a_deflated_directory_whose_stream_inflates_to_bytes_is_refused() {
    // One byte is the hard case: the stream then ends cleanly as well.
    let directory = Member {
        declared: 0,
        crc: Some(0),
        ..deflated("P-0001/BinaryData/hidden/", b"x")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
        CARRIES,
    );
}

#[test]
fn a_deflated_directory_with_bytes_behind_its_stream_is_refused() {
    let mut data = EMPTY_DEFLATE.to_vec();
    data.extend_from_slice(b"hidden");
    let directory = Member {
        method: 8,
        crc: Some(0),
        declared: 0,
        ..member("P-0001/BinaryData/hidden/", &data)
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
        CARRIES,
    );
}

#[test]
fn a_stored_directory_whose_headers_agree_on_hidden_bytes_is_refused() {
    let directory = Member {
        declared: 0,
        crc: Some(0),
        ..member("P-0001/BinaryData/hidden/", &[7u8; 4500])
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
        CARRIES,
    );
}

#[test]
fn a_stored_directory_holding_an_empty_deflate_stream_is_refused() {
    // Stored, those two bytes are content, whatever they look like.
    let directory = Member {
        declared: 0,
        crc: Some(0),
        ..member("P-0001/BinaryData/hidden/", &EMPTY_DEFLATE)
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
        CARRIES,
    );
}

#[test]
fn a_directory_that_declares_bytes_it_does_not_store_is_refused() {
    let directory = Member {
        declared: 5,
        ..member("P-0001/BinaryData/hidden/", b"")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
        CARRIES,
    );
}

#[test]
fn a_record_that_runs_past_the_archive_is_refused() {
    // Both headers agree on a size the archive does not hold.
    let directory = Member {
        local: |f| f.compressed = 0x00ff_0000,
        central: |f| f.compressed = 0x00ff_0000,
        ..member("P-0001/BinaryData/hidden/", b"")
    };
    refused_because(
        import_knxproj_bytes(project_with(vec![directory]), "f.knxproj"),
        "truncated",
    );
}

// Controls: shapes real writers produce must keep importing.

fn every_member(edit: impl Fn(Member) -> Member) -> Vec<Member> {
    vec![
        edit(member("P-0001.signature", b"x")),
        edit(deflated("P-0001/0.xml", INSTALLATION)),
        edit(member("P-0001/Project.xml", PROJECT_INFO)),
        edit(deflated("P-0001/BinaryData/a.dat", b"bytes bytes bytes")),
    ]
}

#[test]
fn every_member_with_a_data_descriptor_imports() {
    for signature in [true, false] {
        imports(zip(&every_member(|m| with_descriptor(m, signature))));
    }
}

#[test]
fn a_protected_payload_with_data_descriptors_opens() {
    for signature in [true, false] {
        let payload = zip(&[
            with_descriptor(deflated("P-0001/0.xml", INSTALLATION), signature),
            with_descriptor(member("P-0001/Project.xml", PROJECT_INFO), signature),
        ]);
        let bytes = zip(&[
            member("P-0001.signature", b"x"),
            with_descriptor(member("P-0001.zip", &payload), signature),
        ]);
        let container = Container::open_with_password(bytes, "fictional").unwrap();
        assert!(container.find("P-0001/0.xml").is_some());
    }
}

#[test]
fn an_all_zip64_archive_imports() {
    let members = every_member(|m| Member { zip64: true, ..m });
    imports(zip64_end(&members));
}

#[test]
fn zip64_members_with_data_descriptors_import() {
    // Python's `zipfile` streaming layout: 0xFFFFFFFF in the local header,
    // zeros in its zip64 field, 8-byte sizes in the descriptor.
    for signature in [true, false] {
        imports(zip(&every_member(|m| {
            with_descriptor(Member { zip64: true, ..m }, signature)
        })));
    }
}

#[test]
fn an_archive_with_only_a_zip64_end_record_imports() {
    imports(zip64_end(&every_member(|m| m)));
}

#[test]
fn a_unicode_path_in_both_headers_imports_under_its_decoded_name() {
    // ETS writes the field into both headers of a nested record.
    let outcome = imports(project_with(vec![upath_member(
        "P-0001/BinaryData/raw.dat",
        "P-0001/BinaryData/d\u{e9}cod\u{e9}.dat",
        b"bytes",
    )]));
    assert!(outcome
        .opaque
        .iter()
        .any(|entry| entry.source_path == "P-0001/BinaryData/d\u{e9}cod\u{e9}.dat"));
}

#[test]
fn a_protected_payload_with_a_consistent_unicode_path_opens() {
    let payload = zip(&[
        member("P-0001/0.xml", INSTALLATION),
        member("P-0001/Project.xml", PROJECT_INFO),
        upath_member("P-0001/BinaryData/raw.dat", "P-0001/BinaryData/x.dat", b"x"),
    ]);
    let bytes = zip(&[
        member("P-0001.signature", b"x"),
        member("P-0001.zip", &payload),
    ]);
    let container = Container::open_with_password(bytes, "fictional").unwrap();
    assert!(container.find("P-0001/BinaryData/x.dat").is_some());
}
