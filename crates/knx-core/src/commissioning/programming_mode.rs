//! Counting the devices in programming mode, and the one-octet toggle that switches it remotely.
//!
//! Spec §4.2's counting rule (MP §2.2: wait out the whole time-out, count
//! distinct sources, ignore Layer-2 repetitions) and §4.4's read-modify-
//! write of `0060h` (RES §4.26.3.1: invert bit 0 and bit 7, preserve bits
//! 1–6, write nothing when the mode already matches).

use std::collections::BTreeSet;
use std::fmt;
use std::time::Duration;

use crate::address::IndividualAddress;

/// `[D]` MP §2.2 `NM_IndividualAddress_Read`: time-out 3 s, and *"The
/// Management Client shall always wait until the time-out has elapsed."*
pub const INDIVIDUAL_ADDRESS_READ_TIMEOUT: Duration = Duration::from_secs(3);

/// `[D]` spec §4.3, RES §4.26: a device may *"autonomously and
/// automatically disable its Programming Mode"* four minutes after it was
/// enabled. Optional, so this is a lower bound on how long an observation
/// stays true and on some devices no bound at all.
pub const PROGRAMMING_MODE_AUTO_OFF: Duration = Duration::from_secs(4 * 60);

/// The responders to a broadcast `A_IndividualAddress_Read`, collected over
/// one full time-out.
///
/// A set of source addresses and not a frame count: `[D]` MP §2.2 — *"The
/// Management Client shall not evaluate Layer-2 repetitions."* A repeated
/// frame is not a second device, and the responses carry no data anyway
/// (§4.1: the address arrives as the frame's source address).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProgrammingModeResponders {
    sources: BTreeSet<IndividualAddress>,
    frames: usize,
    waited_full_timeout: bool,
}

impl ProgrammingModeResponders {
    /// A fresh, empty observation. Not yet complete: nothing may be
    /// concluded from it until [`ProgrammingModeResponders::complete`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one `A_IndividualAddress_Response` frame from `source`.
    ///
    /// Called for every frame, repetitions included, because the count of
    /// frames is diagnostic information worth keeping — it is only the
    /// *device* count that must ignore them.
    pub fn observe(&mut self, source: IndividualAddress) {
        self.frames += 1;
        self.sources.insert(source);
    }

    /// Marks the full time-out as elapsed. The only way to obtain a
    /// [`ProgrammingModeWitness`], because a client that returned early on
    /// the first response would be counting one device where there might be
    /// two.
    pub fn complete(self) -> Self {
        Self {
            waited_full_timeout: true,
            ..self
        }
    }

    /// How many distinct devices answered.
    pub fn device_count(&self) -> usize {
        self.sources.len()
    }

    /// How many frames arrived, repetitions included. Always at least the
    /// device count.
    pub fn frame_count(&self) -> usize {
        self.frames
    }

    /// The devices that answered, in topological order.
    pub fn devices(&self) -> impl Iterator<Item = IndividualAddress> + '_ {
        self.sources.iter().copied()
    }

    /// Turns the observation into a witness that exactly one device is in
    /// programming mode, or explains why it cannot.
    ///
    /// `[D]` MP §2.3 exception handling "to 2." enumerates the three cases:
    /// zero responders means no device is in programming mode, more than one
    /// means more than one button is pressed and the procedure must not
    /// continue.
    pub fn single_responder(&self) -> Result<ProgrammingModeWitness, ProgrammingModeCountError> {
        if !self.waited_full_timeout {
            return Err(ProgrammingModeCountError::TimeoutNotElapsed);
        }
        let mut devices = self.sources.iter().copied();
        match (devices.next(), devices.next()) {
            (None, _) => Err(ProgrammingModeCountError::NoDeviceInProgrammingMode),
            (Some(only), None) => Ok(ProgrammingModeWitness { device: only }),
            (Some(_), Some(_)) => Err(ProgrammingModeCountError::SeveralDevices {
                devices: self.sources.iter().copied().collect(),
            }),
        }
    }
}

/// Proof that exactly one device answered a full-length programming-mode
/// read.
///
/// Unconstructible except through
/// [`ProgrammingModeResponders::single_responder`], so the individual-address
/// write of §4.2 step 3 cannot be reached with zero responders or with two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgrammingModeWitness {
    device: IndividualAddress,
}

impl ProgrammingModeWitness {
    /// The address the single responder currently has, which step 3
    /// compares against the new one.
    pub fn current_address(self) -> IndividualAddress {
        self.device
    }
}

/// Why exactly one responder could not be established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgrammingModeCountError {
    /// The observation was read before the 3 s time-out elapsed.
    TimeoutNotElapsed,
    /// Nobody answered: no device is in programming mode. Silence is not
    /// evidence of anything else — §4.1 — because the Standard specifies no
    /// negative response.
    NoDeviceInProgrammingMode,
    /// Several devices answered, so several buttons are pressed.
    SeveralDevices {
        /// Which ones, so the operator can be told where to look.
        devices: Vec<IndividualAddress>,
    },
}

impl std::error::Error for ProgrammingModeCountError {}

impl fmt::Display for ProgrammingModeCountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgrammingModeCountError::TimeoutNotElapsed => write!(
                f,
                "the 3 s time-out has not elapsed, so the responder count is not \
                 final yet"
            ),
            ProgrammingModeCountError::NoDeviceInProgrammingMode => {
                write!(f, "no device answered, so no device is in programming mode")
            }
            ProgrammingModeCountError::SeveralDevices { devices } => {
                write!(f, "{} devices are in programming mode:", devices.len())?;
                for device in devices {
                    write!(f, " {device}")?;
                }
                Ok(())
            }
        }
    }
}

/// The memory address of `curr_prog_mode` on the profiles RES §4.26.3
/// covers.
///
/// On a System B device the meaning of this octet is **not established**
/// (§4.4): PROF §4.4.1.1 profiles System B onto Realisation Type 1 for the
/// mandatory connection-oriented path, so the octet may or may not be
/// `curr_prog_mode`. A read of it is reported as a raw octet whose meaning
/// is unknown for that mask, never as programming-mode state.
pub const CURR_PROG_MODE_ADDRESS: u16 = 0x0060;

/// The two bits RES §4.26.3.1's toggle inverts: bit 0 (`prog_mode`) and
/// bit 7 (`p_parity`).
pub const PROG_MODE_TOGGLE_MASK: u8 = 0b1000_0001;

/// What a read-modify-write of `curr_prog_mode` should actually do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgModeWrite {
    /// The mode already matches, so nothing is written.
    ///
    /// `[D]` RES §4.26.3.4.2/§4.26.3.4.3 wrap the procedure in
    /// `if prog_mode = 0` / `if prog_mode = 1`. A no-op toggle would invert
    /// the parity against an unchanged `prog_mode`, producing exactly the
    /// invalid octet §4.26.3.3 warns about — and footnote 96: *"Typically
    /// the system is restarted if p_parity is invalid."*
    AlreadyInRequestedMode,
    /// Write this octet back to [`CURR_PROG_MODE_ADDRESS`].
    Write(u8),
}

/// Derives the octet to write, from the octet that was read.
///
/// `new = old XOR 0b1000_0001` — invert `prog_mode` and invert `p_parity`,
/// carrying bits 1–6 through untouched. `[D]` RES §4.26.3.1: *"Even if the
/// whole octet is read and written back they shall not be changed when
/// activating and deactivating the Programming Mode."* Masking them off
/// would be a corruption, not a simplification.
///
/// Nothing here sends anything: the write half lives behind the mutation
/// API and, per §4.4, is exercised against the simulator only.
pub fn prog_mode_write(old: u8, enable: bool) -> ProgModeWrite {
    let current = old & 0x01 == 0x01;
    if current == enable {
        return ProgModeWrite::AlreadyInRequestedMode;
    }
    ProgModeWrite::Write(old ^ PROG_MODE_TOGGLE_MASK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).unwrap()
    }

    /// §14 item 7: the count is of distinct sources, and repetitions do not
    /// inflate it.
    #[test]
    fn layer_two_repetitions_do_not_become_a_second_device() {
        let mut responders = ProgrammingModeResponders::new();
        for _ in 0..4 {
            responders.observe(addr(1, 1, 24));
        }
        let responders = responders.complete();
        assert_eq!(responders.frame_count(), 4);
        assert_eq!(responders.device_count(), 1);
        let witness = responders.single_responder().unwrap();
        assert_eq!(witness.current_address(), addr(1, 1, 24));
    }

    #[test]
    fn zero_responders_yields_no_witness_and_says_nobody_answered() {
        let responders = ProgrammingModeResponders::new().complete();
        let err = responders.single_responder().unwrap_err();
        assert_eq!(err, ProgrammingModeCountError::NoDeviceInProgrammingMode);
        assert!(err.to_string().contains("no device"), "{err}");
    }

    #[test]
    fn two_responders_yield_no_witness_and_name_both() {
        let mut responders = ProgrammingModeResponders::new();
        responders.observe(addr(1, 1, 24));
        responders.observe(addr(1, 1, 25));
        responders.observe(addr(1, 1, 24));
        let responders = responders.complete();
        assert_eq!(responders.device_count(), 2);
        let err = responders.single_responder().unwrap_err();
        assert_eq!(
            err,
            ProgrammingModeCountError::SeveralDevices {
                devices: vec![addr(1, 1, 24), addr(1, 1, 25)]
            }
        );
        let text = err.to_string();
        assert!(text.contains("1.1.24") && text.contains("1.1.25"), "{text}");
    }

    /// The client may not conclude anything before the time-out elapses,
    /// because the point of the wait is to count.
    #[test]
    fn an_early_read_is_refused_even_when_exactly_one_device_has_answered() {
        let mut responders = ProgrammingModeResponders::new();
        responders.observe(addr(1, 1, 24));
        let err = responders.single_responder().unwrap_err();
        assert_eq!(err, ProgrammingModeCountError::TimeoutNotElapsed);
    }

    #[test]
    fn the_timeout_is_the_three_seconds_the_clause_states() {
        assert_eq!(INDIVIDUAL_ADDRESS_READ_TIMEOUT, Duration::from_secs(3));
        assert_eq!(PROGRAMMING_MODE_AUTO_OFF, Duration::from_secs(240));
    }

    /// §14 item 8: the toggle inverts bits 0 and 7 and preserves 1–6.
    #[test]
    fn the_toggle_inverts_bit_zero_and_bit_seven_only() {
        // 0b0101_1010: prog_mode off, parity clear, shared bits set.
        assert_eq!(
            prog_mode_write(0b0101_1010, true),
            ProgModeWrite::Write(0b1101_1011)
        );
        // Turning it off again from the result returns the original octet.
        assert_eq!(
            prog_mode_write(0b1101_1011, false),
            ProgModeWrite::Write(0b0101_1010)
        );
    }

    #[test]
    fn the_shared_bits_survive_every_possible_starting_octet() {
        for old in 0..=255u8 {
            let enable = old & 0x01 == 0;
            match prog_mode_write(old, enable) {
                ProgModeWrite::Write(new) => {
                    assert_eq!(
                        new & 0b0111_1110,
                        old & 0b0111_1110,
                        "bits 1-6 must be carried through unchanged, {old:#010b}"
                    );
                    assert_ne!(new & 0x01, old & 0x01, "bit 0 must flip");
                    assert_ne!(new & 0x80, old & 0x80, "bit 7 must flip");
                }
                ProgModeWrite::AlreadyInRequestedMode => {
                    unreachable!("the mode was deliberately chosen to differ")
                }
            }
        }
    }

    #[test]
    fn no_write_happens_when_the_mode_already_matches() {
        for old in 0..=255u8 {
            let already_on = old & 0x01 == 0x01;
            assert_eq!(
                prog_mode_write(old, already_on),
                ProgModeWrite::AlreadyInRequestedMode,
                "{old:#010b} already has prog_mode = {}",
                u8::from(already_on)
            );
        }
    }

    #[test]
    fn the_toggle_mask_is_the_two_bits_and_not_the_whole_octet() {
        assert_eq!(PROG_MODE_TOGGLE_MASK, 0x81);
        assert_eq!(CURR_PROG_MODE_ADDRESS, 0x0060);
    }
}
