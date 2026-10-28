//! Defaults profile v3 keeps source Data for inactive parameters, like the configured path.
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use knx_app::property_validation_bundle::PlanRecord;
use knx_app::system2_validation_preparation::{
    prepare_simulated_candidate, ProjectValidationProfile, ValidationProfile,
};
use knx_core::IndividualAddress;
use knx_productdb::image::{
    build_experimental_system2_image, build_project_experimental_system2_image,
    ExperimentalParameterDisposition as Disposition, ExperimentalSystem2Image,
    ExperimentalSystem2ImagePolicy, ExperimentalSystem2SourceBinding, ImageRequest,
};
use knx_productdb::system2_procedure::{plan_system2_candidate, System2ProcedurePolicy};

use super::defaults_profile_v2::{
    assert_counts, json, public_conn, source_octet, synthetic_defaults, written, PROFILE_V2,
};
use super::{syn_ref, synthetic_package, PROFILE};

const PROFILE_V3: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/system2-validation/up258d12-920504-v3.json"
));
const BACKGROUND: &str = "background-parameter-default-union-base-active-overlay-v1";
const TEMPLATE: &str = "source-template-inactive-legacy-patch-active-overlay-v1";
/// Absolute address of the synthetic inactive parameter (PAR segment 15360 + offset 3).
const HIDDEN: u16 = 15363;

fn refused(value: &serde_json::Value) -> bool {
    ValidationProfile::from_json(&serde_json::to_vec(value).unwrap()).is_err()
}

#[test]
fn defaults_profile_grammar_binds_each_version_to_exactly_one_invisible_reading() {
    let (v2, v3) = (
        ValidationProfile::from_json(PROFILE_V2).unwrap(),
        ValidationProfile::from_json(PROFILE_V3).unwrap(),
    );
    assert_eq!(
        (v3.version, v3.id.as_str()),
        (3, "up258d12-920504-defaults-v3")
    );
    assert_eq!(
        (
            v2.invisible_parameters.as_str(),
            v3.invisible_parameters.as_str()
        ),
        (BACKGROUND, TEMPLATE)
    );
    // v3 changes only identity/version and the invisible-parameter reading.
    let (mut old, mut new) = (json(PROFILE_V2), json(PROFILE_V3));
    for field in ["id", "version", "invisible_parameters"] {
        old.as_object_mut().unwrap().remove(field);
        new.as_object_mut().unwrap().remove(field);
    }
    assert_eq!(old, new);
    // Each version accepts exactly its own reading.
    for (profile, wrong) in [
        (PROFILE, TEMPLATE),
        (PROFILE_V2, TEMPLATE),
        (PROFILE_V3, BACKGROUND),
    ] {
        let mut value = json(profile);
        value["invisible_parameters"] = wrong.into();
        assert!(refused(&value), "{} with {wrong}", value["id"]);
    }
    // A v3 grammar needs the v3 id suffix; an unknown later version refuses.
    let mut value = json(PROFILE_V3);
    value["id"] = "up258d12-920504-defaults-v2".into();
    assert!(refused(&value));
    let mut value = json(PROFILE_V3);
    value["version"] = 4.into();
    value["id"] = "up258d12-920504-defaults-v4".into();
    assert!(refused(&value));
    assert!(ProjectValidationProfile::from_json(PROFILE_V3).is_err());
}

/// The synthetic package plus one placed parameter referenced only in the inactive branch.
fn hidden_parameter_package() -> (Vec<u8>, Vec<u8>) {
    let (package, _) = synthetic_package(20);
    let mut reader = zip::ZipArchive::new(Cursor::new(package)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut source = Vec::new();
    let (q, rq, sel, or2) = (
        syn_ref("Q"),
        syn_ref("RQ"),
        syn_ref("PTSEL"),
        syn_ref("OR2"),
    );
    for index in 0..reader.len() {
        let mut member = reader.by_index(index).unwrap();
        let name = member.name().to_owned();
        let mut text = String::new();
        member.read_to_string(&mut text).unwrap();
        if name.ends_with("Application.xml") {
            // The shared fixture declares Background, as the pinned UP258 source does.
            assert_eq!(
                text.matches(r#"<Options DownloadInvisibleParameters="Background" />"#)
                    .count(),
                1
            );
            // Default 1 differs from the all-zero template octet at offset 3.
            for (anchor, insert) in [
                (
                    "</Parameters>",
                    format!(
                        r#"<Parameter Id="{q}" Name="Hidden" ParameterType="{sel}" Value="1"><Memory CodeSegment="PAR" Offset="3" BitOffset="0" /></Parameter>"#
                    ),
                ),
                (
                    "</ParameterRefs>",
                    format!(r#"<ParameterRef Id="{rq}" RefId="{q}" />"#),
                ),
            ] {
                assert_eq!(text.matches(anchor).count(), 1);
                text = text.replace(anchor, &format!("{insert}{anchor}"));
            }
            let branch = format!(r#"<when test="0"><ComObjectRefRef RefId="{or2}" /></when>"#);
            assert_eq!(text.matches(&branch).count(), 1);
            text = text.replace(
                &branch,
                &format!(r#"<when test="0"><ParameterRefRef RefId="{rq}" /><ComObjectRefRef RefId="{or2}" /></when>"#),
            );
            source = text.clone().into_bytes();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(text.as_bytes()).unwrap();
    }
    (writer.finish().unwrap().into_inner(), source)
}

/// Independent oracle: the named production builder for this profile's revision.
fn oracle(
    conn: &knx_productdb::Connection,
    profile: &[u8],
    target: IndividualAddress,
) -> (ExperimentalSystem2Image, String) {
    let profile = ValidationProfile::from_json(profile).unwrap();
    let request = ImageRequest {
        program_id: profile.program_id.clone(),
        individual_address: target,
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
    };
    let template = profile.version == 3;
    let policy = ExperimentalSystem2ImagePolicy {
        revision: if template { 2 } else { 1 },
        binding: ExperimentalSystem2SourceBinding {
            program_id: profile.program_id,
            schema_namespace: profile.schema_namespace,
            source_sha256: profile.source_sha256,
            package_sha256: Some(profile.package_sha256),
        },
        numeric_order: profile.numeric_order,
        bit_union_offsets: profile.bit_union_offsets,
        legacy_patch_always: profile.legacy_patch_always,
        invisible_parameters: profile.invisible_parameters,
        binary_restrictions: profile.binary_restrictions,
        reference_activation: profile.reference_activation,
    };
    let built = if template {
        build_project_experimental_system2_image(conn, &request, &policy)
    } else {
        build_experimental_system2_image(conn, &request, &policy)
    }
    .unwrap();
    let procedure = System2ProcedurePolicy::source_capacity_preserving_independent_data();
    let plan = plan_system2_candidate(&built.image, target, &procedure).unwrap();
    let sha = PlanRecord::from_plan(&plan).sha256().unwrap();
    (built, sha)
}

/// Per-address bit masks (MSB = bit 0) covered by active and by inactive parameter fields.
fn field_masks(
    code: &knx_productdb::code::ProgramCode,
    built: &ExperimentalSystem2Image,
) -> (BTreeMap<u16, u8>, BTreeMap<u16, u8>) {
    let (mut active, mut inactive) = (BTreeMap::new(), BTreeMap::new());
    for entry in &built.coverage {
        let (Some(segment), Some(field)) = (&entry.segment, entry.field) else {
            continue;
        };
        let base = code
            .segments
            .iter()
            .find(|s| &s.id == segment)
            .unwrap()
            .address;
        let target = if entry.active_reference.is_some() {
            &mut active
        } else {
            &mut inactive
        };
        let first = u32::from(field.bit_offset);
        for bit in first..first + field.size_in_bit {
            let address = u16::try_from(base + field.offset + bit / 8).unwrap();
            *target.entry(address).or_insert(0u8) |= 0x80 >> (bit % 8);
        }
    }
    (active, inactive)
}

fn disposition(image: &ExperimentalSystem2Image, parameter: &str) -> Disposition {
    image
        .coverage
        .iter()
        .find(|c| c.parameter_id == parameter)
        .unwrap()
        .disposition
}

#[test]
fn defaults_v3_keeps_the_template_octet_of_an_inactive_parameter_that_v2_overwrote() {
    let (package, source) = hidden_parameter_package();
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&conn, "hidden.knxprod", &package).unwrap();
    let target = IndividualAddress::from_raw(0x1234);
    let (profile_v2, profile_v3) = (
        synthetic_defaults(PROFILE_V2, &package, &source),
        synthetic_defaults(PROFILE_V3, &package, &source),
    );
    let before = conn.total_changes();
    let v2 = prepare_simulated_candidate(&conn, &profile_v2, target).unwrap();
    let v3 = prepare_simulated_candidate(&conn, &profile_v3, target).unwrap();
    assert_eq!(conn.total_changes(), before);
    assert_eq!(v3.bundle.binding.profile_version, 3);
    // Same procedure policy: identical coverage, only one octet value differs.
    assert_counts(&v2, (21, 48, 25, 23));
    assert_counts(&v3, (21, 48, 25, 23));
    let (old, new) = (written(&v2.bundle), written(&v3.bundle));
    assert_eq!(
        old.keys().collect::<Vec<_>>(),
        new.keys().collect::<Vec<_>>()
    );
    let changed: Vec<u16> = old
        .iter()
        .filter(|(address, byte)| new[address] != **byte)
        .map(|(address, _)| *address)
        .collect();
    assert_eq!(changed, [HIDDEN]);
    assert_eq!(
        (old[&HIDDEN], new[&HIDDEN]),
        (0x01, 0x00),
        "default vs template"
    );
    // The active parameter's overlay (4660 = 0x1234, high octet first) is unchanged.
    assert_eq!((new[&15360], new[&15361]), (0x12, 0x34));
    // Each profile equals its named builder; the v3 reading is the configured path's.
    let (built_v2, sha_v2) = oracle(&conn, &profile_v2, target);
    let (built_v3, sha_v3) = oracle(&conn, &profile_v3, target);
    assert_eq!(v2.bundle.binding.plan_sha256, sha_v2);
    assert_eq!(v3.bundle.binding.plan_sha256, sha_v3);
    let hidden = syn_ref("Q");
    assert_eq!(
        disposition(&built_v2, &hidden),
        Disposition::BackgroundDefault
    );
    assert_eq!(
        disposition(&built_v3, &hidden),
        Disposition::RetainedSourceTemplate
    );
    let disclosed = |c: &knx_app::system2_validation_preparation::PreparedCandidate, text: &str| {
        c.inferences.iter().any(|i| i.contains(text))
    };
    assert!(disclosed(&v2, "SUPERSEDED HYPOTHESIS"));
    assert!(disclosed(&v3, "OBSERVATION-SUPPORTED HYPOTHESIS"));
    assert!(disclosed(
        &v3,
        "No public document defines Background precedence"
    ));
    assert!(!disclosed(&v3, "SUPERSEDED"));
    v2.bundle.plan().unwrap();
    v3.bundle.plan().unwrap();
}

#[test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn exact_public_v3_changes_only_inactive_parameter_octets_of_v2() {
    let (_dir, conn) = public_conn();
    let target = IndividualAddress::from_raw(0x0119);
    let before = conn.total_changes();
    let v2 = prepare_simulated_candidate(&conn, PROFILE_V2, target).unwrap();
    let v3 = prepare_simulated_candidate(&conn, PROFILE_V3, target).unwrap();
    assert_eq!(conn.total_changes(), before);
    assert_counts(&v2, (21, 1514, 775, 739));
    assert_counts(&v3, (21, 1514, 775, 739));
    let (old, new) = (written(&v2.bundle), written(&v3.bundle));
    assert_eq!(
        old.keys().collect::<Vec<_>>(),
        new.keys().collect::<Vec<_>>()
    );
    let changed: Vec<u16> = old
        .iter()
        .filter(|(address, byte)| new[address] != **byte)
        .map(|(address, _)| *address)
        .collect();
    assert_eq!(changed.len(), 94, "{changed:x?}");
    assert!(
        changed.iter().all(|a| (0x3c01..=0x3ceb).contains(a)),
        "{changed:x?}"
    );
    // Independent of the builder: every changed v3 octet is the stored source Data.
    let code = knx_productdb::code::load_program_code(&conn, "M-0001_A-9205-04-9270")
        .unwrap()
        .unwrap();
    for address in &changed {
        assert_eq!(
            new[address],
            source_octet(&code, *address),
            "{address:#06x}"
        );
        assert_ne!(old[address], new[address]);
    }
    // Every changed bit lies in an inactive field; active bits of a shared octet are unchanged.
    let (built_v2, _) = oracle(&conn, PROFILE_V2, target);
    let (active, inactive) = field_masks(&code, &built_v2);
    for address in &changed {
        let diff = old[address] ^ new[address];
        let inactive_bits = inactive.get(address).copied().unwrap_or(0);
        let active_bits = active.get(address).copied().unwrap_or(0);
        assert_eq!(
            diff & !inactive_bits,
            0,
            "{address:#06x}: change outside inactive fields"
        );
        assert_eq!(diff & active_bits, 0, "{address:#06x}: active bits changed");
    }
    let (built, sha) = oracle(&conn, PROFILE_V3, target);
    assert_eq!(v3.bundle.binding.plan_sha256, sha);
    assert!(built.coverage.iter().all(|c| !matches!(
        c.disposition,
        Disposition::BackgroundDefault | Disposition::DefaultUnionBase
    )));
    v3.bundle.plan().unwrap();
}
