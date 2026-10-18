//! Format-2 diagnostic archives: declared memory evidence, strict reader, format 1 intact.
use knx_app::device_diagnostics::{
    acquire_memory, bundle, bundle_export, pending, prepare, prepare_memory, read_bundle,
    read_export, write_new_export, Export, MEMORY_PROFILE,
};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use std::io::{Cursor, Read, Write};
use std::sync::atomic::AtomicBool;

/// A synthetic System-2 program in the exact `project_tool_metadata` shape:
/// a PID78 comparison and two absolute segments (30 + 12 octets).
const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="OBJ" Address="1024" Size="8" /></Code>
<ComObjectTable CodeSegment="OBJ" Offset="0" />
<LoadProcedures><LoadProcedure><LdCtrlConnect />
<LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="00010000005B00000000" />
<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="278" Size="30" Access="51" MemType="3" SegFlags="0" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="15360" Size="12" Access="51" MemType="3" SegFlags="0" />
<LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" />
<LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1">
<Products><Product Id="M-0001_H-1_P-1" /></Products><Hardware2Programs><Hardware2Program Id="H_HP" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-0001_A-0001-01-0000" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn products(program: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let db = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&db, "synthetic.xml", program.as_bytes()).unwrap();
    knx_productdb::ingest_file(&db, "Hardware.xml", HARDWARE).unwrap();
    (dir, db)
}

fn project() -> Project {
    project_with("H_HP")
}

fn project_with(program_ref: &str) -> Project {
    let mut p = Project::new(Language("en".into()));
    p.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
            path: "/home/alice/SECRET".into(),
            ets_id: "SECRET_SOURCE".into(),
        },
        name: "SECRET_ROOM".into(),
        description: None,
        address: Some("1.1.25".parse().unwrap()),
        product_ref: "M-0001_H-1_P-1".into(),
        program_ref: program_ref.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    p
}

fn device() -> SimulatedDevice {
    let d = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        ..Default::default()
    });
    d.preset_property(0, 12, &[0x00, 0x01]);
    d.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5B]);
    d.preset_memory(0x0116, &(1..=30).collect::<Vec<u8>>());
    d.preset_memory(0x3C00, &[0xEE; 12]);
    d
}

async fn export() -> Export {
    let (_dir, db) = products(PROGRAM);
    let plan = prepare_memory(&project(), DeviceId(7), &db).unwrap();
    acquire_memory(
        &device(),
        &plan,
        Default::default(),
        "synthetic-test",
        &AtomicBool::new(false),
        |_| {},
    )
    .await
}

/// Rewrites an archive member by member, with `edit` deciding each member.
fn rewrite(original: &[u8], mut edit: impl FnMut(&str, Vec<u8>) -> Option<Vec<u8>>) -> Vec<u8> {
    let mut input = zip::ZipArchive::new(Cursor::new(original)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(vec![]));
    for i in 0..input.len() {
        let mut f = input.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        if let Some(data) = edit(&name, data) {
            output
                .start_file(&name, zip::write::SimpleFileOptions::default())
                .unwrap();
            output.write_all(&data).unwrap();
        }
    }
    output.finish().unwrap().into_inner()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    format!("{:x}", sha2::Sha256::digest(bytes))
}

#[test]
fn the_profile_is_derived_from_product_data_with_required_identity() {
    let (_dir, db) = products(PROGRAM);
    let plan = prepare_memory(&project(), DeviceId(7), &db).unwrap();
    assert_eq!(plan.base().expected.mask, Some(0x0025));
    assert_eq!(plan.base().expected.manufacturer, Some(0x0001));
    assert_eq!(plan.profile().program_id(), "M-0001_A-0001-01-0000");
    assert_eq!(plan.profile().ranges().total_octets(), 42);
    assert_eq!(plan.profile().range_positions(), [2, 3]);
    // Not the System-2 shape: refused before any adapter exists.
    let (_dir, other) = products(&PROGRAM.replace("MV-0025", "MV-0701"));
    let error = prepare_memory(&project(), DeviceId(7), &other).unwrap_err();
    assert!(error.contains("diagnostics-profile-unavailable"), "{error}");
    // No product mapping for the device: refused.
    let error = prepare_memory(&project_with("H_OTHER"), DeviceId(7), &db).unwrap_err();
    assert!(error.contains("diagnostics-program-unresolved"), "{error}");
}

#[tokio::test]
async fn a_format_2_export_round_trips_with_memory_evidence_and_coverage() {
    let export = export().await;
    let report = &export.report;
    assert_eq!(report.format, 2);
    assert_eq!(report.profile, MEMORY_PROFILE);
    assert_eq!(report.memory_status, "requested");
    assert!(report.simulated && !report.written && report.complete);
    assert_eq!(report.stop_reason, None);
    let memory = export.memory.as_ref().unwrap();
    assert_eq!(memory.program_id, "M-0001_A-0001-01-0000");
    assert_eq!(memory.chunk_octets, 12);
    assert_eq!(memory.chunks.len(), 4);
    assert_eq!(memory.chunks[0].bytes, "0102030405060708090a0b0c");
    assert_eq!(memory.ranges[0].procedure_position, 2);
    assert_eq!(memory.ranges[0].coverage.read, 30);
    assert_eq!(memory.ranges[1].coverage.read, 12);
    assert_eq!(
        memory.hardware_type_comparison.inline_data,
        "00010000005b00000000"
    );
    let bytes = bundle_export(&export).unwrap();
    assert_eq!(read_export(&bytes).unwrap(), export);
    // A format-2 archive is never silently reduced to its identity part.
    assert!(read_bundle(&bytes).is_err());
    let names: Vec<String> = zip::ZipArchive::new(Cursor::new(&bytes))
        .unwrap()
        .file_names()
        .map(str::to_string)
        .collect();
    assert_eq!(names.len(), 5);
    assert!(names.iter().any(|n| n == "memory.json"));
    let text = |name: &str| {
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut s = String::new();
        zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
        s
    };
    for name in ["README.txt", "manifest.json"] {
        let content = text(name);
        for phrase in [
            "not proven runtime addresses",
            "not a backup",
            "not a restore basis",
            "keep private",
        ] {
            assert!(content.contains(phrase), "{name} lacks {phrase}: {content}");
        }
    }
    for secret in ["SECRET", "/home/", "1.1.25"] {
        for name in ["report.json", "memory.json", "manifest.json"] {
            assert!(!text(name).contains(secret), "{name} leaked {secret}");
        }
    }
}

#[tokio::test]
async fn the_writer_is_create_only_owner_only_and_rereads() {
    let export = export().await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory-evidence.zip");
    write_new_export(&path, &export).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let written = std::fs::read(&path).unwrap();
    assert_eq!(read_export(&written).unwrap(), export);
    assert!(
        write_new_export(&path, &export).is_err(),
        "never overwrites"
    );
    assert_eq!(std::fs::read(&path).unwrap(), written);
}

#[tokio::test]
async fn the_strict_reader_refuses_missing_extra_and_tampered_memory_evidence() {
    let original = bundle_export(&export().await).unwrap();
    let missing = rewrite(&original, |name, data| {
        (name != "memory.json").then_some(data)
    });
    assert!(
        read_export(&missing).is_err(),
        "accepted missing memory.json"
    );
    let mut extra = rewrite(&original, |_, data| Some(data));
    {
        let mut zip = zip::ZipWriter::new_append(Cursor::new(&mut extra)).unwrap();
        zip.start_file("dump.bin", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&[0; 4]).unwrap();
        zip.finish().unwrap();
    }
    assert!(read_export(&extra).is_err(), "accepted an extra member");
    let tampered = rewrite(&original, |name, mut data| {
        if name == "memory.json" {
            data.push(b' ');
        }
        Some(data)
    });
    assert!(read_export(&tampered).is_err(), "accepted a hash mismatch");
    // Even with a matching hash, a chunk claimed read with other bytes, or a
    // coverage that does not follow from the chunks, is refused.
    for (from, to) in [
        ("0102030405060708090a0b0c", "0102030405060708090a0b"),
        ("\"read\": 30", "\"read\": 29"),
        ("\"status\": \"read\"", "\"status\": \"not-read\""),
    ] {
        let mut memory = vec![];
        let edited = rewrite(&original, |name, data| {
            if name == "memory.json" {
                let text = String::from_utf8(data).unwrap();
                assert!(text.contains(from), "fixture lacks {from}");
                memory = text.replacen(from, to, 1).into_bytes();
                Some(memory.clone())
            } else {
                Some(data)
            }
        });
        let fixed = rewrite(&edited, |name, data| {
            if name == "manifest.json" {
                let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
                v["files"]["memory.json"]["bytes"] = memory.len().into();
                v["files"]["memory.json"]["sha256"] = sha256(&memory).into();
                Some(serde_json::to_vec_pretty(&v).unwrap())
            } else {
                Some(data)
            }
        });
        assert!(read_export(&fixed).is_err(), "accepted {from} -> {to}");
    }
    // A format-2 archive relabelled as format 1 is refused.
    let relabelled = rewrite(&original, |name, data| {
        if name == "manifest.json" {
            let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
            v["format"] = 1.into();
            Some(serde_json::to_vec_pretty(&v).unwrap())
        } else {
            Some(data)
        }
    });
    assert!(read_export(&relabelled).is_err());
}

#[test]
fn format_1_archives_keep_four_members_and_no_memory_notice() {
    let plan = prepare(&project(), DeviceId(7), None).unwrap();
    let report = pending(&plan, "synthetic", true);
    let bytes = bundle(&report).unwrap();
    assert_eq!(zip::ZipArchive::new(Cursor::new(&bytes)).unwrap().len(), 4);
    let export = read_export(&bytes).unwrap();
    assert_eq!(export.memory, None);
    assert_eq!(read_bundle(&bytes).unwrap(), report);
    assert_eq!(
        bundle_export(&export).unwrap(),
        bytes,
        "byte-identical rewrite"
    );
    let mut manifest = String::new();
    zip::ZipArchive::new(Cursor::new(&bytes))
        .unwrap()
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    assert!(!manifest.contains("notice"));
}

/// The two declared segments of [`PROGRAM`], as they appear in it.
const SEGMENTS: &str = r#"<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="278" Size="30" Access="51" MemType="3" SegFlags="0" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="15360" Size="12" Access="51" MemType="3" SegFlags="0" />"#;

#[tokio::test]
async fn the_largest_admitted_profile_fits_the_archive_limits() {
    // Eight declared ranges of 256 octets: the 2048-octet profile limit,
    // derived from product data and read from a device showing exactly the
    // asserted identity, so the archive is a real, consistent acquisition.
    let segments: String = (0..8u32)
        .map(|i| {
            format!(
                r#"<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="{}" Size="256" Access="51" MemType="3" SegFlags="0" />"#,
                0x1000 + 0x400 * i
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(PROGRAM.contains(SEGMENTS), "fixture drifted");
    let (_dir, db) = products(&PROGRAM.replace(SEGMENTS, &segments));
    let plan = prepare_memory(&project(), DeviceId(7), &db).unwrap();
    assert_eq!(plan.profile().ranges().total_octets(), 2048);
    let device = device();
    for i in 0..8u32 {
        device.preset_memory(0x1000 + 0x400 * i, &[0xFF; 256]);
    }
    let export = acquire_memory(
        &device,
        &plan,
        Default::default(),
        "synthetic",
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert_eq!(export.report.stop_reason, None);
    let memory = export.memory.as_ref().unwrap();
    assert!(memory.chunks.iter().all(|c| c.status == "read"));
    assert!(memory.ranges.iter().all(|r| r.coverage.read == 256));
    let bytes = bundle_export(&export).unwrap();
    assert_eq!(read_export(&bytes).unwrap(), export);
}

/// Re-encodes `report.json`, `properties.json` (always the report's items)
/// and `memory.json` after `edit`, and rewrites the manifest's sizes and
/// hashes to match, so only the reader's semantic checks can refuse it.
fn rehashed(
    original: &[u8],
    edit: impl FnOnce(&mut serde_json::Value, &mut serde_json::Value),
) -> Vec<u8> {
    let member = |name: &str| {
        let mut zip = zip::ZipArchive::new(Cursor::new(original)).unwrap();
        let mut data = vec![];
        zip.by_name(name).unwrap().read_to_end(&mut data).unwrap();
        serde_json::from_slice::<serde_json::Value>(&data).unwrap()
    };
    let (mut report, mut memory) = (member("report.json"), member("memory.json"));
    edit(&mut report, &mut memory);
    let encoded = [
        ("report.json", serde_json::to_vec_pretty(&report).unwrap()),
        (
            "properties.json",
            serde_json::to_vec_pretty(&report["items"]).unwrap(),
        ),
        ("memory.json", serde_json::to_vec_pretty(&memory).unwrap()),
    ];
    let edited = rewrite(original, |name, data| {
        Some(
            encoded
                .iter()
                .find(|(n, _)| *n == name)
                .map_or(data, |(_, v)| v.clone()),
        )
    });
    rewrite(&edited, |name, data| {
        if name != "manifest.json" {
            return Some(data);
        }
        let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
        for (n, bytes) in &encoded {
            v["files"][n]["bytes"] = bytes.len().into();
            v["files"][n]["sha256"] = sha256(bytes).into();
        }
        Some(serde_json::to_vec_pretty(&v).unwrap())
    })
}

type Edit = fn(&mut serde_json::Value, &mut serde_json::Value);

/// Every named edit that the reader accepted; each edit breaks one rule.
fn accepted(original: &[u8], cases: &[(&'static str, Edit)]) -> Vec<&'static str> {
    // The unedited re-encoding is accepted, so a refusal below is the rule's.
    read_export(&rehashed(original, |_, _| {})).expect("re-encoding alone is accepted");
    cases
        .iter()
        .filter(|(_, edit)| read_export(&rehashed(original, edit)).is_ok())
        .map(|(name, _)| *name)
        .collect()
}

#[tokio::test]
async fn memory_evidence_is_refused_unless_the_recorded_identity_admits_it() {
    let original = bundle_export(&export().await).unwrap();
    let cases: &[(&'static str, Edit)] = &[
        ("descriptor differs from the asserted mask", |r, _| {
            r["items"][0]["bytes"] = serde_json::json!([7, 5]);
        }),
        ("asserted mask contradicts the descriptor", |r, _| {
            r["assertedMask"] = 0x0705.into();
        }),
        ("PID12 differs from the asserted manufacturer", |r, _| {
            r["items"][1]["bytes"] = serde_json::json!([0, 2]);
        }),
        ("asserted manufacturer contradicts PID12", |r, _| {
            r["assertedManufacturer"] = 2.into();
        }),
        ("PID78 does not match the recorded inlineData", |r, _| {
            r["items"][2]["bytes"] = serde_json::json!([0, 1, 0, 0, 0, 0x5C]);
        }),
        ("recorded inlineData does not match PID78", |_, m| {
            m["hardwareTypeComparison"]["inlineData"] = "00010000005c00000000".into();
        }),
        ("PID12 returned two elements", |r, _| {
            r["items"][1]["returnedElements"] = 2.into();
        }),
        ("PID78 returned no element count", |r, _| {
            r["items"][2]["returnedElements"] = serde_json::Value::Null;
        }),
        ("the descriptor carries an element count", |r, _| {
            r["items"][0]["returnedElements"] = 1.into();
        }),
        ("a read chunk lost its attempt time", |_, m| {
            m["chunks"][0]["attemptedAt"] = serde_json::Value::Null;
        }),
        ("a read chunk lost its completion time", |_, m| {
            m["chunks"][0]["completedAt"] = serde_json::Value::Null;
        }),
        ("a chunk completed before it was attempted", |_, m| {
            m["chunks"][0]["completedAt"] = "2000-01-01T00:00:00+00:00".into();
        }),
        ("an unread chunk carries a completion time", |r, m| {
            // Otherwise a consistent cancellation before the last chunk.
            let last = &mut m["chunks"][3];
            last["status"] = "not-read".into();
            last["bytes"] = "".into();
            last["receivedOctets"] = 0.into();
            last["attemptedAt"] = serde_json::Value::Null;
            m["ranges"][1]["coverage"]["read"] = 0.into();
            m["ranges"][1]["coverage"]["notRead"] = 12.into();
            r["stopReason"] = "cancelled".into();
        }),
    ];
    assert_eq!(accepted(&original, cases), Vec::<&str>::new());
}
