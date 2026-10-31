//! Verifies split-source joins remain explicit, occurrence-bound and non-authoritative.

use crate::distribution_inspection::*;
use crate::standalone_packages::wp16::{raw_zip, RawMember};
use knx_productdb::distribution::{
    DistributionReferenceState as State, DistributionResolutionBasis as Basis,
};
fn wrap(section: &str) -> Vec<u8> {
    format!(r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">{section}</Manufacturer></ManufacturerData></KNX>"#).into_bytes()
}
#[test]
fn split_archive_links_declared_manufacturer_sources_with_exact_candidate_coordinates() {
    let catalog = wrap(
        r#"<Catalog><CatalogItem Id="C" ProductRefId="P" Hardware2ProgramRefId="H2P"/></Catalog>"#,
    );
    let hardware = wrap(
        r#"<Hardware><Hardware Id="H"><Products><Product Id="P"/></Products><Hardware2Programs><Hardware2Program Id="H2P"><ApplicationProgramRef RefId="A"/></Hardware2Program></Hardware2Programs></Hardware></Hardware>"#,
    );
    let app = wrap(
        r#"<ApplicationPrograms><ApplicationProgram Id="A"><Static/><Dynamic/></ApplicationProgram></ApplicationPrograms>"#,
    );
    let bytes = raw_zip(&[
        RawMember::stored(b"Catalog.xml", &catalog),
        RawMember::stored(b"Hardware.xml", &hardware),
        RawMember::stored(b"Application.xml", &app),
    ]);
    let r = inspect_distribution("split", &bytes).unwrap();
    let product = r
        .references
        .iter()
        .find(|r| r.attribute == "ProductRefId")
        .unwrap();
    assert_eq!(
        product.state,
        State::Resolved,
        "split-source identity join must not leave an actually present declared product dangling"
    );
    assert_eq!(
        product.resolution_basis,
        Basis::DeclaredManufacturerAcrossMembers
    );
    assert_eq!(product.candidates[0].member_ordinal, Some(1));
    assert!(hardware[r
        .resolve_coordinate(product.candidates[0])
        .unwrap()
        .start_byte as usize..]
        .starts_with(b"<Product Id="));
    let program = r
        .references
        .iter()
        .find(|r| r.declared_target == "A")
        .unwrap();
    assert_eq!(program.candidates[0].member_ordinal, Some(2));
    assert_eq!(
        r.identities
            .iter()
            .filter(|i| i.kind == "Product" && i.declared_id == "P")
            .count(),
        1
    );
    let repeated = raw_zip(&[
        RawMember::stored(b"Catalog.xml", &catalog),
        RawMember::stored(b"Hardware.xml", &hardware),
        RawMember::stored(b"OtherHardware.xml", &hardware),
        RawMember::stored(b"Application.xml", &app),
    ]);
    let r = inspect_distribution("ambiguous-split", &repeated).unwrap();
    let product = r
        .references
        .iter()
        .find(|r| r.attribute == "ProductRefId")
        .unwrap();
    assert_eq!(product.state, State::Ambiguous);
    assert_eq!(product.candidates.len(), 2);
    assert!(!r.complete);
    assert!(r
        .limitations
        .contains(&DistributionLimitation::RegistrationNotVerified));
}
#[test]
fn repeated_declared_manufacturer_ids_stay_separate_in_one_original_document() {
    let source=br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><Catalog><CatalogItem Id="C" ProductRefId="P"/></Catalog><Hardware><Hardware Id="H"><Products><Product Id="P"/></Products></Hardware></Hardware></Manufacturer><Manufacturer RefId="M"><Catalog><CatalogItem Id="C" ProductRefId="P"/></Catalog><Hardware><Hardware Id="H"><Products><Product Id="P"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    let r = inspect_distribution("same-declared-id", source).unwrap();
    assert_eq!(r.catalog_items.len(), 2);
    assert_eq!(r.hardware.len(), 2);
    let links = r
        .references
        .iter()
        .filter(|r| r.attribute == "ProductRefId")
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 2);
    for link in &links {
        assert_eq!(link.state, State::Resolved);
        assert_eq!(link.resolution_basis, Basis::OriginalScope);
        assert_eq!(link.candidates.len(), 1);
    }
    assert_ne!(
        r.resolve_coordinate(links[0].manufacturer_source)
            .unwrap()
            .element_ordinal,
        r.resolve_coordinate(links[1].manufacturer_source)
            .unwrap()
            .element_ordinal
    );
}
