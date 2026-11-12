//! Exhaustive scoped settlement and independent evidence retention for written links and flags.

use super::*;
use knx_core::commissioning::parameter_baseline::{LinkBaseline, WrittenObjectFlags};

#[test]
fn link_settlement_truth_table_uses_scope_attempt_source_and_end_not_current_edits() {
    let at = chrono::DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::seconds(123);
    let mut written = request();
    written.links.push(knx_productdb::image::Link {
        object: 7,
        group_address: knx_core::GroupAddress::from_raw(0xffff),
        sending: true,
    });
    let mut now = written.clone();
    now.links.clear();
    now.values.insert("edited".into(), "during run".into());
    let mut cells = 0;
    for partial in [
        None,
        Some(PARAMETERS_ONLY),
        Some(GROUP_ADDRESSES_ONLY),
        Some(PartialDownloadParts {
            parameters: true,
            group_addresses: true,
        }),
    ] {
        for source in [SourcePlan::Unchanged, SourcePlan::ChangedOrUnknown] {
            for attempted_parameters in [false, true] {
                for attempted_links in [false, true] {
                    let attempted = ProgrammingParts {
                        parameters: attempted_parameters,
                        group_addresses: attempted_links,
                        ..ProgrammingParts::NONE
                    };
                    for end in [
                        DownloadEnd::NothingWritten,
                        DownloadEnd::Interrupted(attempted),
                        DownloadEnd::Completed,
                    ] {
                        for resolved in [None, Some(resolved())] {
                            let settlement = settle_download(
                                partial,
                                end,
                                &written,
                                Some(&now),
                                source,
                                resolved.clone(),
                                at,
                            );
                            let scope = written_parts(partial);
                            let expected = match end {
                                DownloadEnd::NothingWritten => LinkBaselineChange::Keep,
                                DownloadEnd::Interrupted(_)
                                    if scope.group_addresses && attempted_links =>
                                {
                                    LinkBaselineChange::Remove
                                }
                                DownloadEnd::Interrupted(_) => LinkBaselineChange::Keep,
                                DownloadEnd::Completed if !scope.group_addresses => {
                                    LinkBaselineChange::Keep
                                }
                                DownloadEnd::Completed
                                    if source == SourcePlan::ChangedOrUnknown =>
                                {
                                    LinkBaselineChange::Remove
                                }
                                DownloadEnd::Completed => {
                                    LinkBaselineChange::Record(LinkBaseline {
                                        program_id: written.program_id.clone(),
                                        at,
                                        links: vec![
                                            BaselineLink {
                                                object: 0,
                                                group_address: knx_core::GroupAddress::from_raw(
                                                    0x0801,
                                                ),
                                                sending: true,
                                            },
                                            BaselineLink {
                                                object: 7,
                                                group_address: knx_core::GroupAddress::from_raw(
                                                    0xffff,
                                                ),
                                                sending: true,
                                            },
                                        ],
                                    })
                                }
                            };
                            assert_eq!(
                                settlement.link_baseline,
                                expected,
                                "{partial:?} {source:?} {end:?} resolved={}",
                                resolved.is_some()
                            );
                            let parameter = match end {
                                DownloadEnd::NothingWritten => BaselineChange::Keep,
                                DownloadEnd::Interrupted(_)
                                    if scope.parameters && attempted_parameters =>
                                {
                                    BaselineChange::Remove
                                }
                                DownloadEnd::Interrupted(_) => BaselineChange::Keep,
                                DownloadEnd::Completed if !scope.parameters => BaselineChange::Keep,
                                DownloadEnd::Completed
                                    if source == SourcePlan::ChangedOrUnknown
                                        || resolved.is_none() =>
                                {
                                    BaselineChange::Remove
                                }
                                DownloadEnd::Completed => {
                                    let resolved = resolved.unwrap();
                                    BaselineChange::Record(ParameterBaseline {
                                        program_id: written.program_id.clone(),
                                        values: resolved.values,
                                        object_flags: resolved.object_flags,
                                        at: resolved.at,
                                    })
                                }
                            };
                            assert_eq!(settlement.parameter_baseline, parameter);
                            cells += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cells, 192);
    for end in [
        AddressEnd::Programmed,
        AddressEnd::Uncertain,
        AddressEnd::NothingWritten,
    ] {
        let s = settle_address(end);
        assert_eq!(s.parameter_baseline, BaselineChange::Keep);
        assert_eq!(s.link_baseline, LinkBaselineChange::Keep);
    }
}

#[test]
fn independent_edits_retain_other_section_and_its_timestamp_and_do_not_resurrect_stale_flags() {
    let old = chrono::DateTime::<Utc>::UNIX_EPOCH;
    let new = old + chrono::Duration::seconds(20);
    let device = DeviceId(1);
    let parameters = ParameterBaseline {
        program_id: "P".into(),
        values: Default::default(),
        object_flags: Some(
            [(
                1,
                WrittenObjectFlags {
                    read: true,
                    write: false,
                    transmit: true,
                    update: false,
                    communication: true,
                    read_on_init: None,
                },
            )]
            .into(),
        ),
        at: old,
    };
    let links = LinkBaseline {
        program_id: "P".into(),
        links: vec![],
        at: old,
    };
    let mut all = knx_core::commissioning::parameter_baseline::ParameterBaselines::new();
    all.apply(&BaselineEdit {
        device,
        parameter: BaselineChange::Record(parameters.clone()),
        links: LinkBaselineChange::Record(links.clone()),
    });
    let mut only_links = links.clone();
    only_links.at = new;
    all.apply(&BaselineEdit {
        device,
        parameter: BaselineChange::Keep,
        links: LinkBaselineChange::Record(only_links.clone()),
    });
    assert_eq!(all.get(device), Some(&parameters));
    let mut unknown = parameters;
    unknown.object_flags = None;
    unknown.at = new;
    all.apply(&BaselineEdit {
        device,
        parameter: BaselineChange::Record(unknown.clone()),
        links: LinkBaselineChange::Keep,
    });
    assert_eq!(all.get_links(device), Some(&only_links));
    assert_eq!(
        all.get(device),
        Some(&unknown),
        "a failed flag resolution cannot retain older flags"
    );
    all.apply(&BaselineEdit {
        device,
        parameter: BaselineChange::Remove,
        links: LinkBaselineChange::Keep,
    });
    assert_eq!(
        all.device_ids(),
        [device].into(),
        "link-only ownership survives parameter removal"
    );
    all.apply(&BaselineEdit::remove(device));
    assert!(all.is_empty());
}

#[test]
fn settlement_propagates_unknown_known_empty_and_known_nonempty_flags_without_stale_fallback() {
    let at = chrono::DateTime::<Utc>::UNIX_EPOCH;
    let flags = WrittenObjectFlags {
        read: true,
        write: false,
        transmit: true,
        update: false,
        communication: true,
        read_on_init: None,
    };
    for expected in [None, Some(Default::default()), Some([(7, flags)].into())] {
        let resolved = WrittenParameters {
            values: [("chosen".into(), "01".into())].into(),
            object_flags: expected.clone(),
            at,
        };
        let result = settle_download(
            Some(PARAMETERS_ONLY),
            DownloadEnd::Completed,
            &request(),
            None,
            SourcePlan::Unchanged,
            Some(resolved),
            at,
        );
        let BaselineChange::Record(record) = result.parameter_baseline else {
            panic!("resolved parameters must be recorded");
        };
        assert_eq!(record.values, [("chosen".into(), "01".into())].into());
        assert_eq!(record.object_flags, expected);
        assert_eq!(result.link_baseline, LinkBaselineChange::Keep);
    }
}
