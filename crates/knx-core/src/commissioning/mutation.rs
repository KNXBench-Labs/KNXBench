//! The authorisation value every write to a device must carry, unconstructible by accident.
//!
//! Spec §2.3: the mutation API *"is constructed from an explicit
//! authorisation value carrying the operator's confirmation and the
//! concrete target, and … cannot be constructed by default"*. There is no
//! `Default`, no `dry_run: bool`, and no way to widen a value's scope or
//! retarget it after the fact.

use std::fmt;

use crate::address::{ContactableAddress, ExcludedAddress, IndividualAddress};

/// What class of write an authorisation covers.
///
/// Separate variants rather than a bitmask, because an operator who
/// confirmed a download did not thereby confirm a restart, and a bitmask
/// invites `ALL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WriteScope {
    /// The load procedures of §7.2/§7.4: load-control events, memory
    /// writes, `PID_PROGRAM_VERSION`.
    Download,
    /// `LoadControl = Unload` on one or more parts (§7.5 steps 01–06).
    Unload,
    /// `A_IndividualAddress_Write` after a button press (§4.2).
    IndividualAddressProgramming,
    /// `A_Restart`, Basic or Master (§8).
    Restart,
    /// The `0060h` read-modify-write of §4.4, which this project performs
    /// against the simulator only.
    ProgrammingModeToggle,
    /// MP §2.18 `NM_IndividualAddress_Reset`: every device in programming
    /// mode goes to `FFFFh` and is restarted there. The target is always
    /// `FFFFh`, because the devices are whoever has the button pressed.
    IndividualAddressReset,
}

impl fmt::Display for WriteScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WriteScope::Download => "download",
            WriteScope::Unload => "unload",
            WriteScope::IndividualAddressProgramming => "individual-address programming",
            WriteScope::Restart => "restart",
            WriteScope::ProgrammingModeToggle => "programming-mode toggle",
            WriteScope::IndividualAddressReset => "individual-address reset",
        })
    }
}

/// What the authorisation points at.
///
/// §2.3: *"Phase 2's simulator is the only thing the mutation API is
/// pointed at until the user says otherwise."* The distinction is in the
/// type so that "until the user says otherwise" is enforced by the
/// compiler rather than by memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetKind {
    /// A simulated device. Needs no operator confirmation, because there
    /// is no operator and nothing to break.
    Simulator,
    /// A physical device on a bus. Requires the confirmation phrase.
    Hardware,
}

/// The phrase an operator must type, verbatim, to authorise a write to a
/// physical device.
///
/// It names the scope and the target, so that a confirmation obtained for
/// one device cannot be replayed against another: the string itself is
/// device-specific.
pub fn required_confirmation_phrase(target: IndividualAddress, scope: WriteScope) -> String {
    format!("I confirm {scope} to {target}")
}

/// Permission to perform one class of write against one concrete device.
///
/// Deliberately not `Default`, not `Copy`, and constructed only through
/// [`WriteAuthorisation::for_simulator`] or
/// [`WriteAuthorisation::for_hardware`]. A function that takes one of
/// these cannot be reached without a caller having produced one, and a
/// function that does not take one cannot write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteAuthorisation {
    target: ContactableAddress,
    scope: WriteScope,
    kind: TargetKind,
}

impl WriteAuthorisation {
    /// Authorises a write against the simulator.
    ///
    /// Still goes through [`ContactableAddress`], so a simulated device may
    /// not be given a project-excluded address either: a fixture that used
    /// `1.1.220` as a harmless stand-in would put the address into scan
    /// plans and diagnostics through the back door.
    pub fn for_simulator(
        target: IndividualAddress,
        scope: WriteScope,
    ) -> Result<Self, AuthorisationRefused> {
        Ok(Self {
            target: ContactableAddress::new(target).map_err(AuthorisationRefused::Excluded)?,
            scope,
            kind: TargetKind::Simulator,
        })
    }

    /// Authorises a write against a physical device, given the operator's
    /// confirmation phrase typed out in full.
    ///
    /// The phrase must equal [`required_confirmation_phrase`] for exactly
    /// this target and scope. A near miss is a refusal, because the point
    /// of the phrase is that it cannot be produced by a caller that did
    /// not know which device it was about to write to.
    pub fn for_hardware(
        target: IndividualAddress,
        scope: WriteScope,
        confirmation: &str,
    ) -> Result<Self, AuthorisationRefused> {
        let target = ContactableAddress::new(target).map_err(AuthorisationRefused::Excluded)?;
        let expected = required_confirmation_phrase(target.address(), scope);
        if confirmation != expected {
            return Err(AuthorisationRefused::ConfirmationMismatch { expected });
        }
        Ok(Self {
            target,
            scope,
            kind: TargetKind::Hardware,
        })
    }

    /// The one device this value authorises.
    pub fn target(&self) -> ContactableAddress {
        self.target
    }

    /// The one class of write this value authorises.
    pub fn scope(&self) -> WriteScope {
        self.scope
    }

    /// Whether this authorises a physical device or the simulator.
    pub fn kind(&self) -> TargetKind {
        self.kind
    }

    /// Checks this authorisation against the write about to happen.
    ///
    /// Every write entry point calls this, so that an authorisation for one
    /// device and scope cannot be handed to a call about another. Returns
    /// the target on success, which is the address the caller is then
    /// allowed to send to — there is no other way to obtain it from here.
    pub fn authorise(
        &self,
        target: IndividualAddress,
        scope: WriteScope,
    ) -> Result<ContactableAddress, AuthorisationRefused> {
        if self.target.address() != target {
            return Err(AuthorisationRefused::WrongTarget {
                authorised: self.target.address(),
                attempted: target,
            });
        }
        if self.scope != scope {
            return Err(AuthorisationRefused::WrongScope {
                authorised: self.scope,
                attempted: scope,
            });
        }
        Ok(self.target)
    }
}

/// Whether a write of `scope` may be performed against real hardware.
///
/// **This is the one place where "no writes to real hardware" stops being
/// absolute**, and it is deliberately an allowlist, not a `bool` on
/// the session and not the removal of a check.
///
/// Design spec §15's non-goal was written as *"no writes to real hardware in
/// phase 2 or phase 3, **and no write at all without a fresh, specific
/// go-ahead naming the device and the operation**"*. The second half is the
/// operative one: the prohibition exists because no operator had named a
/// device and an operation, not because writing is forbidden forever. On
/// 2026-09-26 an operator did exactly that — assign `1.1.67` to the device
/// held in Programming Mode — so the two scopes MP §2.3 needs to carry that
/// out are enabled, and every other scope stays refused.
///
/// Why these two and no others:
///
/// - [`WriteScope::IndividualAddressProgramming`] is the address write
///   itself. It is guarded twice over independently of this function: the
///   broadcast cannot be typed without a
///   [`ProgrammingModeWitness`](crate::commissioning::programming_mode::ProgrammingModeWitness),
///   so it cannot run unless exactly one device answered, and it changes one
///   device's address rather than its application.
/// - [`WriteScope::Restart`] is MP §2.3 step 4's `A_Restart`, which is part
///   of the same procedure. Leaving it out would mean completing the write
///   and then failing to finish the procedure the Standard specifies.
///
/// **Added 2026-09-28:** [`WriteScope::Download`]. The operator named
/// `1.1.67` and its application download (mask `0701h`, button 1 toggling
/// `2/0/53`), after the memory download had run end to end against the
/// simulator (RESEARCH §19.3). The scope opens the memory download
/// (`knx_net::commissioning::memory_download`) only. The property-path
/// `Downloader` refuses a hardware session by itself, because it has never
/// been run end to end against anything but the simulator's property model.
///
/// Still refused on hardware: [`WriteScope::Unload`], which leaves a device
/// without an application and has no procedure of its own that an operator
/// has asked for; and [`WriteScope::ProgrammingModeToggle`], whose `0060h`
/// octet meaning design spec §15 records as unsourced for a System B mask,
/// so this project does not write it blind.
///
/// Also refused: [`WriteScope::IndividualAddressReset`] (K13, MP §2.18). It
/// changes every device in programming mode at once, and no operator has
/// asked for it on the bus yet (KNOWN_LIMITATIONS §140).
pub fn hardware_write_is_authorised(scope: WriteScope) -> bool {
    match scope {
        WriteScope::IndividualAddressProgramming | WriteScope::Restart | WriteScope::Download => {
            true
        }
        WriteScope::Unload
        | WriteScope::ProgrammingModeToggle
        | WriteScope::IndividualAddressReset => false,
    }
}

/// Why a write was not authorised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorisationRefused {
    /// The target is on the project exclusion list.
    Excluded(ExcludedAddress),
    /// The operator's phrase did not match the one this target and scope
    /// require.
    ConfirmationMismatch {
        /// The phrase that was required. Safe to show: it contains no
        /// secret, only the target and the scope.
        expected: String,
    },
    /// The authorisation names a different device than the write does.
    WrongTarget {
        /// The device the authorisation covers.
        authorised: IndividualAddress,
        /// The device the write was about to go to.
        attempted: IndividualAddress,
    },
    /// The authorisation names a different class of write.
    WrongScope {
        /// The scope the authorisation covers.
        authorised: WriteScope,
        /// The scope the write belongs to.
        attempted: WriteScope,
    },
}

impl std::error::Error for AuthorisationRefused {}

impl fmt::Display for AuthorisationRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthorisationRefused::Excluded(excluded) => write!(f, "{excluded}"),
            AuthorisationRefused::ConfirmationMismatch { expected } => write!(
                f,
                "the operator's confirmation does not match; it must read exactly \
                 {expected:?}"
            ),
            AuthorisationRefused::WrongTarget {
                authorised,
                attempted,
            } => write!(
                f,
                "authorised to write to {authorised}, but the write is addressed to \
                 {attempted}"
            ),
            AuthorisationRefused::WrongScope {
                authorised,
                attempted,
            } => write!(
                f,
                "authorised for {authorised}, but the write is a {attempted} operation"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).unwrap()
    }

    /// §14 item 18's refusal, and §2.3: no value is produced by default.
    #[test]
    fn a_write_authorisation_has_no_default_and_no_blanket_constructor() {
        // Compile-time facts, asserted by construction: the only two
        // constructors both demand a concrete target, and one of them
        // additionally demands the operator's phrase. Nothing here can
        // produce an authorisation for "whatever device we happen to be
        // talking to".
        let simulator = WriteAuthorisation::for_simulator(addr(1, 1, 24), WriteScope::Download);
        assert!(simulator.is_ok());
        let hardware =
            WriteAuthorisation::for_hardware(addr(1, 1, 24), WriteScope::Download, "please");
        assert!(hardware.is_err(), "a vague confirmation must not authorise");
    }

    #[test]
    fn the_hardware_confirmation_phrase_names_the_target_and_the_scope() {
        let phrase = required_confirmation_phrase(addr(1, 1, 24), WriteScope::Download);
        assert_eq!(phrase, "I confirm download to 1.1.24");
        let auth = WriteAuthorisation::for_hardware(addr(1, 1, 24), WriteScope::Download, &phrase)
            .unwrap();
        assert_eq!(auth.kind(), TargetKind::Hardware);
        assert_eq!(auth.target().address(), addr(1, 1, 24));
    }

    #[test]
    fn a_phrase_for_one_device_does_not_authorise_another() {
        let phrase = required_confirmation_phrase(addr(1, 1, 24), WriteScope::Download);
        let err = WriteAuthorisation::for_hardware(addr(1, 1, 25), WriteScope::Download, &phrase)
            .unwrap_err();
        assert!(
            matches!(err, AuthorisationRefused::ConfirmationMismatch { .. }),
            "{err}"
        );
        assert!(err.to_string().contains("1.1.25"), "{err}");
    }

    #[test]
    fn a_phrase_for_one_scope_does_not_authorise_another() {
        let phrase = required_confirmation_phrase(addr(1, 1, 24), WriteScope::Download);
        let err = WriteAuthorisation::for_hardware(addr(1, 1, 24), WriteScope::Restart, &phrase)
            .unwrap_err();
        assert!(matches!(
            err,
            AuthorisationRefused::ConfirmationMismatch { .. }
        ));
    }

    /// §14 item 11: the excluded address cannot be authorised, not even
    /// for the simulator.
    #[test]
    fn the_alarm_panel_cannot_be_authorised_for_any_write_or_any_target_kind() {
        let panel = addr(1, 1, 220);
        for scope in [
            WriteScope::Download,
            WriteScope::Unload,
            WriteScope::IndividualAddressProgramming,
            WriteScope::Restart,
            WriteScope::ProgrammingModeToggle,
        ] {
            let simulated = WriteAuthorisation::for_simulator(panel, scope).unwrap_err();
            assert!(
                matches!(simulated, AuthorisationRefused::Excluded(_)),
                "{simulated}"
            );
            let phrase = required_confirmation_phrase(panel, scope);
            let hardware = WriteAuthorisation::for_hardware(panel, scope, &phrase).unwrap_err();
            assert!(
                matches!(hardware, AuthorisationRefused::Excluded(_)),
                "even a perfectly typed phrase must not reach it: {hardware}"
            );
            assert!(hardware.to_string().contains("1.1.220"), "{hardware}");
        }
    }

    #[test]
    fn an_authorisation_refuses_a_write_to_a_different_device() {
        let auth = WriteAuthorisation::for_simulator(addr(1, 1, 24), WriteScope::Download).unwrap();
        let err = auth
            .authorise(addr(1, 1, 25), WriteScope::Download)
            .unwrap_err();
        assert_eq!(
            err,
            AuthorisationRefused::WrongTarget {
                authorised: addr(1, 1, 24),
                attempted: addr(1, 1, 25),
            }
        );
    }

    #[test]
    fn an_authorisation_refuses_a_write_of_a_different_class() {
        let auth = WriteAuthorisation::for_simulator(addr(1, 1, 24), WriteScope::Download).unwrap();
        let err = auth
            .authorise(addr(1, 1, 24), WriteScope::Restart)
            .unwrap_err();
        assert_eq!(
            err,
            AuthorisationRefused::WrongScope {
                authorised: WriteScope::Download,
                attempted: WriteScope::Restart,
            }
        );
        assert!(err.to_string().contains("restart"), "{err}");
    }

    #[test]
    fn a_matching_authorisation_yields_the_contactable_target_and_nothing_else() {
        let auth = WriteAuthorisation::for_simulator(addr(1, 1, 24), WriteScope::Unload).unwrap();
        let target = auth.authorise(addr(1, 1, 24), WriteScope::Unload).unwrap();
        assert_eq!(target.address(), addr(1, 1, 24));
    }
}
