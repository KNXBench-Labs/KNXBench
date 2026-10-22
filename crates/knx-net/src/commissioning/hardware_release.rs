//! Typed consent to run an experimental best-effort download on real hardware.
//!
//! The experimental property-download executor refuses every hardware-kind
//! transport unless a [`HardwareRelease`] is supplied. The release is a
//! separate, more specific consent than the ordinary
//! [`WriteAuthorisation`](knx_core::commissioning::mutation::WriteAuthorisation):
//! it names the target and acknowledges that unwritten (masked) allocation
//! bytes are left untouched and that recovery is best-effort only. See
//! ADR-0152. The value is opaque (no `Default`, no `Copy`) and can only be
//! produced from the exact typed phrase, so a caller cannot reach hardware
//! without naming the device and the best-effort nature.

use std::fmt;

use knx_core::{ContactableAddress, IndividualAddress};

/// Why a hardware-release consent was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareReleaseError {
    /// The target is on the project exclusion list.
    Excluded,
    /// The typed confirmation did not match the required phrase.
    ConfirmationMismatch { expected: String },
}

impl fmt::Display for HardwareReleaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Excluded => write!(f, "target is project-excluded"),
            Self::ConfirmationMismatch { expected } => {
                write!(f, "confirmation mismatch; expected exactly: {expected}")
            }
        }
    }
}

impl std::error::Error for HardwareReleaseError {}

/// The phrase an operator must type, verbatim, to release an experimental
/// download that leaves unwritten (masked) bytes on a physical device. The
/// phrase names the target, so a consent for one device cannot be replayed
/// against another.
pub fn required_release_phrase(target: IndividualAddress) -> String {
    format!(
        "I accept best-effort download to {target} with unwritten masked bytes and no guaranteed recovery"
    )
}

/// Consent that one experimental, best-effort download may run on one physical
/// device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareRelease {
    target: ContactableAddress,
}

impl HardwareRelease {
    /// Authorises a best-effort hardware download for the given target, given
    /// the operator's confirmation phrase typed out in full. A near miss is a
    /// refusal.
    pub fn for_hardware(
        target: IndividualAddress,
        confirmation: &str,
    ) -> Result<Self, HardwareReleaseError> {
        let target = ContactableAddress::new(target).map_err(|_| HardwareReleaseError::Excluded)?;
        let expected = required_release_phrase(target.address());
        if confirmation != expected {
            return Err(HardwareReleaseError::ConfirmationMismatch { expected });
        }
        Ok(Self { target })
    }

    /// The one device this release authorises.
    pub fn target(&self) -> ContactableAddress {
        self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_phrase_is_accepted_and_binds_the_target() {
        let target = IndividualAddress::from_raw(0x0119);
        let release =
            HardwareRelease::for_hardware(target, &required_release_phrase(target)).unwrap();
        assert_eq!(release.target().address(), target);
    }

    #[test]
    fn a_near_miss_phrase_is_refused() {
        let target = IndividualAddress::from_raw(0x0119);
        let wrong = "I accept best-effort download with unwritten masked bytes";
        assert!(matches!(
            HardwareRelease::for_hardware(target, wrong),
            Err(HardwareReleaseError::ConfirmationMismatch { .. })
        ));
    }

    #[test]
    fn a_phrase_for_another_device_is_refused() {
        let a = IndividualAddress::from_raw(0x0119);
        let b = IndividualAddress::from_raw(0x0120);
        assert!(matches!(
            HardwareRelease::for_hardware(a, &required_release_phrase(b)),
            Err(HardwareReleaseError::ConfirmationMismatch { .. })
        ));
    }
}
