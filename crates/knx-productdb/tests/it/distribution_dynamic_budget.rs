//! Keeps large reused Dynamic scalar inputs behind the inspection resource boundary.

use crate::distribution_inspection::*;
use knx_productdb::distribution::DistributionInspectionError as E;
#[test]
fn dynamic_scalar_copy_budget_refuses_before_evaluation_instead_of_expanding_raw_values() {
    let value = "x".repeat(4097);
    let source = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A"><Static><ParameterTypes><ParameterType Id="T"><TypeNumber Type="unsignedInt" SizeInBit="8"/></ParameterType></ParameterTypes><Parameters><Parameter Id="P" ParameterType="T" Value="{value}"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs></Static><Dynamic><choose ParamRefId="R"><when test="1"><ParameterRefRef RefId="R"/></when></choose></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    assert!(
        matches!(
            inspect_distribution("bounded-dynamic", source.as_bytes()),
            Err(E::ResourceLimit(_))
        ),
        "large repeatedly clonable scalar must have a named pre-evaluation bound"
    );
}
