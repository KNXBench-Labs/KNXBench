//! Terminal write classification and human-readable programming log tests.

use knx_app::programming_state::{AddressEnd, DownloadEnd, Settlement};
use knx_core::commissioning::pending::ProgrammingParts;

use super::{address_end, download_end, sentence};
use crate::address_programming::{AddressProgrammingStatus, AddressWritten};
use crate::device_download::{DownloadStatus, Restart, Written};

fn finished(written: Written, restart: Restart) -> DownloadStatus {
    DownloadStatus::Finished {
        written,
        restart,
        restart_note: None,
    }
}

fn failed(written: Written) -> DownloadStatus {
    DownloadStatus::Failed {
        written,
        stopped_in_step: Some(3),
        error: "lost".into(),
        hint: None,
    }
}

#[test]
fn a_download_ends_as_the_device_saw_it() {
    assert_eq!(
        download_end(&DownloadStatus::Running, ProgrammingParts::ALL),
        None
    );
    assert_eq!(
        download_end(
            &finished(Written::Yes, Restart::Acknowledged),
            ProgrammingParts::ALL
        ),
        Some(DownloadEnd::Completed)
    );
    assert_eq!(
        download_end(
            &finished(Written::Yes, Restart::NotInPlan),
            ProgrammingParts::ALL
        ),
        Some(DownloadEnd::Completed)
    );
    assert_eq!(
        download_end(
            &finished(Written::Yes, Restart::Unconfirmed),
            ProgrammingParts::ALL
        ),
        Some(DownloadEnd::Completed)
    );
    assert_eq!(
        download_end(&failed(Written::Partially), ProgrammingParts::ALL),
        Some(DownloadEnd::Interrupted(ProgrammingParts::ALL))
    );
    assert_eq!(
        download_end(&failed(Written::Yes), ProgrammingParts::ALL),
        Some(DownloadEnd::Interrupted(ProgrammingParts::ALL))
    );
    assert_eq!(
        download_end(&failed(Written::No), ProgrammingParts::ALL),
        Some(DownloadEnd::NothingWritten)
    );
}

fn address(written: AddressWritten, done: bool) -> AddressProgrammingStatus {
    if done {
        AddressProgrammingStatus::Finished {
            written,
            previous_address: "15.15.255".into(),
            was_free: true,
            restart_confirmed: false,
        }
    } else {
        AddressProgrammingStatus::Failed {
            written,
            step: Some(4),
            error: "silent".into(),
        }
    }
}

#[test]
fn an_address_programming_ends_as_the_device_saw_it() {
    assert_eq!(
        address_end(&AddressProgrammingStatus::Waiting {
            rounds: 1,
            in_programming_mode: vec![]
        }),
        None
    );
    assert_eq!(
        address_end(&address(AddressWritten::Yes, true)),
        Some(AddressEnd::Programmed)
    );
    assert_eq!(
        address_end(&address(AddressWritten::NoNeed, true)),
        Some(AddressEnd::Programmed)
    );
    assert_eq!(
        address_end(&address(AddressWritten::Unconfirmed, true)),
        Some(AddressEnd::Uncertain)
    );
    assert_eq!(
        address_end(&address(AddressWritten::Unconfirmed, false)),
        Some(AddressEnd::Uncertain)
    );
    assert_eq!(
        address_end(&address(AddressWritten::No, false)),
        Some(AddressEnd::NothingWritten)
    );
    assert_eq!(
        address_end(&AddressProgrammingStatus::Stopped { rounds: 2 }),
        Some(AddressEnd::NothingWritten)
    );
}

#[test]
fn the_log_line_names_both_directions() {
    let settlement = Settlement {
        programmed: ProgrammingParts {
            address: true,
            application: true,
            ..ProgrammingParts::NONE
        },
        pending: ProgrammingParts {
            parameters: true,
            group_addresses: true,
            ..ProgrammingParts::NONE
        },
        touched: ProgrammingParts::ALL,
    };
    assert_eq!(
        sentence("Download to “A” (1.1.5) finished", &settlement),
        "Download to “A” (1.1.5) finished: address and application marked programmed; \
         parameters and group addresses marked not programmed"
    );
    assert_eq!(
        sentence("X", &Settlement::default()),
        "X: programming state left as it was"
    );
}
