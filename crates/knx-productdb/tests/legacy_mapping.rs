//! Legacy EX-IM program databases map onto the product-database model by the measured ETS rules.

use knx_productdb::legacy::{
    map_legacy_database, parse_exim, read_legacy_member, LegacyError, LegacyMapping,
    MappedDynamicNode, MappedProgram, MappingDiagnostic,
};
use knx_productdb::sha256_hex;

fn fixture(path: &str) -> Vec<u8> {
    let full = knx_testsupport::workspace_root()
        .join("crates/knx-productdb/fixtures/legacy")
        .join(path);
    std::fs::read(&full).unwrap_or_else(|e| panic!("fixture {full:?} unreadable: {e}"))
}

fn mapping() -> (LegacyMapping, String) {
    let bytes = fixture("marvin-program-plain.vd4");
    let payload = read_legacy_member(&bytes)
        .unwrap()
        .open_unencrypted()
        .unwrap();
    let document = parse_exim(payload.bytes()).unwrap();
    let sha = sha256_hex(payload.bytes());
    (map_legacy_database(&document, &sha).unwrap(), sha)
}

fn program() -> (MappedProgram, String) {
    let (mapping, sha) = mapping();
    assert_eq!(mapping.programs.len(), 1);
    let id = format!("M-1092_A-LX{}-300", sha[..8].to_ascii_uppercase());
    let program = mapping.programs.into_iter().next().unwrap();
    assert_eq!(program.id, id);
    (program, id)
}

#[test]
fn the_program_row_carries_the_measured_identity_fields() {
    let (program, _) = program();
    assert_eq!(program.exim_program_id, "300");
    assert_eq!(program.manufacturer_id, "M-1092");
    assert_eq!(program.name.as_deref(), Some("Improbability Drive"));
    // ETS: ApplicationNumber = DEVICE_TYPE, ApplicationVersion = PROGRAM_VERSION.
    assert_eq!(program.application_number.as_deref(), Some("7"));
    assert_eq!(program.application_version.as_deref(), Some("22"));
    assert_eq!(program.mask_version.as_deref(), Some("MV-0701"));
    assert_eq!(program.pei_type.as_deref(), Some("0"));
    assert_eq!(program.linkable, Some(false));
    assert_eq!(program.original_manufacturer.as_deref(), Some("M-1092"));
    assert_eq!(program.default_language.as_deref(), Some("de-DE"));
}

#[test]
fn manufacturers_use_the_hex_knx_number() {
    let (mapping, _) = mapping();
    let ids: Vec<_> = mapping
        .manufacturers
        .iter()
        .map(|m| (m.id.as_str(), m.name.as_deref()))
        .collect();
    assert_eq!(ids, [("M-1092", Some("Marvin Test"))]);
}

#[test]
fn parameter_types_follow_the_atomic_type() {
    let (program, id) = program();
    let kind = |n: &str| {
        let t = program
            .parameter_types
            .iter()
            .find(|t| t.id == format!("{id}_PT-{n}"))
            .unwrap_or_else(|| panic!("type {n}"));
        (
            t.kind.as_str(),
            t.size_in_bit,
            t.number_type.as_deref(),
            t.min_inclusive.as_deref(),
            t.max_inclusive.as_deref(),
            t.base.as_deref(),
        )
    };
    assert_eq!(kind("10"), ("None", None, None, None, None, None));
    assert_eq!(
        kind("11"),
        (
            "Number",
            Some(8),
            Some("unsignedInt"),
            Some("0"),
            Some("255"),
            None
        )
    );
    assert_eq!(
        kind("12"),
        (
            "Number",
            Some(8),
            Some("signedInt"),
            Some("-10"),
            Some("10"),
            None
        )
    );
    assert_eq!(
        kind("13"),
        ("Restriction", Some(1), None, None, None, Some("Value"))
    );
}

#[test]
fn enumerations_are_sorted_by_display_order_and_keyed_by_value() {
    let (program, id) = program();
    let values: Vec<_> = program
        .enumerations
        .iter()
        .filter(|e| e.parameter_type_id == format!("{id}_PT-14"))
        .map(|e| {
            (
                e.id.as_str().rsplit('_').next().unwrap().to_string(),
                e.value.as_str(),
                e.text.as_deref(),
                e.display_order,
            )
        })
        .collect();
    assert_eq!(
        values,
        [
            ("EN-0".to_string(), "0", Some("Mostly harmless"), 0),
            ("EN-2".to_string(), "2", Some("Panic"), 1),
            ("EN-1".to_string(), "1", Some("Towel"), 2),
        ]
    );
}

#[test]
fn one_memory_cell_is_one_parameter_with_overriding_refs() {
    let (program, id) = program();
    let level = program
        .parameters
        .iter()
        .find(|p| p.id == format!("{id}_P-1002"))
        .expect("group parameter takes the smallest number");
    assert!(program
        .parameters
        .iter()
        .all(|p| p.id != format!("{id}_P-1003")));
    assert_eq!(level.value.as_deref(), Some("42"));
    assert_eq!(level.access.as_deref(), Some("ReadWrite"));
    // Absolute addressing: one segment at 0, offset = address.
    assert_eq!(
        level.code_segment.as_deref(),
        Some(format!("{id}_AS-0000").as_str())
    );
    assert_eq!((level.offset, level.bit_offset), (Some(16641), Some(0)));
    let refs: Vec<_> = program
        .parameter_refs
        .iter()
        .filter(|r| r.parameter_id == level.id)
        .map(|r| {
            (
                r.id.clone(),
                r.tag.as_deref(),
                r.value.as_deref(),
                r.access.as_deref(),
                r.display_order,
                r.text.as_deref(),
            )
        })
        .collect();
    // A ref overrides only what differs from the shared parameter.
    assert_eq!(level.text.as_deref(), Some("Level"));
    assert_eq!(
        refs,
        [
            (
                format!("{id}_P-1002_R-1002"),
                Some("1002"),
                None,
                None,
                Some(20),
                None
            ),
            (
                format!("{id}_P-1002_R-1003"),
                Some("1003"),
                Some("7"),
                Some("None"),
                Some(30),
                Some("Level (off)")
            ),
        ]
    );
}

#[test]
fn different_types_at_one_address_form_a_union() {
    let (program, id) = program();
    let members: Vec<_> = program
        .parameters
        .iter()
        .filter(|p| p.union_id.is_some())
        .map(|p| (p.id.clone(), p.union_size_in_bit, p.offset, p.bit_offset))
        .collect();
    assert_eq!(
        members,
        [
            (format!("{id}_UP-1004"), Some(8), Some(16642), Some(0)),
            (format!("{id}_UP-1005"), Some(8), Some(16642), Some(0)),
        ]
    );
    let union_ids: std::collections::BTreeSet<_> = program
        .parameters
        .iter()
        .filter_map(|p| p.union_id)
        .collect();
    assert_eq!(union_ids.len(), 1);
}

#[test]
fn access_follows_the_high_access_level() {
    let (program, id) = program();
    let access = |n: &str| {
        program
            .parameters
            .iter()
            .find(|p| p.id == format!("{id}_P-{n}"))
            .and_then(|p| p.access.clone())
    };
    assert_eq!(access("1001").as_deref(), Some("ReadWrite"));
    assert_eq!(access("1010").as_deref(), Some("None"));
}

#[test]
fn communication_objects_group_by_number_with_ref_overrides() {
    let (program, id) = program();
    let objects: Vec<_> = program
        .com_objects
        .iter()
        .map(|o| {
            (
                o.id.clone(),
                o.number,
                o.object_size.as_deref(),
                o.priority.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        objects,
        [
            (format!("{id}_O-0"), Some(0), Some("1 Bit"), Some("Low")),
            (format!("{id}_O-1"), Some(1), Some("1 Bit"), Some("Low")),
        ]
    );
    let alarm = &program.com_objects[0];
    assert_eq!(alarm.name.as_deref(), Some("Alarm"));
    assert_eq!(alarm.function_text.as_deref(), Some("Send"));
    assert_eq!(
        (
            alarm.read_flag.as_deref(),
            alarm.write_flag.as_deref(),
            alarm.transmit_flag.as_deref(),
            alarm.communication_flag.as_deref()
        ),
        (
            Some("Disabled"),
            Some("Disabled"),
            Some("Enabled"),
            Some("Enabled")
        )
    );
    let refs: Vec<_> = program
        .com_object_refs
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                r.tag.as_deref(),
                r.function_text.as_deref(),
                r.object_size.as_deref(),
                r.read_flag.as_deref(),
                r.update_flag.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        refs,
        [
            (
                format!("{id}_O-0_R-10000"),
                Some("10000"),
                None,
                None,
                None,
                None
            ),
            (
                format!("{id}_O-1_R-10001"),
                Some("10001"),
                None,
                None,
                None,
                None
            ),
            // 16 bits: ETS's spelling is "2 Bytes", the file's own "2 Byte".
            (
                format!("{id}_O-1_R-10002"),
                Some("10002"),
                Some("Value"),
                Some("2 Bytes"),
                Some("Enabled"),
                Some("Enabled")
            ),
        ]
    );
}

fn render(nodes: &[MappedDynamicNode], id: &str) -> Vec<String> {
    let depth = |node: &MappedDynamicNode| {
        let (mut d, mut parent) = (0, node.parent_id);
        while let Some(p) = parent {
            parent = nodes.iter().find(|m| m.node_id == p).unwrap().parent_id;
            d += 1;
        }
        d
    };
    nodes
        .iter()
        .map(|n| {
            let mut s = format!("{}{}", "  ".repeat(depth(n)), n.kind);
            if let Some(r) = &n.ref_id {
                s.push(' ');
                s.push_str(r.strip_prefix(&format!("{id}_")).unwrap_or(r));
            }
            if let Some(t) = &n.test {
                s.push_str(&format!(" test={t}"));
            }
            if n.is_default {
                s.push_str(" default");
            }
            s
        })
        .collect()
}

#[test]
fn visibility_becomes_a_dynamic_tree_with_the_measured_shape() {
    let (program, id) = program();
    let tree = render(&program.dynamic, &id);
    let expected = [
        "Dynamic",
        "  Channel",
        "    ComObjectRefRef O-0_R-10000",
        "    ParameterBlock P-1000_R-1000",
        "      ParameterRefRef P-1001_R-1001",
        "      choose P-1001_R-1001",
        // Parameters by display order, then objects (separate number spaces).
        "        when test=0",
        "          ParameterRefRef P-1002_R-1003",
        "          ParameterRefRef UP-1005_R-1005",
        "          ComObjectRefRef O-1_R-10002",
        "        when test=1",
        "          ParameterRefRef P-1006_R-1006",
        "          ParameterRefRef P-1002_R-1002",
        "          ParameterRefRef UP-1004_R-1004",
        "          ComObjectRefRef O-1_R-10001",
        // A root that is not a page joins the first page, as in ETS.
        "      ParameterRefRef P-1010_R-1010",
        "      choose P-1010_R-1010",
        "        when test=1",
        "          ParameterRefRef P-1011_R-1011",
        "    ParameterBlock P-2000_R-2000",
        "      ParameterRefRef P-2001_R-2001",
    ];
    assert_eq!(tree, expected, "\n{}", tree.join("\n"));
    let block = program
        .dynamic
        .iter()
        .find(|n| n.kind == "ParameterBlock")
        .unwrap();
    assert_eq!(
        block.element_id.as_deref(),
        Some(format!("{id}_PB-1000").as_str())
    );
    assert_eq!(block.text.as_deref(), Some("General"));
}

#[test]
fn translations_use_the_measured_column_ids() {
    let (mapping, sha) = mapping();
    let id = format!("M-1092_A-LX{}-300", sha[..8].to_ascii_uppercase());
    let mut rows: Vec<_> = mapping
        .translations
        .iter()
        .map(|t| {
            (
                t.scope.as_str(),
                t.language.as_str(),
                t.ref_id.replace(&id, "APP"),
                t.attribute_name.as_str(),
                t.text.as_str(),
            )
        })
        .collect();
    rows.sort();
    let item = format!("M-1092_CI-LX{}-500", sha[..8].to_ascii_uppercase());
    assert_eq!(
        rows,
        [
            (
                "Catalog",
                "en-US",
                item,
                "Name",
                "Heart of Gold Sensor (en)"
            ),
            (
                "Program",
                "en-US",
                "APP_O-1".to_string(),
                "FunctionText",
                "Receive (en)"
            ),
            (
                "Program",
                "en-US",
                "APP_O-1".to_string(),
                "Text",
                "Level (en)"
            ),
            // 702 shares 701's name (no translation of its own) but
            // overrides the function, so that translation stays even though
            // it equals 701's.
            (
                "Program",
                "en-US",
                "APP_O-1_R-10002".to_string(),
                "FunctionText",
                "Receive (en)"
            ),
            (
                "Program",
                "en-US",
                "APP_P-1001".to_string(),
                "Text",
                "Mode (en)"
            ),
            (
                "Program",
                "en-US",
                "APP_P-1002".to_string(),
                "Text",
                "Level (en)"
            ),
            // 1003 overrides the text, so its translation stays even
            // though it equals the representative's.
            (
                "Program",
                "en-US",
                "APP_P-1002_R-1003".to_string(),
                "Text",
                "Level (en)"
            ),
            (
                "Program",
                "en-US",
                "APP_PT-13_EN-1".to_string(),
                "Text",
                "On (en)"
            ),
        ]
    );
    // COLUMN_ID 99 is not one of the measured meanings: kept out, reported.
    assert!(mapping.diagnostics.iter().any(|d| matches!(
        d,
        MappingDiagnostic::UnknownTextColumn { column_id, rows } if column_id == "99" && *rows == 1
    )));
}

#[test]
fn catalog_hierarchy_comes_from_functional_entities() {
    let (mapping, sha) = mapping();
    let ns = sha[..8].to_ascii_uppercase();
    let sections: Vec<_> = mapping
        .catalog_sections
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                s.parent_id.clone(),
                s.name.as_deref(),
                s.number.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        sections,
        [
            (
                format!("M-1092_CS-LX{ns}-50"),
                None,
                Some("Sensors"),
                Some("1")
            ),
            (
                format!("M-1092_CS-LX{ns}-51"),
                Some(format!("M-1092_CS-LX{ns}-50")),
                Some("Presence"),
                Some("1.1")
            ),
        ]
    );
    let item = &mapping.catalog_items[0];
    assert_eq!(mapping.catalog_items.len(), 1);
    assert_eq!(item.id, format!("M-1092_CI-LX{ns}-500"));
    assert_eq!(item.section_id, format!("M-1092_CS-LX{ns}-51"));
    assert_eq!(item.number.as_deref(), Some("MT-42"));
    assert_eq!(item.name.as_deref(), Some("Heart of Gold Sensor"));
    let hardware = format!("M-1092_H-LX{ns}-100");
    assert_eq!(
        item.product_ref_id.as_deref(),
        Some(format!("{hardware}_P-200").as_str())
    );
    assert_eq!(
        item.hardware2program_ref_id.as_deref(),
        Some(format!("{hardware}_HP-400").as_str())
    );
    assert_eq!(mapping.hardware[0].id, hardware);
    assert_eq!(mapping.products[0].hardware_id, hardware);
    assert_eq!(mapping.products[0].order_number.as_deref(), Some("MT-42"));
    let h2p = &mapping.hardware2programs[0];
    assert_eq!(
        h2p.application_program_ref.as_deref(),
        Some(format!("M-1092_A-LX{ns}-300").as_str())
    );
    assert_eq!(h2p.registration_number.as_deref(), Some("42/2026"));
}

#[test]
fn unmodelled_tables_are_reported_with_their_row_counts() {
    let (mapping, _) = mapping();
    assert!(mapping.diagnostics.iter().any(|d| matches!(
        d,
        MappingDiagnostic::UnmappedTable { name, rows } if name == "s19_block" && *rows == 1
    )));
}

#[test]
fn mapping_is_deterministic() {
    let (a, _) = mapping();
    let (b, _) = mapping();
    assert_eq!(a, b);
}

fn exim(kind: &str, tables: &str) -> Vec<u8> {
    format!(
        "EX-IM\r\nN x\r\nK ETS3\r\nK \r\nD 2026-10-08 09:00:00\r\nV 6.2\r\nH {kind}\r\n{tables}XXX\r\n"
    )
    .into_bytes()
}

const MANUFACTURER_ONLY: &str = "-------------------------------------\r\nT 3 manufacturer\r\n\
C1 T3 1 4 N MANUFACTURER_ID\r\nR 1 T 3 manufacturer\r\n4242\r\n";

#[test]
fn a_database_without_a_program_is_refused() {
    let document = parse_exim(&exim("virtual_device", MANUFACTURER_ONLY)).unwrap();
    let sha = "ab".repeat(32);
    match map_legacy_database(&document, &sha) {
        Err(LegacyError::Mapping { reason }) => {
            assert!(reason.contains("no application program"), "{reason}")
        }
        other => panic!("expected a mapping refusal, got {other:?}"),
    }
}

#[test]
fn a_project_export_is_not_a_product_database() {
    let document = parse_exim(&fixture("src-pr/MARVIN/ets.pr_")).unwrap();
    match map_legacy_database(&document, &"ab".repeat(32)) {
        Err(LegacyError::Mapping { reason }) => {
            assert!(reason.contains("product database"), "{reason}")
        }
        other => panic!("expected a mapping refusal, got {other:?}"),
    }
}

const PROGRAM: &str = "-------------------------------------\r\nT 11 application_program\r\n\
C1 T11 1 4 N PROGRAM_ID\r\nC2 T11 1 4 Y MANUFACTURER_ID\r\nR 1 T 11 application_program\r\n\
300\r\n4242\r\n";

/// A parameter table with `rows` as (id, parent id) pairs, all on page 1.
fn parameters(rows: &[(u32, Option<u32>)]) -> String {
    let mut out = String::from(
        "-------------------------------------\r\nT 15 parameter_type\r\n\
         C1 T15 1 4 N PARAMETER_TYPE_ID\r\nC2 T15 1 4 Y ATOMIC_TYPE_NUMBER\r\n\
         C3 T15 1 4 Y PROGRAM_ID\r\nR 1 T 15 parameter_type\r\n10\r\n0\r\n300\r\n\
         -------------------------------------\r\nT 17 parameter\r\n\
         C1 T17 1 4 N PARAMETER_ID\r\nC2 T17 1 4 Y PROGRAM_ID\r\nC3 T17 1 4 Y PARAMETER_TYPE_ID\r\n\
         C4 T17 3 50 Y PARAMETER_NUMBER\r\nC5 T17 2 2 Y PARAMETER_HIGH_ACCESS\r\n\
         C6 T17 1 4 Y PAR_PARAMETER_ID\r\n",
    );
    for (i, (id, parent)) in rows.iter().enumerate() {
        let parent = parent.map(|p| p.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "R {} T 17 parameter\r\n{id}\r\n300\r\n10\r\n{id}\r\n2\r\n{parent}\r\n",
            i + 1
        ));
    }
    out
}

#[test]
fn rows_that_cannot_be_mapped_are_counted_not_dropped_silently() {
    // A parameter of a program the file does not have, and two without a
    // number to identify them by (summed into one diagnostic).
    let mut tables = format!("{MANUFACTURER_ONLY}{PROGRAM}{}", parameters(&[(1, None)]));
    tables.push_str(
        "R 2 T 17 parameter\r\n2\r\n999\r\n10\r\n2\r\n2\r\n\r\nR 3 T 17 parameter\r\n3\r\n300\r\n10\r\n\r\n2\r\n\r\n\
         R 4 T 17 parameter\r\n4\r\n300\r\n10\r\n\r\n2\r\n\r\n",
    );
    let document = parse_exim(&exim("virtual_device", &tables)).unwrap();
    let mapping = map_legacy_database(&document, &"ab".repeat(32)).unwrap();
    let skipped: Vec<_> = mapping
        .diagnostics
        .iter()
        .filter_map(|d| match d {
            MappingDiagnostic::SkippedRows {
                table,
                reason,
                rows,
            } => Some((table.as_str(), reason.as_str(), *rows)),
            _ => None,
        })
        .collect();
    assert_eq!(
        skipped,
        [
            ("parameter", "PARAMETER_NUMBER is empty", 2),
            ("parameter", "PROGRAM_ID names no mapped program", 1),
        ]
    );
}

#[test]
fn a_deep_parent_chain_is_bounded_and_reported() {
    // 1 is a page; 2..=200 each hang under the previous one.
    let mut rows = vec![(1, None)];
    rows.extend((2..=200).map(|id| (id, Some(id - 1))));
    let tables = format!("{MANUFACTURER_ONLY}{PROGRAM}{}", parameters(&rows));
    let document = parse_exim(&exim("virtual_device", &tables)).unwrap();
    let mapping = map_legacy_database(&document, &"ab".repeat(32)).unwrap();
    let unplaced = mapping
        .diagnostics
        .iter()
        .filter(|d| matches!(d, MappingDiagnostic::UnplacedParameter { .. }))
        .count();
    assert!(unplaced > 0, "a chain deeper than the bound is reported");
    let placed = mapping.programs[0]
        .dynamic
        .iter()
        .filter(|n| n.kind == "ParameterRefRef")
        .count();
    assert_eq!(placed + unplaced, 199);
}
