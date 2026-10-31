//! Pins review findings: Dynamic unknowns, lexical type scope, fallback origins, exact ZIP bytes.

use crate::distribution_inspection::*;
use crate::standalone_packages::wp16::{raw_zip, RawMember};
use knx_productdb::distribution::{
    inspect_distribution_with_limits, DistributionInspection, DistributionLimits as L,
    DistributionReferenceState as State, DistributionResolutionBasis as Basis,
};

fn source(application: &str) -> Vec<u8> {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms>{application}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
    .into_bytes()
}

/// Every unknown as (kind, resolved path, raw name), for exact set comparisons.
fn unknowns(r: &DistributionInspection) -> Vec<(String, String, String)> {
    let mut out = r
        .unknowns
        .iter()
        .map(|u| {
            let path = r.resolve_coordinate(u.source).unwrap().path;
            (u.kind.clone(), path, u.raw_name.clone())
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

const NESTED: &str = r#"<ApplicationProgram Id="A"><Static><Parameters><Parameter Id="P" Value="1"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs><ModuleDefs><ModuleDef Id="MD"><Static><Parameters><Parameter Id="MP" Value="1"/></Parameters><ParameterRefs><ParameterRef Id="MR" RefId="MP"/></ParameterRefs></Static><Dynamic><ChannelIndependentBlock><ParameterBlock Id="MB" Text="m" ModuleKnob="1"><ParameterRefRef RefId="MR"/><ModuleGadget/></ParameterBlock></ChannelIndependentBlock></Dynamic></ModuleDef></ModuleDefs></Static><Dynamic><Channel Id="C" Text="c" Name="n" Number="1"><ParameterBlock Id="B" Text="b" Surprise="yes"><ParameterRefRef RefId="R"/><Gadget Knob="1"/></ParameterBlock></Channel><Module Id="I" RefId="MD"/></Dynamic></ApplicationProgram>"#;

#[test]
fn nested_dynamic_unknowns_are_inventoried_in_application_and_module_scopes() {
    let r = inspect_distribution("nested", &source(NESTED)).unwrap();
    assert_eq!(
        r.applications[0].dynamic_state,
        DistributionPassState::PartiallyEvaluated
    );
    let u = unknowns(&r);
    let dynamic = u
        .iter()
        .filter(|(_, path, _)| path.contains("/Dynamic[1]"))
        .cloned()
        .collect::<Vec<_>>();
    let app =
        "/KNX[1]/ManufacturerData[1]/Manufacturer[1]/ApplicationPrograms[1]/ApplicationProgram[1]";
    let block = format!("{app}/Dynamic[1]/Channel[1]/ParameterBlock[1]");
    let module = format!(
        "{app}/Static[1]/ModuleDefs[1]/ModuleDef[1]/Dynamic[1]/ChannelIndependentBlock[1]/ParameterBlock[1]"
    );
    let expected = [
        ("attribute", block.clone(), "Surprise"),
        ("attribute", format!("{block}/Gadget[1]"), "Knob"),
        ("element", format!("{block}/Gadget[1]"), "Gadget"),
        ("attribute", module.clone(), "ModuleKnob"),
        (
            "element",
            format!("{module}/ModuleGadget[1]"),
            "ModuleGadget",
        ),
    ]
    .map(|(k, p, n)| (k.to_string(), p, n.to_string()));
    let mut expected = expected.to_vec();
    expected.sort();
    assert_eq!(dynamic, expected, "all unknowns: {u:#?}");
}

#[test]
fn grid_layout_vocabulary_is_known_only_where_the_grid_reader_reads_it() {
    let application = r#"<ApplicationProgram Id="A"><Static><Parameters><Parameter Id="P" Value="1"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs></Static><Dynamic><ChannelIndependentBlock><ParameterBlock Id="B" Text="b" Layout="Grid"><Rows><Row Id="r1" Text="row" CollapseIfEmpty="true"/></Rows><Columns><Column Id="c1" Text="col" Width="50%"/></Columns><ParameterRefRef RefId="R" Cell="1,1" IndentLevel="1"/><Row Id="stray"/></ParameterBlock><ParameterRefRef RefId="R" Cell="1,1" IndentLevel="2"/></ChannelIndependentBlock></Dynamic></ApplicationProgram>"#;
    let r = inspect_distribution("grid", &source(application)).unwrap();
    let block = "/KNX[1]/ManufacturerData[1]/Manufacturer[1]/ApplicationPrograms[1]/ApplicationProgram[1]/Dynamic[1]/ChannelIndependentBlock[1]";
    let mut expected = vec![
        (
            "element".to_string(),
            format!("{block}/ParameterBlock[1]/Row[1]"),
            "Row".to_string(),
        ),
        (
            "attribute".to_string(),
            format!("{block}/ParameterBlock[1]/Row[1]"),
            "Id".to_string(),
        ),
        (
            "attribute".to_string(),
            format!("{block}/ParameterRefRef[1]"),
            "Cell".to_string(),
        ),
        (
            "attribute".to_string(),
            format!("{block}/ParameterRefRef[1]"),
            "IndentLevel".to_string(),
        ),
    ];
    expected.sort();
    let dynamic = unknowns(&r)
        .into_iter()
        .filter(|(_, path, _)| path.contains("/Dynamic[1]"))
        .collect::<Vec<_>>();
    assert_eq!(dynamic, expected);
}

#[test]
fn uninspected_dynamic_subtree_is_unknown_rather_than_covered() {
    // Without an application Id the Static pass succeeds but Dynamic is never read. A duplicated
    // Parameter instead fails the Static pass; both must leave Dynamic inventoried as unknown.
    let missing_id = NESTED.replace(r#"<ApplicationProgram Id="A">"#, "<ApplicationProgram>");
    let duplicate = NESTED.replace(
        r#"<Parameter Id="P" Value="1"/>"#,
        r#"<Parameter Id="P" Value="1"/><Parameter Id="P" Value="2"/>"#,
    );
    for (label, application, static_state) in [
        ("missing id", missing_id, DistributionPassState::Parsed),
        (
            "duplicate",
            duplicate,
            DistributionPassState::PartiallyEvaluated,
        ),
    ] {
        let r = inspect_distribution(label, &source(&application)).unwrap();
        assert_eq!(
            r.applications[0].dynamic_state,
            DistributionPassState::NotAttempted,
            "{label}"
        );
        let pass = r
            .reader_passes
            .iter()
            .find(|p| p.reader == "program-static");
        assert_eq!(pass.unwrap().state, static_state, "{label}");
        let u = unknowns(&r);
        for element in [
            "Dynamic",
            "Channel",
            "ParameterBlock",
            "ParameterRefRef",
            "Module",
        ] {
            assert!(
                u.iter().any(|(k, path, n)| k == "element"
                    && n == element
                    && path.contains("ApplicationProgram[1]/Dynamic[1]")),
                "{label}: {element} must stay unknown: {u:#?}"
            );
        }
        assert!(
            u.iter().any(|(k, _, n)| k == "attribute" && n == "Number"),
            "{label}"
        );
    }
}

#[test]
fn module_parameter_type_resolves_lexically_through_the_enclosing_application() {
    let application = r#"<ApplicationProgram Id="A"><Static><ParameterTypes><ParameterType Id="PT-app"><TypeNumber Type="unsignedInt" SizeInBit="8" minInclusive="0" maxInclusive="1"/></ParameterType></ParameterTypes><ModuleDefs><ModuleDef Id="MD"><Static><Parameters><Parameter Id="MP" ParameterType="PT-app" Value="0"/><Parameter Id="MQ" ParameterType="PT-none" Value="0"/></Parameters></Static></ModuleDef></ModuleDefs></Static></ApplicationProgram>"#;
    let r = inspect_distribution("lexical", &source(application)).unwrap();
    let typed = |target: &str| {
        r.references
            .iter()
            .find(|x| x.attribute == "ParameterType" && x.declared_target == target)
            .unwrap()
    };
    assert_eq!(typed("PT-app").state, State::Resolved);
    assert_eq!(
        typed("PT-app").resolution_basis,
        Basis::EnclosingApplicationScope
    );
    assert_eq!(typed("PT-app").candidates.len(), 1);
    assert_eq!(typed("PT-none").state, State::Dangling);
    assert_eq!(typed("PT-none").resolution_basis, Basis::OriginalScope);
}

#[test]
fn missing_declared_identities_are_diagnosed_instead_of_vanishing() {
    let bytes = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A"><Static><Parameters><Parameter Id="P"/></Parameters><ParameterRefs><ParameterRef RefId="P"/></ParameterRefs></Static></ApplicationProgram></ApplicationPrograms><Hardware><Hardware Id="H"><Products><Product Text="no id"/></Products><Hardware2Programs><Hardware2Program Id="H2P"><ApplicationProgramRef/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    let r = inspect_distribution("missing", bytes).unwrap();
    let mut missing = r
        .diagnostics
        .iter()
        .filter(|d| d.code == "missingDeclaredIdentity")
        .map(|d| {
            let path = r.resolve_coordinate(d.source.unwrap()).unwrap().path;
            path.rsplit('/').next().unwrap().to_string()
        })
        .collect::<Vec<_>>();
    missing.sort();
    assert_eq!(
        missing,
        ["ApplicationProgramRef[1]", "ParameterRef[1]", "Product[1]"]
    );
}

#[test]
fn zip_report_bytes_stay_exact_and_inclusive_across_many_members() {
    let product = source(NESTED);
    let mut members = vec![RawMember::stored(b"data/product.xml", &product)];
    let names = (0..64)
        .map(|i| format!("data/empty-{i}.txt"))
        .collect::<Vec<_>>();
    members.extend(names.iter().map(|n| RawMember::stored(n.as_bytes(), b"")));
    let bytes = raw_zip(&members);
    let r = inspect_distribution("many", &bytes).unwrap();
    assert_eq!(r.members.len(), 65);
    let size = serde_json::to_vec(&r).unwrap().len();
    for (cap, positive) in [(size - 1, false), (size, true)] {
        let result = inspect_distribution_with_limits(
            "many",
            &bytes,
            L {
                max_report_bytes: cap,
                ..L::default()
            },
        );
        assert_eq!(result.is_ok(), positive, "cap={cap} size={size}");
    }
}
