//! Commissioning state: the delta between the planned project and the
//! physical installation. Modelled as domain data, not import metadata,
//! because that delta is engineering-critical (DATA_MODEL §7).

pub mod error_code;
pub mod load_state;
pub mod memory;

use std::fmt;

use chrono::{DateTime, Utc};

use load_state::LoadState;

/// `DeviceInstance/@CompletionStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompletionStatus {
    #[default]
    Undefined,
    Editing,
    FinishedDesign,
    Accepted,
}

/// The five `*Loaded` flags plus the download bookkeeping ETS keeps per
/// device.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommissioningState {
    pub completion: CompletionStatus,
    pub individual_address_loaded: bool,
    pub application_program_loaded: bool,
    pub parameters_loaded: bool,
    pub communication_part_loaded: bool,
    pub medium_config_loaded: bool,
    pub last_modified: Option<DateTime<Utc>>,
    pub last_download: Option<DateTime<Utc>>,
    pub broken: bool,
    /// What the device itself reported, which is a different kind of
    /// claim from the five `*_loaded` flags above and therefore a
    /// different field (§11.2). The flags are project intent; this is
    /// device fact.
    pub device_reported: DeviceLoadStates,
}

impl CommissioningState {
    /// What the project believes about `part`. Project intent, no device
    /// involved.
    pub fn intends_loaded(&self, part: LoadPart) -> bool {
        match part {
            LoadPart::IndividualAddress => self.individual_address_loaded,
            LoadPart::ApplicationProgram => self.application_program_loaded,
            LoadPart::Parameters => self.parameters_loaded,
            LoadPart::CommunicationPart => self.communication_part_loaded,
            LoadPart::MediumConfig => self.medium_config_loaded,
        }
    }

    /// Records the project's belief. Deliberately separate from
    /// [`DeviceLoadStates::record`], so that no single call can set both
    /// and thereby launder an intention into a fact.
    pub fn set_intends_loaded(&mut self, part: LoadPart, loaded: bool) {
        let slot = match part {
            LoadPart::IndividualAddress => &mut self.individual_address_loaded,
            LoadPart::ApplicationProgram => &mut self.application_program_loaded,
            LoadPart::Parameters => &mut self.parameters_loaded,
            LoadPart::CommunicationPart => &mut self.communication_part_loaded,
            LoadPart::MediumConfig => &mut self.medium_config_loaded,
        };
        *slot = loaded;
    }

    /// Every part where intent and device fact do not match, including
    /// parts the device was never asked about. An empty result is the only
    /// honest basis for telling a user the device matches the project.
    pub fn disagreements(&self) -> Vec<LoadDisagreement> {
        LoadPart::ALL
            .into_iter()
            .filter_map(|part| {
                let intended_loaded = self.intends_loaded(part);
                let reported = self.device_reported.reported(part);
                let agrees = match reported {
                    Some(LoadState::Loaded) => intended_loaded,
                    Some(_) => !intended_loaded,
                    None => false,
                };
                (!agrees).then_some(LoadDisagreement {
                    part,
                    intended_loaded,
                    reported,
                })
            })
            .collect()
    }

    /// True only when every part was asked and every answer matched.
    pub fn device_matches_project(&self) -> bool {
        self.disagreements().is_empty()
    }
}

/// One of the five parts a device is loaded with, as the project schema
/// names them and as §3.1's per-part Load State Machines see them.
///
/// The project has one flag per part; the device has one LSM per part. The
/// two are counted the same way so that a disagreement is a comparison and
/// not a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LoadPart {
    /// The individual address itself.
    IndividualAddress,
    /// The application program.
    ApplicationProgram,
    /// The parameters.
    Parameters,
    /// The communication part: Group Address Table, Association Table,
    /// Group Object Table.
    CommunicationPart,
    /// The medium-dependent configuration.
    MediumConfig,
}

impl LoadPart {
    /// All five, in project-schema order.
    pub const ALL: [LoadPart; 5] = [
        LoadPart::IndividualAddress,
        LoadPart::ApplicationProgram,
        LoadPart::Parameters,
        LoadPart::CommunicationPart,
        LoadPart::MediumConfig,
    ];
}

impl fmt::Display for LoadPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LoadPart::IndividualAddress => "individual address",
            LoadPart::ApplicationProgram => "application program",
            LoadPart::Parameters => "parameters",
            LoadPart::CommunicationPart => "communication part",
            LoadPart::MediumConfig => "medium configuration",
        })
    }
}

/// What each part's Load State Machine actually answered, the last time
/// one was read.
///
/// `None` is *not* `Unloaded`: it means no device ever said anything about
/// this part, which is the case for every device that has not been
/// contacted. Spec §11.2 requires this to be separate from the project's
/// intent, because *"conflating them produces an application that tells
/// the user a download succeeded because it was asked to succeed."*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DeviceLoadStates {
    individual_address: Option<LoadState>,
    application_program: Option<LoadState>,
    parameters: Option<LoadState>,
    communication_part: Option<LoadState>,
    medium_config: Option<LoadState>,
}

impl DeviceLoadStates {
    /// What the device last reported for `part`, or `None` if it never
    /// reported anything.
    pub fn reported(&self, part: LoadPart) -> Option<LoadState> {
        match part {
            LoadPart::IndividualAddress => self.individual_address,
            LoadPart::ApplicationProgram => self.application_program,
            LoadPart::Parameters => self.parameters,
            LoadPart::CommunicationPart => self.communication_part,
            LoadPart::MediumConfig => self.medium_config,
        }
    }

    /// Records what a read of `PID_LOAD_STATE_CONTROL` returned. The only
    /// way a device fact enters this type.
    pub fn record(&mut self, part: LoadPart, state: LoadState) {
        let slot = match part {
            LoadPart::IndividualAddress => &mut self.individual_address,
            LoadPart::ApplicationProgram => &mut self.application_program,
            LoadPart::Parameters => &mut self.parameters,
            LoadPart::CommunicationPart => &mut self.communication_part,
            LoadPart::MediumConfig => &mut self.medium_config,
        };
        *slot = Some(state);
    }

    /// Forgets every recorded state, which is what a disconnect or a
    /// restart makes true: the facts were about a session that is over.
    pub fn forget(&mut self) {
        *self = Self::default();
    }

    /// True only if the device itself said `Loaded` for this part. A part
    /// nobody asked about is not confirmed.
    pub fn confirms_loaded(&self, part: LoadPart) -> bool {
        self.reported(part) == Some(LoadState::Loaded)
    }
}

/// A part where the project's belief and the device's answer differ, or
/// where the device has not answered at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadDisagreement {
    /// The part in question.
    pub part: LoadPart,
    /// What the project believes.
    pub intended_loaded: bool,
    /// What the device said, if it said anything.
    pub reported: Option<LoadState>,
}

impl fmt::Display for LoadDisagreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let belief = if self.intended_loaded {
            "the project believes it is loaded"
        } else {
            "the project believes it is not loaded"
        };
        match self.reported {
            Some(state) => write!(f, "{}: {belief}, the device reported {state}", self.part),
            None => write!(f, "{}: {belief}, the device has not been asked", self.part),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_commissioning_state_is_all_unloaded_and_undefined() {
        let s = CommissioningState::default();
        assert_eq!(s.completion, CompletionStatus::Undefined);
        assert!(!s.individual_address_loaded);
        assert!(!s.broken);
        assert!(s.last_modified.is_none());
    }

    #[test]
    fn a_fresh_state_has_no_device_facts_at_all() {
        let s = CommissioningState::default();
        for part in LoadPart::ALL {
            assert_eq!(s.device_reported.reported(part), None);
            assert!(!s.device_reported.confirms_loaded(part));
        }
    }

    /// §11.2, the whole point of the split: believing a part is loaded is
    /// not the device saying so.
    #[test]
    fn project_intent_alone_never_becomes_a_device_fact() {
        let mut s = CommissioningState::default();
        for part in LoadPart::ALL {
            s.set_intends_loaded(part, true);
        }
        assert!(!s.device_matches_project());
        assert_eq!(s.disagreements().len(), 5);
        for disagreement in s.disagreements() {
            assert!(disagreement.intended_loaded);
            assert_eq!(disagreement.reported, None);
        }
    }

    #[test]
    fn agreement_needs_both_the_intent_and_the_answer() {
        let mut s = CommissioningState::default();
        for part in LoadPart::ALL {
            s.set_intends_loaded(part, true);
            s.device_reported.record(part, LoadState::Loaded);
        }
        assert!(s.device_matches_project());
        assert!(s.disagreements().is_empty());
    }

    #[test]
    fn a_device_reporting_error_disagrees_with_a_project_that_expects_loaded() {
        let mut s = CommissioningState::default();
        s.set_intends_loaded(LoadPart::Parameters, true);
        s.device_reported
            .record(LoadPart::Parameters, LoadState::Error);
        let disagreements = s.disagreements();
        let parameters = disagreements
            .iter()
            .find(|d| d.part == LoadPart::Parameters)
            .expect("the parameters part must be reported as disagreeing");
        assert_eq!(parameters.reported, Some(LoadState::Error));
        assert!(parameters.intended_loaded);
        assert!(
            parameters.to_string().contains("Error"),
            "the message must name the state the device gave: {parameters}"
        );
    }

    #[test]
    fn an_unloaded_part_the_project_does_not_expect_is_not_a_disagreement() {
        let mut s = CommissioningState::default();
        for part in LoadPart::ALL {
            s.device_reported.record(part, LoadState::Unloaded);
        }
        assert!(s.device_matches_project());
    }

    #[test]
    fn a_restart_forgets_device_facts_but_keeps_project_intent() {
        let mut s = CommissioningState::default();
        s.set_intends_loaded(LoadPart::ApplicationProgram, true);
        for part in LoadPart::ALL {
            let state = if part == LoadPart::ApplicationProgram {
                LoadState::Loaded
            } else {
                LoadState::Unloaded
            };
            s.device_reported.record(part, state);
        }
        assert!(s.device_matches_project());
        s.device_reported.forget();
        assert!(s.intends_loaded(LoadPart::ApplicationProgram));
        assert!(!s.device_matches_project());
    }
}
