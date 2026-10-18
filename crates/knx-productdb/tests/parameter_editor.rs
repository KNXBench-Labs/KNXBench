//! Verifies exact-owner optional editor metadata without changing source or rows.

use knx_productdb::parameter_editor::{
    parameter_editor_evidence, EditorEvidenceIssue as Issue, ParameterEditorEvidence as Evidence,
    MAX_EDITOR_SOURCE_BYTES,
};
use knx_productdb::{ingest_file, open_and_migrate, sha256_hex, store_source_file, SourceFile};
use rusqlite::Connection;

const A: &str = "M-00FA_A-0009-10-ABCD";
const B: &str = "M-00FA_A-000A-10-ABCE";

fn app(id: &str, child: &str) -> String {
    format!(
        r#"<ApplicationProgram Id="{id}" Name="Evidence" ApplicationNumber="9" ApplicationVersion="16" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" PeiType="0" DefaultLanguage="en-US"><Static><ParameterTypes><ParameterType Id="PT" Name="Declared"><{child}/></ParameterType></ParameterTypes><Parameters><Parameter Id="P" Name="Value" ParameterType="PT" Value="10"/></Parameters><ParameterRefs><ParameterRef Id="PR" RefId="P"/></ParameterRefs></Static></ApplicationProgram>"#
    )
}
fn xml(apps: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-00FA"><ApplicationPrograms>{apps}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}
fn db(source: &str) -> (tempfile::TempDir, Connection) {
    let d = tempfile::tempdir().unwrap();
    let c = open_and_migrate(&d.path().join("products.sqlite")).unwrap();
    ingest_file(&c, "M-00FA/Application.xml", source.as_bytes()).unwrap();
    (d, c)
}
fn time() -> String {
    xml(&app(
        A,
        r#"TypeTime SizeInBit="16" minInclusive="0" maxInclusive="3600" Unit="Seconds" UIHint="Duration_hhmmss""#,
    ))
}
fn rebind(c: &Connection, source: &str) {
    store_source_file(
        c,
        &SourceFile {
            source_path: "M-00FA/changed.xml".into(),
            manufacturer_id: Some("M-00FA".into()),
            bytes: source.as_bytes().to_vec(),
        },
    )
    .unwrap();
    c.execute(
        "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
        [sha256_hex(source.as_bytes()), A.into()],
    )
    .unwrap();
}
fn read(c: &Connection) -> Evidence {
    parameter_editor_evidence(c, A, "PR").unwrap()
}
#[test]
fn exact_source_metadata_is_raw_and_read_only() {
    let source = time();
    let (_d, c) = db(&source);
    let changes = c.total_changes();
    c.execute_batch("PRAGMA query_only=ON").unwrap();
    let Evidence::Declared(v) = read(&c) else {
        panic!("expected exact declaration")
    };
    assert_eq!(v.source_sha256, sha256_hex(source.as_bytes()));
    assert_eq!(v.program_id, A);
    assert_eq!(v.parameter_type_id, "PT");
    assert_eq!(v.schema, 20);
    assert_eq!(v.child, "TypeTime");
    assert_eq!(v.attributes["Unit"], "Seconds");
    assert_eq!(v.attributes["UIHint"], "Duration_hhmmss");
    assert_eq!(v.parameter_type_attributes["Name"], "Declared");
    assert_eq!(c.total_changes(), changes);
}
#[test]
fn equal_type_ids_in_other_programs_never_supply_hints() {
    let source = xml(&(app(A, r#"TypeTime SizeInBit="16" Unit="Minutes""#)
        + &app(
            B,
            r#"TypeNumber SizeInBit="8" Type="unsignedInt" UIHint="Slider""#,
        )));
    let (_d, c) = db(&source);
    let Evidence::Declared(v) = read(&c) else {
        panic!("program-local source needed")
    };
    assert_eq!(v.child, "TypeTime");
    assert_eq!(v.attributes["Unit"], "Minutes");
    assert!(!v.attributes.contains_key("UIHint"));
}
#[test]
fn qualified_vendor_attribute_is_not_an_unqualified_uihint() {
    let source = time().replace(
        "<TypeTime ",
        "<TypeTime xmlns:v=\"urn:vendor\" v:UIHint=\"Slider\" ",
    );
    let (_d, c) = db(&source);
    let Evidence::Declared(v) = read(&c) else {
        panic!("qualified evidence must be preserved")
    };
    assert_eq!(v.attributes["v:UIHint"], "Slider");
    assert_eq!(v.attributes["UIHint"], "Duration_hhmmss");
}
#[test]
fn corrupted_same_length_blob_has_named_identity_fallback() {
    let source = time();
    let (_d, c) = db(&source);
    c.execute(
        "UPDATE source_file SET bytes=?1 WHERE sha256=?2",
        rusqlite::params![
            source.replace("Seconds", "Corrupt").as_bytes(),
            sha256_hex(source.as_bytes())
        ],
    )
    .unwrap();
    assert_eq!(read(&c), Evidence::Unavailable(Issue::SourceIdentity));
}
#[test]
fn no_retained_source_is_too_large_to_inspect() {
    // The editor's source bound is the storage ceiling (ADR-0142): SQLite
    // refuses a larger value, so `SourceBudget` only guards builds against an
    // SQLite with a higher limit, and the evidence stays as it was.
    let source = time();
    let (_d, c) = db(&source);
    let before = read(&c);
    let refused = c.execute(
        "UPDATE source_file SET len=?1,bytes=zeroblob(?1) WHERE sha256=?2",
        rusqlite::params![
            i64::try_from(MAX_EDITOR_SOURCE_BYTES + 1).unwrap(),
            sha256_hex(source.as_bytes())
        ],
    );
    assert!(refused.is_err(), "SQLite stored a value above its limit");
    assert_eq!(read(&c), before);
    assert_ne!(before, Evidence::Unavailable(Issue::SourceBudget));
}
#[test]
fn duplicate_type_definitions_are_ambiguous_not_first_wins() {
    let source = time();
    let (_d, c) = db(&source);
    let changed=source.replace("</ParameterTypes>",r#"<ParameterType Id="PT"><TypeTime SizeInBit="16" Unit="Hours"/></ParameterType></ParameterTypes>"#);
    rebind(&c, &changed);
    assert_eq!(read(&c), Evidence::Unavailable(Issue::AmbiguousDefinition));
}
#[test]
fn model_and_child_disagreement_never_admit_specialized_metadata() {
    let source = time();
    let (_d, c) = db(&source);
    let changed = source.replace("TypeTime", "TypeNumber");
    rebind(&c, &changed);
    assert_eq!(read(&c), Evidence::Unavailable(Issue::ModelMismatch));
}
#[test]
fn unknown_namespace_and_dtd_are_named_unsupported_originals_survive() {
    for changed in [
        time().replace("http://knx.org/xml/project/20", "urn:unknown"),
        time().replace("?><KNX", "?><!DOCTYPE KNX><KNX"),
    ] {
        let source = time();
        let (_d, c) = db(&source);
        rebind(&c, &changed);
        let before = c.total_changes();
        assert_eq!(read(&c), Evidence::Unavailable(Issue::UnsupportedXml));
        assert_eq!(c.total_changes(), before);
        assert_eq!(
            knx_productdb::load_source_file(&c, &sha256_hex(changed.as_bytes()))
                .unwrap()
                .unwrap(),
            changed.as_bytes()
        );
    }
}
#[test]
fn missing_reference_is_distinct_and_sqlite_failure_is_not_a_fallback() {
    let (_d, c) = db(&time());
    assert_eq!(
        parameter_editor_evidence(&c, A, "absent").unwrap(),
        Evidence::Unavailable(Issue::MissingReference)
    );
    c.execute_batch("DROP TABLE parameter_ref").unwrap();
    assert!(parameter_editor_evidence(&c, A, "PR").is_err());
}
#[test]
fn nested_wrong_structural_owner_cannot_shadow_direct_type() {
    let source = time();
    let (_d, c) = db(&source);
    let changed=source.replace("</ParameterTypes>",r#"<Unrelated><ParameterType Id="PT"><TypeTime SizeInBit="16" Unit="Hours"/></ParameterType></Unrelated></ParameterTypes>"#);
    rebind(&c, &changed);
    let Evidence::Declared(v) = read(&c) else {
        panic!("exact structural owner needed")
    };
    assert_eq!(v.attributes["Unit"], "Seconds");
}

#[test]
fn normalized_ref_cannot_borrow_an_unrelated_valid_source_type() {
    let source = time()
        .replace("</ParameterTypes>", r#"<ParameterType Id="PT2"><TypeNumber SizeInBit="8" Type="unsignedInt" UIHint="Slider"/></ParameterType></ParameterTypes>"#)
        .replace("</Parameters>", r#"<Parameter Id="P2" Name="Other" ParameterType="PT2" Value="10"/></Parameters>"#);
    let (_d, c) = db(&source);
    c.execute(
        "UPDATE parameter_ref SET parameter_id='P2' WHERE program_id=?1 AND id='PR'",
        [A],
    )
    .unwrap();
    assert_eq!(read(&c), Evidence::Unavailable(Issue::ModelMismatch));
}

#[test]
fn color_and_picture_attributes_remain_evidence_not_a_codec() {
    for (child, name, value) in [
        (r#"TypeColor Space="RGB""#, "Space", "RGB"),
        (
            r#"TypePicture RefId="M-00FA_BG-diagram.png""#,
            "RefId",
            "M-00FA_BG-diagram.png",
        ),
    ] {
        let source = xml(&app(A, child));
        let (_d, c) = db(&source);
        let Evidence::Declared(v) = read(&c) else {
            panic!("expected raw source evidence")
        };
        assert_eq!(v.attributes[name], value);
    }
}

#[test]
fn a_prefixed_root_does_not_authorize_unqualified_descendants() {
    let source = time();
    let (_d, c) = db(&source);
    let changed = source
        .replace("<KNX xmlns=", "<k:KNX xmlns:k=")
        .replace("</KNX>", "</k:KNX>")
        .replace("<ManufacturerData>", "<k:ManufacturerData>")
        .replace("</ManufacturerData>", "</k:ManufacturerData>");
    // Without the default namespace, unprefixed descendants have no KNX
    // owner; one prefixed ancestor cannot authorize the whole document.
    rebind(&c, &changed);
    assert_eq!(read(&c), Evidence::Unavailable(Issue::MissingDeclaration));
}

#[test]
fn evidence_survives_close_and_read_only_reopen_without_file_changes() {
    let source = time();
    let (d, c) = db(&source);
    let expected = read(&c);
    drop(c);
    let path = d.path().join("products.sqlite");
    let before = std::fs::read(&path).unwrap();
    let reopened = knx_productdb::open_read_only(&path).unwrap();
    assert_eq!(read(&reopened.conn), expected);
    assert_eq!(reopened.conn.total_changes(), 0);
    drop(reopened);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn missing_source_and_inconsistent_length_have_distinct_fallbacks() {
    let source = time();
    let (_d, c) = db(&source);
    c.execute(
        "UPDATE source_file SET len=len+1 WHERE sha256=?1",
        [sha256_hex(source.as_bytes())],
    )
    .unwrap();
    assert_eq!(read(&c), Evidence::Unavailable(Issue::SourceIdentity));
    c.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    c.execute(
        "DELETE FROM source_file WHERE sha256=?1",
        [sha256_hex(source.as_bytes())],
    )
    .unwrap();
    assert_eq!(read(&c), Evidence::Unavailable(Issue::MissingSource));
}

#[test]
fn deep_and_large_attribute_sources_are_bounded_without_changes() {
    let base = time();
    let variants = [
        base.replace(
            "</Static>",
            &format!(
                "{}{}{}</Static>",
                "<N>".repeat(70),
                "value",
                "</N>".repeat(70)
            ),
        ),
        base.replace(
            "Unit=\"Seconds\"",
            &format!("Unit=\"{}\"", "x".repeat(4097)),
        ),
    ];
    for source in variants {
        let (_d, c) = db(&base);
        rebind(&c, &source);
        let changes = c.total_changes();
        assert_eq!(read(&c), Evidence::Unavailable(Issue::SourceBudget));
        assert_eq!(c.total_changes(), changes);
    }
}

#[test]
fn malformed_source_is_not_partial_evidence() {
    let base = time();
    for source in [
        base.replace("</KNX>", ""),
        base.replace("Unit=\"Seconds\"", "Unit=\"Seconds\" Unit=\"Minutes\""),
        base.replace("TypeTime ", "TypeTime xmlns=\"urn:foreign\" "),
    ] {
        let (_d, c) = db(&base);
        rebind(&c, &source);
        let result = read(&c);
        assert!(
            matches!(
                result,
                Evidence::Unavailable(Issue::UnsupportedXml | Issue::MissingDeclaration)
            ),
            "{result:?}"
        );
    }
}
