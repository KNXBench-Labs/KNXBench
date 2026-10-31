//! Checks module-expanded observations preserve existing evaluator identities without write rights.

use crate::distribution_inspection::*;
#[test]
fn evaluated_module_instances_keep_distinct_view_bound_scope_chains() {
    let source=br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A"><Static><ModuleDefs><ModuleDef Id="MD"><Static><Parameters><Parameter Id="P-M" Value="1"/></Parameters><ParameterRefs><ParameterRef Id="R-M" RefId="P-M"/></ParameterRefs></Static><Dynamic><ParameterRefRef RefId="R-M"/></Dynamic></ModuleDef></ModuleDefs></Static><Dynamic><Module Id="instance-one" RefId="MD"/><Module Id="instance-two" RefId="MD"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let r = inspect_distribution("modules", source).unwrap();
    let app = &r.applications[0];
    assert_eq!(app.dynamic_state, DistributionPassState::PartiallyEvaluated);
    assert_eq!(app.active_parameter_ref_ids, ["R-M", "R-M"]);
    assert_eq!(
        app.active_parameter_refs.len(),
        2,
        "scoped activations must not be flattened to ambiguous repeated strings"
    );
    assert_eq!(
        app.active_parameter_refs[0].module_scope[0]
            .module_id
            .as_deref(),
        Some("instance-one")
    );
    assert_eq!(
        app.active_parameter_refs[1].module_scope[0]
            .module_id
            .as_deref(),
        Some("instance-two")
    );
    assert_ne!(
        app.active_parameter_refs[0].module_scope[0].module_node,
        app.active_parameter_refs[1].module_scope[0].module_node
    );
    assert_eq!(
        app.active_parameter_refs[0].module_scope[0].module_def_id,
        "MD"
    );
    assert!(r.reader_passes.iter().any(|p| p.reader == "dynamic-subset"
        && p.derived_size > 0
        && p.original_sha256 == knx_productdb::sha256_hex(source)));
}
