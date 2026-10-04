//! Recovery policy for a terminated WebKit web process (KNOWN_LIMITATIONS §133).
//!
//! The frontend owns the close decision (§132): while its
//! `tauri://close-requested` listener exists, `tauri` vetoes every close and
//! waits for the page to answer. A *terminated* web process can never answer,
//! so the window manager's close button would do nothing for good.
//!
//! Termination is the one state that is provably dead rather than busy:
//! WebKit reports it with a reason. A merely slow or hung page reports
//! nothing here and keeps its unsaved-changes guard — closing a busy page
//! without asking is exactly the silent loss §132 removed.
//!
//! The unsaved project lives in the embedded `knx-server`, not in the web
//! process, so reloading the page loses nothing: the reloaded frontend
//! reattaches to the same server state and registers a fresh close handler.
//! Reloads are bounded so a page that crashes on every load cannot spin the
//! machine. Once that budget is spent the frontend is treated as dead and an
//! explicit close request from the user is allowed through.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Why WebKit ended the web process, mirroring
/// `WebKitWebProcessTerminationReason` without tying the policy to GTK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    Crashed,
    ExceededMemoryLimit,
    /// Ended deliberately through `webkit_web_view_terminate_web_process`.
    /// Nothing in this application calls it; whoever did chose to end the
    /// page and must not have that undone behind its back.
    TerminatedByApi,
    /// A reason this WebKit binding does not name. The process is still
    /// gone, so it is handled like a crash.
    Unknown,
}

/// What the shell does about one termination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    Reload,
    /// Leave the page as it is.
    Ignore,
    /// The reload budget is spent: stop reloading and let the user's next
    /// explicit close request through.
    GiveUp,
}

/// Reloads allowed within [`RELOAD_WINDOW`] before the frontend is treated
/// as dead. Three covers a transient crash or two without letting a page
/// that dies on every load cycle indefinitely.
pub const MAX_RELOADS: usize = 3;
/// The span over which [`MAX_RELOADS`] is counted.
pub const RELOAD_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Default)]
pub struct WebProcessSupervisor {
    reloads: Vec<Instant>,
    frontend_dead: bool,
    discard_prompt_open: bool,
}

/// What a close request does once the shell, not the page, has to decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    /// A living frontend answers with its own unsaved-changes guard (§132).
    LeaveToFrontend,
    /// The frontend is dead and nothing is unsaved: close.
    Close,
    /// The frontend is dead and the project has unsaved edits: ask natively
    /// before discarding them.
    ConfirmDiscard,
    /// That question is already on screen; do not stack another one.
    AlreadyAsking,
}

impl WebProcessSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decide what to do about a termination observed at `now`.
    pub fn on_terminated(&mut self, reason: Termination, now: Instant) -> Recovery {
        if reason == Termination::TerminatedByApi {
            return Recovery::Ignore;
        }
        if self.frontend_dead {
            return Recovery::GiveUp;
        }
        self.reloads
            .retain(|&at| now.saturating_duration_since(at) < RELOAD_WINDOW);
        if self.reloads.len() >= MAX_RELOADS {
            self.frontend_dead = true;
            return Recovery::GiveUp;
        }
        self.reloads.push(now);
        Recovery::Reload
    }

    /// Decide a close request. `unsaved` is the server's own modified state;
    /// it is consulted only once the frontend can no longer be asked.
    pub fn on_close_requested(&mut self, unsaved: bool) -> CloseDecision {
        if !self.frontend_dead {
            return CloseDecision::LeaveToFrontend;
        }
        if !unsaved {
            return CloseDecision::Close;
        }
        if self.discard_prompt_open {
            return CloseDecision::AlreadyAsking;
        }
        self.discard_prompt_open = true;
        CloseDecision::ConfirmDiscard
    }

    /// The native discard question was answered either way.
    pub fn discard_prompt_answered(&mut self) {
        self.discard_prompt_open = false;
    }
}

/// The supervisor shared between the GTK signal handler and the window-event
/// handler. Both run on the main thread, but `tauri`'s handlers must be
/// `Send + Sync`.
pub type SharedSupervisor = Arc<Mutex<WebProcessSupervisor>>;

/// Map WebKit's reason onto the GTK-free policy.
#[cfg(target_os = "linux")]
pub fn termination_from(reason: webkit2gtk::WebProcessTerminationReason) -> Termination {
    use webkit2gtk::WebProcessTerminationReason as Reason;
    match reason {
        Reason::Crashed => Termination::Crashed,
        Reason::ExceededMemoryLimit => Termination::ExceededMemoryLimit,
        Reason::TerminatedByApi => Termination::TerminatedByApi,
        _ => Termination::Unknown,
    }
}

/// Observe the window's web process and reload the page when it dies.
///
/// The handler runs on the GTK main thread. A poisoned lock is recovered
/// rather than propagated: the supervisor holds only counters, and giving
/// up on recovery because an earlier handler panicked would recreate the
/// unclosable window this exists to prevent.
#[cfg(target_os = "linux")]
pub fn watch<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
    supervisor: SharedSupervisor,
) -> tauri::Result<()> {
    window.with_webview(move |platform| {
        use webkit2gtk::WebViewExt;
        platform
            .inner()
            .connect_web_process_terminated(move |view, reason| {
                let termination = termination_from(reason);
                let recovery = supervisor
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .on_terminated(termination, Instant::now());
                eprintln!(
                    "knx-desktop: web process terminated ({termination:?}); recovery: {recovery:?}"
                );
                if recovery == Recovery::Reload {
                    view.reload();
                }
            });
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn every_webkit_termination_reason_maps_to_its_own_policy_reason() {
        use webkit2gtk::WebProcessTerminationReason as Reason;
        assert_eq!(termination_from(Reason::Crashed), Termination::Crashed);
        assert_eq!(
            termination_from(Reason::ExceededMemoryLimit),
            Termination::ExceededMemoryLimit
        );
        assert_eq!(
            termination_from(Reason::TerminatedByApi),
            Termination::TerminatedByApi
        );
        assert_eq!(
            termination_from(Reason::__Unknown(99)),
            Termination::Unknown
        );
    }

    #[test]
    fn a_crash_reloads_the_page_and_keeps_the_frontend_in_charge_of_closing() {
        let mut supervisor = WebProcessSupervisor::new();
        let now = Instant::now();
        assert_eq!(
            supervisor.on_terminated(Termination::Crashed, now),
            Recovery::Reload
        );
        assert_eq!(
            supervisor.on_close_requested(true),
            CloseDecision::LeaveToFrontend
        );
    }

    #[test]
    fn running_out_of_memory_and_unknown_reasons_are_handled_like_a_crash() {
        for reason in [Termination::ExceededMemoryLimit, Termination::Unknown] {
            let mut supervisor = WebProcessSupervisor::new();
            assert_eq!(
                supervisor.on_terminated(reason, Instant::now()),
                Recovery::Reload,
                "{reason:?}"
            );
        }
    }

    #[test]
    fn a_deliberate_termination_is_neither_undone_nor_treated_as_a_dead_frontend() {
        let mut supervisor = WebProcessSupervisor::new();
        let now = Instant::now();
        for _ in 0..(MAX_RELOADS + 2) {
            assert_eq!(
                supervisor.on_terminated(Termination::TerminatedByApi, now),
                Recovery::Ignore
            );
        }
        assert_eq!(
            supervisor.on_close_requested(true),
            CloseDecision::LeaveToFrontend
        );
        // It also does not consume the crash budget.
        assert_eq!(
            supervisor.on_terminated(Termination::Crashed, now),
            Recovery::Reload
        );
    }

    #[test]
    fn a_crash_loop_spends_the_budget_then_lets_an_explicit_close_through() {
        let mut supervisor = WebProcessSupervisor::new();
        let start = Instant::now();
        for i in 0..MAX_RELOADS {
            let at = start + Duration::from_secs(i as u64);
            assert_eq!(
                supervisor.on_terminated(Termination::Crashed, at),
                Recovery::Reload
            );
            assert_eq!(
                supervisor.on_close_requested(true),
                CloseDecision::LeaveToFrontend,
                "after reload {i}"
            );
        }
        let at = start + Duration::from_secs(MAX_RELOADS as u64);
        assert_eq!(
            supervisor.on_terminated(Termination::Crashed, at),
            Recovery::GiveUp
        );
        assert_eq!(supervisor.on_close_requested(false), CloseDecision::Close);
        // Once given up it stays given up: no further reload cycle starts.
        let later = at + RELOAD_WINDOW * 10;
        assert_eq!(
            supervisor.on_terminated(Termination::Crashed, later),
            Recovery::GiveUp
        );
        assert_eq!(supervisor.on_close_requested(false), CloseDecision::Close);
    }

    #[test]
    fn crashes_spread_out_beyond_the_window_never_exhaust_the_budget() {
        let mut supervisor = WebProcessSupervisor::new();
        let start = Instant::now();
        for i in 0..(MAX_RELOADS * 4) {
            let at = start + RELOAD_WINDOW * (i as u32 + 1);
            assert_eq!(
                supervisor.on_terminated(Termination::Crashed, at),
                Recovery::Reload,
                "crash {i}"
            );
        }
        assert_eq!(
            supervisor.on_close_requested(true),
            CloseDecision::LeaveToFrontend
        );
    }

    #[test]
    fn a_crash_exactly_at_the_window_edge_still_counts_against_the_budget() {
        let mut supervisor = WebProcessSupervisor::new();
        let start = Instant::now();
        for _ in 0..MAX_RELOADS {
            assert_eq!(
                supervisor.on_terminated(Termination::Crashed, start),
                Recovery::Reload
            );
        }
        // Strictly inside the window: still the same burst.
        let edge = start + RELOAD_WINDOW - Duration::from_millis(1);
        assert_eq!(
            supervisor.on_terminated(Termination::Crashed, edge),
            Recovery::GiveUp
        );
    }

    #[test]
    fn a_dead_frontend_with_unsaved_edits_asks_once_before_discarding_them() {
        let mut supervisor = WebProcessSupervisor::new();
        let now = Instant::now();
        for _ in 0..=MAX_RELOADS {
            supervisor.on_terminated(Termination::Crashed, now);
        }
        assert_eq!(
            supervisor.on_close_requested(true),
            CloseDecision::ConfirmDiscard
        );
        // Pressing × again while the question is open stacks nothing.
        assert_eq!(
            supervisor.on_close_requested(true),
            CloseDecision::AlreadyAsking
        );
        // "Keep open" answers it; the next × asks again rather than closing.
        supervisor.discard_prompt_answered();
        assert_eq!(
            supervisor.on_close_requested(true),
            CloseDecision::ConfirmDiscard
        );
        // Without unsaved edits a dead frontend simply closes.
        supervisor.discard_prompt_answered();
        assert_eq!(supervisor.on_close_requested(false), CloseDecision::Close);
    }
}
