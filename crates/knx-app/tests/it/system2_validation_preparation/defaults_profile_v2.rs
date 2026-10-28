//! Defaults profile v2 writes independent source data that v1 omitted; grammar binds the policy.
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use knx_app::property_validation_bundle::{OperationRecord, PlanRecord, PreparedBundle};
use knx_app::system2_validation_preparation::{
    prepare_simulated_candidate, PreparedCandidate, ProjectValidationProfile, ValidationProfile,
};
use knx_core::IndividualAddress;
use knx_productdb::image::{
    build_experimental_system2_image, ExperimentalSystem2ImagePolicy,
    ExperimentalSystem2SourceBinding, ImageRequest,
};
use knx_productdb::system2_procedure::{plan_system2_candidate, System2ProcedurePolicy};

use super::{synthetic_package, synthetic_profile, PROFILE, SYN_ID};

pub(super) const PROFILE_V2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/system2-validation/up258d12-920504-v2.json"
));
const DECLARED: &str = "declared-counts-zero-flags-full-table-identity-v1";
const INDEPENDENT: &str = "source-capacity-preserving-independent-data-v1";

pub(super) fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn defaults_profile_grammar_binds_each_version_to_exactly_one_procedure_policy() {
    let v1 = ValidationProfile::from_json(PROFILE).unwrap();
    let v2 = ValidationProfile::from_json(PROFILE_V2).unwrap();
    assert_eq!((v1.version, v1.procedure_policy.as_str()), (1, DECLARED));
    assert_eq!((v2.version, v2.procedure_policy.as_str()), (2, INDEPENDENT));
    assert_eq!(v2.id, "up258d12-920504-defaults-v2");
    // v2 changes only identity/version/procedure, never an image-interpretation rule.
    let (mut old, mut new) = (json(PROFILE), json(PROFILE_V2));
    for field in ["id", "version", "procedure_policy"] {
        old.as_object_mut().unwrap().remove(field);
        new.as_object_mut().unwrap().remove(field);
    }
    assert_eq!(old, new);
    for (version, policy) in [
        (1, INDEPENDENT),
        (2, DECLARED),
        (3, DECLARED),
        (3, INDEPENDENT),
        (0, DECLARED),
        (0, INDEPENDENT),
    ] {
        let mut value = json(PROFILE);
        value["version"] = version.into();
        value["procedure_policy"] = policy.into();
        assert!(
            ValidationProfile::from_json(&serde_json::to_vec(&value).unwrap()).is_err(),
            "version {version} with {policy} must be refused"
        );
    }
    for (old, new) in [
        ("complete-defaults-no-links-v1", "any-project"),
        ("strong-hypothesis-simulator-validation-only", "verified"),
        (INDEPENDENT, "guess"),
    ] {
        let text = String::from_utf8(PROFILE_V2.to_vec())
            .unwrap()
            .replace(old, new);
        assert!(ValidationProfile::from_json(text.as_bytes()).is_err());
    }
    // Project profiles are version 2 too: only a defaults id enters this grammar.
    let mut value = json(PROFILE_V2);
    value["id"] = "up258d12-configured-v2".into();
    assert!(ValidationProfile::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut value = json(PROFILE_V2);
    value["unknown_rule"] = true.into();
    assert!(ValidationProfile::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(ProjectValidationProfile::from_json(PROFILE_V2).is_err());
}

/// The synthetic package with non-zero source octets after each 15-octet table reservation.
fn tailed_package() -> (Vec<u8>, Vec<u8>) {
    let (package, _) = synthetic_package(20);
    let mut reader = zip::ZipArchive::new(Cursor::new(package)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut source = Vec::new();
    for index in 0..reader.len() {
        let mut member = reader.by_index(index).unwrap();
        let name = member.name().to_owned();
        let mut text = String::new();
        member.read_to_string(&mut text).unwrap();
        if name.ends_with("Application.xml") {
            for (id, data) in [
                ("GA", "AAAAAAAAAAAAAAAAAAAAww=="),
                ("AA", "AAAAAAAAAAAAAAAAAAAAWg=="),
            ] {
                let zero = "<Data>AAAAAAAAAAAAAAAAAAAAAA==</Data>";
                let prefix = format!("Id=\"{id}\" Address=");
                let start = text.find(&prefix).unwrap();
                let at = start + text[start..].find(zero).unwrap();
                text.replace_range(at..at + zero.len(), &format!("<Data>{data}</Data>"));
            }
            source = text.clone().into_bytes();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(text.as_bytes()).unwrap();
    }
    (writer.finish().unwrap().into_inner(), source)
}

/// A synthetic complete-defaults profile carrying the grammar fields of `defaults`.
pub(super) fn synthetic_defaults(defaults: &[u8], package: &[u8], source: &[u8]) -> Vec<u8> {
    let mut value = json(&synthetic_profile(20, package, source));
    let defaults = json(defaults);
    for field in [
        "format",
        "version",
        "configuration",
        "invisible_parameters",
        "procedure_policy",
    ] {
        value[field] = defaults[field].clone();
    }
    value["id"] = format!("synthetic-defaults-v{}", defaults["version"]).into();
    value["package_sha256"] = value["product_input"]["package_sha256"].clone();
    value.as_object_mut().unwrap().remove("product_input");
    serde_json::to_vec(&value).unwrap()
}

/// Every written octet by address; overlapping writes would be a planner defect.
pub(super) fn written(bundle: &PreparedBundle) -> BTreeMap<u16, u8> {
    let mut octets = BTreeMap::new();
    for step in &bundle.normalized_plan.steps {
        if let OperationRecord::WriteMemory { address, data, .. } = &step.operation {
            for (offset, byte) in data.iter().enumerate() {
                let at = *address + u16::try_from(offset).unwrap();
                assert!(octets.insert(at, *byte).is_none(), "overlapping write {at}");
            }
        }
    }
    octets
}

/// Independent oracle: the production image builder plus an explicitly named policy.
fn reference_plan_sha256(
    conn: &knx_productdb::Connection,
    profile: &[u8],
    target: IndividualAddress,
    procedure: System2ProcedurePolicy,
) -> String {
    let profile = ValidationProfile::from_json(profile).unwrap();
    let request = ImageRequest {
        program_id: profile.program_id.clone(),
        individual_address: target,
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
    };
    let policy = ExperimentalSystem2ImagePolicy {
        revision: 1,
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
    let image = build_experimental_system2_image(conn, &request, &policy)
        .unwrap()
        .image;
    let plan = plan_system2_candidate(&image, target, &procedure).unwrap();
    PlanRecord::from_plan(&plan).sha256().unwrap()
}

/// Source `Data` of the installed segment holding `address`, never a second origin.
fn source_octet(code: &knx_productdb::code::ProgramCode, address: u16) -> u8 {
    let address = u32::from(address);
    let segment = code
        .segments
        .iter()
        .find(|s| s.address <= address && address < s.address + s.size)
        .unwrap();
    segment.data.as_ref().unwrap()[usize::try_from(address - segment.address).unwrap()]
}

pub(super) fn assert_counts(candidate: &PreparedCandidate, expected: (usize, usize, usize, usize)) {
    assert_eq!(
        (
            candidate.source_positions,
            candidate.allocation_octets,
            candidate.written_octets,
            candidate.preserved_octets
        ),
        expected
    );
}

#[test]
fn defaults_v2_writes_independent_source_octets_outside_the_reservation_v1_omits_them() {
    let (package, source) = tailed_package();
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&conn, "tailed.knxprod", &package).unwrap();
    let target = IndividualAddress::from_raw(0x1234);
    let (profile_v1, profile_v2) = (
        synthetic_defaults(PROFILE, &package, &source),
        synthetic_defaults(PROFILE_V2, &package, &source),
    );
    let before = conn.total_changes();
    let v1 = prepare_simulated_candidate(&conn, &profile_v1, target).unwrap();
    let v2 = prepare_simulated_candidate(&conn, &profile_v2, target).unwrap();
    assert_eq!(conn.total_changes(), before);
    assert_counts(&v1, (21, 48, 23, 25));
    assert_counts(&v2, (21, 48, 25, 23));
    assert_eq!(
        (
            v1.bundle.binding.profile_version,
            v2.bundle.binding.profile_version
        ),
        (1, 2)
    );
    let (old, new) = (written(&v1.bundle), written(&v2.bundle));
    let added: Vec<u16> = new
        .keys()
        .filter(|a| !old.contains_key(a))
        .copied()
        .collect();
    // 278 + 15 and 608 + 15: one octet after each declared 15-octet reservation.
    assert_eq!(added, [293, 623]);
    assert!(
        old.iter()
            .all(|(address, byte)| new.get(address) == Some(byte)),
        "v2 changes no octet v1 writes"
    );
    let code = knx_productdb::code::load_program_code(&conn, SYN_ID)
        .unwrap()
        .unwrap();
    for address in &added {
        assert_eq!(new[address], source_octet(&code, *address));
    }
    assert_eq!(
        (new[&293], new[&623]),
        (0xc3, 0x5a),
        "source content, not fill"
    );
    // v1 stays byte-identical to the old named policy; v2 is the capacity-preserving one.
    assert_eq!(
        v1.bundle.binding.plan_sha256,
        reference_plan_sha256(
            &conn,
            &profile_v1,
            target,
            System2ProcedurePolicy::declared_counts_zero_flags_full_table_identity()
        )
    );
    assert_eq!(
        v2.bundle.binding.plan_sha256,
        reference_plan_sha256(
            &conn,
            &profile_v2,
            target,
            System2ProcedurePolicy::source_capacity_preserving_independent_data()
        )
    );
    let disclosed =
        |c: &PreparedCandidate, text: &str| c.inferences.iter().any(|i| i.contains(text));
    assert!(disclosed(
        &v1,
        "independent source data outside the declared table reservation is NOT written"
    ));
    assert!(disclosed(
        &v2,
        "independent source data outside the declared table reservation is written"
    ));
    v1.bundle.plan().unwrap();
    v2.bundle.plan().unwrap();
}

pub(super) fn public_conn() -> (tempfile::TempDir, knx_productdb::Connection) {
    let path = std::env::var_os("KNXBENCH_PUBLIC_UP258").expect("set KNXBENCH_PUBLIC_UP258");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f"
    );
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package_with_limits(
        &conn,
        "public.knxprod",
        &bytes,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    (dir, conn)
}

#[test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn exact_public_v2_adds_exactly_the_independent_association_tail_that_v1_omits() {
    let (_dir, conn) = public_conn();
    let target = IndividualAddress::from_raw(0x0119);
    let before = conn.total_changes();
    let v1 = prepare_simulated_candidate(&conn, PROFILE, target).unwrap();
    let v2 = prepare_simulated_candidate(&conn, PROFILE_V2, target).unwrap();
    assert_eq!(conn.total_changes(), before);
    assert_counts(&v1, (21, 1514, 760, 754));
    assert_counts(&v2, (21, 1514, 775, 739));
    let (old, new) = (written(&v1.bundle), written(&v2.bundle));
    let added: Vec<u16> = new
        .keys()
        .filter(|a| !old.contains_key(a))
        .copied()
        .collect();
    assert_eq!(added, (0x03f1..=0x03ff).collect::<Vec<u16>>());
    assert!(
        old.iter()
            .all(|(address, byte)| new.get(address) == Some(byte)),
        "v2 changes no octet v1 writes"
    );
    let code = knx_productdb::code::load_program_code(&conn, "M-0001_A-9205-04-9270")
        .unwrap()
        .unwrap();
    let segment = code.segments.iter().find(|s| s.address == 0x0260).unwrap();
    assert!(segment.id.ends_with("AS-0260"), "{}", segment.id);
    assert_eq!(segment.size, 416);
    let data = segment.data.as_ref().unwrap();
    for address in added {
        let offset = usize::from(address - 0x0260);
        assert_eq!(new[&address], data[offset], "source Data at {address:#06x}");
        if let Some(mask) = &segment.mask {
            assert_eq!(mask[offset], 0xff, "writable source octet {address:#06x}");
        }
    }
    v2.bundle.plan().unwrap();
}
