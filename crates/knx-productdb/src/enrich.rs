//! Filling the `Program` and `ProgramRef` layers of an already-mapped
//! project from the product database (ADR-0012).
//!
//! Two rules, both load-bearing:
//!
//! 1. Only `Override::Absent` slots are filled, for `text`/`description`/
//!    `dpt`. `Malformed` and `Value(Instance)` are what the project file
//!    actually said, and the exporter reproduces them; overwriting one
//!    would change the file. `Empty` is also never written into — but a
//!    resolvable program value behind an `Empty` slot is no longer thrown
//!    away either: it is lifted into `Devices::program_defaults`, a side
//!    table the exporter never reads (ADR-0012 gap 2, ADR-0027,
//!    KNOWN_LIMITATIONS §12).
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
            let is_module_based = project
                .devices
                .com_object(com_id)
                .and_then(|c| c.module_instance)
                .is_some();
            let lookup_id = com_object_lookup_id(&program_id, &ref_id, is_module_based);
            let Some(view) = com_object_view(conn, &program_id, &lookup_id, None)? else {
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

/// Reconstructs a module-based communication object's fully-qualified
/// `ComObjectRef` id from its device-level `RefId` and the resolved
/// application-program id — the productdb-local twin of
/// `knx-etsproj::values::module_com_object_ref`, duplicated rather than
/// shared across the crate boundary (see this task's rationale: avoiding
/// a new knx-productdb → knx-etsproj dependency edge for ~10 lines).
fn module_ref_id(program_id: &str, device_ref_id: &str) -> Option<String> {
    let rest = device_ref_id.strip_prefix("MD-")?;
    let (md_digits, rest) = rest.split_once('_')?;
    let mut parts = rest.splitn(3, '_');
    parts.next()?; // M-<m>
    parts.next()?; // MI-<k>
    let tail = parts.next()?; // O-<a>-<b>_R-<c>
    Some(format!("{program_id}_MD-{md_digits}_{tail}"))
}

/// The `ComObjectRef` id to look up for one device-level `RefId`: the
/// module-reconstructed id when `module_based`, the `RefId` itself
/// otherwise, with `module_ref_id`'s own `unwrap_or_else` fallback to the
/// raw `RefId` when reconstruction fails — `enrich()` and any other caller
/// wanting the exact same id share this one implementation instead of each
/// keeping their own copy of the `if`.
pub fn com_object_lookup_id(program_id: &str, ref_id: &str, module_based: bool) -> String {
    if module_based {
        module_ref_id(program_id, ref_id).unwrap_or_else(|| ref_id.to_string())
    } else {
        ref_id.to_string()
    }
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

/// Fills every `Override::Absent` slot on the communication object
/// instance named by `com_id` from `view`, exactly as `enrich()`'s own
/// per-project loop does for each device it resolves. Exposed as `pub` so
/// a caller seeding a single newly created communication object (device
/// creation, `apps/knx-server::domain::create_device_impl`) can reuse this
/// mapping directly instead of duplicating it.
pub fn apply(
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
    // `defaults` collects a program value for a slot the instance itself
    // left `Empty` (ADR-0012 gap 2, ADR-0027) — a fact `fill_absent` never
    // sees, since it only ever writes into `Absent`. `Devices` keeps this
    // beside `com`, not inside its `Override<T>` fields: an `Empty` slot's
    // own state must never read as if it had a value.
    let mut defaults = knx_core::ProgramDefaults::default();
    if let Some((value, layer)) = text {
        match com.text {
            Override::Absent => {
                com.text = Override::Value(Resolved { value, layer });
                changed = true;
            }
            Override::Empty => defaults.text = Some(Resolved { value, layer }),
            Override::Value(_) | Override::Malformed(_) => {}
        }
    }
    if let Some((value, layer)) = description {
        match com.description {
            Override::Absent => {
                com.description = Override::Value(Resolved { value, layer });
                changed = true;
            }
            Override::Empty => defaults.description = Some(Resolved { value, layer }),
            Override::Value(_) | Override::Malformed(_) => {}
        }
    }
    if let Some(dpt) = dpt {
        let layer = layer_of(view.dpt_layer);
        match com.dpt {
            Override::Absent => {
                com.dpt = Override::Value(Resolved { value: dpt, layer });
                changed = true;
            }
            Override::Empty => defaults.dpt = Some(Resolved { value: dpt, layer }),
            Override::Value(_) | Override::Malformed(_) => {}
        }
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
        (
            &mut com.flags.read_on_init,
            view.read_on_init.as_deref(),
            view.read_on_init_layer,
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
    // `com`'s mutable borrow of `project.devices` ends here (last use above);
    // `set_program_defaults` needs its own borrow of the same map, which is
    // why this is not folded into the match arms above.
    if !defaults.is_empty() {
        changed = true;
        project.devices.set_program_defaults(com_id, defaults);
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
             DatapointType="DPST-1-1" WriteFlag="Enabled" ReadOnInitFlag="Enabled" />
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

    /// §117: the sixth flag now crosses the crate boundary instead of
    /// being parsed, stored and quietly forgotten.
    #[test]
    fn an_absent_read_on_init_flag_is_filled_at_the_program_layer() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        enrich(&mut p, &conn).unwrap();
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        let resolved = com.flags.read_on_init.value().expect("read_on_init filled");
        assert!(resolved.value);
        assert_eq!(resolved.layer, Layer::Program);
        // The program states no `CommunicationFlag`, so the fifth stays
        // absent — "not stated" never becomes "stated false".
        assert!(!com.flags.communication.is_present());
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
    fn apply_can_be_called_directly_without_going_through_enrich() {
        let (_dir, conn) = db();
        let view = com_object_view(&conn, "A-1", "A-1_O-1_R-1", None)
            .unwrap()
            .unwrap();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        let mut issues = Vec::new();
        let changed = apply(
            &mut p,
            knx_core::ComObjectInstanceId(1),
            "A-1_O-1_R-1",
            &view,
            &mut issues,
        );
        assert!(changed);
        assert!(issues.is_empty());
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        assert!(com.dpt.value().is_some());
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

    /// The other half of the same fact: an `Empty` slot is untouched, but a
    /// resolvable program value behind it is no longer thrown away either
    /// (ADR-0012 gap 2, ADR-0027, KNOWN_LIMITATIONS §12) — it lands in
    /// `Devices::program_defaults`, next to `com`, not inside it.
    #[test]
    fn an_empty_dpt_lifts_the_program_value_into_program_defaults_instead_of_the_slot() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Empty);
        let report = enrich(&mut p, &conn).unwrap();
        assert_eq!(
            report.com_objects_enriched, 1,
            "lifting still counts as enrichment"
        );
        let com_id = knx_core::ComObjectInstanceId(1);
        let com = p.devices.com_object(com_id).unwrap();
        assert_eq!(
            com.dpt,
            Override::Empty,
            "the instance slot itself is untouched"
        );
        let defaults = p
            .devices
            .program_defaults(com_id)
            .expect("a program value was available to lift");
        assert_eq!(
            defaults.dpt,
            Some(knx_core::Resolved {
                value: knx_core::DptRef {
                    main: 1,
                    sub: Some(1)
                },
                layer: Layer::Program,
            })
        );
        // The program states no description for A-1_O-1, so that field
        // stays unset even though dpt lifted.
        assert!(defaults.description.is_none());
    }

    #[test]
    fn an_empty_description_lifts_the_program_value_into_program_defaults() {
        let (_dir, conn) = db();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        p.devices
            .com_object_mut(knx_core::ComObjectInstanceId(1))
            .unwrap()
            .description = Override::Empty;
        enrich(&mut p, &conn).unwrap();
        let com_id = knx_core::ComObjectInstanceId(1);
        let com = p.devices.com_object(com_id).unwrap();
        assert_eq!(com.description, Override::Empty);
        // A-1_O-1's ComObject/ComObjectRef state no VisibleDescription in
        // this fixture, so there is nothing to lift — the entry stays
        // absent rather than acquiring an empty placeholder.
        assert!(p.devices.program_defaults(com_id).is_none());
    }

    /// A slot already `Value(Instance)` is untouched, and nothing is lifted
    /// either — the same non-overwrite rule as `an_instance_value_is_never_
    /// overwritten`, extended to the new side table.
    #[test]
    fn a_value_slot_gets_no_program_default_entry() {
        let (_dir, conn) = db();
        let instance = Override::Value(knx_core::Resolved {
            value: knx_core::DptRef {
                main: 5,
                sub: Some(1),
            },
            layer: Layer::Instance,
        });
        let mut p = project_with("A-1_O-1_R-1", instance);
        enrich(&mut p, &conn).unwrap();
        assert!(p
            .devices
            .program_defaults(knx_core::ComObjectInstanceId(1))
            .is_none());
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

    // A second Hardware/Program pair, module-based (schema ≥21, ADR-0013) —
    // same XML shape Task 9 used to prove ingestion works, duplicated here
    // (not imported across the module.rs/program.rs test boundary — this
    // crate's tests don't share fixtures across files) plus a Hardware2Program
    // link so `resolve_program` can reach it the same way `db()`'s existing
    // fixture reaches `PROGRAM`.
    const MODULE_HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-00FA">
<Hardware><Hardware Id="H-2" Name="Y" SerialNumber="S2" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-2_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-00FA_A-2504-10-C071" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const MODULE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-00FA">
<ApplicationPrograms><ApplicationProgram Id="M-00FA_A-2504-10-C071" Name="P" ApplicationVersion="10" MaskVersion="MV-0701">
<Static><ComObjectTable/><ComObjectRefs/></Static>
<ModuleDefs><ModuleDef Id="M-00FA_A-2504-10-C071_MD-2" Name="module">
<Static>
<ComObjectTable>
  <ComObject Id="M-00FA_A-2504-10-C071_MD-2_O-2-0" Number="0" Text="OnOff" ObjectSize="1 Bit" DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="M-00FA_A-2504-10-C071_MD-2_O-2-0_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_O-2-0" />
</ComObjectRefs>
</Static>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db_with_module_program() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_hardware(
            &conn,
            "sha-h2",
            "M-00FA/Hardware.xml",
            MODULE_HARDWARE.as_bytes(),
        )
        .unwrap();
        ingest_program(&conn, "sha-mod", "M-00FA/A.xml", MODULE_PROGRAM.as_bytes()).unwrap();
        (dir, conn)
    }

    /// One device with one module-based communication object (ADR-0013):
    /// `module_instance: Some(..)`, `source.ets_id` the raw, unstripped
    /// device-level `RefId` `enrich()` must transform before lookup.
    fn project_with_module_com_object(ets_id: &str) -> Project {
        let mut p = Project::new(Language("de-DE".into()));
        let device_id = p.ids.next_device_id();
        let com_id = p.ids.next_com_object_instance_id();
        let module_id = p.ids.next_module_instance_id();
        p.devices.insert_module_instance(knx_core::ModuleInstance {
            id: module_id,
            device: device_id,
            source: source("MD-2_M-1"),
            repeat_index: "6x1".into(),
            instance_ets_id: "MD-2_M-1_MI-1".into(),
            arguments: vec![],
        });
        p.devices.insert(DeviceInstance {
            id: device_id,
            source: source("P-0001-0_DI-1"),
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "H-2_P-1".into(),
            program_ref: "H-2_HP-1".into(),
            commissioning: Default::default(),
            visibility_calculated: false,
            com_objects: vec![com_id],
            binary_data: vec![],
        });
        p.devices.insert_com_object(ComObjectInstance {
            id: com_id,
            source: source(ets_id),
            device: device_id,
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: Some(module_id),
        });
        p
    }

    #[test]
    fn a_module_based_com_object_is_enriched_through_the_module_hop() {
        let (_dir, conn) = db_with_module_program();
        let mut p = project_with_module_com_object("MD-2_M-1_MI-1_O-2-0_R-1");
        let report = enrich(&mut p, &conn).unwrap();
        assert_eq!(report.com_objects_enriched, 1);
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        match &com.text {
            Override::Value(r) => {
                assert_eq!(
                    p.strings.text(&r.value, p.strings.default_language()),
                    Some("OnOff")
                );
            }
            other => panic!("expected a program-layer text, got {other:?}"),
        }
    }
}
