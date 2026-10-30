//! Pins the presence-only matcher, private evidence order and explicit structural scope.

use super::*;
use crate::code::parse_program_code;
use crate::download_plan::{check_customer_adjustable, DownloadPlanError};

fn parse(body: &str) -> ProgramCode {
    let xml =
        format!(r#"<ApplicationProgram Id="P" xmlns:x="urn:foreign">{body}</ApplicationProgram>"#);
    parse_program_code("synthetic.xml", xml.as_bytes(), "P")
        .unwrap()
        .unwrap()
}

// Actual retained structural slots, excluding raw token storage in both representations.
fn retained_structural_capacity(declarations: &CustomerAdjustableDeclarations) -> usize {
    declarations.declarations.capacity() + declarations.paths.capacity()
}

fn declaration_path(
    declarations: &CustomerAdjustableDeclarations,
    declaration: &Declaration,
) -> Vec<Occurrence> {
    let mut path = Vec::new();
    let mut parent = declaration.path;
    while let Some(index) = parent {
        let node = &declarations.paths[index];
        path.push(node.occurrence);
        parent = node.parent;
    }
    path.reverse();
    path
}

fn deep_marker_source(depth: usize, markers: usize) -> (String, usize) {
    let mut body =
        String::from(r#"<ApplicationProgram Id="P" xmlns="http://knx.org/xml/project/20">"#);
    body.push_str(&"<ModuleDefs><ModuleDef>".repeat(depth));
    body.push_str("<Static><Parameters>");
    for marker in 0..markers {
        body.push_str(&format!(
            r#"<Parameter Id="same" CustomerAdjustable="raw-{}" />"#,
            marker % 3
        ));
    }
    body.push_str("</Parameters></Static>");
    body.push_str(&"</ModuleDef></ModuleDefs>".repeat(depth));
    body.push_str("</ApplicationProgram>");
    // Count the generated structural edges, independently of the production projection.
    (body, 2 * depth + 2 + markers)
}

#[test]
fn retained_structural_storage_is_linear_for_deep_many_markers() {
    let (xml, source_edges) = deep_marker_source(128, 256);
    let code = parse_program_code("synthetic.xml", xml.as_bytes(), "P")
        .unwrap()
        .unwrap();
    assert_eq!(code.customer_adjustable.counts().module_parameters, 256);
    let capacity = retained_structural_capacity(&code.customer_adjustable);
    // Allow geometric Vec growth for shared path nodes and declaration leaves, not depth × markers.
    let linear_bound = 4 * source_edges;
    eprintln!("resource depth=128 markers=256 source_edges={source_edges} retained_slots={capacity} bound={linear_bound}");
    assert!(
        capacity <= linear_bound,
        "retained structural storage must be linear in source edges: {capacity} > {linear_bound}"
    );
}

#[test]
fn varied_deep_marker_shapes_keep_linear_storage_and_every_literal_path_in_order() {
    use Occurrence as O;
    for (depth, markers) in [
        (0, 512),
        (1, 1),
        (8, 64),
        (64, 1),
        (64, 512),
        (128, 256),
        (256, 32),
    ] {
        let (xml, source_edges) = deep_marker_source(depth, markers);
        let code = parse_program_code("synthetic.xml", xml.as_bytes(), "P")
            .unwrap()
            .unwrap();
        let evidence = &code.customer_adjustable;
        assert_eq!(evidence.declarations.len(), markers);
        let capacity = retained_structural_capacity(evidence);
        // Fixed-depth root helpers may repeat their three-edge path; modules share deep prefixes.
        let linear_bound = 8 * source_edges;
        eprintln!(
            "shape depth={depth} markers={markers} source_edges={source_edges} retained_slots={capacity} bound={linear_bound}"
        );
        assert!(capacity <= linear_bound);
        let mut leaves = std::collections::BTreeSet::new();
        for (index, declaration) in evidence.declarations.iter().enumerate() {
            let mut expected = Vec::new();
            for _ in 0..depth {
                expected.extend([O::ModuleDefs(1), O::ModuleDef(1)]);
            }
            expected.extend([O::Static(1), O::Parameters(1), O::Parameter(index + 1)]);
            let path = declaration_path(evidence, declaration);
            assert_eq!(path, expected, "depth={depth} marker={index}");
            let Some(O::Parameter(ordinal)) = path.last() else {
                panic!("missing parameter leaf");
            };
            assert!(leaves.insert(*ordinal), "all marked paths must be distinct");
            assert_eq!(declaration.token, format!("raw-{}", index % 3));
            assert_eq!(
                declaration.context,
                if depth == 0 {
                    CustomerAdjustableContext::RootParameter
                } else {
                    CustomerAdjustableContext::ModuleParameter
                }
            );
        }
        assert_eq!(leaves.len(), markers);
        assert_eq!(evidence.clone(), *evidence);
        assert!(matches!(
            check_customer_adjustable(&code),
            Err(DownloadPlanError::CustomerAdjustableParameters(counts)) if counts.total() == markers
        ));
    }
}

#[test]
fn occurrence_equality_ignores_unused_prefixes_but_not_structural_identity() {
    let marked = r#"<ModuleDef><Static><Parameters><Parameter CustomerAdjustable="raw" /></Parameters></Static></ModuleDef>"#;
    let a = parse(&format!("<ModuleDefs><ModuleDef />{marked}</ModuleDefs>"));
    let b = parse(&format!(
        "<ModuleDefs><ModuleDef><ModuleDefs><ModuleDef><Static /></ModuleDef></ModuleDefs></ModuleDef>{marked}</ModuleDefs>"
    ));
    assert_ne!(
        a.customer_adjustable.paths.len(),
        b.customer_adjustable.paths.len()
    );
    assert_eq!(a.customer_adjustable, b.customer_adjustable);
    let c = parse(&format!("<ModuleDefs>{marked}</ModuleDefs>"));
    assert_ne!(a.customer_adjustable, c.customer_adjustable);
    let empty =
        parse("<ModuleDefs><ModuleDef><Static><Parameters /></Static></ModuleDef></ModuleDefs>");
    assert_eq!(
        empty.customer_adjustable,
        CustomerAdjustableDeclarations::default()
    );
    let (xml, _) = deep_marker_source(4096, 2);
    let deep = parse_program_code("synthetic.xml", xml.as_bytes(), "P")
        .unwrap()
        .unwrap();
    let cloned = deep.customer_adjustable.clone();
    assert_eq!(cloned, deep.customer_adjustable);
    drop(cloned);
    drop(deep);
}

#[test]
fn all_present_tokens_are_retained_and_refused_without_interpretation() {
    for (raw, expected) in [
        ("true", "true"),
        ("Sync", "Sync"),
        ("false", "false"),
        ("0", "0"),
        ("1", "1"),
        ("TRUE", "TRUE"),
        (" \t\n ", "    "),
        ("", ""),
        ("privateSentinel", "privateSentinel"),
        ("a&amp;b&#x9;c", "a&b\tc"),
    ] {
        for options in [false, true] {
            let body = if options {
                format!(r#"<Static><Options CustomerAdjustableParameters="{raw}" /></Static>"#)
            } else {
                format!(
                    r#"<Static><Parameters><Parameter Id="same" CustomerAdjustable="{raw}" /></Parameters></Static>"#
                )
            };
            let code = parse(&body);
            let declarations = &code.customer_adjustable.declarations;
            assert_eq!(declarations.len(), 1, "{raw:?}");
            assert_eq!(declarations[0].token, expected, "{raw:?}");
            assert_eq!(
                declarations[0].context,
                if options {
                    CustomerAdjustableContext::RootOptions
                } else {
                    CustomerAdjustableContext::RootParameter
                }
            );
            assert!(
                matches!(check_customer_adjustable(&code), Err(DownloadPlanError::CustomerAdjustableParameters(counts)) if counts.total() == 1)
            );
        }
    }
}

#[test]
fn root_union_duplicates_and_replaced_options_keep_literal_order_and_paths() {
    let code = parse(
        r#"<Static>
<Options CustomerAdjustableParameters="first" /><Options />
<Parameters><Parameter Id="same" CustomerAdjustable="false" />
<Parameter Id="same" CustomerAdjustable="" />
<Union><Parameter Id="same" CustomerAdjustable="0" /><Parameter Id="same" CustomerAdjustable="1" /></Union>
</Parameters><Options CustomerAdjustableParameters="last" /><Options />
</Static>"#,
    );
    use CustomerAdjustableContext as C;
    use Occurrence as O;
    let expected = [
        (C::RootOptions, "first", vec![O::Static(1), O::Options(1)]),
        (
            C::RootParameter,
            "false",
            vec![O::Static(1), O::Parameters(3), O::Parameter(1)],
        ),
        (
            C::RootParameter,
            "",
            vec![O::Static(1), O::Parameters(3), O::Parameter(2)],
        ),
        (
            C::RootUnionParameter,
            "0",
            vec![O::Static(1), O::Parameters(3), O::Union(3), O::Parameter(1)],
        ),
        (
            C::RootUnionParameter,
            "1",
            vec![O::Static(1), O::Parameters(3), O::Union(3), O::Parameter(2)],
        ),
        (C::RootOptions, "last", vec![O::Static(1), O::Options(4)]),
    ];
    assert_eq!(code.customer_adjustable.declarations.len(), expected.len());
    for (actual, (context, token, path)) in
        code.customer_adjustable.declarations.iter().zip(expected)
    {
        assert_eq!(actual.context, context);
        assert_eq!(actual.token, token);
        assert_eq!(declaration_path(&code.customer_adjustable, actual), path);
    }
    assert!(
        code.options.is_empty(),
        "old last-Options projection is unchanged"
    );
}

#[test]
fn both_module_anchors_and_only_explicit_nested_edges_are_ordered_without_translation() {
    let code = parse(
        r#"<ModuleDefs><ModuleDef Id="same"><Static><Parameters>
<Parameter Id="same" CustomerAdjustable="direct" />
<Union><Parameter Id="same" CustomerAdjustable="union" /></Union></Parameters>
<ModuleDefs><ModuleDef Id="same"><Static><Parameters><Parameter Id="same" CustomerAdjustable="static-nested" /></Parameters></Static></ModuleDef></ModuleDefs>
</Static><ModuleDefs><ModuleDef Id="same"><Static><Parameters><Parameter Id="same" CustomerAdjustable="def-nested" /></Parameters></Static></ModuleDef></ModuleDefs>
</ModuleDef><ModuleDef Id="same"><Static><Parameters><Parameter Id="same" CustomerAdjustable="sibling" /></Parameters></Static></ModuleDef></ModuleDefs>
<Static><ModuleDefs><ModuleDef><Static><Parameters><Parameter Id="same" CustomerAdjustable="static-anchor" /></Parameters>
<Code><AbsoluteSegment Id="foreign" Address="42" Size="1" /></Code></Static></ModuleDef></ModuleDefs></Static>"#,
    );
    use CustomerAdjustableContext as C;
    use Occurrence as O;
    let expected = [
        (
            C::ModuleParameter,
            "direct",
            vec![
                O::ModuleDefs(1),
                O::ModuleDef(1),
                O::Static(1),
                O::Parameters(1),
                O::Parameter(1),
            ],
        ),
        (
            C::ModuleUnionParameter,
            "union",
            vec![
                O::ModuleDefs(1),
                O::ModuleDef(1),
                O::Static(1),
                O::Parameters(1),
                O::Union(2),
                O::Parameter(1),
            ],
        ),
        (
            C::ModuleParameter,
            "static-nested",
            vec![
                O::ModuleDefs(1),
                O::ModuleDef(1),
                O::Static(1),
                O::ModuleDefs(2),
                O::ModuleDef(1),
                O::Static(1),
                O::Parameters(1),
                O::Parameter(1),
            ],
        ),
        (
            C::ModuleParameter,
            "def-nested",
            vec![
                O::ModuleDefs(1),
                O::ModuleDef(1),
                O::ModuleDefs(2),
                O::ModuleDef(1),
                O::Static(1),
                O::Parameters(1),
                O::Parameter(1),
            ],
        ),
        (
            C::ModuleParameter,
            "sibling",
            vec![
                O::ModuleDefs(1),
                O::ModuleDef(2),
                O::Static(1),
                O::Parameters(1),
                O::Parameter(1),
            ],
        ),
        (
            C::ModuleParameter,
            "static-anchor",
            vec![
                O::Static(2),
                O::ModuleDefs(1),
                O::ModuleDef(1),
                O::Static(1),
                O::Parameters(1),
                O::Parameter(1),
            ],
        ),
    ];
    assert_eq!(code.customer_adjustable.declarations.len(), expected.len());
    for (actual, (context, token, path)) in
        code.customer_adjustable.declarations.iter().zip(expected)
    {
        assert_eq!(actual.context, context);
        assert_eq!(actual.token, token);
        assert_eq!(declaration_path(&code.customer_adjustable, actual), path);
    }
    assert!(code.parameters.is_empty());
    assert!(code.segments.is_empty());
    assert!(code.relative_segments.is_empty());
    assert_eq!(
        code.customer_adjustable.counts(),
        CustomerAdjustableCounts {
            module_parameters: 5,
            module_union_parameters: 1,
            ..Default::default()
        }
    );
}

#[test]
fn module_attributes_require_the_unqualified_key_in_both_anchors() {
    for (attrs, token) in [
        (r#"x:CustomerAdjustable="foreign""#, None),
        (
            r#"x:CustomerAdjustable="foreign" CustomerAdjustable="""#,
            Some(""),
        ),
        (
            r#"CustomerAdjustable="false" x:CustomerAdjustable="foreign""#,
            Some("false"),
        ),
    ] {
        let modules = format!(
            r#"<ModuleDefs><ModuleDef><Static><Parameters><Parameter Id="same" {attrs} /></Parameters></Static></ModuleDef></ModuleDefs>"#
        );
        for body in [modules.clone(), format!("<Static>{modules}</Static>")] {
            let code = parse(&body);
            let evidence = &code.customer_adjustable;
            assert_eq!(evidence.declarations.len(), usize::from(token.is_some()));
            assert_eq!(
                evidence.counts().module_parameters,
                usize::from(token.is_some())
            );
            if let Some(token) = token {
                assert_eq!(evidence.declarations[0].token, token);
                assert_eq!(
                    evidence.declarations[0].context,
                    CustomerAdjustableContext::ModuleParameter
                );
                assert!(matches!(
                    check_customer_adjustable(&code),
                    Err(DownloadPlanError::CustomerAdjustableParameters(_))
                ));
            } else {
                assert!(check_customer_adjustable(&code).is_ok());
            }
        }
    }
}

#[test]
fn foreign_attributes_alone_never_match_and_unqualified_wins_both_orders() {
    for (tag, attribute, tail) in [
        ("Options", "CustomerAdjustableParameters", ""),
        (
            "Parameters><Parameter Id=\"p\"",
            "CustomerAdjustable",
            "</Parameters>",
        ),
    ] {
        let code = parse(&format!(
            r#"<Static><{tag} x:{attribute}="foreign" />{tail}</Static>"#
        ));
        assert_eq!(code.customer_adjustable.counts().total(), 0);
        assert!(check_customer_adjustable(&code).is_ok());
        for attrs in [
            format!(r#"x:{attribute}="foreign" {attribute}="""#),
            format!(r#"{attribute}="false" x:{attribute}="foreign""#),
        ] {
            let code = parse(&format!("<Static><{tag} {attrs} />{tail}</Static>"));
            let expected = if attrs.starts_with('x') { "" } else { "false" };
            assert_eq!(code.customer_adjustable.declarations[0].token, expected);
        }
    }
}

#[test]
fn excluded_descendants_and_module_options_do_not_acquire_the_new_category() {
    let parameter =
        r#"<Parameters><Parameter Id="p" CustomerAdjustable="privateSentinel" /></Parameters>"#;
    for body in [
        format!("<Dynamic>{parameter}</Dynamic>"),
        format!("<Static><Extension>{parameter}</Extension></Static>"),
        format!("<Static><Wrapper>{parameter}</Wrapper></Static>"),
        format!("<Static><Parameters><Wrapper>{parameter}</Wrapper></Parameters></Static>"),
        format!("<ModuleDefs><ModuleDef><Wrapper><Static>{parameter}</Static></Wrapper></ModuleDef></ModuleDefs>"),
        format!("<ModuleDefs><Wrapper><ModuleDef><Static>{parameter}</Static></ModuleDef></Wrapper></ModuleDefs>"),
        r#"<ModuleDefs><ModuleDef><Static><Options CustomerAdjustableParameters="Sync" /></Static></ModuleDef></ModuleDefs>"#.into(),
        format!("<ModuleDefs><ModuleDef><Dynamic>{parameter}</Dynamic><Static><Extension>{parameter}</Extension></Static></ModuleDef></ModuleDefs>"),
        r#"<Options CustomerAdjustableParameters="Sync" />"#.into(),
    ] {
        let code = parse(&body);
        assert_eq!(code.customer_adjustable.counts().total(), 0, "{body}");
        assert!(check_customer_adjustable(&code).is_ok());
    }
}

#[test]
fn duplicate_same_attribute_keeps_the_existing_xml_error() {
    let xml = br#"<ApplicationProgram Id="P"><Static><Parameters><Parameter Id="p" CustomerAdjustable="true" CustomerAdjustable="false" /></Parameters></Static></ApplicationProgram>"#;
    assert!(matches!(
        parse_program_code("synthetic.xml", xml, "P"),
        Err(CodeError::Database(crate::ProductDbError::Xml { .. }))
    ));
}

#[test]
fn new_projection_debug_and_typed_error_expose_counts_only() {
    let code = parse(
        r#"<Static><Parameters><Parameter Id="privateIdentifier" CustomerAdjustable="privateSentinel" /></Parameters></Static>"#,
    );
    assert_eq!(
        code.customer_adjustable.declarations[0].token,
        "privateSentinel"
    );
    let error = check_customer_adjustable(&code).unwrap_err();
    for public in [
        format!("{:?}", code.customer_adjustable),
        format!("{error:?}"),
        error.to_string(),
    ] {
        for private in ["privateSentinel", "privateIdentifier", "Static(1)"] {
            assert!(!public.contains(private));
        }
    }
}

#[test]
fn deeply_nested_explicit_modules_use_iterative_storage_not_recursive_calls() {
    let mut body = "<ModuleDefs><ModuleDef>".repeat(4096);
    body.push_str(
        r#"<Static><Parameters><Parameter CustomerAdjustable="" /></Parameters></Static>"#,
    );
    body.push_str(&"</ModuleDef></ModuleDefs>".repeat(4096));
    let code = parse(&body);
    assert_eq!(code.customer_adjustable.counts().module_parameters, 1);
    assert_eq!(
        declaration_path(
            &code.customer_adjustable,
            &code.customer_adjustable.declarations[0]
        )
        .len(),
        8195
    );
}
