//! "Download what is pending": flag advice bound to a concrete scope, on a real synthetic plan.
//!
//! The synthetic program of [`knx_testsupport::vendor_plugin`] without its
//! plug-in declaration plans the complete download and every partial one,
//! so every flag combination is resolved and prepared through the same
//! application service the CLI and the server use.

use knx_app::device_download::{prepare_device_download, PrepareError};
use knx_app::download_scope::{
    advise_device_download, prepare_scoped_download, resolve_scope, ScopeRequest,
    ScopedPrepareError,
};
use knx_core::commissioning::partial_memory_download::{PartialDownloadParts, PartialPlanError};
use knx_core::commissioning::scope_advice::{AddressNeed, AdvisedWrite, ScopeAdvice};
use knx_core::commissioning::LoadPart;
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, IndividualAddress,
    Installation, InstallationId, Language, Project, SourceRef, Topology,
};
use knx_testsupport::vendor_plugin::{
    program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, TARGET,
};

const PARAMETERS: PartialDownloadParts = PartialDownloadParts {
    parameters: true,
    group_addresses: false,
};

const BOTH: PartialDownloadParts = PartialDownloadParts {
    parameters: true,
    group_addresses: true,
};

fn target() -> IndividualAddress {
    TARGET.parse().unwrap()
}

/// The five flags as bits 0..5 of `bits`, in schema order.
fn flags(bits: u8) -> CommissioningState {
    let mut state = CommissioningState::default();
    for (index, part) in LoadPart::ALL.into_iter().enumerate() {
        state.set_intends_loaded(part, bits & (1 << index) != 0);
    }
    state
}

fn products_with(dir: &std::path::Path, program: &str) -> knx_productdb::Connection {
    let products = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    products
}

fn project(state: CommissioningState) -> Project {
    let mut project = Project::new(Language("en".into()));
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
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic actuator".into(),
        description: None,
        address: Some(target()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: state,
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

#[test]
fn every_flag_combination_resolves_and_prepares_exactly_the_advised_scope() {
    let dir = tempfile::tempdir().unwrap();
    let products = products_with(dir.path(), &program_xml(""));
    for bits in 0u8..32 {
        let project = project(flags(bits));
        let advice = ScopeAdvice::for_device(project.devices.get(DeviceId(1)).unwrap());
        assert_eq!(
            advise_device_download(&project, target()).unwrap(),
            advice,
            "{bits:05b}"
        );
        let scoped = prepare_scoped_download(&products, &project, target(), ScopeRequest::Changes)
            .unwrap_or_else(|e| panic!("{bits:05b}: {e}"));
        assert_eq!(scoped.advice, advice, "{bits:05b}");
        assert_eq!(scoped.resolved, advice.download(), "{bits:05b}");
        match (advice.download(), &scoped.prepared) {
            (None, None) => {}
            (Some(scope), Some(prepared)) => {
                assert_eq!(
                    prepared.partial.as_ref().map(|(parts, _)| *parts),
                    scope,
                    "{bits:05b}"
                );
                // The same plan the explicit request for that scope builds.
                let explicit = match scope {
                    None => ScopeRequest::Complete,
                    Some(parts) => ScopeRequest::Partial(parts),
                };
                let explicit = prepare_scoped_download(&products, &project, target(), explicit)
                    .unwrap()
                    .prepared
                    .unwrap();
                assert_eq!(prepared.plan, explicit.plan, "{bits:05b}");
            }
            (scope, prepared) => panic!("{bits:05b}: {scope:?} vs {:?}", prepared.is_some()),
        }
    }
}

#[test]
fn nothing_pending_prepares_no_download_at_all() {
    // An empty product database: any preparation would be refused, so an
    // answer without error proves none was attempted.
    let dir = tempfile::tempdir().unwrap();
    let empty = knx_productdb::open_and_migrate(&dir.path().join("empty.sqlite")).unwrap();
    for bits in [0b11111u8, 0b01111, 0b11110, 0b01110] {
        let project = project(flags(bits));
        let scoped =
            prepare_scoped_download(&empty, &project, target(), ScopeRequest::Changes).unwrap();
        assert!(scoped.prepared.is_none(), "{bits:05b}");
        assert_eq!(scoped.resolved, None, "{bits:05b}");
        assert_eq!(scoped.advice.write, AdvisedWrite::Nothing, "{bits:05b}");
        assert_eq!(scoped.advice.fully_programmed(), bits == 0b11111);
        assert_eq!(scoped.advice.medium_pending, bits & 0b10000 == 0);
        assert_eq!(
            scoped.advice.address,
            if bits & 1 == 0 {
                AddressNeed::Pending
            } else {
                AddressNeed::Current
            }
        );
        // The same request with an explicit scope does try, and is refused.
        assert!(matches!(
            prepare_scoped_download(&empty, &project, target(), ScopeRequest::Complete),
            Err(ScopedPrepareError::Prepare(_))
        ));
    }
}

#[test]
fn an_explicit_complete_download_stays_possible_when_nothing_is_pending() {
    let dir = tempfile::tempdir().unwrap();
    let products = products_with(dir.path(), &program_xml(""));
    let project = project(flags(0b11111));
    let scoped =
        prepare_scoped_download(&products, &project, target(), ScopeRequest::Complete).unwrap();
    let prepared = scoped.prepared.expect("explicit complete");
    assert!(prepared.partial.is_none());
    assert_eq!(scoped.resolved, Some(None));
    assert_eq!(
        prepared.plan,
        prepare_device_download(&products, &project, target())
            .unwrap()
            .plan
    );
}

#[test]
fn an_explicit_partial_request_is_unchanged_by_the_advice() {
    let dir = tempfile::tempdir().unwrap();
    let products = products_with(dir.path(), &program_xml(""));
    // The flags say the application is pending; the operator asked for
    // parameters only and gets exactly that.
    let project = project(flags(0));
    let scoped = prepare_scoped_download(
        &products,
        &project,
        target(),
        ScopeRequest::Partial(PARAMETERS),
    )
    .unwrap();
    assert_eq!(scoped.advice.write, AdvisedWrite::Complete);
    assert_eq!(scoped.resolved, Some(Some(PARAMETERS)));
    let expected = prepare_device_download(&products, &project, target())
        .unwrap()
        .into_partial(PARAMETERS)
        .unwrap();
    assert_eq!(scoped.prepared.unwrap().plan, expected.plan);
}

#[test]
fn a_partial_that_cannot_be_derived_is_refused_never_widened_to_complete() {
    // The same program without its address table (segment, table and load
    // controls): the complete download still plans, a group-addresses-only
    // one writes nothing, and that refusal must not turn into the complete
    // download.
    let mut program = program_xml("");
    for line in [
        "  <AbsoluteSegment Id=\"M-FFF6_A-0001-01-0000_AS-4000\" Address=\"16384\" Size=\"8\">\
         <Data>AAAAAAAAAAA=</Data></AbsoluteSegment>\n",
        "<AddressTable CodeSegment=\"M-FFF6_A-0001-01-0000_AS-4000\" Offset=\"0\" MaxEntries=\"2\" />\n",
        "<LdCtrlUnload LsmIdx=\"1\" />",
        "<LdCtrlLoad LsmIdx=\"1\" />\n",
        "  <LdCtrlAbsSegment LsmIdx=\"1\" SegType=\"0\" Address=\"16384\" Size=\"8\" \
         Access=\"255\" MemType=\"3\" SegFlags=\"128\" />\n",
        "  <LdCtrlLoadCompleted LsmIdx=\"1\" />\n",
    ] {
        assert!(program.contains(line), "fixture line {line:?}");
        program = program.replace(line, "");
    }
    let dir = tempfile::tempdir().unwrap();
    let products = products_with(dir.path(), &program);
    prepare_device_download(&products, &project(flags(0)), target())
        .expect("the complete download still plans");
    let project = project(flags(0b10111));
    match prepare_scoped_download(&products, &project, target(), ScopeRequest::Changes) {
        Err(ScopedPrepareError::Partial(PartialPlanError::NothingToWrite)) => {}
        other => panic!(
            "expected the partial refusal, got {:?}",
            other.map(|s| s.resolved)
        ),
    }
}

#[test]
fn the_advice_needs_the_one_device_at_the_target() {
    let mut project = project(flags(0));
    assert!(matches!(
        advise_device_download(&project, "1.1.99".parse().unwrap()),
        Err(PrepareError::NoDeviceAt(_))
    ));
    let mut twin = project.devices.get(DeviceId(1)).unwrap().clone();
    twin.id = DeviceId(2);
    project.devices.insert(twin);
    assert!(matches!(
        advise_device_download(&project, target()),
        Err(PrepareError::SeveralDevicesAt { .. })
    ));
}

#[test]
fn the_scope_resolves_from_the_project_alone() {
    // No product data at all: resolving reads the flags and nothing else.
    for bits in 0u8..32 {
        let project = project(flags(bits));
        let advice = advise_device_download(&project, target()).unwrap();
        let changes = resolve_scope(&project, target(), ScopeRequest::Changes).unwrap();
        assert_eq!(changes.advice, advice, "{bits:05b}");
        assert_eq!(changes.resolved, advice.download(), "{bits:05b}");
        let explicit =
            resolve_scope(&project, target(), ScopeRequest::Partial(PARAMETERS)).unwrap();
        assert_eq!(explicit.resolved, Some(Some(PARAMETERS)), "{bits:05b}");
        let complete = resolve_scope(&project, target(), ScopeRequest::Complete).unwrap();
        assert_eq!(complete.resolved, Some(None), "{bits:05b}");
    }
    assert!(matches!(
        resolve_scope(
            &project(flags(0)),
            "1.1.99".parse().unwrap(),
            ScopeRequest::Changes
        ),
        Err(PrepareError::NoDeviceAt(_))
    ));
}

#[test]
fn a_changes_request_resolves_again_from_the_project_it_is_prepared_from() {
    let dir = tempfile::tempdir().unwrap();
    let products = products_with(dir.path(), &program_xml(""));
    let mut project = project(flags(0b11011));
    let shown =
        prepare_scoped_download(&products, &project, target(), ScopeRequest::Changes).unwrap();
    assert_eq!(shown.resolved, Some(Some(PARAMETERS)));
    // An edit marks the group addresses pending too: the same request now
    // resolves, and prepares, another scope than the one shown.
    project
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .commissioning
        .communication_part_loaded = false;
    let now =
        prepare_scoped_download(&products, &project, target(), ScopeRequest::Changes).unwrap();
    assert_eq!(now.resolved, Some(Some(BOTH)));
    assert_eq!(
        now.prepared.unwrap().partial.map(|(parts, _)| parts),
        Some(BOTH)
    );
    assert_ne!(now.resolved, shown.resolved);
    // An explicit request never depends on the flags.
    let explicit = prepare_scoped_download(
        &products,
        &project,
        target(),
        ScopeRequest::Partial(PARAMETERS),
    )
    .unwrap();
    assert_eq!(explicit.resolved, Some(Some(PARAMETERS)));
}
