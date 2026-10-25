//! The smallest download scope a device's project programming flags advise (hints, not evidence).
//!
//! The input is the project's belief, the five `*_loaded` flags of
//! [`CommissioningState`], and whether the device has an individual address.
//! Those flags are what an ETS import or an earlier KNXBench run recorded;
//! they are not what the device holds. The advice therefore only says which
//! download would close what the project *marks* pending. Whether the device
//! really carries the application a partial download builds on is checked
//! on the device, by the partial plan's own preconditions, before anything
//! is written ([`super::partial_memory_download`]).
//!
//! The rules:
//!
//! - application pending: the complete download, whatever else is pending;
//! - otherwise parameters and/or group addresses pending: the partial
//!   download of exactly those (CP §3.9.2.4's *"Partial Download Type"*);
//! - otherwise: no download at all.
//!
//! The individual address and the medium configuration are not written by
//! any memory download, so they never widen or narrow the write. They are
//! reported as separate unmet needs instead.

use super::partial_memory_download::PartialDownloadParts;
use super::CommissioningState;
use crate::{DeviceInstance, IndividualAddress};

/// The memory download the flags advise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdvisedWrite {
    /// The application program is pending: the complete download.
    Complete,
    /// Only parameters and/or group addresses are pending.
    Partial(PartialDownloadParts),
    /// No application-side part is pending: nothing to download.
    Nothing,
}

impl AdvisedWrite {
    /// Stable short name, for the API and the CLI.
    pub fn code(self) -> &'static str {
        match self {
            AdvisedWrite::Complete => "complete",
            AdvisedWrite::Partial(parts) => match (parts.parameters, parts.group_addresses) {
                (true, true) => "both",
                (true, false) => "parameters",
                (false, true) => "groupAddresses",
                // A partial download of nothing writes nothing.
                (false, false) => "nothing",
            },
            AdvisedWrite::Nothing => "nothing",
        }
    }
}

/// What the individual address still needs; no memory download writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressNeed {
    /// The project marks the address programmed.
    Current,
    /// The project marks the address pending: address programming owes it.
    Pending,
    /// The device has no individual address in the project.
    Missing,
}

impl AddressNeed {
    /// Stable short name, for the API and the CLI.
    pub fn code(self) -> &'static str {
        match self {
            AddressNeed::Current => "current",
            AddressNeed::Pending => "pending",
            AddressNeed::Missing => "missing",
        }
    }
}

/// The advice for one device, from its project flags only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeAdvice {
    /// The memory download that closes the pending application-side parts.
    pub write: AdvisedWrite,
    /// The individual address, an unmet need of its own.
    pub address: AddressNeed,
    /// `MediumConfigLoaded` is false: no KNXBench download writes it.
    pub medium_pending: bool,
}

impl ScopeAdvice {
    /// The advice for `state`, for a device at `address` (`None`: none).
    pub fn from_flags(state: &CommissioningState, address: Option<IndividualAddress>) -> Self {
        let pending = state.pending_parts();
        let write = if pending.application {
            AdvisedWrite::Complete
        } else if pending.parameters || pending.group_addresses {
            AdvisedWrite::Partial(PartialDownloadParts {
                parameters: pending.parameters,
                group_addresses: pending.group_addresses,
            })
        } else {
            AdvisedWrite::Nothing
        };
        let address = match address {
            None => AddressNeed::Missing,
            Some(_) if pending.address => AddressNeed::Pending,
            Some(_) => AddressNeed::Current,
        };
        Self {
            write,
            address,
            medium_pending: !state.medium_config_loaded,
        }
    }

    /// The advice for `device`, from its flags and its address.
    pub fn for_device(device: &DeviceInstance) -> Self {
        Self::from_flags(&device.commissioning, device.address)
    }

    /// The download to run: `None` for none, `Some(None)` for the complete
    /// one, `Some(Some(parts))` for a partial one.
    pub fn download(&self) -> Option<Option<PartialDownloadParts>> {
        match self.write {
            AdvisedWrite::Complete => Some(None),
            AdvisedWrite::Partial(parts) => Some(Some(parts)),
            AdvisedWrite::Nothing => None,
        }
    }

    /// True only when the project marks all five parts programmed and the
    /// device has an address. Still the project's belief, not device fact.
    pub fn fully_programmed(&self) -> bool {
        self.write == AdvisedWrite::Nothing
            && self.address == AddressNeed::Current
            && !self.medium_pending
    }
}

#[cfg(test)]
mod tests;
