//! T18 slice 3 task 3 (design D20-D26): `GET`/`POST
//! /api/device/{id}/parameters`. Acceptance criteria numbers below match
//! the design doc's own numbered list (`docs/superpowers/specs/
//! 2026-09-11-parameter-editor-design.md`, "Acceptance criteria").

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, ParameterInstance, ParameterInstanceId, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// Two top-level parameters (`P-1` Number 0..=255 default 5, `P-2`
/// Restriction Off/On default "0") plus one `Module` (`MD-1`, one
/// instantiation `MOD-1_M-1`) declaring its own Number parameter
/// `MOD-1_P-1_R-1` — enough to cover both an unscoped and a module-scoped
/// section in the same program (AC1, AC5-AC9).
const WRITE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
  <ParameterType Id="PT-Enum" Name="enum"><TypeRestriction Base="Value" SizeInBit="8">
    <Enumeration Id="PT-Enum_EN-0" Text="Off" Value="0" DisplayOrder="0" />
    <Enumeration Id="PT-Enum_EN-1" Text="On" Value="1" DisplayOrder="1" />
  </TypeRestriction></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
  <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Enum" Access="ReadWrite" Value="0" />
  <Parameter Id="P-3" Name="NoOrder" Text="NoOrder" ParameterType="PT-Num" Access="ReadWrite" Value="3" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="10" Tag="1" />
  <ParameterRef Id="P-2_R-1" RefId="P-2" DisplayOrder="20" Tag="1" />
  <!-- No DisplayOrder attribute at all. Fix round 1, item 1: this must
       come back with displayOrder: null, not skipped, not 0. -->
  <ParameterRef Id="P-3_R-1" RefId="P-3" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-1_R-1" />
  <ParameterRefRef RefId="P-2_R-1" />
  <ParameterRefRef RefId="P-3_R-1" />
  <Module Id="MOD-1_M-1" RefId="MD-1" />
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="MOD-1_PT-Num" Name="num"><TypeNumber maxInclusive="10" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="MOD-1_P-1" Name="Channel" Text="Channel" ParameterType="MOD-1_PT-Num" Access="ReadWrite" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MOD-1_P-1_R-1" RefId="MOD-1_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MOD-1_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// T18 fix round 1, item 9: one `Float` parameter (`PT-Float`,
/// `minInclusive="-100" maxInclusive="200"`) and one `Text` parameter
/// (`PT-Text`, `SizeInBit="16"` — two bytes), each declared exactly the
/// way `insert_parameter_type` (`knx-productdb`) reads them. Exists so one
/// test can walk the whole chain end to end — `parameter_type.
/// min_inclusive`/`max_inclusive`/`size_in_bit` (ingest) into
/// `ParameterView` (`query.rs`) into `validate_kind_and_bounds`
/// (`domain.rs`) — rather than trusting that three layers each separately
/// unit-tested in isolation actually agree once wired together.
const BOUNDS_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Float" Name="temp"><TypeFloat Encoding="DPT 9" minInclusive="-100" maxInclusive="200" /></ParameterType>
  <ParameterType Id="PT-Text" Name="label"><TypeText SizeInBit="16" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-Float" Name="Temp" Text="Temp" ParameterType="PT-Float" Access="ReadWrite" Value="0" />
  <Parameter Id="P-Text" Name="Label" Text="Label" ParameterType="PT-Text" Access="ReadWrite" Value="ok" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-Float_R-1" RefId="P-Float" DisplayOrder="1" Tag="1" />
  <ParameterRef Id="P-Text_R-1" RefId="P-Text" DisplayOrder="2" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-Float_R-1" />
  <ParameterRefRef RefId="P-Text_R-1" />
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Same shape as `WRITE_PROGRAM`'s module, instantiated twice
/// (`MOD-1_M-2`, `MOD-1_M-3`) — AC2 (stale) and AC3 (two sections, same
/// `ets_id` set).
const TWO_INSTANTIATION_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-Top" Name="Top" Text="Top" ParameterType="PT-Num" Access="ReadWrite" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-Top_R-1" RefId="P-Top" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-Top_R-1" />
  <Module Id="MOD-1_M-2" RefId="MD-1" />
  <Module Id="MOD-1_M-3" RefId="MD-1" />
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="MOD-1_PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="MOD-1_P-1" Name="Channel" Text="Channel" ParameterType="MOD-1_PT-Num" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MOD-1_P-1_R-1" RefId="MOD-1_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MOD-1_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Blocking finding 4 (goal-completion task 11, fix round 1): two
/// distinct nesting chains whose *innermost* `Module` shares the same
/// local `module_node` -- `MD-Outer-A`'s and `MD-Outer-B`'s own `Dynamic`
/// trees each hold exactly one node (their nested `Module`), so both land
/// on node id 0 in their own tree, `dynamic_node.node_id` resetting per
/// `(program_id, module_def_id)` as it does -- under two different
/// ancestors (`MOD-A` vs `MOD-B`). The old flat `module_node`-only section
/// key collided these into one section; the fix keys on the full ancestor
/// chain instead.
const NESTED_MODULE_NODE_COLLISION_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterRefs/></Static>
<Dynamic>
  <Module Id="MOD-A" RefId="MD-Outer-A" />
  <Module Id="MOD-B" RefId="MD-Outer-B" />
</Dynamic>
<ModuleDefs>
<ModuleDef Id="MD-Outer-A" Name="outerA">
<Static><ParameterRefs/></Static>
<Dynamic>
  <Module Id="MOD-A_INNER" RefId="MD-Inner" />
</Dynamic>
</ModuleDef>
<ModuleDef Id="MD-Outer-B" Name="outerB">
<Static><ParameterRefs/></Static>
<Dynamic>
  <Module Id="MOD-B_INNER" RefId="MD-Inner" />
</Dynamic>
</ModuleDef>
<ModuleDef Id="MD-Inner" Name="inner">
<Static>
<ParameterTypes>
  <ParameterType Id="MD-Inner_PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="MD-Inner_P-1" Name="Channel" Text="Channel" ParameterType="MD-Inner_PT-Num" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MD-Inner_P-1_R-1" RefId="MD-Inner_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MD-Inner_P-1_R-1" />
</Dynamic>
</ModuleDef>
</ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// The KV v2.5 demo shape verbatim (AC4): declared `ParameterRef`
/// `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`, five `Module` instantiations
/// `M-2`..`M-6` of `ModuleDef` `M-00FA_A-2504-10-C071_MD-2`.
const KV_SHAPE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterRefs/></Static>
<Dynamic>
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-2" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-3" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-4" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-5" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-6" RefId="M-00FA_A-2504-10-C071_MD-2" />
</Dynamic>
<ModuleDefs><ModuleDef Id="M-00FA_A-2504-10-C071_MD-2" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="M-00FA_A-2504-10-C071_MD-2_PT-1" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="M-00FA_A-2504-10-C071_MD-2_P-1" Name="Channel" Text="Channel" ParameterType="M-00FA_A-2504-10-C071_MD-2_PT-1" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="M-00FA_A-2504-10-C071_MD-2_P-1_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="M-00FA_A-2504-10-C071_MD-2_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Task 5's fixture: the shape design doc `2026-09-12-module-scoped-
/// editing-design.md` §E2 says no repository fixture has ever combined --
/// a module-scoped `ParameterRef` (`_MD-2_P-1_R-1`, same id grammar as
/// `KV_SHAPE_PROGRAM` [V] above) that both holds divergent per-channel
/// stored values *and* itself controls a `choose` in the same `ModuleDef`.
/// The `choose` gates which of two further module-scoped fields
/// (`_P-2_R-1`/`_P-3_R-1`) is even present in a section at all -- not just
/// what value an already-present field shows. Two `Module` instantiations,
/// `_M-4`/`_M-5` — two of the KV shape's own real `M-2`..`M-6` (E2) --
/// rather than a bare `M-<n>`, the shape a previous fixture in this file
/// was corrected away from (S6, fix round 2, above `kv_shape_reconstructs
/// _the_five_real_stored_write_ets_ids_exactly`). [A]: the `>10` threshold
/// and the two gated parameters below the gate are this fixture's own
/// invention -- KV's real `MD-2` declares no `choose` at all; only the id
/// grammar is a corpus mirror, not the branch structure.
const CHOOSE_GATED_MODULE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterRefs/></Static>
<Dynamic>
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-4" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-5" RefId="M-00FA_A-2504-10-C071_MD-2" />
</Dynamic>
<ModuleDefs><ModuleDef Id="M-00FA_A-2504-10-C071_MD-2" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="M-00FA_A-2504-10-C071_MD-2_PT-1" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="M-00FA_A-2504-10-C071_MD-2_P-1" Name="Channel" Text="Channel" ParameterType="M-00FA_A-2504-10-C071_MD-2_PT-1" Access="ReadWrite" Value="5" />
  <Parameter Id="M-00FA_A-2504-10-C071_MD-2_P-2" Name="High" Text="High" ParameterType="M-00FA_A-2504-10-C071_MD-2_PT-1" Access="ReadWrite" Value="0" />
  <Parameter Id="M-00FA_A-2504-10-C071_MD-2_P-3" Name="Low" Text="Low" ParameterType="M-00FA_A-2504-10-C071_MD-2_PT-1" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="M-00FA_A-2504-10-C071_MD-2_P-1_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_P-1" DisplayOrder="1" Tag="1" />
  <ParameterRef Id="M-00FA_A-2504-10-C071_MD-2_P-2_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_P-2" DisplayOrder="2" Tag="1" />
  <ParameterRef Id="M-00FA_A-2504-10-C071_MD-2_P-3_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_P-3" DisplayOrder="3" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="M-00FA_A-2504-10-C071_MD-2_P-1_R-1" />
  <choose ParamRefId="M-00FA_A-2504-10-C071_MD-2_P-1_R-1">
    <when test="&gt;10"><ParameterRefRef RefId="M-00FA_A-2504-10-C071_MD-2_P-2_R-1" /></when>
    <when default="true"><ParameterRefRef RefId="M-00FA_A-2504-10-C071_MD-2_P-3_R-1" /></when>
  </choose>
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Two top-level Number parameters whose `Static/ParameterRefs`
/// `DisplayOrder` disagrees with the order `Dynamic/ParameterRefRef`
/// activates them in: `P-1_R-1` declares `DisplayOrder="20"` but is
/// activated first; `P-2_R-1` declares `DisplayOrder="10"` but is
/// activated second. Fix round 1, item 4: the panel's field order is
/// `Activation::parameter_refs`' document order, not a `display_order`
/// sort — sorting by `display_order` would reverse this pair.
const DOCUMENT_ORDER_DISAGREES_WITH_DISPLAY_ORDER_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="First" Text="First" ParameterType="PT-Num" Access="ReadWrite" Value="1" />
  <Parameter Id="P-2" Name="Second" Text="Second" ParameterType="PT-Num" Access="ReadWrite" Value="2" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="20" Tag="1" />
  <ParameterRef Id="P-2_R-1" RefId="P-2" DisplayOrder="10" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-1_R-1" />
  <ParameterRefRef RefId="P-2_R-1" />
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// One top-level Number parameter controlling a `choose` whose `when
/// test` does not parse (AC10: at least one diagnostic).
const DIAGNOSTIC_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <choose ParamRefId="P-1_R-1">
    <when test="bogus"><ParameterRefRef RefId="P-1_R-1" /></when>
  </choose>
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Same shape as `WRITE_PROGRAM`, except its one `Module` declares no
/// `@Id` at all (D37: a nameless instantiation can never be matched to a
/// project-side `ModuleInstance`).
const MODULE_WITHOUT_ID_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="10" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-1_R-1" />
  <Module RefId="MD-1" />
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="MOD-1_PT-Num" Name="num"><TypeNumber maxInclusive="10" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="MOD-1_P-1" Name="Channel" Text="Channel" ParameterType="MOD-1_PT-Num" Access="ReadWrite" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MOD-1_P-1_R-1" RefId="MOD-1_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MOD-1_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn temp_product_db(program_xml: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    // `ingest_file` (not `parse::program::ingest_program` directly) is what
    // also runs the second, `Dynamic`-reading pass over an
    // `ApplicationProgram` file (`ingest.rs`'s own doc comment) -- skipping
    // it would leave `dynamic_node` empty and every `evaluate()` call inert.
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", program_xml.as_bytes()).unwrap();
    (dir, conn)
}

/// A project with one installation and one device (`DeviceId(1)`) whose
/// `program_ref` is `"H-1_HP-1"` (matching `temp_product_db`'s hardware
/// fixture), pre-seeded with `parameters`.
#[allow(clippy::field_reassign_with_default)]
fn state_with_device(
    products: knx_productdb::Connection,
    parameters: Vec<(&str, &str)>,
) -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "device-1".into(),
            ets_id: "device-1".into(),
        },
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: "M-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: parameters
            .into_iter()
            .enumerate()
            .map(|(i, (ets_id, raw))| ParameterInstance {
                id: ParameterInstanceId(i as u32 + 1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: ets_id.into(),
                },
                raw: raw.into(),
            })
            .collect(),
    });
    // Counters cover the ids built above, exactly as the importer leaves
    // them; a fixture at 0 would hand out `ParameterInstanceId(1)` again,
    // which `IdInUse` (ADR-0039) now correctly refuses.
    let parameter_count = project.installations[0].parameters.len() as u32;
    project.ids =
        knx_core::project::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, parameter_count, 0);
    let mut state = knx_server::AppState::default();
    state.product_db = Some(Mutex::new(products));
    *state.project.lock().unwrap() = Some(project);
    state
}

/// Same as `state_with_device`, plus `DeviceId(1)`'s imported
/// `ModuleInstance`s (T18 slice 4, D38-D40): `(source_ets_id,
/// instance_ets_id)` pairs, e.g. `("MD-2_M-4", "MD-2_M-4_MI-1")` -- the
/// project's own retained answer to "which channel is this," which D39's
/// editability rule and D43's `writeEtsId` both key off.
fn state_with_device_and_modules(
    products: knx_productdb::Connection,
    parameters: Vec<(&str, &str)>,
    module_instances: Vec<(&str, &str)>,
) -> knx_server::AppState {
    let state = state_with_device(products, parameters);
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        for (i, (source_ets_id, instance_ets_id)) in module_instances.into_iter().enumerate() {
            project
                .devices
                .insert_module_instance(knx_core::ModuleInstance {
                    id: knx_core::ModuleInstanceId(i as u32 + 1),
                    device: DeviceId(1),
                    source: SourceRef {
                        path: "device-1".into(),
                        ets_id: source_ets_id.into(),
                    },
                    repeat_index: "1x1".into(),
                    instance_ets_id: instance_ets_id.into(),
                    arguments: vec![],
                });
        }
    }
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get_panel(app: axum::Router, device_id: u32) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/device/{device_id}/parameters"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, body_json(response).await)
}

async fn post_panel(
    app: axum::Router,
    device_id: u32,
    ets_id: &str,
    raw: &str,
) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/device/{device_id}/parameters"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "etsId": ets_id, "raw": raw }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, body_json(response).await)
}

fn field<'a>(dto: &'a Value, ets_id: &str) -> Option<&'a Value> {
    dto["sections"].as_array().unwrap().iter().find_map(|s| {
        s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["etsId"] == ets_id)
    })
}

// AC1: a stored field and a defaulted field, correct `value`/`valueSource`.
#[tokio::test]
async fn get_returns_stored_and_defaulted_top_level_fields() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![("P-1_R-1", "7")]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(dto["programId"], "A-1");

    let p1 = field(&dto, "P-1_R-1").expect("P-1_R-1 present");
    assert_eq!(p1["value"], "7");
    assert_eq!(p1["valueSource"], "Stored");
    // Fix round 1, item 1: a declared DisplayOrder survives verbatim.
    assert_eq!(p1["displayOrder"], 10);
    // Fix round 1, item 2: `access` is carried verbatim too.
    assert_eq!(p1["access"], "ReadWrite");

    let p2 = field(&dto, "P-2_R-1").expect("P-2_R-1 present");
    assert_eq!(p2["value"], "0");
    assert_eq!(p2["valueSource"], "ProgramDefault");

    // Fix round 1, item 1: a `ParameterRef` declaring no `DisplayOrder` at
    // all comes back `null`, not skipped and not `0`.
    let p3 = field(&dto, "P-3_R-1").expect("P-3_R-1 present");
    assert!(
        p3["displayOrder"].is_null(),
        "expected null, got {:?}",
        p3["displayOrder"]
    );

    // T3 fix round 1, item 6: a `GET` never mutates anything, so it
    // attaches no tree — `tree` is only ever `Some` on a successful write.
    assert!(
        dto["tree"].is_null(),
        "expected null, got {:?}",
        dto["tree"]
    );
}

// AC2: an undecomposable id and a regex-match-but-undeclared id both land
// in `stale`, not in any section; other valid stored values are
// unaffected.
#[tokio::test]
async fn undecomposable_and_unvalidated_stored_ids_are_reported_stale_not_dropped() {
    let (_dir, products) = temp_product_db(TWO_INSTANTIATION_PROGRAM);
    let state = Arc::new(state_with_device(
        products,
        vec![
            ("P-Top_R-1", "7"),
            ("totally-bogus-id", "x"),
            ("P-Top_R-1_M-99_MI-1_bogus", "y"),
        ],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let stale_ids: Vec<&str> = dto["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["etsId"].as_str().unwrap())
        .collect();
    assert!(stale_ids.contains(&"totally-bogus-id"));
    assert!(stale_ids.contains(&"P-Top_R-1_M-99_MI-1_bogus"));
    assert_eq!(stale_ids.len(), 2);

    // Neither stale id appears in any section's fields.
    for section in dto["sections"].as_array().unwrap() {
        for f in section["fields"].as_array().unwrap() {
            let ets_id = f["etsId"].as_str().unwrap();
            assert_ne!(ets_id, "totally-bogus-id");
            assert_ne!(ets_id, "P-Top_R-1_M-99_MI-1_bogus");
        }
    }

    // The one genuinely valid stored value is untouched.
    let top = field(&dto, "P-Top_R-1").expect("P-Top_R-1 present");
    assert_eq!(top["value"], "7");
    assert_eq!(top["valueSource"], "Stored");
}

// AC3: a Module instantiated twice produces two sections with distinct
// `scope.moduleNode`, both containing the same `etsId` set.
#[tokio::test]
async fn a_module_instantiated_twice_produces_two_sections_with_the_same_ets_id_set() {
    let (_dir, products) = temp_product_db(TWO_INSTANTIATION_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let module_sections: Vec<&Value> = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !s["scope"].is_null())
        .collect();
    assert_eq!(module_sections.len(), 2);

    let module_nodes: std::collections::HashSet<i64> = module_sections
        .iter()
        .map(|s| s["scope"]["moduleNode"].as_i64().unwrap())
        .collect();
    assert_eq!(
        module_nodes.len(),
        2,
        "distinct module_node per instantiation"
    );

    for s in &module_sections {
        let ids: Vec<&str> = s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["etsId"].as_str().unwrap())
            .collect();
        assert_eq!(ids, vec!["MOD-1_P-1_R-1"]);
        assert_eq!(s["fields"][0]["editable"], false);
    }
}

// Blocking finding 4 (goal-completion task 11, fix round 1): two nesting
// chains whose innermost `Module` shares the same local `module_node`
// (`NESTED_MODULE_NODE_COLLISION_PROGRAM`'s doc comment above explains
// why) must still come back as two sections, each with its own field —
// not one section that silently swallowed the second chain's field under
// the first chain's scope.
#[tokio::test]
async fn two_nesting_chains_sharing_a_module_node_do_not_merge_into_one_section() {
    let (_dir, products) = temp_product_db(NESTED_MODULE_NODE_COLLISION_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let module_sections: Vec<&Value> = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !s["scope"].is_null())
        .collect();
    assert_eq!(
        module_sections.len(),
        2,
        "two distinct nesting chains must not collapse into one section: {:?}",
        dto["sections"]
    );

    // The bug this reproduces: both innermost scopes land on the same
    // local `module_node` because `dynamic_node.node_id` resets per
    // `(program_id, module_def_id)` tree, and each outer `ModuleDef`'s own
    // tree holds exactly one node (its nested `Module`).
    let module_nodes: Vec<i64> = module_sections
        .iter()
        .map(|s| s["scope"]["moduleNode"].as_i64().unwrap())
        .collect();
    assert_eq!(
        module_nodes[0], module_nodes[1],
        "both innermost scopes must share one module_node -- that is the collision this guards against: {module_nodes:?}"
    );

    // Despite the colliding module_node, the two chains are still
    // distinguishable by their own (innermost) module_id, and each kept
    // its own field rather than merging into a shared one.
    let module_ids: std::collections::HashSet<&str> = module_sections
        .iter()
        .map(|s| s["scope"]["moduleId"].as_str().unwrap())
        .collect();
    assert_eq!(
        module_ids,
        std::collections::HashSet::from(["MOD-A_INNER", "MOD-B_INNER"]),
        "each chain must keep its own innermost module_id, not one section's borrowed from the other"
    );

    for s in &module_sections {
        let ids: Vec<&str> = s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["etsId"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            vec!["MD-Inner_P-1_R-1"],
            "each section must hold exactly its own one field, not two merged together"
        );
    }
}

// AC4: the KV v2.5 demo shape verbatim — five sections, each showing its
// own stored value, `stale` empty.
#[tokio::test]
async fn kv_shape_stored_values_decompose_into_five_sections_each_showing_its_own_value() {
    let (_dir, products) = temp_product_db(KV_SHAPE_PROGRAM);
    let state = Arc::new(state_with_device(
        products,
        vec![
            ("M-00FA_A-2504-10-C071_MD-2_M-2_MI-1_P-1_R-1", "32"),
            ("M-00FA_A-2504-10-C071_MD-2_M-3_MI-1_P-1_R-1", "48"),
            ("M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1", "17"),
            ("M-00FA_A-2504-10-C071_MD-2_M-5_MI-1_P-1_R-1", "33"),
            ("M-00FA_A-2504-10-C071_MD-2_M-6_MI-1_P-1_R-1", "49"),
        ],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(dto["stale"].as_array().unwrap().is_empty());

    let sections = dto["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 5);

    let mut values: Vec<String> = sections
        .iter()
        .map(|s| {
            let f = s["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["etsId"] == "M-00FA_A-2504-10-C071_MD-2_P-1_R-1")
                .expect("declared field present in every section");
            assert_eq!(f["valueSource"], "Stored");
            f["value"].as_str().unwrap().to_string()
        })
        .collect();
    values.sort();
    assert_eq!(values, vec!["17", "32", "33", "48", "49"]);
}

// AC5: POST a valid top-level etsId/raw returns 200 and the same response
// shows the new value as "Stored" — no second request needed.
#[tokio::test]
async fn exhausted_parameter_ids_refuse_new_values_but_allow_existing_edits() {
    for counter in [u32::MAX - 1, u32::MAX] {
        let (_dir, products) = temp_product_db(WRITE_PROGRAM);
        let state = Arc::new(state_with_device(products, vec![("P-1_R-1", "7")]));
        state.project.lock().unwrap().as_mut().unwrap().ids =
            knx_core::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, counter, 0);
        let app = knx_server::app(state.clone(), None);
        if counter < u32::MAX {
            assert_eq!(
                post_panel(app.clone(), 1, "P-2_R-1", "1").await.0,
                StatusCode::OK
            );
            let project = state.project.lock().unwrap();
            let parameters = &project.as_ref().unwrap().installations[0].parameters;
            assert_eq!(parameters.last().unwrap().id, ParameterInstanceId(u32::MAX));
        }
        let before = state.project.lock().unwrap().clone();
        let (status, dto) = post_panel(app.clone(), 1, "P-3_R-1", "8").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(dto["error"]
            .as_str()
            .unwrap()
            .contains("project parameter_instance ID range exhausted"));
        assert_eq!(*state.project.lock().unwrap(), before);
        let (status, dto) = post_panel(app, 1, "P-1_R-1", "9").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "9");
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        assert_eq!(project.ids.peek_parameter_instance(), u32::MAX);
        assert_eq!(
            project.installations[0].parameters[0].id,
            ParameterInstanceId(1)
        );
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_store::open_and_migrate(&dir.path().join("parameter.knxdb")).unwrap();
        knx_store::save_project(&conn, project).unwrap();
        assert_eq!(knx_store::load_project(&conn).unwrap(), *project);
    }
}

#[tokio::test]
async fn post_a_valid_top_level_value_is_reflected_in_the_same_response() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = post_panel(app, 1, "P-1_R-1", "42").await;
    assert_eq!(status, StatusCode::OK);
    let p1 = field(&dto, "P-1_R-1").unwrap();
    assert_eq!(p1["value"], "42");
    assert_eq!(p1["valueSource"], "Stored");

    // T3 fix round 1, item 6: a successful write carries the server's own
    // freshly rebuilt `ProjectTree` — the same one `apply(state, cmd)`
    // already built from the genuine post-write `CommandStack` — not a
    // `null` a caller would otherwise have to reconstruct by hand.
    assert!(!dto["tree"].is_null(), "expected a tree, got null");
    assert_eq!(dto["tree"]["can_undo"], true);
    assert_eq!(dto["tree"]["can_redo"], false);
}

// AC6: out-of-range Number and non-member Restriction both 400, and
// leave the previously stored value unchanged.
#[tokio::test]
async fn post_out_of_bounds_number_and_non_member_restriction_are_rejected_unchanged() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![("P-1_R-1", "7")]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app.clone(), 1, "P-1_R-1", "999").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post_panel(app.clone(), 1, "P-2_R-1", "7").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "7");
    assert_eq!(field(&dto, "P-2_R-1").unwrap()["value"], "0");
}

// AC7: POST to a field currently `scope: Some(_)` returns 400 and changes
// nothing.
#[tokio::test]
async fn post_to_a_module_scoped_field_is_rejected() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app.clone(), 1, "MOD-1_P-1_R-1", "3").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    let field = field(&dto, "MOD-1_P-1_R-1").unwrap();
    assert_eq!(field["valueSource"], "ProgramDefault");
}

// AC8: POST naming an etsId not declared by the program at all -> 400.
#[tokio::test]
async fn post_an_undeclared_ets_id_is_rejected() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app, 1, "NOPE", "1").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// T18 fix round 1, item 9: `parameter_type.min_inclusive`/`max_inclusive`/
// `size_in_bit` reach the write validator through `ParameterView`, proven
// at the HTTP surface rather than only in each layer's own isolated unit
// test.
#[tokio::test]
async fn parameter_type_bounds_flow_from_product_db_through_to_the_write_validator() {
    let (_dir, products) = temp_product_db(BOUNDS_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    // Float: `parameter_type.max_inclusive` is "200"; one past it is
    // rejected, the boundary itself is accepted.
    let (status, _) = post_panel(app.clone(), 1, "P-Float_R-1", "201").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "201 is past PT-Float's declared max_inclusive of 200"
    );
    let (status, dto) = post_panel(app.clone(), 1, "P-Float_R-1", "200").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-Float_R-1").unwrap()["value"], "200");

    // Text: `parameter_type.size_in_bit` is 16 (two bytes); three ASCII
    // bytes overflows it, two fits exactly.
    let (status, _) = post_panel(app.clone(), 1, "P-Text_R-1", "abc").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "3 bytes overflows PT-Text's declared size_in_bit of 16 (2 bytes)"
    );
    let (status, dto) = post_panel(app.clone(), 1, "P-Text_R-1", "ab").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-Text_R-1").unwrap()["value"], "ab");
}

// AC9: write + undo + redo round-trips through Command::SetParameterValue
// / RestoreParameterValue — insert case (row absent before) and overwrite
// case (row present before) both covered.
#[tokio::test]
async fn write_undo_redo_round_trips_both_insert_and_overwrite() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![("P-1_R-1", "7")]));
    let app = knx_server::app(Arc::clone(&state), None);

    // Insert case: P-2_R-1 has no stored row yet.
    let (status, dto) = post_panel(app.clone(), 1, "P-2_R-1", "1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-2_R-1").unwrap()["value"], "1");

    let undo = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/undo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(undo.status(), StatusCode::OK);
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        field(&dto, "P-2_R-1").unwrap()["valueSource"],
        "ProgramDefault"
    );
    assert!(!state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .parameters
        .iter()
        .any(|p| p.source.ets_id == "P-2_R-1"));

    let redo = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/redo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(redo.status(), StatusCode::OK);
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-2_R-1").unwrap()["value"], "1");

    // Overwrite case: P-1_R-1 already had "7" stored.
    let (status, dto) = post_panel(app.clone(), 1, "P-1_R-1", "9").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "9");

    let undo = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/undo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(undo.status(), StatusCode::OK);
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "7");

    let redo = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/redo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(redo.status(), StatusCode::OK);
    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "9");
}

// AR07: malformed Float bounds cannot turn an unordered comparison into permission.
#[tokio::test]
async fn non_finite_float_bounds_refuse_http_writes_without_changing_project_or_source() {
    for declared in ["NaN", "inf", "-inf", "1e999", "-1e999"] {
        for bound in ["minInclusive", "maxInclusive"] {
            let original = if bound == "minInclusive" {
                "minInclusive=\"-100\""
            } else {
                "maxInclusive=\"200\""
            };
            assert_eq!(BOUNDS_PROGRAM.matches(original).count(), 1);
            let source = BOUNDS_PROGRAM.replace(original, &format!("{bound}=\"{declared}\""));
            let (_dir, products) = temp_product_db(&source);
            let state = Arc::new(state_with_device(
                products,
                vec![("P-Float_R-1", "0"), ("P-Text_R-1", "ok")],
            ));
            let before = state.project.lock().unwrap().clone();
            assert_eq!(
                before.as_ref().unwrap().installations[0].parameters.len(),
                2
            );
            let app = knx_server::app(Arc::clone(&state), None);
            let (status, panel) = get_panel(app.clone(), 1).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(field(&panel, "P-Float_R-1").unwrap()["value"], "0");

            let (status, error) = post_panel(app.clone(), 1, "P-Float_R-1", "1.5").await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{bound}={declared}");
            let role = if bound == "minInclusive" {
                "min_inclusive"
            } else {
                "max_inclusive"
            };
            assert!(error["error"]
                .as_str()
                .unwrap()
                .contains(&format!("non-finite {role}")));
            assert_eq!(*state.project.lock().unwrap(), before);
            {
                let products = state.product_db.as_ref().unwrap().lock().unwrap();
                let retained = knx_productdb::load_source_file(
                    &products,
                    &knx_productdb::sha256_hex(source.as_bytes()),
                )
                .unwrap()
                .unwrap();
                assert_eq!(retained, source.as_bytes());
            }

            let (status, panel) = post_panel(app, 1, "P-Text_R-1", "up").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(field(&panel, "P-Text_R-1").unwrap()["value"], "up");
        }
    }
}

// AR07 resource admission, not a manufacturer grammar claim. Inert shared
// leaves exhaust work without a huge DTO. An unseen later duplicate module
// makes the previously admitted prefix insufficient as write authority.
#[tokio::test]
async fn work_limited_parameter_prefix_refuses_writes_without_changing_project_or_source() {
    let original_module = r#"<Module Id="MOD-1_M-1" RefId="MD-1" />"#;
    assert_eq!(WRITE_PROGRAM.matches(original_module).count(), 1);
    let mut definitions = String::new();
    for level in 1..=5 {
        definitions.push_str(&format!(
            r#"<ModuleDef Id="PUBLIC-WORK-F-{level}" Name="public work"><Dynamic>"#
        ));
        for branch in 0..4 {
            definitions.push_str(&format!(
                r#"<Module Id="PUBLIC-WORK-{level}-{branch}" RefId="PUBLIC-WORK-F-{}" />"#,
                level + 1
            ));
        }
        definitions.push_str("</Dynamic></ModuleDef>");
    }
    definitions.push_str(r#"<ModuleDef Id="PUBLIC-WORK-F-6" Name="public inert leaf"><Dynamic>"#);
    let leaf_count = knx_productdb::dynamic::MAX_EVALUATION_WORK / 4usize.pow(5) + 1;
    definitions.push_str(&"<Assign />".repeat(leaf_count));
    definitions.push_str("</Dynamic></ModuleDef>");
    let source = WRITE_PROGRAM.replace(
        original_module,
        &format!(
            r#"{original_module}<Module Id="PUBLIC-WORK-ROOT" RefId="PUBLIC-WORK-F-1" />{original_module}"#
        ),
    );
    assert_eq!(source.matches("</ModuleDefs>").count(), 1);
    let source = source.replace("</ModuleDefs>", &format!("{definitions}</ModuleDefs>"));
    let (_dir, products) = temp_product_db(&source);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![("P-1_R-1", "5"), ("MOD-1_M-1_MI-1_P-1_R-1", "1")],
        vec![("M-1", "M-1_MI-1")],
    ));
    let before = state.project.lock().unwrap().clone();
    assert_eq!(
        before.as_ref().unwrap().installations[0].parameters.len(),
        2
    );
    let app = knx_server::app(Arc::clone(&state), None);
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "5");
    assert!(field(&dto, "MOD-1_P-1_R-1").is_some());
    let markers: Vec<_> = dto["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"] == "evaluationWorkBudgetExhausted")
        .collect();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0]["severity"], "warning");
    assert!(markers[0]["message"]
        .as_str()
        .unwrap()
        .contains("read-only"));
    assert!(markers[0]["detail"]
        .as_str()
        .unwrap()
        .contains("EvaluationWorkBudgetExhausted"));

    for ets_id in ["P-1_R-1", "MOD-1_M-1_MI-1_P-1_R-1"] {
        let (status, error) = post_panel(app.clone(), 1, ets_id, "7").await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "incomplete prefix must not authorize {ets_id}"
        );
        assert!(
            error["error"].as_str().unwrap().contains("not writable")
                || error["error"]
                    .as_str()
                    .unwrap()
                    .contains("no editable field")
        );
        assert_eq!(*state.project.lock().unwrap(), before);
    }
    let fields: Vec<_> = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["fields"].as_array().unwrap())
        .collect();
    assert_eq!(fields.len(), 4);
    for field in fields {
        assert_eq!(field["editable"], false);
        assert!(field.get("writeEtsId").is_some());
        assert!(field["writeEtsId"].is_null());
    }
    let products = state.product_db.as_ref().unwrap().lock().unwrap();
    let retained =
        knx_productdb::load_source_file(&products, &knx_productdb::sha256_hex(source.as_bytes()))
            .unwrap()
            .unwrap();
    assert_eq!(retained, source.as_bytes());
}

// AR07: a known unsupported controller is not a missing declaration or a
// valid choice; hidden fields cannot be edited, but independent fields can.
#[tokio::test]
async fn unsupported_controller_warning_survives_http_and_refuses_hidden_field_writes() {
    let old_type = "<ParameterType Id=\"PT-Num\" Name=\"num\"><TypeNumber maxInclusive=\"255\" minInclusive=\"0\" SizeInBit=\"8\" Type=\"unsignedInt\" /></ParameterType>";
    assert_eq!(WRITE_PROGRAM.matches(old_type).count(), 1);
    let source = WRITE_PROGRAM.replace(
        old_type,
        "<ParameterType Id=\"PT-Num\" Name=\"num\"><TypeText SizeInBit=\"16\" /></ParameterType>",
    );
    let reference = "<ParameterRefRef RefId=\"P-2_R-1\" />";
    assert_eq!(source.matches(reference).count(), 1);
    let source = source.replace(
        reference,
        r#"<choose ParamRefId="P-1_R-1">
<when test="5"><ParameterRefRef RefId="P-2_R-1" /></when>
<when default="true"><ParameterRefRef RefId="P-2_R-1" /></when>
</choose>"#,
    );
    let (_dir, products) = temp_product_db(&source);
    let state = Arc::new(state_with_device(
        products,
        vec![("P-1_R-1", "5"), ("P-2_R-1", "0"), ("P-3_R-1", "3")],
    ));
    let before = state.project.lock().unwrap().clone();
    let app = knx_server::app(Arc::clone(&state), None);
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(field(&dto, "P-2_R-1").is_none());
    assert!(field(&dto, "P-3_R-1").is_some());
    let diagnostics = dto["diagnostics"].as_array().unwrap();
    let warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d["kind"] == "unsupportedControlKind")
        .collect();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["severity"], "warning");
    assert_eq!(warnings[0]["message"], "A choice's controlling parameter uses an unsupported type; its branches were not evaluated.");
    assert!(warnings[0]["detail"].as_str().unwrap().contains("Text"));
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d["kind"] == "refBelowSkippedNode")
            .count(),
        2
    );
    assert_eq!(*state.project.lock().unwrap(), before);

    let (status, _) = post_panel(app.clone(), 1, "P-2_R-1", "1").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(*state.project.lock().unwrap(), before);
    let (status, dto) = post_panel(app, 1, "P-3_R-1", "4").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-3_R-1").unwrap()["value"], "4");
}

// AC10: `diagnostics.len()` equals `Activation::diagnostics.len()` for a
// fixture producing at least one diagnostic (an unparsable `when/@test`).
#[tokio::test]
async fn diagnostics_are_carried_through_one_to_one() {
    let (_dir, products) = temp_product_db(DIAGNOSTIC_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    // The `when/@test` neither parses nor is a `default`, so `evaluate`
    // reports both the unparsable condition and the resulting no-match --
    // exactly what a bare count of `Activation::diagnostics` would produce,
    // which is what this asserts: nothing here is deduplicated or dropped.
    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics[0]["message"],
        "A choice's condition could not be understood."
    );
    assert!(diagnostics[0]["detail"]
        .as_str()
        .unwrap()
        .contains("UnparsableTest"));
    assert_eq!(
        diagnostics[1]["message"],
        "A choice did not match any of its options."
    );
    assert!(diagnostics[1]["detail"]
        .as_str()
        .unwrap()
        .contains("NoBranchMatched"));
}

// Coordinator addition: `parameter_views`' inner joins can silently drop
// a row whose `parameter` does not resolve — that must surface as a
// diagnostic naming the dropped-row count, not vanish.
#[tokio::test]
async fn a_parameter_ref_whose_parameter_row_is_missing_is_reported_not_dropped_silently() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    products
        .execute("DELETE FROM parameter WHERE id = 'P-2'", [])
        .unwrap();
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(field(&dto, "P-2_R-1").is_none());
    assert!(field(&dto, "P-1_R-1").is_some());

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["detail"]
        .as_str()
        .unwrap()
        .contains("1 dropped by an unresolved parameter/parameter_type join")));
}

// Fix round 1, item 4: field order is document order (`Activation::
// parameter_refs`), not a `display_order` sort. `P-1_R-1` is activated
// first but declares the higher `DisplayOrder`; a `display_order` sort
// would put `P-2_R-1` first instead.
#[tokio::test]
async fn field_order_is_document_order_not_display_order() {
    let (_dir, products) = temp_product_db(DOCUMENT_ORDER_DISAGREES_WITH_DISPLAY_ORDER_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let sections = dto["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 1);
    let ids: Vec<&str> = sections[0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["etsId"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["P-1_R-1", "P-2_R-1"],
        "document (activation) order, not a display_order sort which would put P-2_R-1 first"
    );
}

// Fix round 1, item 5: a device whose `program_ref` resolves to nothing
// (product database present and ingested, but no `hardware2program` row
// matches) returns the empty panel, not an error.
#[tokio::test]
async fn a_program_ref_that_resolves_to_nothing_returns_the_empty_panel() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = state_with_device(products, vec![("P-1_R-1", "7")]);
    {
        let mut project = state.project.lock().unwrap();
        project
            .as_mut()
            .unwrap()
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .program_ref = "H-1_HP-does-not-exist".into();
    }
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(dto["programId"].is_null());
    assert!(dto["sections"].as_array().unwrap().is_empty());
    assert!(dto["diagnostics"].as_array().unwrap().is_empty());
    // The stale-but-unresolvable stored value still round-trips through
    // as `stale`, same as the `product_db == None` path -- it is not
    // silently dropped just because the program didn't resolve either.
    let stale_ids: Vec<&str> = dto["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["etsId"].as_str().unwrap())
        .collect();
    assert_eq!(stale_ids, vec!["P-1_R-1"]);
}

// T18 task 3 (design D39, D42, D43): the KV v2.5 demo shape, now with its
// five imported `ModuleInstance`s declared too. Each section earns
// editability (a single authoritative instance each) and each field's own
// `writeEtsId` reconstructs to exactly the real stored id the corpus (E2)
// shows -- not a guess, the same string the module's own suffix and the
// instance's own `MI-` digit produce.
#[tokio::test]
async fn kv_shape_reconstructs_the_five_real_stored_write_ets_ids_exactly() {
    let (_dir, products) = temp_product_db(KV_SHAPE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![
            ("M-00FA_A-2504-10-C071_MD-2_M-2_MI-1_P-1_R-1", "32"),
            ("M-00FA_A-2504-10-C071_MD-2_M-3_MI-1_P-1_R-1", "48"),
            ("M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1", "17"),
            ("M-00FA_A-2504-10-C071_MD-2_M-5_MI-1_P-1_R-1", "33"),
            ("M-00FA_A-2504-10-C071_MD-2_M-6_MI-1_P-1_R-1", "49"),
        ],
        // S6 (fix round 2): the real KV v2.5 demo's `ModuleInstance`
        // `RefId`s are `MD-2_M-<n>` (read from `P-03DE/0.xml`), not the
        // bare `M-<n>` this fixture declared before -- a bare `M-<n>` is
        // the shortest possible anchored suffix, so it was exercising
        // the loosest version of the `_`-anchoring rule rather than the
        // corpus's own shape.
        vec![
            ("MD-2_M-2", "MD-2_M-2_MI-1"),
            ("MD-2_M-3", "MD-2_M-3_MI-1"),
            ("MD-2_M-4", "MD-2_M-4_MI-1"),
            ("MD-2_M-5", "MD-2_M-5_MI-1"),
            ("MD-2_M-6", "MD-2_M-6_MI-1"),
        ],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(dto["stale"].as_array().unwrap().is_empty());

    let sections = dto["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 5);

    let mut write_ids: Vec<String> = sections
        .iter()
        .map(|s| {
            let f = s["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["etsId"] == "M-00FA_A-2504-10-C071_MD-2_P-1_R-1")
                .expect("declared field present in every section");
            assert_eq!(f["editable"], true);
            f["writeEtsId"].as_str().unwrap().to_string()
        })
        .collect();
    write_ids.sort();
    assert_eq!(
        write_ids,
        vec![
            "M-00FA_A-2504-10-C071_MD-2_M-2_MI-1_P-1_R-1",
            "M-00FA_A-2504-10-C071_MD-2_M-3_MI-1_P-1_R-1",
            "M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1",
            "M-00FA_A-2504-10-C071_MD-2_M-5_MI-1_P-1_R-1",
            "M-00FA_A-2504-10-C071_MD-2_M-6_MI-1_P-1_R-1",
        ]
    );
}

// D39/D43: a module-scoped field with exactly one authoritative
// `ModuleInstance` is genuinely writable -- the write lands on the
// module-qualified `writeEtsId`, not the bare declared id, and shows up
// as "Stored" in the very same response.
#[tokio::test]
async fn post_to_a_module_scoped_field_with_a_single_authoritative_instance_succeeds() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("M-1", "M-1_MI-1")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = post_panel(app, 1, "MOD-1_M-1_MI-1_P-1_R-1", "7").await;
    assert_eq!(status, StatusCode::OK);
    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["value"], "7");
    assert_eq!(f["valueSource"], "Stored");
    assert_eq!(f["editable"], true);
    assert_eq!(f["writeEtsId"], "MOD-1_M-1_MI-1_P-1_R-1");
}

// D43: the write above is a normal `Command::SetParameterValue` on the
// same undo stack as any other edit -- undo restores the module-scoped
// field to its program default exactly like an unscoped one.
#[tokio::test]
async fn undo_restores_a_module_scoped_write_to_program_default() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("M-1", "M-1_MI-1")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app.clone(), 1, "MOD-1_M-1_MI-1_P-1_R-1", "7").await;
    assert_eq!(status, StatusCode::OK);

    let undo = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/undo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(undo.status(), StatusCode::OK);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["valueSource"], "ProgramDefault");
}

// D40: two imported `ModuleInstance`s both claiming the same program
// module is a genuine ambiguity this slice refuses to guess through --
// its fields stay read-only, and the reason is named in a diagnostic.
// The sibling instantiation with zero matching instances is named by its
// own, distinct diagnostic (D39 rule 2) -- the two reasons are not
// conflated into one message.
#[tokio::test]
async fn two_module_instances_sharing_a_ref_id_leave_module_scoped_fields_read_only() {
    let (_dir, products) = temp_product_db(TWO_INSTANTIATION_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("M-2", "M-2_MI-1"), ("M-2", "M-2_MI-2")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let module_sections: Vec<&Value> = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !s["scope"].is_null())
        .collect();
    assert_eq!(module_sections.len(), 2);
    for s in &module_sections {
        assert_eq!(s["fields"][0]["editable"], false);
        assert!(s["fields"][0]["writeEtsId"].is_null());
    }

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["message"]
        == "Two or more imported module instances share this module; its fields are read-only."
        && d["detail"].as_str().unwrap().contains("M-2_MI-1")
        && d["detail"].as_str().unwrap().contains("M-2_MI-2")));
    assert!(diagnostics.iter().any(|d| d["message"]
        == "No imported module instance matches this module; its fields are read-only."));
}

// D37/D39: a `Module` with no `@Id` at all was already diagnosed before
// this task; now it also stays read-only for the same reason its
// `ModuleScope.module_id` is `None` -- there is nothing to look an
// authoritative `ModuleInstance` up by.
#[tokio::test]
async fn a_module_with_no_id_is_read_only_and_diagnosed() {
    let (_dir, products) = temp_product_db(MODULE_WITHOUT_ID_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let module_section = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| !s["scope"].is_null())
        .expect("the nameless Module still produces a section");
    assert_eq!(module_section["fields"][0]["editable"], false);
    assert!(module_section["fields"][0]["writeEtsId"].is_null());

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["message"]
        == "A module instance has no identifier and cannot be matched to stored values."));
}

// D41: a stored row whose `MI-` digit disagrees with the one
// authoritative `ModuleInstance` is stale, not silently overwritten by
// or silently preferred over the authority's own answer -- it keeps its
// raw value in `stale` and the field falls back to the program default.
#[tokio::test]
async fn a_stored_row_whose_mi_digit_disagrees_with_the_authority_is_stale() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![("MOD-1_M-1_MI-2_P-1_R-1", "9")],
        vec![("M-1", "M-1_MI-1")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let stale_ids: Vec<&str> = dto["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["etsId"].as_str().unwrap())
        .collect();
    assert_eq!(stale_ids, vec!["MOD-1_M-1_MI-2_P-1_R-1"]);
    assert_eq!(dto["stale"][0]["raw"], "9");

    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["valueSource"], "ProgramDefault");
}

// D43: the panel is the single authority on what is writable -- even
// once a section has earned editability, the bare declared id (as
// opposed to its own `writeEtsId`) is still not a valid write target.
#[tokio::test]
async fn the_bare_declared_id_is_rejected_even_when_the_section_is_editable() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("M-1", "M-1_MI-1")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app.clone(), 1, "MOD-1_P-1_R-1", "3").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["valueSource"], "ProgramDefault");
    assert_eq!(f["editable"], true);
}

/// S4 (fix round 2): a malformed program declaring the same
/// `Module/@Id` twice -- ETS's own id grammar makes `@Id` unique per
/// instantiation so the corpus never shows this, but nothing before this
/// fix round refused it. Same shape as `TWO_INSTANTIATION_PROGRAM`,
/// except both `Module` elements share one `@Id` (`MOD-1_M-1`) instead
/// of getting distinct ones.
const REPEATED_MODULE_ID_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-Top" Name="Top" Text="Top" ParameterType="PT-Num" Access="ReadWrite" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-Top_R-1" RefId="P-Top" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-Top_R-1" />
  <Module Id="MOD-1_M-1" RefId="MD-1" />
  <Module Id="MOD-1_M-1" RefId="MD-1" />
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="MOD-1_PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="MOD-1_P-1" Name="Channel" Text="Channel" ParameterType="MOD-1_PT-Num" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MOD-1_P-1_R-1" RefId="MOD-1_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MOD-1_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

// S1 (fix round 2) -- reviewer's P1: a lone authority whose digit isn't
// `1` reconstructs that digit, not a hardcoded `MI-1` guess. Kills the
// mutation that hardcodes `MI-1` in the reconstruction (M2).
#[tokio::test]
async fn a_lone_authority_whose_digit_is_not_one_reconstructs_that_digit() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![("MOD-1_M-1_MI-2_P-1_R-1", "9")],
        vec![("M-1", "M-1_MI-2")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["editable"], true);
    assert_eq!(f["writeEtsId"], "MOD-1_M-1_MI-2_P-1_R-1");
    assert_eq!(f["value"], "9");
    assert_eq!(f["valueSource"], "Stored");
}

// S1 (fix round 2) -- reviewer's P2: an empty `instance_ets_id` (a
// pre-schema-6 project) is `Malformed`, never promoted to `Found("1")`.
// The only way to reach `MiAuthority::Malformed` at all in the committed
// suite (also closes I6). Kills the mutation that promotes an empty
// `instance_ets_id` to `Found("1")` (M8).
#[tokio::test]
async fn an_empty_instance_ets_id_is_malformed_not_promoted_to_mi_1() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("M-1", "")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["editable"], false);
    assert!(f["writeEtsId"].is_null());

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["message"]
        == "An imported module instance's identifier has an unexpected shape; this module's fields are read-only."
        && d["detail"].as_str().unwrap().contains("has Id ''")));
}

// S2 (fix round 2): `resolve_mi_authority`'s doc comment claims the
// leading underscore is what keeps a short suffix from matching --
// prove it. `"MOD-1_M-1"` both contains and (unanchored) ends with
// `"OD-1_M-1"`; only the anchored `ends_with("_OD-1_M-1")` is false, so
// this instance must not earn authority. Kills the `contains` mutation
// (M4) and the ends_with-without-underscore mutation (M6).
#[tokio::test]
async fn an_unanchored_suffix_match_does_not_earn_module_authority() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("OD-1_M-1", "OD-1_M-1_MI-1")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["editable"], false);
    assert!(f["writeEtsId"].is_null());

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["message"]
        == "No imported module instance matches this module; its fields are read-only."));
}

// S3 (fix round 2): a wrong-`MI-` digit or a foreign module's id is
// module-qualified and genuinely well-formed -- the parameter *is*
// declared, only its instance is wrong -- so the rejection message must
// not claim it "is not a parameter declared by this program". Reuses
// `decompose_module_qualified`, never a second id-shape parser (this
// task's own rule).
#[tokio::test]
async fn a_wrong_mi_digit_or_foreign_module_write_is_rejected_honestly() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("M-1", "M-1_MI-1")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = post_panel(app.clone(), 1, "MOD-1_M-1_MI-9_P-1_R-1", "3").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = dto["error"].as_str().unwrap().to_string();
    assert!(!msg.contains("is not a parameter declared by this program"));
    assert!(msg.contains("no editable field's write target matches it"));

    let (status, dto) = post_panel(app, 1, "MOD-1_M-2_MI-1_P-1_R-1", "3").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = dto["error"].as_str().unwrap().to_string();
    assert!(!msg.contains("is not a parameter declared by this program"));
    assert!(msg.contains("no editable field's write target matches it"));
}

// S4 (fix round 2) -- reviewer's P9: two sections sharing one
// program-side `module_id` must not silently reconstruct the same
// `write_ets_id`. Even with a genuine authoritative `ModuleInstance`
// present, the repeat itself is refused (mirrors the `Ambiguous`
// project-side refusal, D40) rather than letting both sections collapse
// onto one write target.
#[tokio::test]
async fn a_program_repeating_one_module_id_refuses_to_guess_which_section_is_authoritative() {
    let (_dir, products) = temp_product_db(REPEATED_MODULE_ID_PROGRAM);
    let state = Arc::new(state_with_device_and_modules(
        products,
        vec![],
        vec![("M-1", "M-1_MI-1")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let module_sections: Vec<&Value> = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !s["scope"].is_null())
        .collect();
    assert_eq!(module_sections.len(), 2);
    for s in &module_sections {
        assert_eq!(s["fields"][0]["editable"], false);
        assert!(s["fields"][0]["writeEtsId"].is_null());
    }

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .filter(|d| d["message"]
                == "Two or more sections in this program declare the same module id; its fields are read-only.")
            .count()
            >= 2
    );
}

// S5 (fix round 2) -- reviewer's P4: D41's collision path is reachable
// without any imported `ModuleInstance` at all -- two stored rows for
// the same module and declared parameter, differing only in their `MI-`
// digit, on a device with no authority to prefer one over the other.
// Kills the mutation that lets the row silently overwrite instead of
// diagnosing and staling the loser (M10).
#[tokio::test]
async fn two_stored_rows_differing_only_in_mi_digit_collide_without_any_authority() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(
        products,
        vec![
            ("MOD-1_M-1_MI-1_P-1_R-1", "1"),
            ("MOD-1_M-1_MI-2_P-1_R-1", "2"),
        ],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let stale_ids: Vec<&str> = dto["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["etsId"].as_str().unwrap())
        .collect();
    assert_eq!(stale_ids, vec!["MOD-1_M-1_MI-2_P-1_R-1"]);
    assert_eq!(dto["stale"][0]["raw"], "2");

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["message"]
        == "Two stored values target the same module-scoped parameter; the later one is ignored."
        && d["detail"]
            .as_str()
            .unwrap()
            .contains("MOD-1_M-1_MI-1_P-1_R-1")
        && d["detail"]
            .as_str()
            .unwrap()
            .contains("MOD-1_M-1_MI-2_P-1_R-1")));
    // I2 (fix round 2): the collision diagnostic now carries the
    // module's own scope, not `None`.
    assert!(diagnostics.iter().any(|d| d["message"]
        == "Two stored values target the same module-scoped parameter; the later one is ignored."
        && d["scope"]["moduleId"] == "MOD-1_M-1"));

    let f = field(&dto, "MOD-1_P-1_R-1").expect("MOD-1_P-1_R-1 present");
    assert_eq!(f["value"], "1");
    assert_eq!(f["valueSource"], "Stored");
    assert_eq!(f["editable"], false);
}

// I1 (fix round 2): a second stored row for the same unscoped id is now
// diagnosed and kept in `stale`, not silently dropped the way the
// committed round left it.
#[tokio::test]
async fn a_duplicate_unscoped_stored_row_is_diagnosed_and_kept_stale() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(
        products,
        vec![("P-1_R-1", "4"), ("P-1_R-1", "5")],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let stale_ids: Vec<&str> = dto["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["etsId"].as_str().unwrap())
        .collect();
    assert_eq!(stale_ids, vec!["P-1_R-1"]);
    assert_eq!(dto["stale"][0]["raw"], "5");

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["message"]
        == "Two stored values target the same parameter; the later one is ignored."
        && d["detail"].as_str().unwrap().contains("keeping '4'")));

    let f = field(&dto, "P-1_R-1").expect("P-1_R-1 present");
    assert_eq!(f["value"], "4");
    assert_eq!(f["valueSource"], "Stored");
}

// ---------------------------------------------------------------------
// Task 5: the sharp claim E2 flags and no existing test here settles --
// that the server actually runs D42's second, scoped `evaluate` call,
// rather than reusing the first (unscoped) one for everything but a
// field's raw value/valueSource. Every field-presence assertion above
// this point is satisfied identically whether or not that second call
// runs, because none of `WRITE_PROGRAM`/`TWO_INSTANTIATION_PROGRAM`/
// `KV_SHAPE_PROGRAM` has a `choose` whose outcome a module-scoped stored
// value can flip -- only `CHOOSE_GATED_MODULE_PROGRAM` does.
// ---------------------------------------------------------------------

fn field_ids_for_module(dto: &Value, module_id: &str) -> Vec<String> {
    dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["scope"]["moduleId"] == module_id)
        .unwrap_or_else(|| panic!("no section for module '{module_id}'"))["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["etsId"].as_str().unwrap().to_string())
        .collect()
}

// Item 1: with the scoped values supplied, the two instantiations produce
// different active *sets* -- not merely different values for the same
// field, but a different field present at all (`_P-2_R-1` vs `_P-3_R-1`),
// named by their exact `ref_id`s.
#[tokio::test]
async fn a_module_scoped_choose_shows_different_field_sets_when_stored_values_diverge() {
    let (_dir, products) = temp_product_db(CHOOSE_GATED_MODULE_PROGRAM);
    let state = Arc::new(state_with_device(
        products,
        vec![(
            "M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1",
            "20", // >10: M-4 selects the "High" branch
        )],
        // M-5 has no stored row at all, so it falls back to the program
        // default (5, not >10) and selects the "Low" branch instead.
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        dto["stale"].as_array().unwrap().is_empty(),
        "{:?}",
        dto["stale"]
    );

    let m4 = field_ids_for_module(&dto, "M-00FA_A-2504-10-C071_MD-2_M-4");
    let m5 = field_ids_for_module(&dto, "M-00FA_A-2504-10-C071_MD-2_M-5");
    assert_eq!(
        m4,
        vec![
            "M-00FA_A-2504-10-C071_MD-2_P-1_R-1",
            "M-00FA_A-2504-10-C071_MD-2_P-2_R-1",
        ],
        "M-4's own stored value (20, >10) selects the High branch"
    );
    assert_eq!(
        m5,
        vec![
            "M-00FA_A-2504-10-C071_MD-2_P-1_R-1",
            "M-00FA_A-2504-10-C071_MD-2_P-3_R-1",
        ],
        "M-5, with no stored row of its own, sees the program default (5, not >10) \
         and selects the Low branch"
    );
    assert_ne!(
        m4, m5,
        "the two sections' active field sets genuinely differ"
    );
}

// Item 2: the explicit contrast -- with no stored rows anywhere, both
// instantiations fall back to the identical program default and agree on
// the same active set. This is the pre-T18-slice-4 behaviour, kept so the
// diverging case above is read against a baseline, not in isolation.
#[tokio::test]
async fn a_module_scoped_choose_shows_the_same_field_set_without_any_stored_values() {
    let (_dir, products) = temp_product_db(CHOOSE_GATED_MODULE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let m4 = field_ids_for_module(&dto, "M-00FA_A-2504-10-C071_MD-2_M-4");
    let m5 = field_ids_for_module(&dto, "M-00FA_A-2504-10-C071_MD-2_M-5");
    let expected = vec![
        "M-00FA_A-2504-10-C071_MD-2_P-1_R-1".to_string(),
        "M-00FA_A-2504-10-C071_MD-2_P-3_R-1".to_string(),
    ];
    assert_eq!(
        m4, expected,
        "the program default (5) is not >10: Low branch"
    );
    assert_eq!(
        m5, m4,
        "no stored rows anywhere: both instantiations see the identical program default"
    );
}
