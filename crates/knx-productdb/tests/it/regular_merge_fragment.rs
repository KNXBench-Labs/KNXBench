//! A declared MergeId fragment is refused by the regular planner, whatever its contents.

use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::IndividualAddress;
use knx_productdb::code::LoadStep;
use knx_productdb::download_plan::{
    plan_memory_download, plan_memory_download_with_inferences, DownloadPlanError,
};
use knx_productdb::image::{build_download_image, DownloadImage, ImageRequest};

const PROGRAM: &str = "M-0083_A-0027-15-0BAC";
const MAIN: &str = r#"<LoadProcedure><LdCtrlConnect /><LdCtrlUnload LsmIdx="3" /><LdCtrlLoad LsmIdx="3" /><LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="2" Access="255" MemType="3" SegFlags="128" /><LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlRestart /><LdCtrlDisconnect /></LoadProcedure>"#;
const NO_CONNECT: &str = r#"<LoadProcedure><LdCtrlUnload LsmIdx="3" /><LdCtrlLoad LsmIdx="3" /><LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="2" Access="255" MemType="3" SegFlags="128" /><LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlRestart /><LdCtrlDisconnect /></LoadProcedure>"#;
const UNMODELLED_MAIN: &str = r#"<LoadProcedure><LdCtrlConnect /><LdCtrlWriteProp ObjIdx="3" PropId="13" /><LdCtrlDisconnect /></LoadProcedure>"#;
const KNOWN_STEPS: &str = r#"<LdCtrlConnect /><LdCtrlUnload LsmIdx="3" /><LdCtrlDisconnect />"#;
const UNMODELLED_STEP: &str = r#"<LdCtrlWriteProp ObjIdx="3" PropId="13" Verify="true" />"#;

/// `AA BB` at 4400h, the payload every plan of this file must carry.
const PAYLOAD: [u8; 2] = [0xAA, 0xBB];

/// A fragment whose `MergeId` is written verbatim, so odd ids stay odd.
fn fragment(merge_id: &str, steps: &str) -> String {
    format!(r#"<LoadProcedure MergeId="{merge_id}">{steps}</LoadProcedure>"#)
}

/// A mask-0701 product with `procedures` in document order and chosen identity.
fn product(procedures: &[&str], mask: &str, style: &str, application: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0083"><ApplicationPrograms>
<ApplicationProgram Id="{PROGRAM}" Name="Merge fragment" ApplicationNumber="{application}" ApplicationVersion="21" PeiType="1" MaskVersion="{mask}" LoadProcedureStyle="{style}"><Static>
<Code><AbsoluteSegment Id="A" Address="17408" Size="2"><Data>qrs=</Data></AbsoluteSegment></Code>
<LoadProcedures>{}</LoadProcedures>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#,
        procedures.concat()
    )
}

fn regular(procedures: &[&str]) -> String {
    product(procedures, "MV-0701", "ProductProcedure", "39")
}

/// The public path: ingest the source, then build the image the planner is given.
fn image_of(xml: &str) -> DownloadImage {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "merge-fragment.xml", xml.as_bytes()).expect("ingests");
    let request = ImageRequest {
        program_id: PROGRAM.into(),
        individual_address: IndividualAddress::from_raw(0x1101),
        values: Default::default(),
        links: Vec::new(),
        flag_overrides: Default::default(),
    };
    build_download_image(&conn, &request).expect("the public image builder builds")
}

fn writes(plan: &MemoryDownloadPlan) -> Vec<(u16, Vec<u8>)> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            MemoryDownloadStep::WriteMemory { address, octets } => Some((*address, octets.clone())),
            _ => None,
        })
        .collect()
}

/// Both public entry points, so neither can keep a private way around the refusal.
fn refusals(image: &DownloadImage) -> [Result<MemoryDownloadPlan, DownloadPlanError>; 2] {
    [
        plan_memory_download(image),
        plan_memory_download_with_inferences(image).map(|(plan, _)| plan),
    ]
}

/// The refusal text of a source that must not plan; a built plan is the failure.
fn refused_text(xml: &str) -> String {
    let image = image_of(xml);
    let [plain, inferred] = refusals(&image);
    let Err(plain) = plain else {
        panic!(
            "a MergeId fragment was planned: {:?}",
            writes(&plain.unwrap())
        );
    };
    let Err(inferred) = inferred else {
        panic!("the inferring entry point planned a MergeId fragment");
    };
    assert_eq!(plain, inferred, "both entry points refuse alike");
    plain.to_string()
}

fn assert_names_fragment(text: &str, position: usize, merge_id: &str) {
    assert!(text.contains("MergeId"), "{text}");
    assert!(
        text.contains(&format!("load procedure {position} ")),
        "{text}"
    );
    assert!(text.contains(&format!("{merge_id:?}")), "{text}");
}

/// The exact plan of the fragment-free product; any fragment must leave it unreachable.
const CONTROL_PLAN: &str = "MemoryDownloadPlan { mask: MaskVersion(1793), manufacturer: 131, steps: [Connect, LoadRecord(MemoryLoadRecord([52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])), LoadRecord(MemoryLoadRecord([49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])), LoadRecord(MemoryLoadRecord([51, 0, 0, 68, 0, 0, 2, 255, 3, 128, 0])), WriteMemory { address: 17408, octets: [170, 187] }, LoadRecord(MemoryLoadRecord([50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])), Restart, Disconnect] }";

#[test]
fn control_without_a_fragment_plans_the_exact_payload_and_plan() {
    let image = image_of(&regular(&[MAIN]));
    for result in refusals(&image) {
        let plan = result.expect("one regular procedure plans");
        assert_eq!(writes(&plan), [(0x4400, PAYLOAD.to_vec())]);
        assert_eq!(format!("{plan:?}"), CONTROL_PLAN);
    }
}

#[test]
fn a_fragment_before_or_after_the_main_procedure_refuses_with_its_source_position() {
    let steps = fragment("7", KNOWN_STEPS);
    let before = refused_text(&regular(&[&steps, MAIN]));
    assert_names_fragment(&before, 1, "7");
    let after = refused_text(&regular(&[MAIN, &steps]));
    assert_names_fragment(&after, 2, "7");
}

#[test]
fn nonempty_empty_whitespace_and_unknown_ids_are_all_declared_fragments() {
    for merge_id in [
        "1",
        "",
        " ",
        "  \u{00a0}",
        "no-such-mask-procedure",
        "a&quot;b",
    ] {
        let shown = refused_text(&regular(&[MAIN, &fragment(merge_id, KNOWN_STEPS)]));
        let raw = merge_id.replace("&quot;", "\"");
        assert_names_fragment(&shown, 2, &raw);
    }
}

#[test]
fn the_typed_refusal_carries_the_one_based_position_and_the_raw_id() {
    let image = image_of(&regular(&[MAIN, &fragment("", ""), &fragment("2", "")]));
    for result in refusals(&image) {
        assert_eq!(
            result,
            Err(DownloadPlanError::MergeFragment {
                position: 2,
                merge_id: String::new()
            })
        );
    }
}

#[test]
fn the_contents_of_a_fragment_never_matter() {
    for steps in ["", KNOWN_STEPS, UNMODELLED_STEP, "<LdCtrlLoad />"] {
        let shown = refused_text(&regular(&[MAIN, &fragment("3", steps)]));
        assert_names_fragment(&shown, 2, "3");
        assert!(!shown.contains("LdCtrl"), "no step content in {shown}");
    }
}

#[test]
fn with_two_fragments_the_first_in_source_order_is_named_with_its_raw_id() {
    let (first, second) = (fragment(" 9 ", ""), fragment("2", KNOWN_STEPS));
    assert_names_fragment(&refused_text(&regular(&[MAIN, &first, &second])), 2, " 9 ");
    assert_names_fragment(&refused_text(&regular(&[&second, MAIN, &first])), 1, "2");
}

#[test]
fn the_procedure_count_still_wins_over_a_fragment() {
    let steps = fragment("1", KNOWN_STEPS);
    for procedures in [vec![steps.as_str()], vec![MAIN, MAIN, steps.as_str()]] {
        let count = procedures.iter().filter(|p| **p == MAIN).count();
        let image = image_of(&regular(&procedures));
        for result in refusals(&image) {
            assert_eq!(result, Err(DownloadPlanError::ProcedureCount(count)));
        }
    }
}

#[test]
fn a_fragment_wins_over_a_main_that_would_fail_later() {
    let steps = fragment("1", KNOWN_STEPS);
    for broken in [NO_CONNECT, UNMODELLED_MAIN] {
        let shown = refused_text(&regular(&[broken, &steps]));
        assert_names_fragment(&shown, 2, "1");
    }
    let image = image_of(&regular(&[NO_CONNECT]));
    assert_eq!(
        plan_memory_download(&image),
        Err(DownloadPlanError::DoesNotConnectFirst)
    );
}

#[test]
fn kind_and_identity_refusals_come_before_the_fragment_check() {
    let steps = fragment("1", KNOWN_STEPS);
    let both = [MAIN, steps.as_str()];
    let cases = [
        (
            product(&both, "MV-07B0", "ProductProcedure", "39"),
            DownloadPlanError::NotMemoryMapped(Some("MV-07B0".into())),
        ),
        (
            product(&both, "MV-0701", "MergedProcedure", "39"),
            DownloadPlanError::NotAProductProcedure(Some("MergedProcedure".into())),
        ),
        (
            product(&both, "MV-07B0", "MergedProcedure", "39"),
            DownloadPlanError::NotMemoryMapped(Some("MV-07B0".into())),
        ),
        (
            product(&both, "MV-0701", "ProductProcedure", "65536"),
            DownloadPlanError::Attribute {
                name: "ApplicationNumber",
                value: Some("65536".into()),
            },
        ),
    ];
    for (xml, expected) in cases {
        let image = image_of(&xml);
        for result in refusals(&image) {
            assert_eq!(result, Err(expected.clone()));
        }
    }
    let mut image = image_of(&regular(&both));
    image.code.program_id = "A-0027-15-0BAC".into();
    assert!(matches!(
        plan_memory_download(&image),
        Err(DownloadPlanError::ProgramId(_))
    ));
}

#[test]
fn stored_source_parser_image_and_refusal_keep_every_fragment_and_step() {
    let (first, second) = (fragment("", UNMODELLED_STEP), fragment("2", KNOWN_STEPS));
    let xml = regular(&[&first, MAIN, &second]);
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "merge-fragment.xml", xml.as_bytes()).expect("ingests");
    let stored_sha: String = conn
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id = ?1",
            [PROGRAM],
            |row| row.get(0),
        )
        .expect("the program row names its stored source");
    assert_eq!(stored_sha, knx_productdb::blob::sha256_hex(xml.as_bytes()));
    let stored = |when: &str| {
        let bytes = knx_productdb::blob::load_source_file(&conn, &stored_sha)
            .expect("reads")
            .expect("the source is stored");
        assert_eq!(bytes, xml.as_bytes(), "stored source bytes {when}");
    };
    stored("after ingest");

    let request = ImageRequest {
        program_id: PROGRAM.into(),
        individual_address: IndividualAddress::from_raw(0x1101),
        values: Default::default(),
        links: Vec::new(),
        flag_overrides: Default::default(),
    };
    let image = build_download_image(&conn, &request).expect("builds");
    stored("after the image");
    let known = |steps: &[LoadStep]| steps.to_vec();
    let write_prop = LoadStep::Unmodelled {
        name: "LdCtrlWriteProp".into(),
        attributes: [("ObjIdx", "3"), ("PropId", "13"), ("Verify", "true")]
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .into(),
        has_children: false,
    };
    let procedures = &image.code.load_procedures;
    let ids: Vec<_> = procedures.iter().map(|p| p.merge_id.as_deref()).collect();
    assert_eq!(ids, [Some(""), None, Some("2")]);
    assert_eq!(known(&procedures[0].steps), [write_prop]);
    assert_eq!(procedures[1].steps.len(), 7);
    assert_eq!(procedures[1].steps[0], LoadStep::Connect);
    assert_eq!(
        known(&procedures[2].steps),
        [
            LoadStep::Connect,
            LoadStep::Unload { lsm: 3 },
            LoadStep::Disconnect
        ]
    );

    let [plain, inferred] = refusals(&image);
    assert!(plain.is_err() && inferred.is_err(), "the planner refuses");
    stored("after the planning refusal");
    let reloaded = knx_productdb::code::load_program_code(&conn, PROGRAM)
        .expect("reads")
        .expect("known");
    assert_eq!(reloaded.load_procedures, image.code.load_procedures);
}
