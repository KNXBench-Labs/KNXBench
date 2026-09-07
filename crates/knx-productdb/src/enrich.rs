//! Filling the `Program` and `ProgramRef` layers of an already-mapped
//! project from the product database (ADR-0012).
//!
//! Two rules, both load-bearing:
//!
//! 1. Only `Override::Absent` slots are filled. `Empty`, `Malformed` and
//!    `Value(Instance)` are what the project file actually said, and the
//!    exporter reproduces them; overwriting one would change the file.
//! 2. Nothing is guessed. A datapoint type stated as a list of
//!    alternatives (RESEARCH §4.2) fills nothing and is reported.
//!
//! Values written here carry `Layer::Program` or `Layer::ProgramRef`,
//! neither of which `Layer::is_exported()` accepts, so enrichment can
//! never leak into an export.

use knx_core::{
    ComObjectInstanceId, DeviceId, DptRef, Layer, ObjectSize, Override, Project, Resolved, Text,
};
use rusqlite::{Connection, OptionalExtension};

use crate::query::{com_object_view, resolve_program, ComObjectView, ValueLayer};
use crate::ProductDbError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnrichmentIssue {
    ProgramMissing {
        device_ets_id: String,
        program_ref: String,
    },
    ComObjectRefMissing {
        device_ets_id: String,
        ref_id: String,
    },
    AmbiguousDpt {
        ref_id: String,
        alternatives: Vec<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichmentReport {
    /// False when the database holds no application program at all — the
    /// "product database missing" state ADR-0005 requires to be ordinary,
    /// not an error path.
    pub available: bool,
    pub devices_resolved: usize,
    pub com_objects_enriched: usize,
    pub issues: Vec<EnrichmentIssue>,
}

fn has_any_program(conn: &Connection) -> Result<bool, ProductDbError> {
    let found: Option<i64> = conn
        .query_row("SELECT 1 FROM application_program LIMIT 1", [], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(found.is_some())
}

/// One device's identity, program reference and communication-object refs,
/// gathered before the mutable enrichment pass below.
type DeviceSnapshot = (DeviceId, String, String, Vec<(ComObjectInstanceId, String)>);

pub fn enrich(
    project: &mut Project,
    conn: &Connection,
) -> Result<EnrichmentReport, ProductDbError> {
    let mut report = EnrichmentReport {
        available: has_any_program(conn)?,
        ..EnrichmentReport::default()
    };
    if !report.available {
        return Ok(report);
    }

    // Collected first, so the mutable pass below borrows nothing else.
    let devices: Vec<DeviceSnapshot> = project
        .devices
        .iter()
        .map(|d| {
            let coms = d
                .com_objects
                .iter()
                .filter_map(|&id| {
                    project
                        .devices
                        .com_object(id)
                        .map(|c| (id, c.source.ets_id.clone()))
                })
                .collect();
            (d.id, d.source.ets_id.clone(), d.program_ref.clone(), coms)
        })
        .collect();

    for (_device, device_ets_id, program_ref, coms) in devices {
        let Some(program_id) = resolve_program(conn, &program_ref)? else {
            report.issues.push(EnrichmentIssue::ProgramMissing {
                device_ets_id,
                program_ref,
            });
            continue;
        };
        report.devices_resolved += 1;
        for (com_id, ref_id) in coms {
            let Some(view) = com_object_view(conn, &program_id, &ref_id)? else {
                report.issues.push(EnrichmentIssue::ComObjectRefMissing {
                    device_ets_id: device_ets_id.clone(),
                    ref_id,
                });
                continue;
            };
            if apply(project, com_id, &ref_id, &view, &mut report.issues) {
                report.com_objects_enriched += 1;
            }
        }
    }
    Ok(report)
}

/// Writes `value` only into an `Override::Absent` slot. Every other state
/// — `Empty`, `Malformed`, `Value` — is what the project file said, and
/// the exporter reproduces it; overwriting one would change the file.
fn fill_absent<T>(slot: &mut Override<T>, value: T, layer: Layer) -> bool {
    if matches!(slot, Override::Absent) {
        *slot = Override::Value(Resolved { value, layer });
        return true;
    }
    false
}

fn layer_of(layer: ValueLayer) -> Layer {
    match layer {
        ValueLayer::Program => Layer::Program,
        ValueLayer::ProgramRef => Layer::ProgramRef,
    }
}

fn apply(
    project: &mut Project,
    com_id: ComObjectInstanceId,
    ref_id: &str,
    view: &ComObjectView,
    issues: &mut Vec<EnrichmentIssue>,
) -> bool {
    // The datapoint type is decided before the mutable borrow, because
    // refusing an ambiguous list is a report entry, not a write.
    let dpt = match view.dpt_list.as_deref() {
        None => None,
        Some(list) => {
            let alternatives: Vec<&str> = list.split_whitespace().collect();
            match alternatives.as_slice() {
                [] => None,
                [one] => match DptRef::parse(one) {
                    Ok(dpt) => Some(dpt),
                    Err(_) => {
                        issues.push(EnrichmentIssue::AmbiguousDpt {
                            ref_id: ref_id.to_string(),
                            alternatives: vec![(*one).to_string()],
                        });
                        None
                    }
                },
                many => {
                    // RESEARCH §4.2: a list of acceptable alternatives.
                    // Which one applies cannot be decided from one sample,
                    // and guessing would be invented compatibility.
                    issues.push(EnrichmentIssue::AmbiguousDpt {
                        ref_id: ref_id.to_string(),
                        alternatives: many.iter().map(|s| (*s).to_string()).collect(),
                    });
                    None
                }
            }
        }
    };

    let text = view
        .text
        .as_ref()
        .map(|t| (Text::Literal(t.clone()), layer_of(view.text_layer)));
    let description = view
        .visible_description
        .as_ref()
        .map(|t| (Text::Literal(t.clone()), layer_of(view.description_layer)));
    let size = view.object_size.as_deref().and_then(parse_object_size);

    let Some(com) = project.devices.com_object_mut(com_id) else {
        return false;
    };
    let mut changed = false;
    if let Some((value, layer)) = text {
        changed |= fill_absent(&mut com.text, value, layer);
    }
    if let Some((value, layer)) = description {
        changed |= fill_absent(&mut com.description, value, layer);
    }
    if let Some(dpt) = dpt {
        changed |= fill_absent(&mut com.dpt, dpt, layer_of(view.dpt_layer));
    }
    for (slot, stated, layer) in [
        (&mut com.flags.read, view.read.as_deref(), view.read_layer),
        (
            &mut com.flags.write,
            view.write.as_deref(),
            view.write_layer,
        ),
        (
            &mut com.flags.transmit,
            view.transmit.as_deref(),
            view.transmit_layer,
        ),
        (
            &mut com.flags.update,
            view.update.as_deref(),
            view.update_layer,
        ),
        (
            &mut com.flags.communication,
            view.communication.as_deref(),
            view.communication_layer,
        ),
    ] {
        // "Enabled"/"Disabled" are the only two values the source uses;
        // anything else stays absent rather than becoming a guessed false.
        let flag = match stated {
            Some("Enabled") => Some(true),
            Some("Disabled") => Some(false),
            _ => None,
        };
        if let Some(flag) = flag {
            changed |= fill_absent(slot, flag, layer_of(layer));
        }
    }
    if com.size.is_none() {
        if let Some((value, layer)) = size.map(|s| (s, layer_of(view.object_size_layer))) {
            com.size = Some(Resolved { value, layer });
            changed = true;
        }
    }
    changed
}

/// `"1 Bit"` -> `Bit(1)`, `"14 Bytes"` -> `Byte(14)`. An unrecognized
/// spelling yields `None`; the size stays unknown rather than wrong.
fn parse_object_size(text: &str) -> Option<ObjectSize> {
    let (number, unit) = text.split_once(' ')?;
    let n: u16 = number.parse().ok()?;
    match unit {
        "Bit" | "Bits" => u8::try_from(n).ok().map(ObjectSize::Bit),
        "Byte" | "Bytes" => Some(ObjectSize::Byte(n)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::{hardware::ingest_hardware, program::ingest_program};
    use knx_core::{
        ComObjectInstance, DeviceInstance, Language, Layer, Override, Project, ResolvedFlags,
        SourceRef,
    };

    // Same fixtures as Task 10, plus a ref whose DatapointType is a list.
    const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="22"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" />
  <ComObject Id="A-1_O-2" Number="2" Text="Wert" ObjectSize="2 Bytes" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
  <ComObjectRef Id="A-1_O-2_R-1" RefId="A-1_O-2" DatapointType="DPST-9-21 DPST-9-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_hardware(&conn, "sha-h", "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap();
        ingest_program(&conn, "sha-p", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        (dir, conn)
    }

    fn source(ets_id: &str) -> SourceRef {
        SourceRef {
            path: "P-0001/0.xml".into(),
            ets_id: ets_id.into(),
        }
    }

    /// One device with one communication object whose `text`, `dpt` and
    /// `flags` are all `Absent` at instance level.
    fn project_with(com_ref_id: &str, dpt: Override<knx_core::DptRef>) -> Project {
        let mut p = Project::new(Language("de-DE".into()));
        let device_id = p.ids.next_device_id();
        let com_id = p.ids.next_com_object_instance_id();
        p.devices.insert(DeviceInstance {
            id: device_id,
            source: source("P-0001-0_DI-1"),
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "H-1_P-1".into(),
            program_ref: "H-1_HP-1".into(),
            commissioning: Default::default(),
            visibility_calculated: false,
            com_objects: vec![com_id],
            binary_data: vec![],
        });
        p.devices.insert_com_object(ComObjectInstance {
            id: com_id,
            source: source(com_ref_id),
            device: device_id,
            number: 1,
            text: Override::Absent,
            description: Override::Absent,
            dpt,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        p
    }

    #[test]
    fn an_absent_text_is_filled_at_the_program_layer() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        let report = enrich(&mut p, &conn).unwrap();
        assert!(report.available);
        assert_eq!(report.com_objects_enriched, 1);
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        match &com.text {
            Override::Value(r) => {
                assert_eq!(r.layer, Layer::Program);
                assert_eq!(
                    p.strings.text(&r.value, p.strings.default_language()),
                    Some("Schalten")
                );
            }
            other => panic!("expected a program-layer text, got {other:?}"),
        }
    }

    #[test]
    fn an_absent_datapoint_type_is_filled_and_a_size_is_set() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        enrich(&mut p, &conn).unwrap();
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        match &com.dpt {
            Override::Value(r) => {
                assert_eq!(r.layer, Layer::Program);
                assert_eq!(
                    r.value,
                    knx_core::DptRef {
                        main: 1,
                        sub: Some(1)
                    }
                );
            }
            other => panic!("expected a program-layer dpt, got {other:?}"),
        }
        assert_eq!(
            com.size.map(|s| s.value),
            Some(knx_core::ObjectSize::Bit(1))
        );
    }

    #[test]
    fn an_empty_instance_attribute_is_never_overwritten() {
        // 497 of the reference project's communication objects carry
        // DatapointType="". Replacing that with a program value would make
        // the exporter write the attribute as absent instead of empty,
        // changing the file (ADR-0012).
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Empty);
        enrich(&mut p, &conn).unwrap();
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        assert_eq!(com.dpt, Override::Empty);
    }

    #[test]
    fn an_instance_value_is_never_overwritten() {
        let (_dir, conn) = db();
        let instance = Override::Value(knx_core::Resolved {
            value: knx_core::DptRef {
                main: 5,
                sub: Some(1),
            },
            layer: Layer::Instance,
        });
        let mut p = project_with("A-1_O-1_R-1", instance.clone());
        enrich(&mut p, &conn).unwrap();
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        assert_eq!(com.dpt, instance);
    }

    #[test]
    fn a_datapoint_type_list_fills_nothing_and_is_reported() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-2_R-1", Override::Absent);
        let report = enrich(&mut p, &conn).unwrap();
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        assert_eq!(
            com.dpt,
            Override::Absent,
            "an ambiguous list is never guessed"
        );
        assert!(report.issues.iter().any(|i| matches!(
            i,
            EnrichmentIssue::AmbiguousDpt { alternatives, .. } if alternatives.len() == 2
        )));
    }

    #[test]
    fn a_missing_program_is_reported_per_device_and_leaves_the_model_alone() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        p.devices
            .get_mut(knx_core::DeviceId(1))
            .unwrap()
            .program_ref = "H-9_HP-9".into();
        let report = enrich(&mut p, &conn).unwrap();
        assert_eq!(report.com_objects_enriched, 0);
        assert!(matches!(
            report.issues.as_slice(),
            [EnrichmentIssue::ProgramMissing { program_ref, .. }] if program_ref == "H-9_HP-9"
        ));
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        assert_eq!(com.text, Override::Absent);
    }

    #[test]
    fn an_empty_database_reports_unavailable_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("empty.sqlite")).unwrap();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        let report = enrich(&mut p, &conn).unwrap();
        assert!(!report.available);
        assert_eq!(report.com_objects_enriched, 0);
    }
}
