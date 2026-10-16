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

#[test]
#[ignore = "requires the public Siemens UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn public_up258_package_pins_the_documented_refusals() {
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
    let image =
        build_system2_image(&conn, &request).expect_err("the real source is not fully witnessed");
    // The exact first refusal of the digest-pinned source: the 16-bit
    // parameter P-901 ("Sensor_Short_Signal_T1", default 120, AS-3C00
    // offset 240) has zero template octets at its placement, and the
    // source declares no ParameterByteOrder. The missing witness must
    // remain an explicit unsupported result, never a partial image.
    assert!(
        matches!(image, ImageError::Unsupported { ref what }
            if what.contains("P-901_R-901: System2 default disagrees with source bits")),
        "the documented source-bits refusal was expected, got: {image}"
    );
}
