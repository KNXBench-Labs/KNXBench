//! Runs one individual-address programming in the background: the button wait, then MP §2.3.
//!
//! Programming the individual address is not a download (docs/GLOSSARY.md).
//! The route (`address_programming_routes.rs`) checks the phrase and that no
//! other bus session runs, then hands this module an
//! [`AddressProgrammingAuthorisation`]. The loop is
//! `knx_net::commissioning::programming_button_wait`, the one the CLI uses.
//!
//! Stopping is allowed only while waiting for the button (ADR-0046 §3):
//! nothing is written until exactly one device is found, and from then on
//! MP §2.3 runs to its end. A stop that arrives later is recorded as
//! "too late" and changes nothing.

use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::IndividualAddress;
use knx_net::commissioning::individual_address_write::{
    AddressRestart, IndividualAddressWriteError, Occupancy,
};
use knx_net::commissioning::programming_button_wait::{
    program_individual_address, AddressProgrammingAuthorisation, ButtonEvent,
    ButtonProgrammingError, ButtonWait,
};
use serde::Serialize;
use tokio::task::JoinHandle;

use crate::bus::BusTunnel;
use crate::device_download::TunnelTransport;

/// Whether the device now answers at the new address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AddressWritten {
    /// Written, then read back and restarted at the new address.
    Yes,
    /// The device already had the address; read back and restarted.
    NoNeed,
    /// Nothing was written.
    No,
    /// Written, but the device did not answer at the new address in step
    /// 4 (KNOWN_LIMITATIONS §7 item 2). Check it before anything else.
    Unconfirmed,
}

/// Where a programming stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AddressProgrammingStatus {
    /// Asking round by round who is in programming mode.
    Waiting {
        rounds: u32,
        /// The last round's answer, by current address.
        in_programming_mode: Vec<String>,
    },
    /// Exactly one device answered; MP §2.3 runs and cannot be stopped.
    Programming { previous_address: String },
    Finished {
        written: AddressWritten,
        previous_address: String,
        /// Whether the new address was free before (`false`: this device
        /// already held it).
        was_free: bool,
        /// Whether the device acknowledged step 4's Basic Restart. `false`
        /// does not weaken `written`: the device answered at the new address
        /// before the restart went out. Some devices never acknowledge a
        /// Basic Restart (MP §3.7.1.1.3, p. 80; RESEARCH §19).
        restart_confirmed: bool,
    },
    /// Stopped by the user while waiting. Nothing was written.
    Stopped { rounds: u32 },
    Failed {
        written: AddressWritten,
        /// The MP §2.3 step (1-4) that failed, when the procedure had
        /// started.
        step: Option<u8>,
        error: String,
    },
}

impl AddressProgrammingStatus {
    fn is_active(&self) -> bool {
        matches!(
            self,
            AddressProgrammingStatus::Waiting { .. } | AddressProgrammingStatus::Programming { .. }
        )
    }
}

/// One line of the log: only rounds whose answer changed, and the find.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AddressProgrammingEvent {
    Round {
        number: u32,
        in_programming_mode: Vec<String>,
    },
    Found {
        current_address: String,
    },
}

#[derive(Debug)]
struct Shared {
    status: Mutex<AddressProgrammingStatus>,
    events: Mutex<Vec<AddressProgrammingEvent>>,
    stop_requested: AtomicBool,
}

/// One programming, running or finished. Kept after it ends so its result
/// stays readable until the next one replaces it.
pub struct AddressProgrammingSession {
    id: u64,
    new_address: IndividualAddress,
    wait_seconds: u64,
    shared: Arc<Shared>,
    task: Option<JoinHandle<()>>,
}

/// What a stop request achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome {
    /// Still waiting: the wait ends after the current round.
    WillStop,
    /// MP §2.3 has started, or everything is over: nothing to stop.
    TooLate,
}

impl AddressProgrammingSession {
    /// Starts the wait on `tunnel`. The caller has already checked the
    /// phrase and that no other bus session runs.
    pub fn start(
        id: u64,
        tunnel: Box<dyn BusTunnel>,
        new_address: IndividualAddress,
        authorisation: AddressProgrammingAuthorisation,
        timing: knx_net::SessionTiming,
        wait: ButtonWait,
    ) -> Self {
        let shared = Arc::new(Shared {
            status: Mutex::new(AddressProgrammingStatus::Waiting {
                rounds: 0,
                in_programming_mode: Vec::new(),
            }),
            events: Mutex::new(Vec::new()),
            stop_requested: AtomicBool::new(false),
        });
        let task = tokio::spawn(run(
            tunnel,
            new_address,
            authorisation,
            timing,
            wait,
            Arc::clone(&shared),
        ));
        Self {
            id,
            new_address,
            wait_seconds: wait.give_up_after.as_secs(),
            shared,
            task: Some(task),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn new_address(&self) -> IndividualAddress {
        self.new_address
    }

    pub fn wait_seconds(&self) -> u64 {
        self.wait_seconds
    }

    pub fn status(&self) -> AddressProgrammingStatus {
        self.shared
            .status
            .lock()
            .expect("programming status poisoned")
            .clone()
    }

    pub fn is_active(&self) -> bool {
        self.status().is_active()
    }

    /// Asks the wait to end. Checked under the status lock, so a stop can
    /// never race the switch to `Programming`: either the observer sees the
    /// flag before it reports the find, or the stop sees `Programming`.
    pub fn request_stop(&self) -> StopOutcome {
        let status = self
            .shared
            .status
            .lock()
            .expect("programming status poisoned");
        if matches!(*status, AddressProgrammingStatus::Waiting { .. }) {
            self.shared.stop_requested.store(true, Ordering::SeqCst);
            StopOutcome::WillStop
        } else {
            StopOutcome::TooLate
        }
    }

    /// Status first, then events (the line scan's rule).
    pub fn snapshot_since(
        &self,
        since: usize,
    ) -> (
        AddressProgrammingStatus,
        usize,
        Vec<AddressProgrammingEvent>,
    ) {
        let status = self.status();
        let events = self
            .shared
            .events
            .lock()
            .expect("programming events poisoned");
        let start = since.min(events.len());
        (status, events.len(), events[start..].to_vec())
    }

    /// Waits for the run to end. For tests; the routes poll.
    pub async fn join(&mut self) {
        if let Some(task) = self.task.take() {
            if task.await.is_err() {
                *self
                    .shared
                    .status
                    .lock()
                    .expect("programming status poisoned") = AddressProgrammingStatus::Failed {
                    // Unknown: the task may have died after the write.
                    written: AddressWritten::Unconfirmed,
                    step: None,
                    error: "the programming task stopped unexpectedly".to_string(),
                };
            }
        }
    }
}

fn addresses(list: &[IndividualAddress]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

/// The observer: records, and answers the stop flag. Runs under the
/// status lock so [`AddressProgrammingSession::request_stop`] is atomic
/// with the switch to `Programming`.
fn observe(
    shared: &Shared,
    last: &mut Option<Vec<IndividualAddress>>,
    event: &ButtonEvent,
) -> ControlFlow<()> {
    let mut status = shared.status.lock().expect("programming status poisoned");
    match event {
        ButtonEvent::Round {
            number,
            in_programming_mode,
        } => {
            if last.as_ref() != Some(in_programming_mode) {
                shared
                    .events
                    .lock()
                    .expect("programming events poisoned")
                    .push(AddressProgrammingEvent::Round {
                        number: *number,
                        in_programming_mode: addresses(in_programming_mode),
                    });
                *last = Some(in_programming_mode.clone());
            }
            *status = AddressProgrammingStatus::Waiting {
                rounds: *number,
                in_programming_mode: addresses(in_programming_mode),
            };
            if shared.stop_requested.load(Ordering::SeqCst) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
        ButtonEvent::Found { current_address } => {
            shared
                .events
                .lock()
                .expect("programming events poisoned")
                .push(AddressProgrammingEvent::Found {
                    current_address: current_address.to_string(),
                });
            *status = AddressProgrammingStatus::Programming {
                previous_address: current_address.to_string(),
            };
            ControlFlow::Continue(())
        }
    }
}

async fn run(
    tunnel: Box<dyn BusTunnel>,
    new_address: IndividualAddress,
    authorisation: AddressProgrammingAuthorisation,
    timing: knx_net::SessionTiming,
    wait: ButtonWait,
    shared: Arc<Shared>,
) {
    let status = {
        let transport = TunnelTransport(tunnel.as_ref());
        let mut last = None;
        let result = program_individual_address(
            &transport,
            AuthorisationPlan::Skip,
            timing,
            new_address,
            authorisation,
            wait,
            |event| observe(&shared, &mut last, event),
        )
        .await;
        match result {
            Ok(report) => AddressProgrammingStatus::Finished {
                written: if report.procedure.wrote {
                    AddressWritten::Yes
                } else {
                    AddressWritten::NoNeed
                },
                previous_address: report.previous_address.to_string(),
                was_free: report.procedure.occupancy == Occupancy::NotOccupied,
                restart_confirmed: report.procedure.restart == AddressRestart::Acknowledged,
            },
            Err(ButtonProgrammingError::Stopped { rounds }) => {
                AddressProgrammingStatus::Stopped { rounds }
            }
            Err(err) => failed(&err),
        }
    };
    *shared.status.lock().expect("programming status poisoned") = status;
    let _ = tunnel.disconnect().await;
}

/// KNOWN_LIMITATIONS §7 item 2: a failure after the write must not read as
/// "nothing happened".
fn failed(err: &ButtonProgrammingError) -> AddressProgrammingStatus {
    let (written, step) = match err {
        ButtonProgrammingError::Procedure(IndividualAddressWriteError::Session {
            step,
            report,
            ..
        }) => (
            if report.wrote {
                AddressWritten::Unconfirmed
            } else {
                AddressWritten::No
            },
            Some(*step),
        ),
        _ => (AddressWritten::No, None),
    };
    AddressProgrammingStatus::Failed {
        written,
        step,
        error: err.to_string(),
    }
}
