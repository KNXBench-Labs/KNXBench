//! Atomic arbitration between accepted cancellation and one live commit claim.
//!
//! This primitive performs no database work and does not prove rollback,
//! settled cancellation or successful COMMIT. It is not an installer control.

use std::sync::atomic::{AtomicU8, Ordering};

/// A snapshot of one publication arbitration, not a database outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PublicationState {
    /// Neither cancellation nor the live commit claim has won yet.
    Open,
    /// Cancellation won; the caller must still unwind and settle its work.
    CancelAccepted,
    /// One worker claimed publication; its actual COMMIT can still fail.
    CommitClaimed,
}

/// A cooperative stop channel, kept separate from ordinary work errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// The operation's persistent gate accepted cancellation.
    CancelAccepted,
}

/// The exact outcome of a cancellation request against the persistent gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    /// This request won the transition from Open to CancelAccepted.
    Accepted,
    /// An earlier cancellation request already won; the gate stays latched.
    AlreadyAccepted,
    /// A worker already claimed publication; its outcome remains unknown here.
    TooLate,
}

/// One non-resettable arbitration between cancellation and a live commit claim.
///
/// Only `Open -> CancelAccepted` and `Open -> CommitClaimed` can occur. Both
/// transitions use a strong compare-exchange on the same atomic; only one can
/// win. There is no reset or gate clone: share the same gate by borrow or `Arc`.
///
/// All state accesses use conservative `SeqCst` ordering for one easily audited
/// total order. Synchronization of work/results remains the caller's concern.
/// No lock, callback or database operation runs inside this primitive.
#[derive(Debug)]
pub struct PublicationGate {
    state: AtomicU8,
}

impl Default for PublicationGate {
    fn default() -> Self {
        Self::new()
    }
}

impl PublicationGate {
    /// Creates a fresh Open gate for a distinct operation.
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(PublicationState::Open as u8),
        }
    }

    /// Returns an atomic snapshot; observing Open does not reserve publication.
    pub fn state(&self) -> PublicationState {
        match self.state.load(Ordering::SeqCst) {
            value if value == PublicationState::Open as u8 => PublicationState::Open,
            value if value == PublicationState::CancelAccepted as u8 => {
                PublicationState::CancelAccepted
            }
            value if value == PublicationState::CommitClaimed as u8 => {
                PublicationState::CommitClaimed
            }
            _ => unreachable!("only publication states are stored"),
        }
    }

    /// Attempts to latch cancellation without waiting for the worker.
    ///
    /// Acceptance excludes a future commit claim, but does not prove that work
    /// has stopped, rolled back or finished cleanup. A late request never stops
    /// a commit-claimed operation or changes its state.
    #[must_use]
    pub fn try_cancel(&self) -> CancelOutcome {
        match self.state.compare_exchange(
            PublicationState::Open as u8,
            PublicationState::CancelAccepted as u8,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => CancelOutcome::Accepted,
            Err(value) if value == PublicationState::CancelAccepted as u8 => {
                CancelOutcome::AlreadyAccepted
            }
            Err(value) if value == PublicationState::CommitClaimed as u8 => CancelOutcome::TooLate,
            Err(_) => unreachable!("strong compare-exchange failed on a terminal state"),
        }
    }

    /// Returns true only for the unique Open-to-CommitClaimed winner.
    ///
    /// The caller must finish all fallible precommit work before claiming and
    /// issue its one live COMMIT immediately afterward. Scratch commits and
    /// savepoint releases must not consume this gate. False never authorizes a
    /// COMMIT, even for a second request after a successful claim. True does not
    /// prove that the eventual database COMMIT succeeds.
    #[must_use]
    pub fn try_claim_commit(&self) -> bool {
        self.state
            .compare_exchange(
                PublicationState::Open as u8,
                PublicationState::CommitClaimed as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    /// Reports typed Stop only after accepted cancellation, never after a claim.
    ///
    /// An Open observation may race with later cancellation; callers still need
    /// subsequent checkpoints and the final atomic claim. This is not settlement
    /// evidence and must not abort rollback or other required cleanup work.
    pub fn checkpoint(&self) -> Result<(), Stop> {
        match self.state() {
            PublicationState::CancelAccepted => Err(Stop::CancelAccepted),
            PublicationState::Open | PublicationState::CommitClaimed => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CancelOutcome, PublicationGate, PublicationState, Stop};
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn joint_start_cancel_and_two_commit_claimants_have_exactly_one_winner() {
        let gate = Arc::new(PublicationGate::new());
        let start = Barrier::new(4);

        let (cancel, claims) = thread::scope(|scope| {
            let start = &start;
            let cancel_gate = Arc::clone(&gate);
            let cancel = scope.spawn(move || {
                start.wait();
                cancel_gate.try_cancel()
            });
            let first_gate = Arc::clone(&gate);
            let first = scope.spawn(move || {
                start.wait();
                first_gate.try_claim_commit()
            });
            let second_gate = Arc::clone(&gate);
            let second = scope.spawn(move || {
                start.wait();
                second_gate.try_claim_commit()
            });
            start.wait();
            let cancel = cancel.join();
            let first = first.join();
            let second = second.join();
            (cancel.unwrap(), [first.unwrap(), second.unwrap()])
        });

        match cancel {
            CancelOutcome::Accepted => {
                assert_eq!(claims, [false, false]);
                assert_eq!(gate.state(), PublicationState::CancelAccepted);
                assert_eq!(gate.checkpoint(), Err(Stop::CancelAccepted));
                assert_eq!(gate.try_cancel(), CancelOutcome::AlreadyAccepted);
            }
            CancelOutcome::TooLate => {
                assert!(matches!(claims, [true, false] | [false, true]));
                assert_eq!(gate.state(), PublicationState::CommitClaimed);
                assert_eq!(gate.checkpoint(), Ok(()));
                assert_eq!(gate.try_cancel(), CancelOutcome::TooLate);
            }
            CancelOutcome::AlreadyAccepted => panic!("there was only one cancellation request"),
        }
        assert!(!gate.try_claim_commit());
    }

    #[test]
    fn ordered_commit_claim_makes_cancellation_too_late_without_stopping() {
        let gate = PublicationGate::new();
        let claimed = Barrier::new(2);

        let (commit, cancel) = thread::scope(|scope| {
            let commit = scope.spawn(|| {
                let first = gate.try_claim_commit();
                let second = gate.try_claim_commit();
                let state = gate.state();
                claimed.wait();
                (first, second, state)
            });
            let cancel = scope.spawn(|| {
                claimed.wait();
                let first = gate.try_cancel();
                let second = gate.try_cancel();
                (first, second, gate.state(), gate.checkpoint())
            });
            let commit = commit.join();
            let cancel = cancel.join();
            (commit.unwrap(), cancel.unwrap())
        });

        assert_eq!(commit, (true, false, PublicationState::CommitClaimed));
        assert_eq!(
            cancel,
            (
                CancelOutcome::TooLate,
                CancelOutcome::TooLate,
                PublicationState::CommitClaimed,
                Ok(())
            )
        );
        assert!(!gate.try_claim_commit());
        assert_eq!(gate.state(), PublicationState::CommitClaimed);
        assert_eq!(gate.checkpoint(), Ok(()));
    }

    #[test]
    fn ordered_cancellation_wins_before_commit_claim() {
        let gate = PublicationGate::new();
        let cancelled = Barrier::new(2);

        let (cancel, commit) = thread::scope(|scope| {
            let cancel = scope.spawn(|| {
                let outcome = gate.try_cancel();
                let state = gate.state();
                cancelled.wait();
                (outcome, state)
            });
            let commit = scope.spawn(|| {
                cancelled.wait();
                let first = gate.try_claim_commit();
                let second = gate.try_claim_commit();
                (first, second, gate.state(), gate.checkpoint())
            });
            let cancel = cancel.join();
            let commit = commit.join();
            (cancel.unwrap(), commit.unwrap())
        });

        assert_eq!(
            cancel,
            (CancelOutcome::Accepted, PublicationState::CancelAccepted)
        );
        assert_eq!(
            commit,
            (
                false,
                false,
                PublicationState::CancelAccepted,
                Err(Stop::CancelAccepted)
            )
        );
        assert_eq!(gate.try_cancel(), CancelOutcome::AlreadyAccepted);
        assert_eq!(gate.state(), PublicationState::CancelAccepted);
    }

    #[test]
    fn commit_claim_is_unique_and_late_cancellation_is_too_late() {
        let gate = PublicationGate::new();

        assert!(gate.try_claim_commit());
        assert_eq!(gate.state(), PublicationState::CommitClaimed);
        assert!(!gate.try_claim_commit());
        assert_eq!(gate.try_cancel(), CancelOutcome::TooLate);
        assert_eq!(gate.try_cancel(), CancelOutcome::TooLate);
        assert_eq!(gate.state(), PublicationState::CommitClaimed);
        assert!(!gate.try_claim_commit());
        assert_eq!(gate.checkpoint(), Ok(()));
    }

    #[test]
    fn accepted_cancellation_blocks_every_commit_claim() {
        let gate = PublicationGate::new();
        assert_eq!(gate.try_cancel(), CancelOutcome::Accepted);

        assert!(!gate.try_claim_commit());
        assert!(!gate.try_claim_commit());
        assert_eq!(gate.state(), PublicationState::CancelAccepted);
        assert_eq!(gate.checkpoint(), Err(Stop::CancelAccepted));
        assert_eq!(gate.try_cancel(), CancelOutcome::AlreadyAccepted);
    }

    #[test]
    fn accepted_cancellation_remains_a_typed_stop_at_every_checkpoint() {
        let gate = PublicationGate::new();
        assert_eq!(gate.try_cancel(), CancelOutcome::Accepted);

        assert_eq!(gate.checkpoint(), Err(Stop::CancelAccepted));
        assert_eq!(gate.checkpoint(), Err(Stop::CancelAccepted));
        assert_eq!(gate.try_cancel(), CancelOutcome::AlreadyAccepted);
        assert_eq!(gate.checkpoint(), Err(Stop::CancelAccepted));
        assert_eq!(gate.state(), PublicationState::CancelAccepted);
    }

    #[test]
    fn cancellation_is_persistent_and_repeated_requests_are_distinct() {
        let gate = PublicationGate::new();

        assert_eq!(gate.try_cancel(), CancelOutcome::Accepted);
        assert_eq!(gate.state(), PublicationState::CancelAccepted);
        assert_eq!(gate.try_cancel(), CancelOutcome::AlreadyAccepted);
        assert_eq!(gate.state(), PublicationState::CancelAccepted);
    }

    #[test]
    fn default_gate_starts_open_and_not_stopped() {
        let gate = PublicationGate::default();

        assert_eq!(gate.state(), PublicationState::Open);
        assert_eq!(gate.checkpoint(), Ok(()));
    }

    #[test]
    fn starts_open_and_not_stopped() {
        let gate = PublicationGate::new();

        assert_eq!(gate.state(), PublicationState::Open);
        assert_eq!(gate.checkpoint(), Ok(()));
    }
}
