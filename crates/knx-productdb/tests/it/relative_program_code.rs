//! Relative source segments remain distinct from absolute executable memory images.

use knx_productdb::code::{parse_program_code, CodeError, ProgramCode};

fn parse(body: &str) -> Result<Option<ProgramCode>, CodeError> {
    let xml = format!(
        r#"<KNX><ApplicationProgram Id="P"><Static><Code>{body}</Code></Static></ApplicationProgram></KNX>"#
    );
    parse_program_code("synthetic.xml", xml.as_bytes(), "P")
}

fn error(body: &str) -> String {
    match parse(body) {
        Err(CodeError::Malformed { cause, .. }) => cause,
        other => panic!("expected explicit source refusal, got {other:?}"),
    }
}

#[test]
fn relative_metadata_and_blobs_are_retained_without_inventing_absolute_addresses() {
    let code = parse(r#"<RelativeSegment Id="R" Offset="17" Size="3" LoadStateMachine="4" Name="Parameters" Future="retain-me"><Data>AQID</Data><Mask>/wD/</Mask></RelativeSegment>"#).unwrap().unwrap();
    assert!(code.segments.is_empty());
    assert!(code.segment("R").is_none());
    assert_eq!(code.relative_segments.len(), 1);
    let segment = &code.relative_segments[0];
    assert_eq!(segment.id, "R");
    assert_eq!(segment.offset, 17);
    assert_eq!(segment.size, 3);
    assert_eq!(segment.load_state_machine, 4);
    assert_eq!(segment.data.as_deref(), Some(&[1, 2, 3][..]));
    assert_eq!(segment.mask.as_deref(), Some(&[255, 0, 255][..]));
    assert_eq!(segment.other.len(), 2);
    assert_eq!(segment.other["Name"], "Parameters");
    assert_eq!(segment.other["Future"], "retain-me");
}

#[test]
fn absent_source_data_is_not_synthesized_even_for_a_large_declared_size() {
    let code =
        parse(r#"<RelativeSegment Id="R" Offset="0" Size="4294967295" LoadStateMachine="255"/>"#)
            .unwrap()
            .unwrap();
    let segment = &code.relative_segments[0];
    assert_eq!(segment.size, u32::MAX);
    assert_eq!(segment.load_state_machine, u8::MAX);
    assert!(segment.data.is_none());
    assert!(segment.mask.is_none());
}

#[test]
fn namespaces_whitespace_and_cdata_are_read_by_existing_blob_rules() {
    let code = parse(r#"<k:RelativeSegment xmlns:k="urn:synthetic" Id="R" Offset="0" Size="3" LoadStateMachine="4"><k:Data> A Q <![CDATA[I D]]> </k:Data></k:RelativeSegment>"#).unwrap().unwrap();
    assert_eq!(
        code.relative_segments[0].data.as_deref(),
        Some(&[1, 2, 3][..])
    );
}

#[test]
fn zero_length_present_data_is_distinct_from_absent_data() {
    let code = parse(r#"<RelativeSegment Id="R" Offset="0" Size="0" LoadStateMachine="0"><Data/></RelativeSegment>"#).unwrap().unwrap();
    assert_eq!(code.relative_segments[0].data, Some(vec![]));
}

#[test]
fn mixed_sources_keep_per_kind_order_and_never_alias_relative_to_absolute() {
    let code = parse(r#"<RelativeSegment Id="R2" Offset="9" Size="0" LoadStateMachine="4"/><AbsoluteSegment Id="A" Address="1024" Size="1"><Data>/w==</Data></AbsoluteSegment><RelativeSegment Id="R1" Offset="3" Size="0" LoadStateMachine="2"/>"#).unwrap().unwrap();
    assert_eq!(
        code.relative_segments
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        ["R2", "R1"]
    );
    assert_eq!(code.segments.len(), 1);
    assert_eq!(code.segment("A").unwrap().address, 1024);
}

#[test]
fn every_required_relative_field_must_be_explicit_and_well_formed() {
    let full = r#"<RelativeSegment Id="R" Offset="0" Size="1" LoadStateMachine="4"/>"#;
    for field in [
        r#"Id="R""#,
        r#"Offset="0""#,
        r#"Size="1""#,
        r#"LoadStateMachine="4""#,
    ] {
        assert!(error(&full.replace(field, "")).contains("RelativeSegment"));
    }
    for (field, value) in [
        ("Id", ""),
        ("Offset", "-1"),
        ("Offset", "4294967296"),
        ("Size", "invalid"),
        ("LoadStateMachine", "256"),
        ("LoadStateMachine", "-1"),
    ] {
        let original = match field {
            "Id" => "R",
            "Offset" => "0",
            "Size" => "1",
            _ => "4",
        };
        assert!(error(&full.replace(
            &format!("{field}=\"{original}\""),
            &format!("{field}=\"{value}\"")
        ))
        .contains(field));
    }
}

#[test]
fn data_mask_invalid_base64_and_decoded_length_mismatches_refuse() {
    for child in [
        "<Data>?</Data>",
        "<Mask>?</Mask>",
        "<Data>AQ==</Data>",
        "<Mask>AQ==</Mask>",
        "<Data/>",
    ] {
        let source = format!(
            r#"<RelativeSegment Id="R" Offset="0" Size="3" LoadStateMachine="4">{child}</RelativeSegment>"#
        );
        assert!(error(&source).contains("RelativeSegment"));
    }
}

#[test]
fn duplicate_blob_and_unknown_nested_content_are_not_silently_dropped() {
    for child in [
        "<Data>AQ==</Data><Data>AQ==</Data>",
        "<Mask>/w==</Mask><Mask>/w==</Mask>",
        "<Future Important=\"yes\"/>",
        "<Data><Inner/></Data>",
    ] {
        let source = format!(
            r#"<RelativeSegment Id="R" Offset="0" Size="1" LoadStateMachine="4">{child}</RelativeSegment>"#
        );
        assert!(error(&source).contains("RelativeSegment") || error(&source).contains("Data"));
    }
}

#[test]
fn duplicate_relative_or_cross_kind_identity_is_refused_in_either_order() {
    let relative = r#"<RelativeSegment Id="X" Offset="0" Size="0" LoadStateMachine="4"/>"#;
    let absolute = r#"<AbsoluteSegment Id="X" Address="0" Size="0"/>"#;
    for source in [
        format!("{relative}{relative}"),
        format!("{relative}{absolute}"),
        format!("{absolute}{relative}"),
    ] {
        assert!(error(&source).contains("duplicate segment Id"));
    }
}

#[test]
fn foreign_application_and_outside_code_segments_are_not_selected() {
    let xml = r#"<KNX><ApplicationProgram Id="OTHER"><Static><Code><RelativeSegment/></Code></Static></ApplicationProgram><ApplicationProgram Id="P"><Dynamic><RelativeSegment/></Dynamic><Static><Code><RelativeSegment Id="R" Offset="0" Size="0" LoadStateMachine="4"/></Code></Static></ApplicationProgram></KNX>"#;
    let code = parse_program_code("synthetic.xml", xml.as_bytes(), "P")
        .unwrap()
        .unwrap();
    assert_eq!(code.relative_segments.len(), 1);
    assert!(
        parse_program_code("synthetic.xml", xml.as_bytes(), "absent")
            .unwrap()
            .is_none()
    );
}
