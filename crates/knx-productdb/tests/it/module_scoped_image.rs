//! Discriminating tests for the module-scoped parameter admission path in
//! the image builder (ADR-0177): a module-local `Memory/@BaseOffset`
//! placement resolves per instantiation from the bound argument, and every
//! refusal branch names its cause. Synthetic fixture only — no manufacturer
//! runtime semantics.

use std::collections::BTreeMap;

use knx_core::{DeviceId, IndividualAddress, ModuleInstance, ModuleInstanceId, SourceRef};
use knx_productdb::device_evaluation::{evaluate_device, DeviceEvaluation};
use knx_productdb::image::{build_download_image, ImageError, ImageRequest};

const ID: &str = "M-0002_A-0002-01-0000";

/// One program with one `ModuleDef` (`MD`) carrying a numeric argument
/// `Base` (`ParamOffsBase`-style) and a module-local parameter `MP` whose
/// `Memory` uses `BaseOffset="Base"`, plus one instantiation `MI` binding
/// `Base="4"`. Segment `S` is 32 octets so the written octet is readable
/// back through `DownloadImage::segment`.
const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0002">
<ApplicationPrograms><ApplicationProgram Id="M-0002_A-0002-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="S" Address="17408" Size="32"><Data>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</Data></AbsoluteSegment></Code>
<ParameterTypes><ParameterType Id="PT" Name="Mode"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="3" /></ParameterType></ParameterTypes>
<ModuleDefs><ModuleDef Id="MD" Name="module">
<Arguments><Argument Id="MD_A-1" Name="Base" /></Arguments>
<Static>
<Parameters><Parameter Id="MP" Name="Module param" ParameterType="PT" Value="1"><Memory CodeSegment="S" Offset="2" BitOffset="0" BaseOffset="Base" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="MP_R" RefId="MP" /></ParameterRefs>
</Static>
<Dynamic><ParameterRefRef RefId="MP_R" /></Dynamic>
</ModuleDef></ModuleDefs>
</Static><Dynamic><ChannelIndependentBlock><Module Id="MI_M-1" RefId="MD"><NumericArg RefId="MD_A-1" Value="4" /></Module></ChannelIndependentBlock></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn fixture(xml: &str) -> (tempfile::TempDir, knx_productdb::Connection, ImageRequest) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "synthetic.xml", xml.as_bytes()).unwrap();
    let request = ImageRequest {
        program_id: ID.into(),
        individual_address: IndividualAddress::from_raw(0x1234),
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
        priority_statements: BTreeMap::new(),
    };
    (dir, conn, request)
}

fn build(
    conn: &knx_productdb::Connection,
    request: &ImageRequest,
) -> Result<knx_productdb::image::DownloadImage, ImageError> {
    build_download_image(conn, request)
}

/// A module-scoped value key (`MI_M-1_MI-1_MP_R`) is decomposed and written
/// to `base + offset` = 4 + 2 = octet 6 of segment `S`; the default (1)
/// lands there without a supplied value, and a supplied 3 overwrites it.
#[test]
fn module_scoped_parameter_resolves_base_offset_per_instantiation() {
    let (_dir, conn, request) = fixture(PROGRAM);
    let image = build(&conn, &request).unwrap();
    let octets = &image.segment("S").unwrap().octets;
    assert_eq!(octets[6], 1, "product default 1 at base 4 + offset 2");

    let (_dir, conn, mut request) = fixture(PROGRAM);
    request.values.insert("MP_R".to_string(), "3".to_string());
    let image = build(&conn, &request).unwrap();
    let octets = &image.segment("S").unwrap().octets;
    assert_eq!(octets[6], 3, "supplied module-scoped value 3 at octet 6");
    // Neighbouring octets untouched: the write is exactly one octet wide.
    assert_eq!(octets[5], 0);
    assert_eq!(octets[7], 0);
}

/// A second instantiation binding a different `Base` writes to its own
/// octet — the base is per instantiation, never shared.
#[test]
fn each_instantiation_uses_its_own_bound_base() {
    let two = PROGRAM.replace(
        "</ChannelIndependentBlock></Dynamic>\n</ApplicationProgram>",
        "<Module Id=\"MI2_M-2\" RefId=\"MD\"><NumericArg RefId=\"MD_A-1\" Value=\"10\" /></Module></ChannelIndependentBlock></Dynamic>\n</ApplicationProgram>",
    );
    assert!(two.contains("MI2_M-2"));
    let (_dir, conn, mut request) = fixture(&two);
    request.values.insert("MP_R".to_string(), "3".to_string());
    let image = build(&conn, &request).unwrap();
    eprintln!("parameter_fields: {:?}", image.parameter_fields);
    let octets = &image.segment("S").unwrap().octets;
    assert_eq!(octets[4 + 2], 3, "first instance: base 4 + offset 2");
    assert_eq!(
        octets[10 + 2],
        3,
        "second instance: base 10 + offset 2 — same value, own base"
    );
}

/// `BaseOffset` naming an argument the `Module` does not bind refuses by
/// name instead of guessing a base.
#[test]
fn unbound_base_offset_argument_is_refused_by_name() {
    let unbound = PROGRAM.replace(
        "<Module Id=\"MI_M-1\" RefId=\"MD\"><NumericArg RefId=\"MD_A-1\" Value=\"4\" /></Module>",
        "<Module Id=\"MI_M-1\" RefId=\"MD\" />",
    );
    assert!(unbound.contains("<Module Id=\"MI_M-1\" RefId=\"MD\" />"));
    let (_dir, conn, request) = fixture(&unbound);
    let result = build(&conn, &request);
    assert!(
        matches!(&result, Err(ImageError::Unsupported { what }) if what.contains("unbound") && what.contains("Base")),
        "{result:?}"
    );
}

/// A non-numeric bound argument refuses by name.
#[test]
fn non_numeric_base_offset_argument_is_refused_by_name() {
    let nonnum = PROGRAM.replace("Value=\"4\"", "Value=\"high\"");
    let (_dir, conn, request) = fixture(&nonnum);
    let result = build(&conn, &request);
    assert!(
        matches!(&result, Err(ImageError::Unsupported { what }) if what.contains("not numeric")),
        "{result:?}"
    );
}

/// A base beyond 32 bits refuses by name instead of truncating.
#[test]
fn base_offset_beyond_32_bits_is_refused_by_name() {
    let huge = PROGRAM.replace("Value=\"4\"", "Value=\"4294967296\""); // 2^32
    let (_dir, conn, request) = fixture(&huge);
    let result = build(&conn, &request);
    assert!(
        matches!(&result, Err(ImageError::Unsupported { what }) if what.contains("exceeds 32-bit")),
        "{result:?}"
    );
}

/// `BaseOffset + Offset` overflowing u32 refuses by name instead of wrapping.
#[test]
fn base_plus_offset_overflow_is_refused_not_wrapped() {
    // u32::MAX base + Offset 2 wraps without checked_add.
    let overflow = PROGRAM.replace("Value=\"4\"", "Value=\"4294967295\"");
    let (_dir, conn, request) = fixture(&overflow);
    let result = build(&conn, &request);
    assert!(
        matches!(&result, Err(ImageError::Unsupported { what }) if what.contains("exceeds 32-bit source placement")),
        "{result:?}"
    );
}

/// A `ModuleDef`-local `Memory/@BaseOffset` placement is only ever activated
/// inside a module scope; the scope-less guard in the builder is defensive.
/// This test pins the admission contract from the other side: a module-scoped
/// `ComObjectRef` still refuses by name (`a module instance`), unchanged by
/// the parameter admission.
#[test]
fn module_scoped_com_object_ref_still_refuses_by_name() {
    let with_obj = PROGRAM.replace(
        "<Dynamic><ParameterRefRef RefId=\"MP_R\" /></Dynamic>\n</ModuleDef></ModuleDefs>",
        "<Dynamic><ParameterRefRef RefId=\"MP_R\" /><ComObjectRefRef RefId=\"MO_R\" /></Dynamic>\n</ModuleDef></ModuleDefs>",
    );
    let with_obj = with_obj.replace(
        "<ParameterRefs><ParameterRef Id=\"MP_R\" RefId=\"MP\" /></ParameterRefs>",
        "<ParameterRefs><ParameterRef Id=\"MP_R\" RefId=\"MP\" /></ParameterRefs>\n<ComObjectTable CodeSegment=\"S\" Offset=\"28\"><ComObject Id=\"MO\" Number=\"1\" ObjectSize=\"1 Byte\" /></ComObjectTable>\n<ComObjectRefs><ComObjectRef Id=\"MO_R\" RefId=\"MO\" /></ComObjectRefs>",
    );
    assert!(with_obj.contains("MO_R"));
    let (_dir, conn, request) = fixture(&with_obj);
    let result = build(&conn, &request);
    assert!(
        matches!(&result, Err(ImageError::Unsupported { what }) if what == "MO_R: a module instance"),
        "{result:?}"
    );
}

// ---------------------------------------------------------------------------
// WP-48 F14: per-instance value ownership versus the image seam's unscoped policy.
//
// Verified policy (ADR-0177, `image.rs` value lookup): `build_download_image` consumes
// `ImageRequest.values` as an unscoped map keyed by declared `ParameterRef` id. Each
// instantiation resolves its own `BaseOffset`, but a value is looked up scoped first and
// then falls back to the unscoped (program-level) value; no caller populates the scoped
// half of the `ValueMap` on the image path, so the fallback is the only value source today.
// A module-qualified key (`<prefix>_M-<m>_MI-<k>_<suffix>`) is NOT decomposed here and is
// refused by name. Per-instance value isolation therefore exists only in
// `device_evaluation::evaluate_device`; it is not wired into image building (finding F14a).
// ---------------------------------------------------------------------------

/// Real-shaped ids: `ModuleDef` `X_MD-1`, local reference `X_MD-1_R-1`, instances
/// `X_MD-1_M-1` (base 4) and `X_MD-1_M-2` (base 10), both using the same local reference.
fn two_instance_program() -> String {
    PROGRAM
        .replace("Id=\"MD\"", "Id=\"X_MD-1\"")
        .replace("RefId=\"MD\"", "RefId=\"X_MD-1\"")
        .replace("Id=\"MP_R\" RefId=\"MP\"", "Id=\"X_MD-1_R-1\" RefId=\"MP\"")
        .replace("RefId=\"MP_R\"", "RefId=\"X_MD-1_R-1\"")
        .replace("Id=\"MI_M-1\"", "Id=\"X_MD-1_M-1\"")
        .replace(
            "</ChannelIndependentBlock></Dynamic>\n</ApplicationProgram>",
            "<Module Id=\"X_MD-1_M-2\" RefId=\"X_MD-1\"><NumericArg RefId=\"MD_A-1\" Value=\"10\" /></Module></ChannelIndependentBlock></Dynamic>\n</ApplicationProgram>",
        )
}

const LOCAL_REF: &str = "X_MD-1_R-1";

fn expected_segment(writes: &[(usize, u8)]) -> Vec<u8> {
    let mut octets = vec![0u8; 32];
    for (index, value) in writes {
        octets[*index] = *value;
    }
    octets
}

/// The fallback policy, pinned: one unscoped value reaches every instantiation at its own
/// base; the product default does the same; every other octet stays as the template.
/// CURRENT POLICY PIN (WP-48 F14 / KL-068): flips when per-instance image values land.
#[test]
fn current_policy_unscoped_value_is_the_only_value_source_and_reaches_every_instance() {
    let program = two_instance_program();
    assert!(program.contains("X_MD-1_M-2") && program.contains("X_MD-1_M-1"));

    let (_dir, conn, request) = fixture(&program);
    let image = build(&conn, &request).unwrap();
    assert_eq!(
        image.segment("S").unwrap().octets,
        expected_segment(&[(6, 1), (12, 1)]),
        "product default at base 4+2 and base 10+2, nothing else"
    );

    let (_dir, conn, mut request) = fixture(&program);
    request.values.insert(LOCAL_REF.into(), "3".into());
    let image = build(&conn, &request).unwrap();
    assert_eq!(
        image.segment("S").unwrap().octets,
        expected_segment(&[(6, 3), (12, 3)]),
        "one unscoped value is shared by both instances"
    );
}

/// Two real module-qualified keys with different values are refused by name and write
/// nothing: the image seam must not silently fall back to the default or drop either value.
/// CURRENT POLICY PIN (WP-48 F14 / KL-068): flips when per-instance image values land.
#[test]
fn current_policy_module_qualified_keys_with_different_values_are_refused_not_silently_dropped() {
    let program = two_instance_program();
    let first = format!("X_MD-1_M-1_MI-1_{}", "R-1");
    let second = format!("X_MD-1_M-2_MI-1_{}", "R-1");
    for values in [
        vec![(first.as_str(), "3"), (second.as_str(), "2")],
        vec![(first.as_str(), "3")],
        // An unscoped value beside a scoped one must not mask the refusal.
        vec![(LOCAL_REF, "1"), (second.as_str(), "2")],
    ] {
        let (_dir, conn, mut request) = fixture(&program);
        for (key, value) in &values {
            request.values.insert((*key).into(), (*value).into());
        }
        let result = build(&conn, &request);
        assert!(
            matches!(&result, Err(ImageError::Value { cause, .. }) if cause.contains("no such ParameterRef")),
            "{values:?}: {result:?}"
        );
    }
}

fn module_instance(k: u32, source: &str) -> ModuleInstance {
    ModuleInstance {
        id: ModuleInstanceId(k),
        device: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: source.into(),
        },
        instance_ets_id: format!("{source}_MI-1"),
        repeat_index: String::new(),
        arguments: vec![],
    }
}

fn evaluate(stored: &[(&str, &str)], instances: &[ModuleInstance]) -> DeviceEvaluation {
    let (_dir, conn, _request) = fixture(&two_instance_program());
    let stored = stored
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    evaluate_device(&conn, ID, stored, instances).unwrap()
}

fn scoped_value<'a>(evaluation: &'a DeviceEvaluation, module_id: &str) -> Option<&'a str> {
    let scope = evaluation
        .activation
        .parameter_refs
        .iter()
        .find_map(|active| {
            active
                .scope
                .as_ref()
                .filter(|scope| scope.module_id.as_deref() == Some(module_id))
        })
        .unwrap_or_else(|| panic!("{module_id} is not activated"));
    evaluation.values.get(Some(scope), LOCAL_REF)
}

/// Where per-instance values do exist (the evaluator): two real scoped keys with different
/// values, one shared local reference; a third reader never sees another instance's value.
#[test]
fn evaluator_keeps_two_scoped_values_apart_for_one_shared_local_reference() {
    let instances = [
        module_instance(1, "MD-1_M-1"),
        module_instance(2, "MD-1_M-2"),
    ];
    let evaluation = evaluate(
        &[("X_MD-1_M-1_MI-1_R-1", "3"), ("X_MD-1_M-2_MI-1_R-1", "2")],
        &instances,
    );
    assert!(evaluation.stale.is_empty(), "{:?}", evaluation.stale);
    assert_eq!(evaluation.validated_scoped.len(), 2);
    assert_eq!(scoped_value(&evaluation, "X_MD-1_M-1"), Some("3"));
    assert_eq!(scoped_value(&evaluation, "X_MD-1_M-2"), Some("2"));

    // Only the first instance stores a value: the second keeps the program default, never
    // the sibling's 3.
    let evaluation = evaluate(&[("X_MD-1_M-1_MI-1_R-1", "3")], &instances);
    assert_eq!(scoped_value(&evaluation, "X_MD-1_M-1"), Some("3"));
    assert_eq!(scoped_value(&evaluation, "X_MD-1_M-2"), Some("1"));
}

/// An `MI-` counter that disagrees with the owning instance, and a `RefId` collision that
/// makes the owner ambiguous, are stale: no scoped value is validated for either.
#[test]
fn evaluator_refuses_mi_counter_mismatch_and_refid_collision() {
    let instances = [module_instance(1, "MD-1_M-1")];
    let evaluation = evaluate(&[("X_MD-1_M-1_MI-2_R-1", "3")], &instances);
    assert!(evaluation.validated_scoped.is_empty());
    assert_eq!(evaluation.stale.len(), 1);
    assert_eq!(scoped_value(&evaluation, "X_MD-1_M-1"), Some("1"));

    // Two instances claiming the same `RefId` and counter make the owner ambiguous
    // (ADR-0177 amended 2026-11-21 via `knx_core::resolve_module_owner`); an instance of
    // another module (`M-2`) is no owner of an `M-1` key either.
    let collision = [
        module_instance(1, "MD-1_M-1"),
        module_instance(2, "MD-1_M-1"),
        module_instance(3, "MD-1_M-2"),
    ];
    let evaluation = evaluate(&[("X_MD-1_M-1_MI-1_R-1", "3")], &collision);
    assert!(
        evaluation.validated_scoped.is_empty(),
        "an ambiguous owner is never first-matched"
    );
    assert_eq!(evaluation.stale.len(), 1);
    assert_eq!(scoped_value(&evaluation, "X_MD-1_M-1"), Some("1"));
}
