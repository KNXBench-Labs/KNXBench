//! Every combination of the five `*Loaded` flags and an absent address, against the scope rules.

use super::*;
use crate::commissioning::load_state::LoadState;
use crate::commissioning::partial_memory_download::PartialDownloadParts;
use crate::commissioning::{CommissioningState, LoadPart};
use crate::IndividualAddress;

/// The state whose five flags are bits 0..5 of `bits`, in schema order:
/// address, application, parameters, communication part, medium.
fn state(bits: u8) -> CommissioningState {
    let mut state = CommissioningState::default();
    for (index, part) in LoadPart::ALL.into_iter().enumerate() {
        state.set_intends_loaded(part, bits & (1 << index) != 0);
    }
    state
}

fn address() -> Option<IndividualAddress> {
    Some("1.1.7".parse().unwrap())
}

const PARAMETERS: PartialDownloadParts = PartialDownloadParts {
    parameters: true,
    group_addresses: false,
};
const GROUP_ADDRESSES: PartialDownloadParts = PartialDownloadParts {
    parameters: false,
    group_addresses: true,
};
const BOTH: PartialDownloadParts = PartialDownloadParts {
    parameters: true,
    group_addresses: true,
};

#[test]
fn every_flag_combination_with_and_without_an_address_follows_the_rules() {
    for bits in 0u8..32 {
        let flags = state(bits);
        for at in [address(), None] {
            let advice = ScopeAdvice::from_flags(&flags, at);
            let case = format!("flags {bits:05b}, address {at:?}: {advice:?}");
            let expected = match (
                flags.application_program_loaded,
                flags.parameters_loaded,
                flags.communication_part_loaded,
            ) {
                (false, _, _) => AdvisedWrite::Complete,
                (true, false, false) => AdvisedWrite::Partial(BOTH),
                (true, false, true) => AdvisedWrite::Partial(PARAMETERS),
                (true, true, false) => AdvisedWrite::Partial(GROUP_ADDRESSES),
                (true, true, true) => AdvisedWrite::Nothing,
            };
            assert_eq!(advice.write, expected, "{case}");
            let address_need = match (at, flags.individual_address_loaded) {
                (None, _) => AddressNeed::Missing,
                (Some(_), false) => AddressNeed::Pending,
                (Some(_), true) => AddressNeed::Current,
            };
            assert_eq!(advice.address, address_need, "{case}");
            assert_eq!(advice.medium_pending, !flags.medium_config_loaded, "{case}");
            assert_eq!(
                advice.fully_programmed(),
                bits == 0b11111 && at.is_some(),
                "{case}"
            );
        }
    }
}

#[test]
fn address_and_medium_never_change_what_is_written() {
    // Bits 0 (address) and 4 (medium) vary; the application-side write must not.
    for core in 0u8..8 {
        let base = core << 1;
        let reference = ScopeAdvice::from_flags(&state(base), address()).write;
        for extra in [0b00001u8, 0b10000, 0b10001] {
            for at in [address(), None] {
                assert_eq!(
                    ScopeAdvice::from_flags(&state(base | extra), at).write,
                    reference,
                    "core {core:03b} extra {extra:05b} address {at:?}"
                );
            }
        }
    }
}

#[test]
fn the_download_scope_is_none_for_nothing_and_the_partial_parts_otherwise() {
    let advice = |bits| ScopeAdvice::from_flags(&state(bits), address());
    assert_eq!(advice(0b11111).download(), None);
    assert_eq!(advice(0b11101).download(), Some(None));
    assert_eq!(advice(0b11011).download(), Some(Some(PARAMETERS)));
    assert_eq!(advice(0b10111).download(), Some(Some(GROUP_ADDRESSES)));
    assert_eq!(advice(0b10011).download(), Some(Some(BOTH)));
}

#[test]
fn a_pending_medium_or_address_alone_writes_nothing_and_is_not_fully_programmed() {
    for (bits, at) in [
        (0b01111u8, address()),
        (0b11110, address()),
        (0b11111, None),
    ] {
        let advice = ScopeAdvice::from_flags(&state(bits), at);
        assert_eq!(advice.write, AdvisedWrite::Nothing, "{bits:05b} {at:?}");
        assert_eq!(advice.download(), None, "{bits:05b} {at:?}");
        assert!(!advice.fully_programmed(), "{bits:05b} {at:?}");
    }
}

#[test]
fn device_reports_and_bookkeeping_do_not_change_the_advice() {
    // The advice is from the project's flags only: what a device once
    // reported, completion and the broken mark are other claims.
    for bits in 0u8..32 {
        let plain = state(bits);
        let mut decorated = plain.clone();
        for part in LoadPart::ALL {
            decorated.device_reported.record(part, LoadState::Loaded);
        }
        decorated.broken = true;
        decorated.completion = crate::commissioning::CompletionStatus::Tested;
        assert_eq!(
            ScopeAdvice::from_flags(&decorated, address()),
            ScopeAdvice::from_flags(&plain, address()),
            "{bits:05b}"
        );
    }
}

#[test]
fn the_codes_are_stable() {
    let codes: Vec<_> = [
        AdvisedWrite::Complete,
        AdvisedWrite::Partial(PARAMETERS),
        AdvisedWrite::Partial(GROUP_ADDRESSES),
        AdvisedWrite::Partial(BOTH),
        AdvisedWrite::Nothing,
    ]
    .iter()
    .map(|write| write.code())
    .collect();
    assert_eq!(
        codes,
        [
            "complete",
            "parameters",
            "groupAddresses",
            "both",
            "nothing"
        ]
    );
    assert_eq!(
        [
            AddressNeed::Current,
            AddressNeed::Pending,
            AddressNeed::Missing
        ]
        .map(AddressNeed::code),
        ["current", "pending", "missing"]
    );
}

#[test]
fn for_device_reads_the_device_flags_and_address() {
    let mut project = crate::commissioning::pending::fixture::project();
    let device = project
        .devices
        .get_mut(crate::commissioning::pending::fixture::D1)
        .unwrap();
    device.commissioning = state(0b11011);
    device.address = None;
    let advice = ScopeAdvice::for_device(device);
    assert_eq!(advice.write, AdvisedWrite::Partial(PARAMETERS));
    assert_eq!(advice.address, AddressNeed::Missing);
}
