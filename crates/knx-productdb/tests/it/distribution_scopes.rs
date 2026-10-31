//! Ensures same-namespace extensions cannot smuggle declarations through legacy local-name readers.

use crate::distribution_inspection::*;
#[test]
fn misplaced_same_namespace_declarations_are_preserved_but_never_used_by_scoped_readers() {
    let source=br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><Catalog><Extension><CatalogItem Id="injected"/></Extension><CatalogItem Id="C"/></Catalog><ApplicationPrograms><ApplicationProgram Id="A"><Static><Extension><Parameter Id="P" Value="injected"/></Extension><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs><ApplicationProgram Id="injected-app"/></Static><Dynamic/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let r = inspect_distribution("authored", source).unwrap();
    assert_eq!(
        r.catalog_items.len(),
        1,
        "CatalogItem must have an accepted Catalog/CatalogSection ancestry"
    );
    assert_eq!(r.catalog_items[0].id, Some("C".into()));
    assert_eq!(r.applications.len(), 1);
    assert_eq!(r.applications[0].parameter_fallbacks[0].value, None);
    assert!(r
        .unknowns
        .iter()
        .any(|u| u.kind == "element" && u.raw_name == "Parameter"));
    assert!(r
        .references
        .iter()
        .any(|r| r.declared_target == "P" && r.candidates.is_empty()));
}
