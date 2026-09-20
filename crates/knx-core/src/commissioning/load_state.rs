//! The Load State Machine of `PID_LOAD_STATE_CONTROL`: states, events and the transition table.
//!
//! Read and write use *different* encodings of the same property, so
//! [`LoadState`] and [`LoadEvent`] are separate enums rather than one enum
//! with a direction flag (design spec §5.1, §11.2). The transition table of
//! §5.4 is data here, not scattered `if`s, and it answers exactly one
//! question: is the state a device reports after an event a legal
//! outcome of that event?

use std::fmt;

/// A load state, as read from `PID_LOAD_STATE_CONTROL` (PID 5).
///
/// Values and remarks are `[D]` RES Table 92 (design spec §5.2). `Unloading` and
/// `LoadCompleting` are optional states: a conforming device may never
/// show them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LoadState {
    /// 0, mandatory. *"No data is loaded."*
    Unloaded,
    /// 1, mandatory. *"Only in this state the associated data shall be
    /// considered as valid"* — the definition of "the download worked",
    /// and the only one this implementation may use.
    Loaded,
    /// 2, mandatory. *"Load process is active."*
    Loading,
    /// 3, mandatory. *"Error in data detected or error during load
    /// process."* Escaped only by [`LoadEvent::Unload`].
    Error,
    /// 4, optional. *"Unload process is active"* — RES §4.23.2.3.2 warns
    /// a Management Client *"may never observe"* it.
    Unloading,
    /// 5, optional. *"Intermediate state between Loading and Loaded"*. A
    /// device in this state may not answer at all, which is conformant.
    LoadCompleting,
}

impl LoadState {
    /// Decodes the octet a property read returns, or reports it unknown
    /// rather than guessing. RES Table 92 defines 0–5; nothing defines 6
    /// upwards, so nothing here invents a meaning for it.
    pub fn from_octet(octet: u8) -> Result<Self, UnknownLoadState> {
        match octet {
            0 => Ok(LoadState::Unloaded),
            1 => Ok(LoadState::Loaded),
            2 => Ok(LoadState::Loading),
            3 => Ok(LoadState::Error),
            4 => Ok(LoadState::Unloading),
            5 => Ok(LoadState::LoadCompleting),
            other => Err(UnknownLoadState(other)),
        }
    }

    /// The octet RES Table 92 gives this state.
    pub fn octet(self) -> u8 {
        match self {
            LoadState::Unloaded => 0,
            LoadState::Loaded => 1,
            LoadState::Loading => 2,
            LoadState::Error => 3,
            LoadState::Unloading => 4,
            LoadState::LoadCompleting => 5,
        }
    }

    /// Whether the loadable part's data may be treated as valid. True for
    /// `Loaded` and for nothing else, per RES Table 92's *"in all other
    /// states the data shall be considered as invalid"*.
    pub fn data_is_valid(self) -> bool {
        self == LoadState::Loaded
    }

    /// Whether a device in this state is allowed to stay silent. Only
    /// `LoadCompleting`: `[D]` RES Table 94's footnote says *"a device may
    /// be offline during state LoadCompleting and therefore not react to
    /// load events from a MaC"*. Silence anywhere else is a fault; silence
    /// here is the device working.
    pub fn may_be_silent(self) -> bool {
        self == LoadState::LoadCompleting
    }

    /// Every state, for exhaustive tests and for rendering.
    pub const ALL: [LoadState; 6] = [
        LoadState::Unloaded,
        LoadState::Loaded,
        LoadState::Loading,
        LoadState::Error,
        LoadState::Unloading,
        LoadState::LoadCompleting,
    ];
}

impl fmt::Display for LoadState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            LoadState::Unloaded => "Unloaded",
            LoadState::Loaded => "Loaded",
            LoadState::Loading => "Loading",
            LoadState::Error => "Error",
            LoadState::Unloading => "Unloading",
            LoadState::LoadCompleting => "LoadCompleting",
        };
        f.write_str(name)
    }
}

/// A property read of `PID_LOAD_STATE_CONTROL` returned an octet RES
/// Table 92 does not define. Surfaced, never mapped onto a neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnknownLoadState(pub u8);

impl std::error::Error for UnknownLoadState {}

impl fmt::Display for UnknownLoadState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "load state octet {:#04x} is not one of RES Table 92's states 0..=5",
            self.0
        )
    }
}

/// A load event, as written to `PID_LOAD_STATE_CONTROL` (PID 5).
///
/// Values and reactions are `[D]` RES Table 93 (design spec §5.3).
/// `Device Restart` is in the transition table but is not an event that
/// can be written to this property, so it is [`Stimulus::Restart`]
/// instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LoadEvent {
    /// `00h`. *"Nothing"*.
    NoOperation,
    /// `01h`. *"the request to start the loading of the loadable part."*
    StartLoading,
    /// `02h`. *"the request to complete the Loading of the loadable
    /// part"*, possibly via `LoadCompleting` if a checksum takes over 2 s.
    LoadCompleted,
    /// `03h`. *"additional load information like memory allocation"* —
    /// the subtype and its fields are in
    /// [`super::load_control`](super::load_control).
    AdditionalLoadControls,
    /// `04h`. *"the request to unload the loadable part."* The only event
    /// that escapes [`LoadState::Error`].
    Unload,
}

impl LoadEvent {
    /// The event octet, which is octet 0 of the ten-octet payload of
    /// design spec §7.3.
    pub fn octet(self) -> u8 {
        match self {
            LoadEvent::NoOperation => 0x00,
            LoadEvent::StartLoading => 0x01,
            LoadEvent::LoadCompleted => 0x02,
            LoadEvent::AdditionalLoadControls => 0x03,
            LoadEvent::Unload => 0x04,
        }
    }

    /// Every event, for exhaustive tests and for rendering.
    pub const ALL: [LoadEvent; 5] = [
        LoadEvent::NoOperation,
        LoadEvent::StartLoading,
        LoadEvent::LoadCompleted,
        LoadEvent::AdditionalLoadControls,
        LoadEvent::Unload,
    ];
}

impl fmt::Display for LoadEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            LoadEvent::NoOperation => "No Operation",
            LoadEvent::StartLoading => "Start Loading",
            LoadEvent::LoadCompleted => "Load Completed",
            LoadEvent::AdditionalLoadControls => "Additional Load Controls",
            LoadEvent::Unload => "Unload",
        };
        f.write_str(name)
    }
}

/// What provoked a transition. RES Table 94 has one row that is not a
/// writable event — `Device Restart` — and folding it into [`LoadEvent`]
/// would let a caller try to write it to the property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stimulus {
    /// A write of a load event to `PID_LOAD_STATE_CONTROL`.
    Event(LoadEvent),
    /// The device restarted, however that happened.
    Restart,
}

impl fmt::Display for Stimulus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stimulus::Event(event) => event.fmt(f),
            Stimulus::Restart => f.write_str("Device Restart"),
        }
    }
}

/// The device's mask version, from Device Descriptor Type 0, as far as the
/// transition table cares about it.
///
/// It exists here for one reason: `[D, corpus]` PROF §5.3 footnote a
/// forbids the `Loaded` → `Error` alternative for mask `0912h` couplers
/// (design spec §5.4). A profile that is silent about a cell leaves RES Table 94
/// as it is, so the permitted-outcome set is **narrowed** per mask and
/// never widened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MaskVersion(pub u16);

impl MaskVersion {
    /// Mask `0912h`, the coupler mask PROF §5.3 footnote a singles out.
    pub const COUPLER_0912: MaskVersion = MaskVersion(0x0912);

    /// Whether this mask forbids `Loaded` → `Error` on a `Load Completed`
    /// event. `[D, corpus]` PROF §5.3 footnote a: *"This is not allowed
    /// for mask version 0912h Couplers. Mask 0912h shall stay in state
    /// 'Loaded' in case of an error."*
    pub fn forbids_loaded_to_error_on_load_completed(self) -> bool {
        self == MaskVersion::COUPLER_0912
    }
}

impl fmt::Display for MaskVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04X}h", self.0)
    }
}

/// The set of states a stimulus may legally leave a part in.
///
/// A set and not a single state, because RES §4.23.2.3.3 says so:
/// *"For events that are client errors, more than one transition is
/// allowed; the recommended transitions should be implemented but the
/// optional transitions may be implemented alternatively."* An
/// implementation that treats the optional outcome as a device defect is
/// wrong, which is why this type exists at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermittedOutcomes {
    /// The transition RES Table 94 recommends, or its only one. Always
    /// present: no cell in Table 94 is empty.
    recommended: LoadState,
    /// Alternatives a conforming device may choose instead.
    alternatives: Vec<LoadState>,
    /// Intermediate states the device may pass through and may be observed
    /// in on the way. `I:` in Table 94's own notation.
    intermediate: Vec<LoadState>,
}

impl PermittedOutcomes {
    /// The transition Table 94 recommends, or its only one.
    pub fn recommended(&self) -> LoadState {
        self.recommended
    }

    /// Alternative final states a conforming device may choose instead of
    /// [`Self::recommended`].
    pub fn alternatives(&self) -> &[LoadState] {
        &self.alternatives
    }

    /// States the device may be *observed in on the way* to a final one.
    /// Observing one of these means "still working", not "finished here".
    pub fn intermediate(&self) -> &[LoadState] {
        &self.intermediate
    }

    /// Whether `observed` is a legal thing to read back. Intermediate
    /// states count: §5.5's wait loop polls, and polling into the middle
    /// of a transition is normal.
    pub fn accepts(&self, observed: LoadState) -> bool {
        observed == self.recommended
            || self.alternatives.contains(&observed)
            || self.intermediate.contains(&observed)
    }

    /// Whether `observed` is a *settled* outcome rather than a state the
    /// device is passing through. The wait loop of §5.5 keeps polling
    /// while this is false.
    pub fn is_settled(&self, observed: LoadState) -> bool {
        observed == self.recommended || self.alternatives.contains(&observed)
    }
}

/// RES Table 94, reproduced (design spec §5.4), narrowed per mask version where
/// a profile removes an alternative.
///
/// `mask` is `None` when the device's mask version is not yet known —
/// before Device Descriptor Type 0 has been read, which is step 02 of
/// every procedure in §7. With no mask, no narrowing is applied: an
/// unknown device gets Table 94 as written, because narrowing on a guess
/// would reject a legal transition.
pub fn permitted_outcomes(
    from: LoadState,
    stimulus: Stimulus,
    mask: Option<MaskVersion>,
) -> PermittedOutcomes {
    use LoadEvent::*;
    use LoadState::*;

    // Helpers naming Table 94's own notation, so each cell below reads
    // like the cell it transcribes.
    let only = |state: LoadState| PermittedOutcomes {
        recommended: state,
        alternatives: Vec::new(),
        intermediate: Vec::new(),
    };
    // `R: a / O: b` — a recommended final state and an optional one.
    let r_or_o = |recommended: LoadState, optional: LoadState| PermittedOutcomes {
        recommended,
        alternatives: vec![optional],
        intermediate: Vec::new(),
    };
    // `I: i, M: m` — a mandatory final state reached through an
    // intermediate one that may or may not be observable.
    let through = |intermediate: LoadState, mandatory: LoadState| PermittedOutcomes {
        recommended: mandatory,
        alternatives: Vec::new(),
        intermediate: vec![intermediate],
    };
    // `R: I: i1, M: m1 / O: I: i2, M: m2` — both alternatives have their
    // own intermediate state.
    let two_paths =
        |i1: LoadState, m1: LoadState, i2: LoadState, m2: LoadState| PermittedOutcomes {
            recommended: m1,
            alternatives: vec![m2],
            intermediate: vec![i1, i2],
        };
    // `R: Error / O: I: LoadCompleting, M: Loaded` — the shape every
    // `from LoadCompleting` cell but `Unload` and `Device Restart` takes.
    let error_or_completing_to_loaded = || PermittedOutcomes {
        recommended: Error,
        alternatives: vec![Loaded],
        intermediate: vec![LoadCompleting],
    };

    let outcomes = match (stimulus, from) {
        // No Operation: the state is unchanged, whatever it was.
        (Stimulus::Event(NoOperation), state) => only(state),

        // Start Loading (01h).
        (Stimulus::Event(StartLoading), Unloaded) => only(Loading),
        (Stimulus::Event(StartLoading), Loaded) => only(Loading),
        (Stimulus::Event(StartLoading), Loading) => only(Loading),
        (Stimulus::Event(StartLoading), Error) => only(Error),
        (Stimulus::Event(StartLoading), Unloading) => only(Error),
        (Stimulus::Event(StartLoading), LoadCompleting) => error_or_completing_to_loaded(),

        // Load Completed (02h).
        (Stimulus::Event(LoadCompleted), Unloaded) => r_or_o(Unloaded, Error),
        (Stimulus::Event(LoadCompleted), Loaded) => r_or_o(Loaded, Error),
        (Stimulus::Event(LoadCompleted), Loading) => through(LoadCompleting, Loaded),
        (Stimulus::Event(LoadCompleted), Error) => only(Error),
        (Stimulus::Event(LoadCompleted), Unloading) => only(Error),
        (Stimulus::Event(LoadCompleted), LoadCompleting) => error_or_completing_to_loaded(),

        // Additional/Segment Load Controls (03h).
        (Stimulus::Event(AdditionalLoadControls), Unloaded) => r_or_o(Unloaded, Error),
        (Stimulus::Event(AdditionalLoadControls), Loaded) => only(Error),
        (Stimulus::Event(AdditionalLoadControls), Loading) => only(Loading),
        (Stimulus::Event(AdditionalLoadControls), Error) => only(Error),
        (Stimulus::Event(AdditionalLoadControls), Unloading) => only(Error),
        (Stimulus::Event(AdditionalLoadControls), LoadCompleting) => {
            error_or_completing_to_loaded()
        }

        // Unload (04h).
        (Stimulus::Event(Unload), Unloaded) => only(Unloaded),
        (Stimulus::Event(Unload), Loaded) => through(Unloading, Unloaded),
        (Stimulus::Event(Unload), Loading) => through(Unloading, Unloaded),
        (Stimulus::Event(Unload), Error) => through(Unloading, Unloaded),
        (Stimulus::Event(Unload), Unloading) => through(Unloading, Unloaded),
        (Stimulus::Event(Unload), LoadCompleting) => {
            two_paths(Unloading, Unloaded, LoadCompleting, Loaded)
        }

        // Device Restart.
        (Stimulus::Restart, Unloaded) => only(Unloaded),
        // *"Loaded (Error in case of error detection at start-up)"*: a
        // device that self-checks at boot may downgrade a good part.
        (Stimulus::Restart, Loaded) => r_or_o(Loaded, Error),
        (Stimulus::Restart, Loading) => r_or_o(Loading, Error),
        (Stimulus::Restart, Error) => only(Error),
        (Stimulus::Restart, Unloading) => only(Unloaded),
        (Stimulus::Restart, LoadCompleting) => PermittedOutcomes {
            recommended: Unloaded,
            alternatives: vec![Loaded],
            intermediate: vec![LoadCompleting],
        },
    };

    narrow_for_mask(from, stimulus, mask, outcomes)
}

/// Applies the one narrowing of RES Table 94 that PROF states, and only
/// that one. A profile silent about a cell leaves the cell alone.
fn narrow_for_mask(
    from: LoadState,
    stimulus: Stimulus,
    mask: Option<MaskVersion>,
    mut outcomes: PermittedOutcomes,
) -> PermittedOutcomes {
    let Some(mask) = mask else {
        return outcomes;
    };
    let is_loaded_load_completed =
        from == LoadState::Loaded && stimulus == Stimulus::Event(LoadEvent::LoadCompleted);
    if is_loaded_load_completed && mask.forbids_loaded_to_error_on_load_completed() {
        outcomes.alternatives.retain(|s| *s != LoadState::Error);
    }
    outcomes
}

/// Whether any event other than `Unload` can move a part out of `Error`.
/// The answer is no, and design spec §5.4 says the consequence out loud: *"A
/// retry loop that re-sends Start Loading after an error will loop
/// forever; recovery is unload, then start again."* Exposed as a function
/// so a caller can assert it rather than remember it.
pub fn escapes_error(event: LoadEvent) -> bool {
    event == LoadEvent::Unload
}

/// A pure model of one loadable part's Load State Machine, for dry runs
/// and for the simulator to build on.
///
/// It applies the **recommended** transition of [`permitted_outcomes`],
/// because a model has to choose one and the recommended one is the one
/// RES asks devices to implement. Its value is that it cannot reach
/// `Loaded` except the way a real device can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadStateModel {
    state: LoadState,
    mask: Option<MaskVersion>,
}

impl LoadStateModel {
    /// A part in `Unloaded`, the state a device with nothing loaded holds.
    pub fn new(mask: Option<MaskVersion>) -> Self {
        Self {
            state: LoadState::Unloaded,
            mask,
        }
    }

    /// A part starting in `state`, for modelling a device that is already
    /// configured.
    pub fn starting_at(state: LoadState, mask: Option<MaskVersion>) -> Self {
        Self { state, mask }
    }

    /// The current state.
    pub fn state(&self) -> LoadState {
        self.state
    }

    /// Applies a stimulus and returns the new state, taking the
    /// recommended transition.
    pub fn apply(&mut self, stimulus: Stimulus) -> LoadState {
        let outcomes = permitted_outcomes(self.state, stimulus, self.mask);
        self.state = outcomes.recommended();
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §14 item 1: the transition table, exhaustively — 6 states × 6 rows
    /// (five writable events plus Device Restart), asserting that every
    /// cell has at least one outcome and that both `R:`/`O:` alternatives
    /// are accepted where Table 94 lists them.
    #[test]
    fn every_state_and_stimulus_pair_has_a_permitted_outcome() {
        let stimuli: Vec<Stimulus> = LoadEvent::ALL
            .iter()
            .copied()
            .map(Stimulus::Event)
            .chain([Stimulus::Restart])
            .collect();
        assert_eq!(stimuli.len(), 6);
        let mut cells = 0;
        for from in LoadState::ALL {
            for stimulus in &stimuli {
                let outcomes = permitted_outcomes(from, *stimulus, None);
                assert!(
                    outcomes.accepts(outcomes.recommended()),
                    "{from} + {stimulus} must accept its own recommendation"
                );
                for alternative in outcomes.alternatives() {
                    assert!(
                        outcomes.accepts(*alternative),
                        "{from} + {stimulus} must accept the optional outcome {alternative}"
                    );
                }
                cells += 1;
            }
        }
        assert_eq!(cells, 36);
    }

    #[test]
    fn no_operation_never_changes_the_state() {
        for from in LoadState::ALL {
            let outcomes = permitted_outcomes(from, Stimulus::Event(LoadEvent::NoOperation), None);
            assert_eq!(outcomes.recommended(), from);
            assert!(outcomes.alternatives().is_empty());
        }
    }

    /// §14 item 2: `Error` is a trap.
    #[test]
    fn error_is_a_trap_for_every_event_but_unload() {
        for event in LoadEvent::ALL {
            let outcomes = permitted_outcomes(LoadState::Error, Stimulus::Event(event), None);
            if event == LoadEvent::Unload {
                assert_eq!(outcomes.recommended(), LoadState::Unloaded);
                assert!(escapes_error(event));
            } else {
                assert_eq!(
                    outcomes.recommended(),
                    LoadState::Error,
                    "{event} must leave Error at Error"
                );
                assert!(outcomes.alternatives().is_empty());
                assert!(!escapes_error(event));
            }
        }
    }

    #[test]
    fn a_restart_does_not_escape_error_either() {
        let outcomes = permitted_outcomes(LoadState::Error, Stimulus::Restart, None);
        assert_eq!(outcomes.recommended(), LoadState::Error);
        assert!(outcomes.alternatives().is_empty());
    }

    #[test]
    fn unload_terminates_at_unloaded_from_every_state_but_load_completing() {
        for from in LoadState::ALL {
            if from == LoadState::LoadCompleting {
                continue;
            }
            let outcomes = permitted_outcomes(from, Stimulus::Event(LoadEvent::Unload), None);
            assert_eq!(outcomes.recommended(), LoadState::Unloaded, "from {from}");
        }
    }

    #[test]
    fn unloaded_is_accepted_directly_after_an_unload_without_observing_unloading() {
        // RES §4.23.2.3.2: a Management Client *"may never observe the
        // state Unloading"*. So `Unloaded` must be a settled outcome, and
        // `Unloading` must be accepted but not required.
        let outcomes =
            permitted_outcomes(LoadState::Loaded, Stimulus::Event(LoadEvent::Unload), None);
        assert!(outcomes.is_settled(LoadState::Unloaded));
        assert!(outcomes.accepts(LoadState::Unloading));
        assert!(!outcomes.is_settled(LoadState::Unloading));
    }

    #[test]
    fn start_loading_from_loaded_invalidates_the_part_immediately() {
        // §5.4: there is no "prepare a download and then commit it".
        let outcomes = permitted_outcomes(
            LoadState::Loaded,
            Stimulus::Event(LoadEvent::StartLoading),
            None,
        );
        assert_eq!(outcomes.recommended(), LoadState::Loading);
        assert!(!outcomes.recommended().data_is_valid());
    }

    #[test]
    fn load_completing_is_the_only_state_allowed_to_stay_silent() {
        for state in LoadState::ALL {
            assert_eq!(state.may_be_silent(), state == LoadState::LoadCompleting);
        }
    }

    #[test]
    fn load_completed_from_loading_passes_through_load_completing() {
        let outcomes = permitted_outcomes(
            LoadState::Loading,
            Stimulus::Event(LoadEvent::LoadCompleted),
            None,
        );
        assert_eq!(outcomes.recommended(), LoadState::Loaded);
        assert_eq!(outcomes.intermediate(), &[LoadState::LoadCompleting]);
        assert!(outcomes.accepts(LoadState::LoadCompleting));
        assert!(!outcomes.is_settled(LoadState::LoadCompleting));
    }

    #[test]
    fn mask_0912_may_not_answer_error_to_load_completed_from_loaded() {
        // PROF §5.3 footnote a, the one narrowing this implementation
        // applies. Unknown mask keeps Table 94 as written.
        let stimulus = Stimulus::Event(LoadEvent::LoadCompleted);
        let unnarrowed = permitted_outcomes(LoadState::Loaded, stimulus, None);
        assert!(unnarrowed.accepts(LoadState::Error));

        let narrowed =
            permitted_outcomes(LoadState::Loaded, stimulus, Some(MaskVersion::COUPLER_0912));
        assert_eq!(narrowed.recommended(), LoadState::Loaded);
        assert!(!narrowed.accepts(LoadState::Error));
    }

    #[test]
    fn narrowing_never_widens_another_cell() {
        // The mask narrows exactly one cell. Every other cell must be
        // identical with and without the mask — a narrowing that widens
        // is the bug this asserts against.
        let stimuli: Vec<Stimulus> = LoadEvent::ALL
            .iter()
            .copied()
            .map(Stimulus::Event)
            .chain([Stimulus::Restart])
            .collect();
        for from in LoadState::ALL {
            for stimulus in &stimuli {
                let plain = permitted_outcomes(from, *stimulus, None);
                let masked = permitted_outcomes(from, *stimulus, Some(MaskVersion::COUPLER_0912));
                let is_the_narrowed_cell = from == LoadState::Loaded
                    && *stimulus == Stimulus::Event(LoadEvent::LoadCompleted);
                if is_the_narrowed_cell {
                    assert_ne!(plain, masked);
                } else {
                    assert_eq!(plain, masked, "{from} + {stimulus} must be untouched");
                }
                // Narrowing can only remove alternatives, never add.
                assert!(masked.alternatives().len() <= plain.alternatives().len());
            }
        }
    }

    #[test]
    fn an_unspecified_mask_applies_no_narrowing() {
        let stimulus = Stimulus::Event(LoadEvent::LoadCompleted);
        let other_mask = permitted_outcomes(LoadState::Loaded, stimulus, Some(MaskVersion(0x07B0)));
        assert!(other_mask.accepts(LoadState::Error));
    }

    #[test]
    fn state_octets_round_trip_and_unknown_octets_are_surfaced() {
        for state in LoadState::ALL {
            assert_eq!(LoadState::from_octet(state.octet()), Ok(state));
        }
        assert_eq!(LoadState::from_octet(6), Err(UnknownLoadState(6)));
        assert_eq!(LoadState::from_octet(255), Err(UnknownLoadState(255)));
        assert!(LoadState::from_octet(6)
            .unwrap_err()
            .to_string()
            .contains("0x06"));
    }

    #[test]
    fn event_octets_are_the_values_res_table_93_gives() {
        assert_eq!(LoadEvent::NoOperation.octet(), 0x00);
        assert_eq!(LoadEvent::StartLoading.octet(), 0x01);
        assert_eq!(LoadEvent::LoadCompleted.octet(), 0x02);
        assert_eq!(LoadEvent::AdditionalLoadControls.octet(), 0x03);
        assert_eq!(LoadEvent::Unload.octet(), 0x04);
    }

    #[test]
    fn only_loaded_counts_as_valid_data() {
        for state in LoadState::ALL {
            assert_eq!(state.data_is_valid(), state == LoadState::Loaded);
        }
    }

    /// §14 item 12, the property test: any sequence of stimuli applied to
    /// the model never reports `Loaded` unless a `Load Completed` was
    /// applied while the part was in `Loading`.
    #[test]
    fn the_model_never_reaches_loaded_without_a_load_completed_from_loading() {
        let stimuli: Vec<Stimulus> = LoadEvent::ALL
            .iter()
            .copied()
            .map(Stimulus::Event)
            .chain([Stimulus::Restart])
            .collect();

        // Exhaustive over every sequence of length up to 4 — 6^1 + 6^2 +
        // 6^3 + 6^4 = 1554 sequences, which is cheap and, unlike a random
        // generator, is the same set on every run.
        fn walk(stimuli: &[Stimulus], depth: usize, model: LoadStateModel, earned: bool) {
            if depth == 0 {
                return;
            }
            for stimulus in stimuli {
                let mut next = model.clone();
                let was_loading = next.state() == LoadState::Loading;
                let now = next.apply(*stimulus);
                let earned_now = earned
                    || (was_loading
                        && *stimulus == Stimulus::Event(LoadEvent::LoadCompleted)
                        && now == LoadState::Loaded);
                if now == LoadState::Loaded {
                    assert!(
                        earned_now,
                        "reached Loaded without a Load Completed from Loading"
                    );
                }
                // Leaving Loaded means the next arrival has to be earned
                // again.
                let carry = if now == LoadState::Loaded {
                    earned_now
                } else {
                    false
                };
                walk(stimuli, depth - 1, next, carry);
            }
        }

        walk(&stimuli, 4, LoadStateModel::new(None), false);
    }

    #[test]
    fn the_model_walks_the_happy_path_of_section_7_2() {
        let mut model = LoadStateModel::new(None);
        assert_eq!(model.state(), LoadState::Unloaded);
        assert_eq!(
            model.apply(Stimulus::Event(LoadEvent::StartLoading)),
            LoadState::Loading
        );
        assert_eq!(
            model.apply(Stimulus::Event(LoadEvent::AdditionalLoadControls)),
            LoadState::Loading
        );
        assert_eq!(
            model.apply(Stimulus::Event(LoadEvent::LoadCompleted)),
            LoadState::Loaded
        );
        assert!(model.state().data_is_valid());
    }

    #[test]
    fn a_model_in_error_only_leaves_via_unload() {
        let mut model = LoadStateModel::starting_at(LoadState::Error, None);
        for event in [
            LoadEvent::NoOperation,
            LoadEvent::StartLoading,
            LoadEvent::LoadCompleted,
            LoadEvent::AdditionalLoadControls,
        ] {
            assert_eq!(model.apply(Stimulus::Event(event)), LoadState::Error);
        }
        assert_eq!(
            model.apply(Stimulus::Event(LoadEvent::Unload)),
            LoadState::Unloaded
        );
    }
}
