//! Tests declared dependency evidence: inventoried and source-bound, never executed or applied.

use crate::distribution_inspection::*;
use crate::standalone_packages::wp16::{raw_zip, RawMember};
use knx_productdb::distribution::{
    inspect_distribution_with_limits, DistributionAssignmentForm as Form,
    DistributionConditionApplicability, DistributionDependencyDisposition,
    DistributionDependencyEvidence as Evidence, DistributionDependencyKind as Kind,
    DistributionEndpointRole as Role, DistributionFunctionResolution,
    DistributionGraphCycle as Cycle, DistributionImpactScope as Impact, DistributionInspection,
    DistributionInspectionError as E, DistributionLimits as L, DistributionPresence as Presence,
    DistributionReferenceState as State, DistributionSourceRef,
};
use std::collections::{BTreeSet, HashSet};

// Independently authored fixtures; no vendor XML, scripts or handlers are copied.
const DECLARATIONS: &[u8] = include_bytes!("distribution_dependencies/declarations.xml");
const EDGE: &[u8] = include_bytes!("distribution_dependencies/edge-cases.xml");
const MODULE: &[u8] = include_bytes!("distribution_dependencies/module-scope.xml");
const AMBIGUOUS: &[u8] = include_bytes!("distribution_dependencies/ambiguous-ref.xml");
const PURE: &[u8] = include_bytes!("distribution_dependencies/pure-choose.xml");

fn inspect(bytes: &[u8]) -> DistributionInspection {
    inspect_distribution("authored.xml", bytes).unwrap()
}
fn rows(r: &DistributionInspection, kind: Kind) -> Vec<&Evidence> {
    r.dependency_evidence
        .iter()
        .filter(|e| e.kind == kind)
        .collect()
}
fn original<'a>(r: &DistributionInspection, bytes: &'a [u8], s: DistributionSourceRef) -> &'a [u8] {
    let c = r.resolve_coordinate(s).unwrap();
    &bytes[c.start_byte as usize..c.end_byte as usize]
}
fn attr<'a>(e: &'a Evidence, name: &str) -> (Presence, Option<&'a str>) {
    let a = e
        .attributes
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("{name} not reported"));
    (a.presence, a.value.as_deref())
}
type Endpoint = (Option<String>, Option<State>, Vec<DistributionSourceRef>);
fn endpoints(e: &Evidence, role: Role) -> Vec<Endpoint> {
    e.endpoints
        .iter()
        .filter(|p| p.role == role)
        .map(|p| (p.raw_ref.clone(), p.reference_state, p.candidates.clone()))
        .collect()
}
fn resolved(r: &DistributionInspection, id: &str) -> Endpoint {
    (
        Some(id.into()),
        Some(State::Resolved),
        parameter_refs(r, id),
    )
}
/// ParameterRef declarations with this identity, in element order.
fn parameter_refs(r: &DistributionInspection, id: &str) -> Vec<DistributionSourceRef> {
    r.identities
        .iter()
        .filter(|i| i.kind == "ParameterRef" && i.declared_id == id)
        .map(|i| i.source)
        .collect()
}
fn fallbacks(
    r: &DistributionInspection,
) -> Vec<(String, Option<String>, DistributionFallbackOrigin)> {
    r.applications[0]
        .parameter_fallbacks
        .iter()
        .map(|f| (f.parameter_ref_id.clone(), f.value.clone(), f.origin))
        .collect()
}
fn manufacturer(r: &DistributionInspection) -> DistributionSourceRef {
    r.references[0].manufacturer_source
}

#[test]
fn declaration_inventory_does_not_apply_assign_or_calculation() {
    let pure = inspect(PURE);
    let r = inspect(DECLARATIONS);
    assert_eq!(
        r.dependency_evidence
            .iter()
            .map(|e| e.kind)
            .collect::<Vec<_>>(),
        [
            Kind::ParameterCalculation,
            Kind::ParameterValidation,
            Kind::Script,
            Kind::Assign,
            Kind::ButtonHandler
        ],
        "every admitted declaration is reported once, in element order"
    );
    assert!(pure.dependency_evidence.is_empty());
    let raw = vec![
        (
            "R-A".to_string(),
            Some("1".to_string()),
            DistributionFallbackOrigin::Parameter,
        ),
        (
            "R-B".to_string(),
            Some("2".to_string()),
            DistributionFallbackOrigin::Parameter,
        ),
    ];
    assert_eq!(fallbacks(&pure), raw);
    assert_eq!(
        fallbacks(&r),
        raw,
        "Assign and calculation never change fallbacks"
    );

    let assign = rows(&r, Kind::Assign)[0];
    assert!(original(&r, DECLARATIONS, assign.source).starts_with(b"<Assign "));
    assert_eq!(endpoints(assign, Role::AssignTarget), [resolved(&r, "R-B")]);
    assert_eq!(endpoints(assign, Role::AssignSource), [resolved(&r, "R-A")]);
    assert_eq!(assign.assignment_form, Some(Form::SourceReference));
    let calc = rows(&r, Kind::ParameterCalculation)[0];
    assert_eq!(
        endpoints(calc, Role::CalculationLeft),
        [resolved(&r, "R-A")]
    );
    assert_eq!(
        endpoints(calc, Role::CalculationRight),
        [resolved(&r, "R-B")]
    );
    for e in &r.dependency_evidence {
        assert_eq!(
            e.disposition,
            DistributionDependencyDisposition::RetainedNotExecuted
        );
        assert_eq!(e.application_source, r.applications[0].source);
        assert_eq!(e.manufacturer_source, manufacturer(&r));
        for p in &e.endpoints {
            assert!(original(&r, DECLARATIONS, p.source).starts_with(b"<"));
        }
    }
    let json = serde_json::to_string(&r).unwrap();
    assert!(
        !json.contains("return 999"),
        "script text is never echoed or run"
    );
    assert!(!json.contains("authoredForward()"));
    assert!(!r.complete);
    for limitation in [
        DistributionLimitation::InitializationNotPerformed,
        DistributionLimitation::WriteAuthorityNotPerformed,
        DistributionLimitation::DependenciesNotExecuted,
    ] {
        assert!(r.limitations.contains(&limitation), "{limitation:?}");
    }
    assert_eq!(r.format_version, 3);
    assert_eq!(serde_json::to_value(&r).unwrap()["formatVersion"], 3);
}

#[test]
fn source_calculation_inventory_is_not_native_authority() {
    let r = inspect(DECLARATIONS);
    let calc = rows(&r, Kind::ParameterCalculation);
    assert_eq!(calc.len(), 1);
    let calc = calc[0];
    assert!(original(&r, DECLARATIONS, calc.source).starts_with(b"<ParameterCalculation "));
    assert_eq!(attr(calc, "Id"), (Presence::PresentValue, Some("C")));
    assert_eq!(
        attr(calc, "LRTransformationFunc"),
        (Presence::PresentValue, Some("authoredForward"))
    );
    assert_eq!(
        attr(calc, "RLTransformationFunc"),
        (Presence::PresentValue, Some("authoredReverse"))
    );
    assert_eq!(calc.attributes.len(), 3);
    assert_eq!(
        calc.member_lists
            .iter()
            .map(|m| (m.role, m.presence))
            .collect::<Vec<_>>(),
        [
            (Role::CalculationLeft, Presence::PresentValue),
            (Role::CalculationRight, Presence::PresentValue)
        ]
    );
    let lists = calc
        .member_lists
        .iter()
        .map(|m| original(&r, DECLARATIONS, m.source.unwrap()))
        .collect::<Vec<_>>();
    assert!(lists[0].starts_with(b"<LParameters>") && lists[1].starts_with(b"<RParameters>"));
    // Side memberships, not an L x R expansion or a direction.
    assert_eq!(calc.endpoints.len(), 2);
    assert_eq!(
        calc.function_resolution,
        Some(DistributionFunctionResolution::NotAttempted)
    );
    assert_eq!(calc.impact_scope, Impact::ApplicationImpactUnknown);
    assert_eq!(calc.graph_cycle, Cycle::NotApplicable);
    assert_eq!(calc.module_definition_source, None);
    let text = serde_json::to_string(calc).unwrap();
    for word in ["input", "output", "direction", "trigger", "reverse\""] {
        assert!(!text.contains(word), "{word}: {text}");
    }
    // The exact reference allowlist now follows calculation memberships too.
    for p in &calc.endpoints {
        let reference = r
            .references
            .iter()
            .find(|x| x.source == p.source && x.attribute == "RefId")
            .expect("membership reference row");
        assert_eq!(reference.state, State::Resolved);
        assert_eq!(reference.candidates, p.candidates);
    }
    assert!(r
        .limitations
        .contains(&DistributionLimitation::WriteAuthorityNotPerformed));
}

#[test]
fn validation_declared_not_run_for_two_ordinary_values() {
    let r = inspect(DECLARATIONS);
    let v = rows(&r, Kind::ParameterValidation);
    assert_eq!(v.len(), 1);
    let v = v[0];
    assert!(original(&r, DECLARATIONS, v.source).starts_with(b"<ParameterValidation "));
    assert_eq!(attr(v, "Id"), (Presence::PresentValue, Some("V")));
    assert_eq!(
        attr(v, "ValidationFunc"),
        (Presence::PresentValue, Some("authoredCheck"))
    );
    assert_eq!(
        endpoints(v, Role::ValidationMember),
        [resolved(&r, "R-A"), resolved(&r, "R-B")]
    );
    assert_eq!(
        v.member_lists
            .iter()
            .map(|m| (m.role, m.presence))
            .collect::<Vec<_>>(),
        [(Role::ValidationMember, Presence::PresentValue)]
    );
    for (_, value, _) in fallbacks(&r) {
        let n: u32 = value.unwrap().parse().unwrap();
        assert!(
            (1..=64).contains(&n),
            "ordinary values inside the declared 1..64"
        );
    }
    let row = serde_json::to_value(v).unwrap();
    let keys = row
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        [
            "applicationSource",
            "assignmentForm",
            "attributes",
            "conditionContext",
            "content",
            "disposition",
            "endpoints",
            "functionResolution",
            "graphCycle",
            "graphCycleAnchor",
            "impactScope",
            "kind",
            "manufacturerSource",
            "memberLists",
            "moduleDefinitionSource",
            "source"
        ]
        .map(String::from)
        .into(),
        "no validation success/failure result is reported"
    );
    assert_eq!(row["disposition"], "retainedNotExecuted");
}

#[test]
fn handler_mode_and_opaque_script_are_not_function_resolution() {
    let r = inspect(DECLARATIONS);
    let b = rows(&r, Kind::ButtonHandler);
    assert_eq!(b.len(), 1);
    let b = b[0];
    assert!(original(&r, DECLARATIONS, b.source).starts_with(b"<Button "));
    assert_eq!(attr(b, "Id"), (Presence::PresentValue, Some("B")));
    assert_eq!(
        attr(b, "Text"),
        (Presence::PresentValue, Some("Passive action label"))
    );
    assert_eq!(
        attr(b, "EventHandler"),
        (Presence::PresentValue, Some("notProvenCallable"))
    );
    assert_eq!(
        attr(b, "EventHandlerOnline"),
        (Presence::PresentValue, Some("ConnectionOriented")),
        "online mode is metadata, not a second function"
    );
    assert!(b.endpoints.is_empty());
    assert_eq!(
        b.function_resolution,
        Some(DistributionFunctionResolution::NotAttempted)
    );
    assert_eq!(b.impact_scope, Impact::ApplicationImpactUnknown);

    let s = rows(&r, Kind::Script);
    assert_eq!(s.len(), 1);
    let s = s[0];
    assert!(original(&r, DECLARATIONS, s.source).starts_with(b"<Script>"));
    let text = std::str::from_utf8(DECLARATIONS).unwrap();
    let start = text.find("<Script>").unwrap() + "<Script>".len();
    let end = text.find("</Script>").unwrap();
    let content = s.content.as_ref().expect("raw content binding");
    assert_eq!(
        (content.start_byte, content.end_byte),
        (start as u64, end as u64)
    );
    assert_eq!(content.size, (end - start) as u64);
    assert_eq!(
        content.sha256,
        knx_productdb::sha256_hex(&DECLARATIONS[start..end])
    );
    assert!(DECLARATIONS[start..end].starts_with(b"<![CDATA["));
    assert!(s.endpoints.is_empty() && s.attributes.is_empty());
    assert_eq!(s.function_resolution, None, "no lexical function inventory");
    assert_eq!(s.impact_scope, Impact::ApplicationImpactUnknown);
    let json = serde_json::to_string(&r).unwrap();
    assert!(
        !json.contains("onlyInComment"),
        "comment text never becomes a callable"
    );

    let edge = inspect(EDGE);
    let buttons = rows(&edge, Kind::ButtonHandler);
    assert_eq!(buttons.len(), 2);
    for name in ["EventHandler", "EventHandlerOnline"] {
        assert_eq!(attr(buttons[0], name), (Presence::PresentEmpty, Some("")));
        assert_eq!(attr(buttons[1], name), (Presence::Absent, None));
    }
    assert_eq!(attr(buttons[1], "Text"), (Presence::Absent, None));
}

#[test]
fn declared_graph_cycle_and_unresolved_endpoints_are_not_runtime_effects() {
    let r = inspect(EDGE);
    let a = rows(&r, Kind::Assign);
    assert_eq!(a.len(), 6, "the foreign-namespace Assign is not an Assign");
    assert_eq!(
        a.iter().map(|e| e.assignment_form).collect::<Vec<_>>(),
        [
            Some(Form::SourceReference),
            Some(Form::SourceReference),
            Some(Form::LiteralValue),
            Some(Form::ConflictingDeclarations),
            Some(Form::MissingSourceAndValue),
            Some(Form::SourceReference)
        ]
    );
    assert_eq!(
        a.iter().map(|e| e.graph_cycle).collect::<Vec<_>>(),
        [
            Cycle::DeclaredGraphCycle,
            Cycle::DeclaredGraphCycle,
            Cycle::NotApplicable,
            Cycle::DeclaredGraphCycle,
            Cycle::NotApplicable,
            Cycle::NotApplicable
        ]
    );
    let edges = a
        .iter()
        .filter(|e| e.graph_cycle == Cycle::DeclaredGraphCycle)
        .map(|e| {
            (
                endpoints(e, Role::AssignSource)[0].2.clone(),
                endpoints(e, Role::AssignTarget)[0].2.clone(),
            )
        })
        .collect::<HashSet<_>>();
    let (ra, rb) = (parameter_refs(&r, "R-A"), parameter_refs(&r, "R-B"));
    assert_eq!(edges, HashSet::from([(ra.clone(), rb.clone()), (rb, ra)]));
    let anchors = a.iter().map(|e| e.graph_cycle_anchor).collect::<Vec<_>>();
    let first = Some(a[0].source);
    assert_eq!(anchors, [first, first, None, first, None, None]);
    assert_eq!(attr(a[0], "Value"), (Presence::Absent, None));
    assert_eq!(attr(a[2], "Value"), (Presence::PresentEmpty, Some("")));
    assert_eq!(attr(a[2], "SourceParamRefRef"), (Presence::Absent, None));
    assert_eq!(attr(a[3], "Value"), (Presence::PresentValue, Some("4")));
    assert_eq!(
        endpoints(a[4], Role::AssignTarget),
        [(Some("missing".into()), Some(State::Dangling), vec![])]
    );
    assert!(endpoints(a[4], Role::AssignSource).is_empty());
    for role in [Role::AssignTarget, Role::AssignSource] {
        assert_eq!(
            endpoints(a[5], role),
            [(Some(String::new()), Some(State::Dangling), vec![])]
        );
        let p = a[5].endpoints.iter().find(|p| p.role == role).unwrap();
        assert_eq!(p.presence, Presence::PresentEmpty);
    }
    for e in &a {
        assert_eq!(e.impact_scope, Impact::DeclaredEndpointsOnly);
        assert_eq!(e.function_resolution, None);
    }

    let r = inspect(AMBIGUOUS);
    let a = rows(&r, Kind::Assign);
    assert_eq!(a.len(), 1);
    let candidates = parameter_refs(&r, "R-A");
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        endpoints(a[0], Role::AssignSource),
        [(Some("R-A".into()), Some(State::Ambiguous), candidates)]
    );
    assert_eq!(a[0].graph_cycle, Cycle::NotApplicable);
    assert_eq!(a[0].graph_cycle_anchor, None);
    assert_eq!(
        r.applications[0].dynamic_state,
        DistributionPassState::NotAttempted
    );
}

#[test]
fn dependencies_resolve_with_original_namespace_and_module_scope() {
    let r = inspect(MODULE);
    let a = rows(&r, Kind::Assign);
    assert_eq!(a.len(), 2);
    let app = r.applications[0].source;
    let definition = r
        .identities
        .iter()
        .find(|i| i.kind == "ModuleDef" && i.declared_id == "MOD")
        .unwrap()
        .source;
    let scoped = |scope: DistributionSourceRef, id: &str| {
        r.identities
            .iter()
            .filter(|i| i.kind == "ParameterRef" && i.declared_id == id)
            .filter(|i| i.application_scope == Some(scope))
            .map(|i| i.source)
            .collect::<Vec<_>>()
    };
    assert_eq!(a[0].module_definition_source, None);
    assert_eq!(a[1].module_definition_source, Some(definition));
    let source = |e: &Evidence| endpoints(e, Role::AssignSource)[0].clone();
    assert_eq!(
        source(a[0]),
        (
            Some("R-A".into()),
            Some(State::Resolved),
            scoped(app, "R-A")
        )
    );
    assert_eq!(
        source(a[1]),
        (
            Some("R-A".into()),
            Some(State::Resolved),
            scoped(definition, "R-A")
        )
    );
    assert_ne!(source(a[0]).2, source(a[1]).2);
    assert!(a.iter().all(|e| e.graph_cycle == Cycle::NoDeclaredCycle));
    for e in &a {
        for p in &e.endpoints {
            let reference = r
                .references
                .iter()
                .find(|x| x.source == p.source && x.attribute == p.attribute)
                .expect("Assign attribute follows the exact allowlist");
            assert_eq!(reference.candidates, p.candidates);
        }
    }

    // ModuleDef Static declarations bind their definition and resolve in it.
    let module = std::str::from_utf8(MODULE).unwrap().replace(
        "</ParameterRefs></Static><Dynamic><Assign TargetParamRefRef=\"R-B\" SourceParamRefRef=\"R-A\"/></Dynamic></ModuleDef>",
        "</ParameterRefs><ParameterCalculations><ParameterCalculation Id=\"MC\">\
         <LParameters><ParameterRefRef RefId=\"R-A\"/></LParameters></ParameterCalculation>\
         </ParameterCalculations><Script>/* inert */</Script></Static><Dynamic>\
         <Assign TargetParamRefRef=\"R-B\" SourceParamRefRef=\"R-A\"/></Dynamic></ModuleDef>",
    );
    assert!(module.contains("<Script>/* inert */</Script></Static><Dynamic><Assign"));
    let m = inspect(module.as_bytes());
    let calc = rows(&m, Kind::ParameterCalculation);
    let script = rows(&m, Kind::Script);
    assert_eq!((calc.len(), script.len()), (1, 1));
    let definition = m
        .identities
        .iter()
        .find(|i| i.kind == "ModuleDef")
        .unwrap()
        .source;
    assert_eq!(calc[0].module_definition_source, Some(definition));
    assert_eq!(script[0].module_definition_source, Some(definition));
    let local = m
        .identities
        .iter()
        .filter(|i| i.declared_id == "R-A" && i.application_scope == Some(definition))
        .map(|i| i.source)
        .collect::<Vec<_>>();
    assert_eq!(
        endpoints(calc[0], Role::CalculationLeft),
        [(Some("R-A".into()), Some(State::Resolved), local)]
    );
    let content = script[0].content.as_ref().unwrap();
    assert_eq!(
        &module.as_bytes()[content.start_byte as usize..content.end_byte as usize],
        b"/* inert */"
    );

    let r = inspect(EDGE);
    assert!(r
        .unknowns
        .iter()
        .any(|u| u.kind == "element" && u.raw_name == "f:Assign"));
    for e in &r.dependency_evidence {
        assert!(!original(&r, EDGE, e.source).starts_with(b"<f:"));
    }

    // No borrowing of parameter targets from another member, even with the same manufacturer.
    let foreign = std::str::from_utf8(PURE)
        .unwrap()
        .replace("APP-A", "APP-B")
        .replace("R-A", "R-X");
    assert!(foreign.contains("ParameterRef Id=\"R-X\""));
    let zip = raw_zip(&[
        RawMember::stored(b"a/declarations.xml", DECLARATIONS),
        RawMember::stored(b"b/other.xml", foreign.as_bytes()),
        RawMember::stored(
            b"c/borrowing.xml",
            std::str::from_utf8(DECLARATIONS)
                .unwrap()
                .replace("APP-A", "APP-C")
                .replace("SourceParamRefRef=\"R-A\"", "SourceParamRefRef=\"R-X\"")
                .as_bytes(),
        ),
    ]);
    let r = inspect(&zip);
    let a = rows(&r, Kind::Assign);
    assert_eq!(a.len(), 2);
    assert_eq!(a[0].source.member_ordinal, Some(0));
    assert_eq!(a[1].source.member_ordinal, Some(2));
    assert_eq!(
        endpoints(a[1], Role::AssignSource),
        [(Some("R-X".into()), Some(State::Dangling), vec![])]
    );
    let xml = inspect(DECLARATIONS);
    let member0 = r
        .dependency_evidence
        .iter()
        .filter(|e| e.source.member_ordinal == Some(0))
        .count();
    assert_eq!(member0, xml.dependency_evidence.len());
    for e in &r.dependency_evidence {
        assert!(r.resolve_coordinate(e.source).is_ok());
    }
}

#[test]
fn subset_activity_is_not_handler_reachability() {
    let pure = inspect(PURE);
    let r = inspect(DECLARATIONS);
    assert_eq!(
        r.applications[0].active_parameter_ref_ids,
        pure.applications[0].active_parameter_ref_ids
    );
    for e in rows(&r, Kind::Assign)
        .into_iter()
        .chain(rows(&r, Kind::ButtonHandler))
    {
        assert_eq!(
            e.condition_context.applicability,
            DistributionConditionApplicability::NotEvaluated
        );
        assert!(e.condition_context.conditions.is_empty());
    }
    let nested = std::str::from_utf8(DECLARATIONS).unwrap().replace(
        "<ParameterRefRef RefId=\"R-B\"/></when>",
        "<ParameterRefRef RefId=\"R-B\"/><Assign TargetParamRefRef=\"R-A\" Value=\"7\"/></when>",
    );
    assert!(nested.contains("<Assign TargetParamRefRef=\"R-A\" Value=\"7\"/></when>"));
    let r = inspect(nested.as_bytes());
    let a = rows(&r, Kind::Assign);
    assert_eq!(a.len(), 2);
    let conditions = &a[0].condition_context.conditions;
    assert_eq!(conditions.len(), 2);
    assert!(original(&r, nested.as_bytes(), conditions[0]).starts_with(b"<choose "));
    assert!(original(&r, nested.as_bytes(), conditions[1]).starts_with(b"<when "));
    assert_eq!(
        a[0].condition_context.applicability,
        DistributionConditionApplicability::NotEvaluated
    );
    assert_eq!(
        fallbacks(&r)[0].1.as_deref(),
        Some("1"),
        "literal Assign is no write"
    );
    assert_eq!(
        r.applications[0].active_parameter_ref_ids,
        pure.applications[0].active_parameter_ref_ids
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("knownInactive"));
}

#[test]
fn malformed_declarations_keep_absent_and_empty_distinct() {
    let source = std::str::from_utf8(DECLARATIONS).unwrap().replace(
        "</ParameterValidations>",
        "<ParameterValidation><ParameterRefRef RefId=\"R-A\"/></ParameterValidation>\
         <ParameterValidation Id=\"\" xmlns:f=\"urn:authored:foreign\"><f:Parameters>\
         <ParameterRefRef RefId=\"R-A\"/></f:Parameters></ParameterValidation>\
         </ParameterValidations><ParameterCalculations><ParameterCalculation>\
         <LParameters/><RParameters><ParameterRefRef/></RParameters>\
         </ParameterCalculation></ParameterCalculations>",
    );
    assert!(source.contains("<LParameters/><RParameters><ParameterRefRef/></RParameters>"));
    let r = inspect(source.as_bytes());
    let v = rows(&r, Kind::ParameterValidation);
    assert_eq!(v.len(), 3);
    for wrong in &v[1..] {
        assert!(
            wrong.endpoints.is_empty(),
            "no member without the exact wrapper"
        );
        assert_eq!(
            wrong
                .member_lists
                .iter()
                .map(|m| (m.presence, m.source))
                .collect::<Vec<_>>(),
            [(Presence::Absent, None)]
        );
    }
    assert_eq!(attr(v[1], "Id"), (Presence::Absent, None));
    assert_eq!(attr(v[2], "Id"), (Presence::PresentEmpty, Some("")));
    let c = rows(&r, Kind::ParameterCalculation);
    assert_eq!(c.len(), 2);
    let c = c[1];
    assert_eq!(attr(c, "Id"), (Presence::Absent, None));
    assert_eq!(
        c.member_lists
            .iter()
            .map(|m| (m.role, m.presence))
            .collect::<Vec<_>>(),
        [
            (Role::CalculationLeft, Presence::PresentEmpty),
            (Role::CalculationRight, Presence::PresentValue)
        ]
    );
    assert_eq!(c.endpoints.len(), 1);
    assert_eq!(c.endpoints[0].presence, Presence::Absent);
    assert_eq!(c.endpoints[0].reference_state, None);
    assert_eq!(c.endpoints[0].raw_ref, None);
}

#[test]
fn additive_reports_bounded_deterministic_and_read_only() {
    let original_bytes = EDGE.to_vec();
    let r = inspect(EDGE);
    assert_eq!(original_bytes, EDGE);
    let count = r.output_record_count();
    let mut without = r.clone();
    without.dependency_evidence.clear();
    let nested: usize = r
        .dependency_evidence
        .iter()
        .map(|e| {
            1 + e.attributes.len()
                + e.member_lists.len()
                + e.endpoints.len()
                + e.endpoints
                    .iter()
                    .map(|p| p.candidates.len())
                    .sum::<usize>()
                + e.condition_context.conditions.len()
        })
        .sum();
    assert!(nested > 0);
    assert_eq!(
        count - without.output_record_count(),
        nested,
        "every new row is charged"
    );
    let limited = |limits: L| inspect_distribution_with_limits("authored.xml", EDGE, limits);
    for (cap, positive) in [(count - 1, false), (count, true)] {
        let result = limited(L {
            max_output_records: cap,
            ..L::default()
        });
        assert_eq!(result.is_ok(), positive, "records cap={cap} count={count}");
        if !positive {
            assert!(matches!(result, Err(E::ResourceLimit(_))));
        }
    }
    let size = r.serialized_size().unwrap();
    for (cap, positive) in [(size - 1, false), (size, true)] {
        let result = limited(L {
            max_report_bytes: cap,
            ..L::default()
        });
        assert_eq!(result.is_ok(), positive, "bytes cap={cap} size={size}");
    }
    let references = r.references.len()
        + r.references
            .iter()
            .map(|x| x.candidates.len())
            .sum::<usize>()
        + r.dependency_evidence
            .iter()
            .flat_map(|e| &e.endpoints)
            .map(|p| 1 + p.candidates.len())
            .sum::<usize>();
    for (cap, positive) in [(references - 1, false), (references, true)] {
        let result = limited(L {
            max_references: cap,
            ..L::default()
        });
        assert_eq!(
            result.is_ok(),
            positive,
            "references cap={cap} total={references}"
        );
    }
    assert_eq!(
        serde_json::to_vec(&r).unwrap(),
        serde_json::to_vec(&inspect(EDGE)).unwrap(),
        "repeat inspection is byte-equivalent"
    );
}

#[test]
fn format_two_report_without_dependency_evidence_is_refused_not_read_as_empty() {
    let mut json = serde_json::to_value(inspect(DECLARATIONS)).unwrap();
    let object = json.as_object_mut().unwrap();
    assert!(object.remove("dependencyEvidence").is_some());
    object.insert("formatVersion".into(), 2.into());
    assert!(
        serde_json::from_value::<DistributionInspection>(json).is_err(),
        "a report without the pass must not read as a source without declarations"
    );
}

/// Authored application: four plain refs plus `extra_refs`, then the given Dynamic body.
fn assign_app(extra_refs: &str, dynamic: &str) -> Vec<u8> {
    format!(
        "<KNX xmlns=\"http://knx.org/xml/project/20\"><ManufacturerData><Manufacturer RefId=\"M-1234\">\
         <ApplicationPrograms><ApplicationProgram Id=\"APP-A\" Name=\"Authored\" ApplicationNumber=\"7\" \
         ApplicationVersion=\"1\" MaskVersion=\"MV-0701\"><Static><ParameterTypes><ParameterType Id=\"T\">\
         <TypeNumber SizeInBit=\"8\" Type=\"unsignedInt\" minInclusive=\"1\" maxInclusive=\"64\"/>\
         </ParameterType></ParameterTypes><Parameters><Parameter Id=\"P-A\" ParameterType=\"T\" Value=\"1\"/>\
         </Parameters><ParameterRefs><ParameterRef Id=\"R-A\" RefId=\"P-A\"/><ParameterRef Id=\"R-B\" RefId=\"P-A\"/>\
         <ParameterRef Id=\"R-C\" RefId=\"P-A\"/><ParameterRef Id=\"R-D\" RefId=\"P-A\"/>{extra_refs}</ParameterRefs>\
         </Static><Dynamic>{dynamic}</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer>\
         </ManufacturerData></KNX>"
    )
    .into_bytes()
}

#[test]
fn self_loops_and_longer_declared_cycles_share_one_anchor() {
    let xml = assign_app(
        "",
        "<Assign TargetParamRefRef=\"R-D\" SourceParamRefRef=\"R-D\"/>\
         <Assign TargetParamRefRef=\"R-B\" SourceParamRefRef=\"R-A\"/>\
         <Assign TargetParamRefRef=\"R-C\" SourceParamRefRef=\"R-B\"/>\
         <Assign TargetParamRefRef=\"R-A\" SourceParamRefRef=\"R-C\"/>\
         <Assign TargetParamRefRef=\"R-D\" SourceParamRefRef=\"R-C\"/>",
    );
    let r = inspect(&xml);
    let a = rows(&r, Kind::Assign);
    assert_eq!(
        a.iter().map(|e| e.graph_cycle).collect::<Vec<_>>(),
        [
            Cycle::DeclaredGraphCycle,
            Cycle::DeclaredGraphCycle,
            Cycle::DeclaredGraphCycle,
            Cycle::DeclaredGraphCycle,
            Cycle::NoDeclaredCycle
        ]
    );
    assert_eq!(
        a.iter().map(|e| e.graph_cycle_anchor).collect::<Vec<_>>(),
        [
            Some(a[0].source),
            Some(a[1].source),
            Some(a[1].source),
            Some(a[1].source),
            None
        ],
        "the self-loop is its own cycle; the three-edge cycle is anchored at its first Assign"
    );
}

#[test]
fn ambiguous_endpoints_never_certify_that_no_cycle_was_declared() {
    // R-C is declared twice: the loop R-A -> R-B -> R-C -> R-A may or may not be one cycle.
    let xml = assign_app(
        "<ParameterRef Id=\"R-C\" RefId=\"P-A\"/>",
        "<Assign TargetParamRefRef=\"R-B\" SourceParamRefRef=\"R-A\"/>\
         <Assign TargetParamRefRef=\"R-C\" SourceParamRefRef=\"R-B\"/>\
         <Assign TargetParamRefRef=\"R-A\" SourceParamRefRef=\"R-C\"/>\
         <Assign TargetParamRefRef=\"R-D\" SourceParamRefRef=\"R-A\"/>\
         <Assign TargetParamRefRef=\"R-A\" SourceParamRefRef=\"R-C\" Value=\"3\"/>",
    );
    let r = inspect(&xml);
    let a = rows(&r, Kind::Assign);
    assert_eq!(parameter_refs(&r, "R-C").len(), 2);
    assert_eq!(
        a.iter().map(|e| e.graph_cycle).collect::<Vec<_>>(),
        [
            Cycle::CycleNotDetermined,
            Cycle::CycleNotDetermined,
            Cycle::CycleNotDetermined,
            Cycle::NoDeclaredCycle,
            Cycle::CycleNotDetermined
        ],
        "an ambiguous edge on a possible loop leaves every Assign on it undetermined"
    );
    let first = Some(a[0].source);
    assert_eq!(
        a.iter().map(|e| e.graph_cycle_anchor).collect::<Vec<_>>(),
        [first, first, first, None, first]
    );
}
