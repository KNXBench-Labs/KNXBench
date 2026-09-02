//! Commissioning state: the delta between the planned project and the
//! physical installation. Modelled as domain data, not import metadata,
//! because that delta is engineering-critical (DATA_MODEL §7).

use chrono::{DateTime, Utc};

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
}
