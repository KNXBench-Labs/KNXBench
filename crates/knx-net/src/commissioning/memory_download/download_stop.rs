//! The one-operation stop token of a regular download: a stop lands only before the first change.

use std::sync::atomic::{AtomicU8, Ordering};

/// What a [`DownloadStop::request`] meant at the moment it was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopRequest {
    /// The request is held. The run it belongs to sends no memory write,
    /// user-memory write, load record, property write or restart. That is
    /// all it promises: connect, key, read and disconnect frames may still
    /// go out, and an error found first is still the run's error.
    WillStop,
    /// The first change was already committed; the run goes on unchanged.
    TooLate,
    /// The run is over (or was over before the request); nothing to stop.
    Ended,
}

/// The token's life. Each state is reached once; the numbers only index
/// the atomic and never leave this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum State {
    /// Not claimed, nothing requested.
    Fresh = 0,
    /// Not claimed yet, a stop is already requested.
    FreshStopRequested = 1,
    /// Claimed by its one run; the window to the first change is open.
    Running = 2,
    /// Claimed, and a stop was accepted before the first change.
    RunningStopRequested = 3,
    /// The first change was committed; no stop can land any more.
    Committed = 4,
    /// The run ended without committing a change.
    Ended = 5,
    /// The run ended after committing the first change.
    EndedAfterCommit = 6,
}

impl State {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Fresh,
            1 => Self::FreshStopRequested,
            2 => Self::Running,
            3 => Self::RunningStopRequested,
            4 => Self::Committed,
            5 => Self::Ended,
            _ => Self::EndedAfterCommit,
        }
    }
}

/// Stop control for exactly one regular download.
///
/// A token is claimed by one run, once; it cannot be reset, retargeted or
/// reused, whatever that run did. Requests and the run's single commit
/// serialise on one atomic, so an accepted [`StopRequest::WillStop`] always
/// beats the commit and a commit that won always answers
/// [`StopRequest::TooLate`]. Nothing here waits or drops a future.
#[derive(Debug)]
pub struct DownloadStop {
    state: AtomicU8,
}

impl Default for DownloadStop {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadStop {
    /// A fresh token: unclaimed, nothing requested.
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(State::Fresh as u8),
        }
    }

    /// Synthetic stand-in for cross-crate tests, not evidence of an actual
    /// run, send or device effect. Nobody can claim or settle this token;
    /// it answers `TooLate` indefinitely. Production owners use `new`.
    #[doc(hidden)]
    pub fn already_committed() -> Self {
        Self {
            state: AtomicU8::new(State::Committed as u8),
        }
    }

    /// Asks the run to stop before its first change. Idempotent while
    /// pending; `TooLate` once the first change is committed; `Ended` once
    /// a claimed executor is over, even one that stopped. A token refused
    /// before ownership stays unclaimed; `WillStop` does not prove liveness.
    pub fn request(&self) -> StopRequest {
        let previous = self
            .state
            .fetch_update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |current| match State::from_u8(current) {
                    State::Fresh | State::FreshStopRequested => {
                        Some(State::FreshStopRequested as u8)
                    }
                    State::Running | State::RunningStopRequested => {
                        Some(State::RunningStopRequested as u8)
                    }
                    State::Committed | State::Ended | State::EndedAfterCommit => None,
                },
            );
        match previous.map_err(State::from_u8) {
            Ok(_) => StopRequest::WillStop,
            Err(State::Committed) => StopRequest::TooLate,
            Err(_) => StopRequest::Ended,
        }
    }

    /// Whether an accepted stop is waiting for the run to act on it: the
    /// request was made and the first change was not committed.
    pub fn stop_pending(&self) -> bool {
        matches!(
            State::from_u8(self.state.load(Ordering::SeqCst)),
            State::FreshStopRequested | State::RunningStopRequested
        )
    }

    /// Whether the run committed its first change (it may have ended since).
    /// This is token state, not proof of a sent frame or device effect;
    /// synthetic `already_committed` tokens do not establish a real run.
    pub fn mutation_committed(&self) -> bool {
        matches!(
            State::from_u8(self.state.load(Ordering::SeqCst)),
            State::Committed | State::EndedAfterCommit
        )
    }

    /// Takes the token for one run. `None` when it was claimed before,
    /// whether that run is still going or long over.
    pub(super) fn claim(&self) -> Option<Claim<'_>> {
        self.state
            .fetch_update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |current| match State::from_u8(current) {
                    State::Fresh => Some(State::Running as u8),
                    State::FreshStopRequested => Some(State::RunningStopRequested as u8),
                    _ => None,
                },
            )
            .ok()
            .map(|_| Claim { token: self })
    }
}

/// The running side of a claimed [`DownloadStop`]. Dropping it, on return
/// or unwind, ends the token for good.
#[derive(Debug)]
pub(super) struct Claim<'a> {
    token: &'a DownloadStop,
}

impl Claim<'_> {
    /// Whether a stop was accepted and the first change has not started.
    pub(super) fn stop_requested(&self) -> bool {
        State::from_u8(self.token.state.load(Ordering::SeqCst)) == State::RunningStopRequested
    }

    /// The commitment point, taken synchronously before the first changing
    /// step starts. `true`: the change may start and no stop can land now.
    /// `false`: a stop was accepted first and the change must not start.
    pub(super) fn commit(&self) -> bool {
        self.token
            .state
            .fetch_update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |current| match State::from_u8(current) {
                    State::Running | State::Committed => Some(State::Committed as u8),
                    _ => None,
                },
            )
            .is_ok()
    }
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        let _ = self
            .token
            .state
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some(match State::from_u8(current) {
                    State::Committed | State::EndedAfterCommit => State::EndedAfterCommit as u8,
                    _ => State::Ended as u8,
                })
            });
    }
}
