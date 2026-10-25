//! Two data-bearing source segments at one start never plan as a single silently kept payload.

use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::IndividualAddress;
use knx_productdb::download_plan::{plan_memory_download, DownloadPlanError};
use knx_productdb::image::{build_download_image, DownloadImage, ImageRequest};

const PROGRAM: &str = "M-0083_A-0027-15-0BAC";
const START: u32 = 0x4400;
/// `AA BB` and `CC DD`.
const FIRST: &str = "qrs=";
const SECOND: &str = "zN0=";
const NO_PAYLOAD_STACK: &str = r#"<AbsoluteSegment Id="STACK" Address="1944" Size="1" />"#;
const STACK_ALLOCATION: &str = r#"<LdCtrlAbsSegment LsmIdx="3" SegType="1" Address="1944" Size="1" Access="0" MemType="2" SegFlags="0" />"#;

fn segment(id: &str, address: u32, data: &str) -> String {
    masked(id, address, data, "")
}

/// `mask` is base64, or empty for none.
fn masked(id: &str, address: u32, data: &str, mask: &str) -> String {
    let mask = if mask.is_empty() {
        String::new()
    } else {
        format!("<Mask>{mask}</Mask>")
    };
    format!(
        r#"<AbsoluteSegment Id="{id}" Address="{address}" Size="2"><Data>{data}</Data>{mask}</AbsoluteSegment>"#
    )
}

fn allocation(address: u32) -> String {
    format!(
        r#"<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="{address}" Size="2" Access="255" MemType="3" SegFlags="128" />"#
    )
}

/// A mask-0701 product whose procedure allocates `allocations` on machine 3.
fn product(segments: &[String], allocations: &[String]) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0083"><ApplicationPrograms>
<ApplicationProgram Id="{PROGRAM}" Name="Duplicate start" ApplicationNumber="39" ApplicationVersion="21" PeiType="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code>{}</Code>
<LoadProcedures><LoadProcedure><LdCtrlConnect /><LdCtrlUnload LsmIdx="3" /><LdCtrlLoad LsmIdx="3" />{}<LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlRestart /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#,
        segments.concat(),
        allocations.concat()
    )
}

/// The public path: ingest the source, then build the image the planner is given.
fn image_of(xml: &str) -> DownloadImage {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "duplicate-start.xml", xml.as_bytes()).expect("ingests");
    let request = ImageRequest {
        program_id: PROGRAM.into(),
        individual_address: IndividualAddress::from_raw(0x1101),
        values: Default::default(),
        links: Vec::new(),
        flag_overrides: Default::default(),
    };
    build_download_image(&conn, &request).expect("the public image builder keeps every Data")
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

fn survivors(image: &DownloadImage) -> Vec<(&str, u32, &[u8])> {
    image
        .segments
        .iter()
        .map(|segment| (segment.id.as_str(), segment.address, &segment.octets[..]))
        .collect()
}

#[test]
fn import_and_the_public_builder_keep_both_data_bearing_segments_in_source_order() {
    for (first, second, expected) in [
        (
            ("A", FIRST),
            ("B", SECOND),
            [("A", &[0xAA, 0xBB]), ("B", &[0xCC, 0xDD])],
        ),
        (
            ("B", SECOND),
            ("A", FIRST),
            [("B", &[0xCC, 0xDD]), ("A", &[0xAA, 0xBB])],
        ),
    ] {
        let xml = product(
            &[
                segment(first.0, START, first.1),
                segment(second.0, START, second.1),
            ],
            &[allocation(START)],
        );
        let image = image_of(&xml);
        let kept: Vec<_> = survivors(&image);
        let expected: Vec<_> = expected
            .iter()
            .map(|(id, octets)| (*id, START, &octets[..]))
            .collect();
        assert_eq!(kept, expected);
    }
}

#[test]
fn controls_a_single_segment_distinct_starts_and_a_no_payload_allocation_still_plan() {
    let single = image_of(&product(
        &[segment("A", START, FIRST)],
        &[allocation(START)],
    ));
    let plan = plan_memory_download(&single).expect("one segment per start plans");
    assert_eq!(writes(&plan), [(0x4400, vec![0xAA, 0xBB])]);

    let distinct = image_of(&product(
        &[segment("A", START, FIRST), segment("B", 0x4500, SECOND)],
        &[allocation(START), allocation(0x4500)],
    ));
    let plan = plan_memory_download(&distinct).expect("distinct starts plan");
    assert_eq!(
        writes(&plan),
        [(0x4400, vec![0xAA, 0xBB]), (0x4500, vec![0xCC, 0xDD])]
    );

    let stack = image_of(&product(
        &[segment("A", START, FIRST), NO_PAYLOAD_STACK.to_string()],
        &[STACK_ALLOCATION.to_string(), allocation(START)],
    ));
    assert_eq!(
        survivors(&stack).len(),
        1,
        "the stack segment ships no Data"
    );
    let plan = plan_memory_download(&stack).expect("a no-payload allocation is intentional");
    assert_eq!(writes(&plan), [(0x4400, vec![0xAA, 0xBB])]);
}

/// Refusal naming the address and both source ids, not a plan that carries one payload.
fn refused_pair(first: String, second: String, ids: (&str, &str)) {
    let image = image_of(&product(&[first, second], &[allocation(START)]));
    let result = plan_memory_download(&image);
    let Err(error) = result else {
        panic!(
            "segments {} and {} both carry Data at 4400h, yet a plan was built that writes only {:?}",
            ids.0,
            ids.1,
            result.as_ref().map(writes).unwrap_or_default()
        );
    };
    assert_eq!(
        error,
        DownloadPlanError::DuplicateSegmentStart {
            address: 0x4400,
            first: ids.0.into(),
            second: ids.1.into(),
        }
    );
    let shown = error.to_string();
    assert!(
        shown.contains("4400h") && shown.contains(ids.0) && shown.contains(ids.1),
        "{shown}"
    );
}

fn refused(first: (&str, &str), second: (&str, &str)) {
    refused_pair(
        segment(first.0, START, first.1),
        segment(second.0, START, second.1),
        (first.0, second.0),
    );
}

#[test]
fn conflicting_payloads_at_one_start_refuse_when_a_comes_first() {
    refused(("A", FIRST), ("B", SECOND));
}

#[test]
fn conflicting_payloads_at_one_start_refuse_when_b_comes_first() {
    refused(("B", SECOND), ("A", FIRST));
}

/// Equal octets and masks that never collide are no precedence rule either.
#[test]
fn identical_payloads_and_disjoint_or_total_masks_are_equally_ambiguous() {
    refused(("A", FIRST), ("B", FIRST));
    let (low, high, none) = ("/wA=", "AP8=", "AAA=");
    for (a, b) in [(low, high), (high, low), (none, none), (none, low)] {
        refused_pair(
            masked("A", START, FIRST, a),
            masked("B", START, SECOND, b),
            ("A", "B"),
        );
    }
}

#[test]
fn a_duplicate_start_that_is_never_allocated_stays_a_never_written_refusal() {
    let image = image_of(&product(
        &[segment("A", START, FIRST), segment("B", START, SECOND)],
        &[allocation(0x4500)],
    ));
    assert_eq!(
        plan_memory_download(&image),
        Err(DownloadPlanError::NeverWritten("A".into()))
    );
}
