//! Decodes property-based application run-state observations without inferring device function.

use std::fmt;

/// A state read from PID_RUN_STATE_CONTROL, not an event written to it.
///
/// Resources v01.10.01 §4.24.2.3.1, Table 95, printed p. 299 defines
/// these complete octet values for the property-based Type 1 realization.
/// They are not a decoder for the separately specified memory realization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunState {
    Halted,
    Running,
    Ready,
    Terminated,
    Starting,
    ShuttingDown,
}

impl RunState {
    /// Decodes one complete property-state octet; undefined values stay errors.
    pub fn from_octet(octet: u8) -> Result<Self, UnknownRunState> {
        match octet {
            0x00 => Ok(Self::Halted),
            0x01 => Ok(Self::Running),
            0x02 => Ok(Self::Ready),
            0x03 => Ok(Self::Terminated),
            0x04 => Ok(Self::Starting),
            0x05 => Ok(Self::ShuttingDown),
            other => Err(UnknownRunState(other)),
        }
    }
}

impl fmt::Display for RunState {
    /// The state in words for people, not the variant name.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Halted => "halted",
            Self::Running => "running",
            Self::Ready => "ready",
            Self::Terminated => "terminated",
            Self::Starting => "starting",
            Self::ShuttingDown => "shutting down",
        })
    }
}

/// An observation whose octet has no meaning in Resources Table 95.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnknownRunState(pub u8);

impl std::error::Error for UnknownRunState {}

impl fmt::Display for UnknownRunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "property run state {:#04x} is undefined in Resources Table 95",
            self.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_state_octets_have_literal_meanings() {
        for (octet, expected) in [
            (0x00, RunState::Halted),
            (0x01, RunState::Running),
            (0x02, RunState::Ready),
            (0x03, RunState::Terminated),
            (0x04, RunState::Starting),
            (0x05, RunState::ShuttingDown),
        ] {
            assert_eq!(
                RunState::from_octet(octet),
                Ok(expected),
                "octet {octet:#04x}"
            );
        }
    }

    #[test]
    fn display_is_human_words_for_each_state() {
        for (state, words) in [
            (RunState::Halted, "halted"),
            (RunState::Running, "running"),
            (RunState::Ready, "ready"),
            (RunState::Terminated, "terminated"),
            (RunState::Starting, "starting"),
            (RunState::ShuttingDown, "shutting down"),
        ] {
            assert_eq!(state.to_string(), words);
        }
    }

    #[test]
    fn every_undefined_property_state_remains_its_own_error() {
        for octet in 0x06..=0xFF {
            assert_eq!(RunState::from_octet(octet), Err(UnknownRunState(octet)));
        }
    }
}
