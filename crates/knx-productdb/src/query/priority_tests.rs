//! Verifies literal priority provenance through product ingestion, queries and native reopen.

use super::{com_object_view, com_object_views, ValueLayer};
use crate::parse::translation::{insert_translations, TranslationScope};
use crate::{open_and_migrate, open_read_only, parse::program::ingest_program};
use rusqlite::Connection;

struct Case {
    program: Option<&'static str>,
    reference: Option<&'static str>,
    selected: Option<&'static str>,
    layer: Option<ValueLayer>,
}

const CASES: &[Case] = &[
    Case {
        program: None,
        reference: None,
        selected: None,
        layer: None,
    },
    Case {
        program: Some("Low"),
        reference: None,
        selected: Some("Low"),
        layer: Some(ValueLayer::Program),
    },
    Case {
        program: Some("High"),
        reference: None,
        selected: Some("High"),
        layer: Some(ValueLayer::Program),
    },
    Case {
        program: Some("Alert"),
        reference: None,
        selected: Some("Alert"),
        layer: Some(ValueLayer::Program),
    },
    Case {
        program: None,
        reference: Some("High"),
        selected: Some("High"),
        layer: Some(ValueLayer::ProgramRef),
    },
    Case {
        program: Some("Low"),
        reference: Some("Alert"),
        selected: Some("Alert"),
        layer: Some(ValueLayer::ProgramRef),
    },
    Case {
        program: Some("Low"),
        reference: Some("Low"),
        selected: Some("Low"),
        layer: Some(ValueLayer::ProgramRef),
    },
    Case {
        program: Some(""),
        reference: None,
        selected: Some(""),
        layer: Some(ValueLayer::Program),
    },
    Case {
        program: Some("Low"),
        reference: Some(""),
        selected: Some(""),
        layer: Some(ValueLayer::ProgramRef),
    },
    Case {
        program: None,
        reference: Some(""),
        selected: Some(""),
        layer: Some(ValueLayer::ProgramRef),
    },
    Case {
        program: Some("  Low  "),
        reference: None,
        selected: Some("  Low  "),
        layer: Some(ValueLayer::Program),
    },
    Case {
        program: Some("Low"),
        reference: Some("   "),
        selected: Some("   "),
        layer: Some(ValueLayer::ProgramRef),
    },
    Case {
        program: Some("VendorPriority"),
        reference: None,
        selected: Some("VendorPriority"),
        layer: Some(ValueLayer::Program),
    },
    Case {
        program: Some("Low"),
        reference: Some("low"),
        selected: Some("low"),
        layer: Some(ValueLayer::ProgramRef),
    },
    Case {
        program: Some("Low"),
        reference: Some("<unknown & literal>"),
        selected: Some("<unknown & literal>"),
        layer: Some(ValueLayer::ProgramRef),
    },
];

fn priority_attribute(value: Option<&str>) -> String {
    value.map_or_else(String::new, |raw| {
        let escaped = raw
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        format!(" Priority=\"{escaped}\"")
    })
}

fn ingest_cases(conn: &Connection, program_id: &str, cases: &[Case]) {
    let mut objects = String::new();
    let mut references = String::new();
    for (index, case) in cases.iter().enumerate() {
        objects.push_str(&format!(
            "<ComObject Id=\"O-{index}\" Number=\"{index}\"{} />",
            priority_attribute(case.program)
        ));
        references.push_str(&format!(
            "<ComObjectRef Id=\"O-{index}_R-1\" RefId=\"O-{index}\"{} />",
            priority_attribute(case.reference)
        ));
    }
    let source = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{program_id}" Name="Priority witness" ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ComObjectTable>{objects}</ComObjectTable><ComObjectRefs>{references}</ComObjectRefs></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let digest = crate::sha256_hex(source.as_bytes());
    ingest_program(
        conn,
        &digest,
        &format!("M-0001/{program_id}.xml"),
        source.as_bytes(),
    )
    .unwrap();
}

fn check_queries(conn: &Connection, program_id: &str, cases: &[Case]) {
    let ids: Vec<_> = (0..cases.len())
        .map(|index| format!("O-{index}_R-1"))
        .collect();
    let borrowed: Vec<_> = ids.iter().map(String::as_str).collect();
    let batch = com_object_views(conn, program_id, &borrowed, None).unwrap();
    assert_eq!(batch.len(), cases.len());
    for (index, case) in cases.iter().enumerate() {
        let view = &batch[&ids[index]];
        assert_eq!(
            view.priority.as_deref(),
            case.selected,
            "{program_id}/{} literal",
            ids[index]
        );
        assert_eq!(
            view.priority_layer, case.layer,
            "{program_id}/{} origin",
            ids[index]
        );
        assert_eq!(
            com_object_view(conn, program_id, &ids[index], None)
                .unwrap()
                .as_ref(),
            Some(view)
        );
    }
    assert!(com_object_view(conn, program_id, "unknown-ref", None)
        .unwrap()
        .is_none());
}

#[test]
fn priority_ingestion_keeps_each_raw_layer_separate() {
    let conn = open_and_migrate(std::path::Path::new(":memory:")).unwrap();
    ingest_cases(&conn, "A-1", CASES);
    for (index, case) in CASES.iter().enumerate() {
        let values: (Option<String>, Option<String>) = conn.query_row(
            "SELECT o.priority, r.priority FROM com_object o JOIN com_object_ref r ON r.program_id=o.program_id AND r.com_object_id=o.id WHERE o.program_id='A-1' AND r.id=?1",
            [format!("O-{index}_R-1")], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(values.0.as_deref(), case.program, "object {index}");
        assert_eq!(values.1.as_deref(), case.reference, "reference {index}");
    }
}

#[test]
fn priority_queries_report_presence_based_origin_without_interpretation() {
    let conn = open_and_migrate(std::path::Path::new(":memory:")).unwrap();
    ingest_cases(&conn, "A-1", CASES);
    let changes = conn.total_changes();
    check_queries(&conn, "A-1", CASES);
    assert_eq!(
        conn.total_changes(),
        changes,
        "queries must not update stored declarations"
    );
}

#[test]
fn priority_provenance_survives_close_and_read_only_native_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    ingest_cases(&conn, "A-1", CASES);
    check_queries(&conn, "A-1", CASES);
    conn.close().unwrap();
    let before = std::fs::read(&path).unwrap();
    let reopened = open_read_only(&path).unwrap();
    assert_eq!(reopened.migrated_from, None);
    assert!(
        !reopened.conn.path().unwrap().is_empty(),
        "not an in-memory migration"
    );
    check_queries(&reopened.conn, "A-1", CASES);
    reopened.conn.close().unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "read-only reopen must not rewrite the catalogue"
    );
}

#[test]
fn priority_provenance_stays_with_its_program_even_when_ids_repeat() {
    let conn = open_and_migrate(std::path::Path::new(":memory:")).unwrap();
    let other = &[Case {
        program: Some("High"),
        reference: Some("Alert"),
        selected: Some("Alert"),
        layer: Some(ValueLayer::ProgramRef),
    }];
    ingest_cases(&conn, "A-1", CASES);
    ingest_cases(&conn, "A-2", other);
    check_queries(&conn, "A-1", CASES);
    check_queries(&conn, "A-2", other);
    assert!(com_object_view(&conn, "A-3", "O-0_R-1", None)
        .unwrap()
        .is_none());
}

#[test]
fn display_translation_cannot_change_priority_literal_or_origin() {
    let conn = open_and_migrate(std::path::Path::new(":memory:")).unwrap();
    ingest_cases(&conn, "A-1", CASES);
    assert!(insert_translations(
        &conn,
        TranslationScope::Program,
        "A-1",
        "de-DE",
        "O-1_R-1",
        "Priority",
        "Alert"
    )
    .unwrap());
    let plain = com_object_view(&conn, "A-1", "O-1_R-1", None)
        .unwrap()
        .unwrap();
    let translated = com_object_view(&conn, "A-1", "O-1_R-1", Some("de-DE"))
        .unwrap()
        .unwrap();
    assert_eq!(translated, plain);
    assert_eq!(translated.priority.as_deref(), Some("Low"));
    assert_eq!(translated.priority_layer, Some(ValueLayer::Program));
}
