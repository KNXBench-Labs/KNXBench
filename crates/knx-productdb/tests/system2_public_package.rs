//! Opt-in evidence for the public Siemens UP 258D12 package (issue #22).
//!
//! The package is a public manufacturer download
//! (<https://knx-td.sihub.siemens.com/product_data/ets5/UP258_DB12__920504d_ETS5.knxprod>);
//! it is never embedded in this repository. Set `KNXBENCH_PUBLIC_UP258` to
//! the downloaded file to run this test. No hardware, no bus, no admission.

use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_core::IndividualAddress;
use knx_productdb::code::{load_program_code, LoadStep};
use knx_productdb::download_plan::{check_program_kind, DownloadPlanError};
use knx_productdb::image::{build_system2_image, ImageError, ImageRequest};
use knx_productdb::system2::{project_tool_metadata, ProjectedStep};
use knx_productdb::{install_package_with_limits, open_and_migrate, PackageLimits};

/// The exact public package this evidence pins, by digest.
const PACKAGE_SHA256: &str = "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f";
/// The reported application program of issue #22.
const PROGRAM: &str = "M-0001_A-9205-04-9270";

fn install_public_up258() -> (tempfile::TempDir, knx_productdb::Connection) {
    let path = std::env::var_os("KNXBENCH_PUBLIC_UP258")
        .map(PathBuf::from)
        .expect("set KNXBENCH_PUBLIC_UP258 to the downloaded UP258_DB12__920504d_ETS5.knxprod");
    let bytes = std::fs::read(&path).expect("readable public package");
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        PACKAGE_SHA256,
        "the public package at {path:?} differs from the pinned source"
    );

    let dir = tempfile::tempdir().expect("isolated directory");
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).expect("isolated database");
    let name = "UP258_DB12__920504d_ETS5.knxprod";
    install_package_with_limits(&conn, name, &bytes, PackageLimits::LARGE)
        .expect("bounded public package import");
    (dir, conn)
}

#[test]
#[ignore = "requires the public Siemens UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn public_up258_package_pins_the_documented_refusals() {
    let (_dir, conn) = install_public_up258();
    let code = load_program_code(&conn, PROGRAM)
        .expect("application source readable")
        .expect("exact application installed");

    // Declared identity matches the issue: BCU 2 / System 2, product procedure.
    assert_eq!(code.mask_version.as_deref(), Some("MV-0025"));
    assert_eq!(
        code.load_procedure_style.as_deref(),
        Some("ProductProcedure")
    );
    assert_eq!(code.load_procedures.len(), 1);
    let procedure = &code.load_procedures[0];
    assert_eq!(procedure.merge_id, None);
    assert_eq!(procedure.steps.len(), 21, "the 21 ordered source steps");
    assert_eq!(procedure.unmodelled().count(), 0);
    // Literal oracle transcribed from the digest-pinned source, independently
    // of LoadStep parsing and metadata projection. Every source position counts;
    // these are declarations, NOT normalized property operations.
    assert_eq!(
        procedure.steps,
        vec![
            LoadStep::Connect,
            LoadStep::CompareProp {
                object_index: 0,
                property_id: 78,
                data: vec![0, 1, 0, 0, 0, 0x5B, 0, 0, 0, 0],
            },
            LoadStep::Unload { lsm: 1 },
            LoadStep::Unload { lsm: 2 },
            LoadStep::Unload { lsm: 3 },
            LoadStep::Load { lsm: 1 },
            LoadStep::AbsSegment {
                lsm: 1,
                segment_type: 0,
                address: 0x0116,
                size: 330,
                access: 0x33,
                memory_type: 3,
                flags: 0,
            },
            LoadStep::TaskSegment {
                lsm: 1,
                address: 0x0116
            },
            LoadStep::Load { lsm: 2 },
            LoadStep::AbsSegment {
                lsm: 2,
                segment_type: 0,
                address: 0x0260,
                size: 416,
                access: 0x33,
                memory_type: 3,
                flags: 0,
            },
            LoadStep::TaskSegment {
                lsm: 2,
                address: 0x0260
            },
            LoadStep::Load { lsm: 3 },
            LoadStep::AbsSegment {
                lsm: 3,
                segment_type: 0,
                address: 0x0400,
                size: 512,
                access: 0x33,
                memory_type: 3,
                flags: 0,
            },
            LoadStep::AbsSegment {
                lsm: 3,
                segment_type: 0,
                address: 0x3C00,
                size: 256,
                access: 0x33,
                memory_type: 3,
                flags: 0,
            },
            LoadStep::TaskSegment { lsm: 3, address: 0 },
            LoadStep::TaskCtrl2 {
                lsm: 3,
                callback: 0,
                address: 0x0400,
                segment_0: 0,
                segment_1: 0,
            },
            LoadStep::TaskSegment {
                lsm: 5,
                address: 0x0400
            },
            LoadStep::LoadCompleted { lsm: 1 },
            LoadStep::LoadCompleted { lsm: 2 },
            LoadStep::LoadCompleted { lsm: 3 },
            LoadStep::Disconnect,
        ]
    );
    assert_eq!(
        code.segments
            .iter()
            .map(|s| (s.id.as_str(), s.address, s.size))
            .collect::<Vec<_>>(),
        vec![
            ("M-0001_A-9205-04-9270_AS-0116", 0x0116, 330),
            ("M-0001_A-9205-04-9270_AS-0260", 0x0260, 416),
            ("M-0001_A-9205-04-9270_AS-0400", 0x0400, 512),
            ("M-0001_A-9205-04-9270_AS-3C00", 0x3C00, 256),
        ]
    );
    assert!(code.relative_segments.is_empty());
    assert!(!code.has_fixup_list);
    assert_eq!(
        code.program_attributes
            .get("ApplicationNumber")
            .map(String::as_str),
        Some("37381")
    );
    assert_eq!(
        code.program_attributes
            .get("ApplicationVersion")
            .map(String::as_str),
        Some("4")
    );
    assert_eq!(
        code.program_attributes
            .get("PreEts4Style")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        code.options,
        BTreeMap::from([("DownloadInvisibleParameters".into(), "Background".into())])
    );

    // Exact pointers and source positions come from the independently read XML.
    let metadata = project_tool_metadata(&code).expect("exact metadata witness");
    assert_eq!(metadata.table_address, 0x0400);
    assert_eq!(metadata.steps.len(), procedure.steps.len());
    assert_eq!(procedure.steps[0], LoadStep::Connect);
    assert_eq!(procedure.steps[20], LoadStep::Disconnect);
    assert!(!procedure.steps.contains(&LoadStep::Restart));
    assert_eq!(
        procedure.steps[15],
        LoadStep::TaskCtrl2 {
            lsm: 3,
            callback: 0,
            address: 0x0400,
            segment_0: 0,
            segment_1: 0,
        }
    );
    for (position, (projected, source)) in metadata.steps.iter().zip(&procedure.steps).enumerate() {
        if position == 16 {
            assert_eq!(
                projected,
                &ProjectedStep::ToolTableReference { address: 0x0400 }
            );
            assert_eq!(
                source,
                &LoadStep::TaskSegment {
                    lsm: 5,
                    address: 0x0400,
                }
            );
        } else {
            assert_eq!(
                projected,
                &ProjectedStep::Retained(source),
                "source position {position} must retain its exact step"
            );
        }
    }

    // Public planner policy stays refused; this is not a device memory-capability claim.
    assert_eq!(
        check_program_kind(&code),
        Err(DownloadPlanError::NotMemoryMapped(Some("MV-0025".into())))
    );

    // Complete-source image assembly still refuses; the default-only
    // request must not silently succeed where the witness is missing.
    let request = ImageRequest {
        program_id: PROGRAM.into(),
        individual_address: IndividualAddress::from_raw(0x0102),
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
    };
    assert!(code.has_legacy_patch_always);
    let before = conn.total_changes();
    let image =
        build_system2_image(&conn, &request).expect_err("the real source is not fully witnessed");
    // The source declares an uninterpreted patch policy. Its shared integrity
    // guard now precedes the historical P-901/default-bit refusal. Neither
    // the directive's meaning nor the remaining multibyte encoding is guessed.
    assert!(
        matches!(image, ImageError::Unsupported { ref what }
            if what == "System2 LegacyPatchAlways requires a separate source interpretation"),
        "the exact source-integrity refusal was expected, got: {image}"
    );
    assert_eq!(conn.total_changes(), before);
}

/// Exercise the exact reporter comparison through the existing resolver and
/// Dynamic evaluator, but never mistake activation for image completeness.
#[test]
#[ignore = "requires the public Siemens UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn public_up258_configured_values_do_not_bypass_the_source_stopline() {
    use knx_productdb::dynamic::{evaluate, load_program_trees, resolve_values};

    let (_dir, conn) = install_public_up258();
    let code = load_program_code(&conn, PROGRAM).unwrap().unwrap();
    let trees = load_program_trees(&conn, PROGRAM).unwrap();
    let before = conn.total_changes();
    for requested in ["300", "600"] {
        let request = ImageRequest {
            program_id: PROGRAM.into(),
            individual_address: IndividualAddress::from_raw(0x0102),
            values: BTreeMap::from([
                (format!("{PROGRAM}_P-900_R-900"), "0".into()),
                (format!("{PROGRAM}_P-903_R-903"), requested.into()),
            ]),
            links: Vec::new(),
            flag_overrides: BTreeMap::new(),
        };
        let supplied = request.values.clone().into_iter().collect();
        let values = resolve_values(&conn, PROGRAM, &supplied).unwrap();
        assert_eq!(
            values.get_unscoped(&format!("{PROGRAM}_P-901_R-901")),
            Some("120")
        );
        assert_eq!(
            values.get_unscoped(&format!("{PROGRAM}_P-903_R-903")),
            Some(requested)
        );
        let activation = evaluate(&trees, &values);
        let count = |reference: &str| {
            activation
                .parameter_refs
                .iter()
                .filter(|r| r.ref_id == reference)
                .count()
        };
        assert_eq!(count(&format!("{PROGRAM}_P-903_R-903")), 1);
        assert_eq!(count(&format!("{PROGRAM}_P-903_R-904")), 0);
        // Existing diagnostics remain visible, not stripped to obtain an image.
        assert_eq!(activation.diagnostics.len(), 11);
        let preview = knx_productdb::byte_order_probe::preview_system2_probe(
            &conn,
            &request,
            &[
                format!("{PROGRAM}_P-903_R-903"),
                format!("{PROGRAM}_P-900_R-900"),
            ],
        )
        .expect("the exact two source fields can be previewed without bus admission");
        assert_eq!(preview.diagnostics, activation.diagnostics);
        assert!(preview.has_uninterpreted_legacy_patch);
        assert_eq!(preview.declared_order, None);
        assert_eq!(preview.fields.len(), 2);
        assert_eq!(
            preview.fields[0].parameter_ref,
            format!("{PROGRAM}_P-903_R-903")
        );
        assert_eq!(preview.fields[0].address, 0x3CF8);
        assert_eq!(preview.fields[0].field.size_in_bit, 16);
        assert_eq!(preview.fields[0].value.to_string(), requested);
        assert_eq!(preview.fields[1].address, 0x3CFA);
        assert_eq!(preview.fields[1].value, 0);
        assert!(!preview
            .fields
            .iter()
            .any(|f| f.parameter_ref.ends_with("R-904")));
        let error = build_system2_image(&conn, &request)
            .expect_err("configured values do not establish LegacyPatchAlways semantics");
        assert!(matches!(error, ImageError::Unsupported { ref what }
            if what == "System2 LegacyPatchAlways requires a separate source interpretation"));
        assert_eq!(
            check_program_kind(&code),
            Err(DownloadPlanError::NotMemoryMapped(Some("MV-0025".into())))
        );
        assert_eq!(conn.total_changes(), before);
        assert_eq!(load_program_code(&conn, PROGRAM).unwrap().unwrap(), code);
    }
}

#[test]
#[ignore = "requires the public Siemens UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn public_up258_declares_four_read_only_evidence_ranges_and_its_hardware_type() {
    use knx_core::commissioning::declared_memory::DeclaredRange;
    use knx_productdb::declared_segments::derive_declared_segments;

    let (_dir, conn) = install_public_up258();
    let code = load_program_code(&conn, PROGRAM)
        .expect("application source readable")
        .expect("exact application installed");
    let profile = derive_declared_segments(&code).expect("exact System-2 evidence profile");
    assert_eq!(profile.program_id, PROGRAM);
    assert_eq!(profile.mask, 0x0025);
    let range = |address, length| DeclaredRange { address, length };
    assert_eq!(
        profile.ranges.ranges(),
        [
            range(0x0116, 330),
            range(0x0260, 416),
            range(0x0400, 512),
            range(0x3C00, 256),
        ]
    );
    assert_eq!(profile.ranges.total_octets(), 1514);
    assert_eq!(profile.range_positions, [6, 9, 12, 13]);
    assert_eq!(profile.compare_position, 1);
    assert_eq!(profile.hardware_type, [0, 1, 0, 0, 0, 0x5B, 0, 0, 0, 0]);
    assert_eq!(profile.ranges.chunks().len(), 128);
}
