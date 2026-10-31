//! Regression tests for credential-safe admission of retained diagnostic results.

use super::{admit_retained_diagnostics, admit_retained_support};
use crate::device_diagnostics::{
    self, ChunkReading, Comparison, Coverage, Export, MemoryEvidence, RangeEvidence,
};
use crate::support_report::{build_report, ReportInput};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use knx_net::commissioning::diagnostic_export::Identity;
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use serde_json::Value;
use std::io::Read;
use std::sync::atomic::AtomicBool;

const PRIVATE: [&str; 6] = [
    "CUSTOMER_PASSWORD",
    "PRIVATE_PATH",
    "PRIVATE_DESCRIPTION",
    "PRIVATE_PRODUCT",
    "PRIVATE_PROGRAM",
    "1.1.25",
];

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
            path: "PRIVATE_PATH".into(),
            ets_id: "CUSTOMER_PASSWORD".into(),
        },
        name: "CUSTOMER_PASSWORD".into(),
        description: Some("PRIVATE_DESCRIPTION".into()),
        address: Some("1.1.25".parse().unwrap()),
        product_ref: "PRIVATE_PRODUCT".into(),
        program_ref: "PRIVATE_PROGRAM".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

async fn identity_export() -> Export {
    let project = project();
    let plan = device_diagnostics::prepare(&project, DeviceId(7), None).unwrap();
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        ..Default::default()
    });
    device.preset_property(0, 12, &[0, 1]);
    device.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5b]);
    let report = device_diagnostics::acquire(
        &device,
        &plan,
        Default::default(),
        "retained-admission-test",
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert!(report.complete);
    assert!(device_diagnostics::bundle(&report).is_ok());
    Export {
        report,
        memory: None,
    }
}

fn file<'a>(files: &'a [super::BundleFile], name: &str) -> &'a [u8] {
    &files.iter().find(|file| file.name == name).unwrap().bytes
}

#[tokio::test]
async fn real_generic_identity_result_admits_only_fixed_producer_files() {
    let export = identity_export().await;
    let admitted = match admit_retained_diagnostics(&export, "retained-admission-test") {
        Ok(admitted) => admitted,
        Err(error) => panic!(
            "safe generic identity result was withheld: {}",
            error.code()
        ),
    };
    let files = admitted.into_files();
    assert_eq!(
        files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["report.json", "properties.json", "README.txt"]
    );
    let report: Value = serde_json::from_slice(file(&files, "report.json")).unwrap();
    let properties: Value = serde_json::from_slice(file(&files, "properties.json")).unwrap();
    assert_eq!(report["items"], properties);
    assert_eq!(report["profile"], device_diagnostics::PROFILE);
    assert_eq!(report["written"], false);
    let joined = files
        .iter()
        .flat_map(|file| file.bytes.iter().copied())
        .collect::<Vec<_>>();
    let text = String::from_utf8_lossy(&joined);
    for sentinel in PRIVATE {
        assert!(
            !text.contains(sentinel),
            "retained files contain private fixture data"
        );
    }
}

#[test]
fn projectless_support_result_admits_its_existing_closed_report_and_readme() {
    let report = build_report(
        None,
        None,
        None,
        &ReportInput {
            program_id: Some("M-0001_A-0001-01-0001".into()),
            firmware: Some("1.2.3".into()),
            mask: Some(0x0025),
            ..Default::default()
        },
        "retained-admission-test",
    );
    assert!(report.bundle().is_ok());
    let admitted = match admit_retained_support(&report, "retained-admission-test") {
        Ok(admitted) => admitted,
        Err(error) => panic!("safe support result was withheld: {}", error.code()),
    };
    let files = admitted.into_files();
    assert_eq!(
        files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["report.json", "README.txt"]
    );
    let decoded: Value = serde_json::from_slice(file(&files, "report.json")).unwrap();
    assert_eq!(decoded["kind"], "knxbench-support-report");
    assert_eq!(decoded["written"], false);
    assert_eq!(
        decoded["facts"],
        serde_json::to_value(&report.facts).unwrap()
    );
}

fn read_item(
    order: usize,
    query: &str,
    property_id: Option<u8>,
    bytes: Vec<u8>,
) -> device_diagnostics::Reading {
    device_diagnostics::Reading {
        order,
        query: query.into(),
        object_index: property_id.map(|_| 0),
        property_id,
        start_index: property_id.map(|_| 1),
        requested_elements: property_id.map(|_| 1),
        expected_bytes: bytes.len(),
        status: "read".into(),
        received_bytes: bytes.len(),
        returned_elements: property_id.map(|_| 1),
        bytes,
        attempted_at: Some("2026-10-30T10:00:00Z".into()),
        completed_at: Some("2026-10-30T10:00:01Z".into()),
        error: None,
    }
}

fn opaque_memory_export() -> Export {
    let mut report = device_diagnostics::pending(
        &device_diagnostics::Prepared {
            target: knx_core::ContactableAddress::new("1.1.25".parse().unwrap()).unwrap(),
            expected: Identity {
                mask: Some(0x0025),
                manufacturer: Some(1),
            },
            reference: "01".repeat(32),
        },
        "retained-admission-test",
        true,
    );
    report.format = 2;
    report.profile = device_diagnostics::MEMORY_PROFILE.into();
    report.memory_status = "requested".into();
    report.memory_reason = "Source-declared candidate ranges of the device's application, read only after the device showed the asserted identity; see memory.json.".into();
    report.complete = true;
    report.items = vec![
        read_item(1, "descriptor-type-0", None, vec![0, 0x25]),
        read_item(2, "manufacturer-id", Some(12), vec![0, 1]),
        read_item(3, "hardware-type", Some(78), vec![0, 1, 0, 0, 0, 0x5b]),
    ];
    let secret = b"Passw0rd";
    let memory = MemoryEvidence {
        profile: device_diagnostics::MEMORY_PROFILE.into(),
        notice: "Source-declared candidate locations, not proven runtime addresses. This is not a backup and not a restore basis. It contains device configuration: keep private.".into(),
        program_id: "M-0001_A-0001-01-0001".into(),
        hardware_type_comparison: Comparison {
            object_index: 0,
            property_id: 78,
            procedure_position: 0,
            inline_data: "00010000005b".into(),
        },
        chunk_octets: 12,
        ranges: vec![RangeEvidence {
            order: 1,
            address: 0x1000,
            length: secret.len() as u16,
            procedure_position: 1,
            coverage: Coverage {
                read: secret.len() as u32,
                ..Default::default()
            },
        }],
        chunks: vec![ChunkReading {
            order: 1,
            range: 1,
            address: 0x1000,
            requested_octets: secret.len() as u8,
            status: "read".into(),
            received_octets: secret.len(),
            bytes: secret.iter().map(|byte| format!("{byte:02x}")).collect(),
            attempted_at: Some("2026-10-30T10:00:02Z".into()),
            completed_at: Some("2026-10-30T10:00:03Z".into()),
            error: None,
        }],
    };
    Export {
        report,
        memory: Some(memory),
    }
}

#[test]
fn consented_memory_evidence_is_admitted_with_its_private_warning() {
    // Owner decision 2026-10-31: declared memory leaves only after consent, with
    // the producer's own "keep private" warning; it is not credential-classified.
    let export = opaque_memory_export();
    let archive = device_diagnostics::bundle_export(&export)
        .expect("the witness must satisfy the real format-2 writer grammar");
    assert_eq!(device_diagnostics::read_export(&archive).unwrap(), export);
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&archive)).unwrap();
    let mut expected = Vec::new();
    zip.by_name("memory.json")
        .unwrap()
        .read_to_end(&mut expected)
        .unwrap();
    let encoded = b"5061737377307264";
    assert!(expected.windows(encoded.len()).any(|part| part == encoded));
    let files = match admit_retained_diagnostics(&export, "retained-admission-test") {
        Ok(files) => files.into_files(),
        Err(error) => panic!("consented memory was refused: {}", error.code()),
    };
    let names: Vec<_> = files.iter().map(|file| file.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "report.json",
            "properties.json",
            "memory.json",
            "README.txt"
        ]
    );
    assert_eq!(
        files[2].bytes, expected,
        "memory.json is the producer's bytes"
    );
    let readme = String::from_utf8(files[3].bytes.clone()).unwrap();
    assert!(readme.contains("keep private"), "{readme}");
}

#[test]
fn oversized_support_report_is_refused_without_exposing_its_value() {
    let mut report = build_report(None, None, None, &ReportInput::default(), "test");
    report.facts.push(crate::support_report::Fact {
        field: "firmware".into(),
        origin: "user".into(),
        status: "observed".into(),
        value: Some("SENSITIVE".repeat(70_000)),
    });
    assert_eq!(report.bundle().unwrap_err(), "report-size-limit");
    let error = match admit_retained_support(&report, "test") {
        Ok(_) => panic!("oversized support report was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
    assert!(!error.code().contains("SENSITIVE"));
}

#[tokio::test]
async fn malformed_identity_result_is_not_reinterpreted_after_writer_refusal() {
    let mut export = identity_export().await;
    export.report.profile = "future-private-profile".into();
    assert_eq!(
        device_diagnostics::bundle_export(&export).unwrap_err(),
        "diagnostics-invalid-report"
    );
    let error = match admit_retained_diagnostics(&export, "retained-admission-test") {
        Ok(_) => panic!("a malformed producer result was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
    assert!(!error.code().contains("future-private-profile"));
}

#[tokio::test]
async fn identity_response_with_an_unexpected_length_is_never_admitted() {
    let mut export = identity_export().await;
    export.report.items[0].bytes.push(0x5a);
    export.report.items[0].received_bytes += 1;
    assert_eq!(
        device_diagnostics::bundle_export(&export).unwrap_err(),
        "diagnostics-invalid-coverage"
    );
    let error = match admit_retained_diagnostics(&export, "retained-admission-test") {
        Ok(_) => panic!("an identity response outside the closed shape was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
}

#[tokio::test]
async fn writer_valid_partial_identity_payload_is_not_credential_free() {
    let mut export = identity_export().await;
    let item = &mut export.report.items[0];
    item.status = "partial".into();
    item.bytes = b"synthetic-private-response".to_vec();
    item.received_bytes = item.bytes.len();
    item.returned_elements = None;
    item.error = Some("unexpected-length".into());
    for item in &mut export.report.items[1..] {
        item.status = "not-read".into();
        item.received_bytes = 0;
        item.returned_elements = None;
        item.bytes.clear();
        item.attempted_at = None;
        item.completed_at = None;
        item.error = None;
    }
    export.report.stop_reason = Some("descriptor-unverified".into());
    let archive = device_diagnostics::bundle_export(&export)
        .expect("the partial witness must satisfy the real writer grammar");
    assert_eq!(device_diagnostics::read_export(&archive).unwrap(), export);
    let error = match admit_retained_diagnostics(&export, "retained-admission-test") {
        Ok(_) => panic!("an unclassified partial response was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
}

#[tokio::test]
async fn writer_valid_format_one_memory_reason_is_not_a_safe_free_text_channel() {
    let mut export = identity_export().await;
    export.report.memory_reason = "synthetic private diagnostic note".into();
    let archive = device_diagnostics::bundle_export(&export)
        .expect("the reason witness must satisfy the real format-1 writer grammar");
    assert_eq!(device_diagnostics::read_export(&archive).unwrap(), export);
    let error = match admit_retained_diagnostics(&export, "retained-admission-test") {
        Ok(_) => panic!("an unclassified diagnostic reason was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
}

#[tokio::test]
async fn identity_build_timestamp_and_error_fields_need_the_producer_grammar() {
    let export = identity_export().await;
    let mut invalid_build = export.clone();
    invalid_build.report.build = "private build description".into();
    let mut invalid_timestamp = export.clone();
    invalid_timestamp.report.collected_at = "local sometime".into();
    let mut invalid_error = export;
    invalid_error.report.items[0].error = Some("private transport detail".into());
    for invalid in [invalid_build, invalid_timestamp, invalid_error] {
        assert!(device_diagnostics::bundle_export(&invalid).is_err());
        let error = match admit_retained_diagnostics(&invalid, "retained-admission-test") {
            Ok(_) => panic!("an unclassified diagnostic field was admitted"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "retained-source-unavailable");
    }
}

#[test]
fn unknown_support_kind_is_refused_even_if_the_legacy_writer_can_serialize_it() {
    let mut report = build_report(None, None, None, &ReportInput::default(), "test");
    report.kind = "future-private-support";
    assert!(report.bundle().is_ok());
    let error = match admit_retained_support(&report, "test") {
        Ok(_) => panic!("an unknown support kind was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
    assert!(!error.code().contains("future-private-support"));
}

#[test]
fn credential_shaped_manual_firmware_is_not_certified_by_token_syntax() {
    let mut report = build_report(None, None, None, &ReportInput::default(), "test");
    let firmware = report
        .facts
        .iter_mut()
        .find(|fact| fact.field == "firmware")
        .unwrap();
    firmware.status = "observed".into();
    firmware.value = Some("CredentialToken7".into());
    assert!(report.bundle().is_ok());
    let error = match admit_retained_support(&report, "test") {
        Ok(_) => panic!("an unclassified manual support value was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
}

#[test]
fn support_written_claim_is_not_a_credential_free_report() {
    let mut report = build_report(None, None, None, &ReportInput::default(), "test");
    report.written = true;
    assert!(report.bundle().is_ok());
    let error = match admit_retained_support(&report, "test") {
        Ok(_) => panic!("a support report with a write claim was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
}

#[test]
fn duplicate_support_fact_identity_is_not_guessed() {
    let mut report = build_report(None, None, None, &ReportInput::default(), "test");
    report.facts.push(report.facts[0].clone());
    assert!(report.bundle().is_ok());
    let error = match admit_retained_support(&report, "test") {
        Ok(_) => panic!("a duplicate support fact was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "retained-source-unavailable");
}
