//! Characterizes literal resource paths, duplicate identities and original namespace boundaries.

use crate::standalone_packages::wp16::{raw_zip, RawMember};
use knx_productdb::distribution::inspect_distribution;
fn xml(body: &str) -> Vec<u8> {
    format!(r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1">{body}</Manufacturer></ManufacturerData></KNX>"#).into_bytes()
}
fn path_case(attrs: &str, expected: &str) {
    let xml = xml(&format!("<Baggages><Baggage Id=\"B\" {attrs}/></Baggages>"));
    let zip = raw_zip(&[
        RawMember::stored(b"dir/source.xml", &xml),
        RawMember::stored(b"decoy/help.png", b"not used"),
    ]);
    let r = inspect_distribution("paths.zip", &zip).unwrap();
    assert_eq!(r.resource_declarations[0].resolution, expected);
}
macro_rules! unsafe_path {
    ($name:ident,$attrs:literal) => {
        #[test]
        fn $name() {
            path_case($attrs, "unsafePath");
        }
    };
}
unsafe_path!(absolute_target, r#"Name="help.png" TargetPath="/media""#);
unsafe_path!(drive_target, r#"Name="help.png" TargetPath="C:\media""#);
unsafe_path!(unc_target, r#"Name="help.png" TargetPath="\\host\share""#);
unsafe_path!(dot_target, r#"Name="help.png" TargetPath="a/./b""#);
unsafe_path!(traversal_target, r#"Name="help.png" TargetPath="a/../b""#);
unsafe_path!(
    empty_interior_segment,
    r#"Name="help.png" TargetPath="a//b""#
);
unsafe_path!(trailing_separator, r#"Name="help.png" TargetPath="a/""#);
unsafe_path!(control_in_target, r#"Name="help.png" TargetPath="a&#9;b""#);
unsafe_path!(missing_target, r#"Name="help.png""#);
unsafe_path!(missing_name, r#"TargetPath="""#);
unsafe_path!(empty_name, r#"Name="" TargetPath="""#);
unsafe_path!(name_is_not_a_basename, r#"Name="a/help.png" TargetPath="""#);
#[test]
fn no_suffix_search_for_missing_payload() {
    path_case(r#"Name="help.png" TargetPath="""#, "missingMember");
}
#[test]
fn encoded_looking_segments_remain_literal_and_are_not_decoded() {
    let xml =
        xml(r#"<Baggages><Baggage Id="%2FB" Name="%2Fhelp.png" TargetPath="%2e%2e"/></Baggages>"#);
    let zip = raw_zip(&[RawMember::stored(b"source.xml", &xml)]);
    let r = inspect_distribution("literal.zip", &zip).unwrap();
    let d = &r.resource_declarations[0];
    assert_eq!(d.id.as_deref(), Some("%2FB"));
    assert_eq!(
        d.expected_logical_path.as_deref(),
        Some("source.baggages/%2e%2e/%2Fhelp.png")
    );
    assert_eq!(d.resolution, "missingMember");
}
#[test]
fn unknown_suffix_does_not_guess_a_baggage_directory() {
    let xml = xml(r#"<Baggages><Baggage Id="B" Name="x" TargetPath=""/></Baggages>"#);
    let zip = raw_zip(&[RawMember::stored(b"source.XML", &xml)]);
    let r = inspect_distribution("suffix.zip", &zip).unwrap();
    assert_eq!(r.resource_declarations[0].resolution, "mappingUnavailable");
}
#[test]
fn windows_names_join_only_in_the_dedicated_logical_view() {
    let xml = xml(r#"<Baggages><Baggage Id="B" Name="help.png" TargetPath="one\two"/></Baggages>"#);
    let zip = raw_zip(&[
        RawMember::stored(br"dir\source.xml", &xml),
        RawMember::stored(br"dir\source.baggages\one\two\help.png", b"metadata"),
    ]);
    let r = inspect_distribution("windows.zip", &zip).unwrap();
    assert_eq!(r.resource_declarations[0].member_candidates, [1]);
    assert_eq!(
        r.members[1].raw_name,
        br"dir\source.baggages\one\two\help.png"
    );
}
#[test]
fn duplicate_declarations_stay_ambiguous_but_share_one_derived_payload() {
    let xml = xml(
        r#"<ApplicationPrograms><ApplicationProgram Id="A" IconFile="B"/></ApplicationPrograms><Baggages><Baggage Id="B" Name="x.png" TargetPath=""/><Baggage Id="B" Name="x.png" TargetPath=""/></Baggages>"#,
    );
    let png = super::distribution_baggage::authored_png();
    let zip = raw_zip(&[
        RawMember::stored(b"source.xml", &xml),
        RawMember::stored(b"source.baggages/x.png", &png),
    ]);
    let r = inspect_distribution("duplicates.zip", &zip).unwrap();
    assert_eq!(r.resource_declarations.len(), 2);
    assert_eq!(r.resource_previews.len(), 1);
    let c = &r.resource_consumers[0];
    assert_eq!(
        c.reference_state,
        Some(knx_productdb::distribution::DistributionReferenceState::Ambiguous)
    );
    assert_eq!(c.declaration_candidates.len(), 2);
    assert_ne!(c.declaration_candidates[0], c.declaration_candidates[1]);
}
#[test]
fn foreign_namespace_and_wrong_host_shapes_remain_unknown() {
    let xml = xml(
        r#"<Baggages xmlns="urn:foreign"><Baggage Id="F" Name="x" TargetPath=""/></Baggages><ApplicationPrograms><ApplicationProgram Id="A"><Static><ParameterTypes><ParameterType Id="T"><TypePicture xmlns="urn:foreign" RefId="F"/></ParameterType></ParameterTypes><NotExtension><Baggage RefId="F"/></NotExtension><Bogus><TypePicture RefId="F"/></Bogus></Static></ApplicationProgram></ApplicationPrograms>"#,
    );
    let r = inspect_distribution("foreign.xml", &xml).unwrap();
    assert!(r.resource_declarations.is_empty());
    assert!(r.resource_consumers.is_empty());
    assert!(!r.unknowns.is_empty());
}
#[test]
fn repeated_manufacturer_scopes_do_not_claim_a_unique_cross_member_join() {
    let decl = xml(r#"<Baggages><Baggage Id="B" Name="x" TargetPath=""/></Baggages>"#);
    let apps=br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1"><ApplicationPrograms><ApplicationProgram Id="A" IconFile="B"/></ApplicationPrograms></Manufacturer><Manufacturer RefId="M-1"/></ManufacturerData></KNX>"#;
    let zip = raw_zip(&[
        RawMember::stored(b"decl.xml", &decl),
        RawMember::stored(b"apps.xml", apps),
    ]);
    let r = inspect_distribution("scope.zip", &zip).unwrap();
    let c = &r.resource_consumers[0];
    assert_eq!(
        c.reference_state,
        Some(knx_productdb::distribution::DistributionReferenceState::Ambiguous)
    );
    assert_eq!(c.reason, "ambiguousManufacturerScope");
    assert_eq!(c.declaration_candidates.len(), 1);
}

#[test]
fn picture_under_noncanonical_module_definition_is_not_a_consumer() {
    let xml = xml(
        r#"<ApplicationPrograms><ApplicationProgram Id="A"><Bogus><ModuleDefs><ModuleDef Id="D"><Static><ParameterTypes><ParameterType Id="T"><TypePicture RefId="B"/></ParameterType></ParameterTypes></Static></ModuleDef></ModuleDefs></Bogus></ApplicationProgram></ApplicationPrograms>"#,
    );
    let r = inspect_distribution("wrong-host.xml", &xml).unwrap();
    assert!(
        r.resource_consumers.is_empty(),
        "noncanonical module paths must remain unknown"
    );
}

#[test]
fn namespace_is_not_borrowed_across_members() {
    let decl = xml(r#"<Baggages><Baggage Id="B" Name="x" TargetPath=""/></Baggages>"#);
    let app = String::from_utf8(xml(
        r#"<ApplicationPrograms><ApplicationProgram Id="A" IconFile="B"/></ApplicationPrograms>"#,
    ))
    .unwrap()
    .replace("project/20", "project/14");
    let zip = raw_zip(&[
        RawMember::stored(b"decl.xml", &decl),
        RawMember::stored(b"app.xml", app.as_bytes()),
    ]);
    let r = inspect_distribution("namespaces.zip", &zip).unwrap();
    assert!(r.resource_consumers[0].declaration_candidates.is_empty());
}
