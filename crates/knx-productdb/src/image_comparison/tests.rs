//! Discriminating table vectors with unequal addresses, associations and raw bytes.

use super::*;
use crate::code::{ProgramCode, TablePlacement};
use crate::image::SegmentImage;
use knx_core::commissioning::device_backup::{BackupRegion, PriorApplication};
use knx_core::commissioning::load_state::MaskVersion;
use knx_core::commissioning::memory_download::MemoryDownloadStep;

fn fixture() -> (DownloadImage, DeviceBackup, MemoryDownloadPlan) {
    let placement = |id: &str| {
        Some(TablePlacement {
            code_segment: Some(id.into()),
            offset: Some(1),
            max_entries: Some(4),
        })
    };
    let image = DownloadImage {
        code: ProgramCode {
            address_table: placement("A"),
            association_table: placement("B"),
            ..Default::default()
        },
        segments: vec![
            SegmentImage {
                id: "A".into(),
                address: 0x4000,
                octets: vec![0xEE, 3, 0x11, 0x43, 0x12, 0x03, 0x29, 0x05],
                mask: None,
            },
            SegmentImage {
                id: "B".into(),
                address: 0x4100,
                octets: vec![0xDD, 2, 1, 7, 2, 12],
                mask: None,
            },
        ],
        parameters: Default::default(),
        texts: Default::default(),
        objects: vec![],
        inferences: vec![],
        parameter_fields: vec![],
        table_writes: vec![],
    };
    let held = DeviceBackup {
        target: "1.1.67".parse().unwrap(),
        mask: MaskVersion(0x0701),
        manufacturer: 0x83,
        load_states: vec![],
        regions: vec![
            BackupRegion {
                address: 0x4000,
                octets: vec![0xBE, 3, 0x11, 0x44, 0x12, 0x04, 0x29, 0x06],
            },
            BackupRegion {
                address: 0x4100,
                octets: vec![0xEF, 2, 2, 9, 1, 15],
            },
        ],
        prior_application: PriorApplication::NotRecorded,
    };
    let plan = MemoryDownloadPlan {
        mask: MaskVersion(0x0701),
        manufacturer: 0x83,
        steps: image
            .segments
            .iter()
            .map(|s| MemoryDownloadStep::WriteMemory {
                address: s.address as u16,
                octets: s.octets.clone(),
            })
            .collect(),
    };
    (image, held, plan)
}

#[test]
fn numeric_parameter_rows_use_captured_bits_and_never_swap_project_and_device_values() {
    use crate::image::{AttributedValue, ParameterAttribution};
    use knx_core::commissioning::parameter_image::ParameterField;
    let (mut image, mut held, mut plan) = fixture();
    image.segments.push(SegmentImage {
        id: "P".into(),
        address: 0x4400,
        octets: vec![0xAB, 0xAC],
        mask: None,
    });
    let field = ParameterField {
        offset: 0,
        bit_offset: 5,
        size_in_bit: 9,
    };
    image.parameter_fields.push(ParameterAttribution {
        reference: "P-Delay_R-1".into(),
        name: "Delay".into(),
        segment: "P".into(),
        field,
        value: AttributedValue::Number {
            value: 235,
            nonnegative_signed: false,
        },
    });
    plan.steps.push(MemoryDownloadStep::WriteMemory {
        address: 0x4400,
        octets: vec![0xAB, 0xAC],
    });
    held.regions.push(BackupRegion {
        address: 0x4400,
        octets: vec![0xFD, 0x64],
    });
    let mut unknown = held.clone();
    unknown.mask = MaskVersion(0x0025);
    assert!(
        decode(&image, &unknown, &plan)
            .iter()
            .all(|r| r.kind == "raw"),
        "unknown profile never borrows parameter layout"
    );
    let rows = decode(&image, &held, &plan);
    let parameter = rows
        .iter()
        .find(|r| r.kind == "parameter")
        .expect("captured parameter row");
    assert_eq!(parameter.label, "Delay · P-Delay_R-1 · bit 5 / 9");
    assert_eq!(parameter.address, 0x4400);
    assert_eq!(parameter.project.as_deref(), Some("235"));
    assert_eq!(parameter.device.as_deref(), Some("345"));
    assert_eq!(parameter.reason, None);
    assert_eq!(rows.iter().filter(|r| r.kind == "raw").count(), 3);
    held.regions.last_mut().unwrap().octets.pop();
    let rows = decode(&image, &held, &plan);
    let parameter = rows.iter().find(|r| r.kind == "parameter").unwrap();
    assert_eq!(parameter.device, None);
    assert!(parameter.reason.is_some());
    // Later table/segment placement may overwrite a captured field: do not
    // claim those bytes still represent the original parameter.
    if let MemoryDownloadStep::WriteMemory { octets, .. } = plan.steps.last_mut().unwrap() {
        octets[0] = 0;
    }
    let rows = decode(&image, &held, &plan);
    assert_eq!(
        rows.iter().find(|r| r.kind == "parameter").unwrap().project,
        None
    );
}

#[test]
fn tables_use_placements_big_endian_and_each_sides_own_tsap() {
    let (image, held, plan) = fixture();
    let rows = decode(&image, &held, &plan);
    let find = |kind, label| {
        rows.iter()
            .find(|r| r.kind == kind && r.label == label)
            .unwrap()
    };
    let ia = find("individualAddress", "");
    assert_eq!(ia.address, 0x4002);
    assert_eq!(ia.project.as_deref(), Some("1.1.67"));
    assert_eq!(ia.device.as_deref(), Some("1.1.68"));
    let ga = find("groupAddress", "1");
    assert_eq!(ga.project.as_deref(), Some("2/2/3"));
    assert_eq!(ga.device.as_deref(), Some("2/2/4"));
    let association = find("association", "0");
    assert_eq!(
        association.project.as_deref(),
        Some("TSAP 1 → 2/2/3 · object 7")
    );
    assert_eq!(
        association.device.as_deref(),
        Some("TSAP 2 → 5/1/6 · object 9")
    );
    // Typed attribution carries the same facts; the caption is never parsed.
    let entry = |tsap, object, raw| {
        Some(AssociationSlot::Entry(AssociationEntry {
            tsap,
            object,
            group_address: knx_core::GroupAddress::from_raw(raw),
        }))
    };
    assert_eq!(
        association.association,
        Some(AssociationAttribution {
            project: entry(1, 7, 0x1203),
            device: entry(2, 9, 0x2906),
        })
    );
    assert!(rows
        .iter()
        .filter(|r| r.kind != "association")
        .all(|r| r.association.is_none()));
    let raw: Vec<_> = rows.iter().filter(|r| r.kind == "raw").collect();
    assert_eq!(
        raw.len(),
        2,
        "even bytes outside declared tables stay visible"
    );
    assert_eq!(raw[0].project.as_deref(), Some("EE 03 11 43 12 03 29 05"));
    assert_eq!(raw[0].device.as_deref(), Some("BE 03 11 44 12 04 29 06"));
}

#[test]
fn partial_and_malformed_table_data_are_unknown_not_empty_or_fabricated() {
    let (image, mut held, plan) = fixture();
    held.regions[0].octets.truncate(4);
    held.regions[1].octets[2] = 0;
    let rows = decode(&image, &held, &plan);
    let ga = rows
        .iter()
        .find(|r| r.kind == "groupAddress" && r.label == "1")
        .unwrap();
    assert_eq!(ga.device, None);
    assert!(ga.reason.is_some());
    let association = rows
        .iter()
        .find(|r| r.kind == "association" && r.label == "0")
        .unwrap();
    assert!(
        association.reason.is_some(),
        "TSAP zero is not a group address"
    );
    let attribution = association.association.unwrap();
    assert!(attribution.project.is_some());
    assert_eq!(attribution.device, None, "unknown, not beyond or empty");
    assert!(rows
        .iter()
        .any(|r| r.kind == "raw" && r.device.as_deref() == Some("BE 03 11 44")));
}

#[test]
fn corrupt_counts_never_attribute_adjacent_memory_as_table_entries() {
    let (image, mut held, plan) = fixture();
    held.regions[0].octets[1] = 255;
    held.regions[1].octets[1] = 255;
    held.regions.push(BackupRegion {
        address: 0x4008,
        octets: vec![0x22; 512],
    });
    let rows = decode(&image, &held, &plan);
    assert!(rows.iter().filter(|r| r.kind == "groupAddress").count() <= 4);
    assert!(rows.iter().filter(|r| r.kind == "association").count() <= 4);
    assert!(rows
        .iter()
        .filter(|r| r.kind == "groupAddress")
        .all(|r| r.device.is_none()));
    assert!(rows
        .iter()
        .find(|r| r.kind == "addressCount")
        .unwrap()
        .reason
        .is_some());
    assert_eq!(rows.iter().filter(|r| r.kind == "raw").count(), 3);
}

#[test]
fn unverified_mask_decodes_no_tables_but_keeps_all_raw_bytes() {
    let (image, mut held, plan) = fixture();
    held.mask = MaskVersion(0x0025);
    let rows = decode(&image, &held, &plan);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.kind == "raw"));
}

#[test]
fn a_slot_past_one_sides_count_is_beyond_not_unknown() {
    let (image, mut held, plan) = fixture();
    held.regions[1].octets[1] = 1;
    let rows = decode(&image, &held, &plan);
    let second = rows
        .iter()
        .find(|r| r.kind == "association" && r.label == "1")
        .unwrap();
    assert_eq!(second.device, None, "display stays unchanged");
    let attribution = second.association.unwrap();
    assert_eq!(attribution.device, Some(AssociationSlot::Beyond));
    assert!(matches!(
        attribution.project,
        Some(AssociationSlot::Entry(_))
    ));
}
