//! Access keys and access levels, in which a smaller level number means more power.
//!
//! Spec §10, from AL §3.5.7 and MP §3.5.1. The inversion is the whole
//! reason this module exists: an implementation that treats a larger level
//! as more access parses every frame correctly and is wrong about all of
//! them.

use std::cmp::Ordering;
use std::fmt;

/// The key that means "no key". `[D]` AL §3.5.7: a partner that does not
/// authorise gets *"the maximum access level protected with FFFFFFFFh"*,
/// so this value is the free level's key and never a secret.
pub const FREE_ACCESS_KEY: u32 = 0xFFFF_FFFF;

/// A four-octet access key, `unsigned32`.
///
/// `[D]` AL §3.5.7: *"the key that shall be four octets long and of data
/// type unsigned32"*. Not a string, not a passphrase, not derived from one.
///
/// The type deliberately has no `Display` and its `Debug` does not print
/// the value: a key in a log is a key on disk.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessKey(u32);

impl AccessKey {
    /// Wraps a key the operator supplied.
    ///
    /// `FFFFFFFFh` is rejected rather than accepted as a key: it is the
    /// free-access sentinel, and MP §3.5.1 guards its own procedure with
    /// `key != FFFFFFFFh`, so a caller that "has a key" of `FFFFFFFFh`
    /// actually has none and must take the no-key path.
    pub fn new(key: u32) -> Result<Self, NotAKey> {
        if key == FREE_ACCESS_KEY {
            return Err(NotAKey::FreeAccessSentinel);
        }
        Ok(Self(key))
    }

    /// The four octets, most significant first.
    pub fn octets(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    /// The raw value, for the one place that has to encode it.
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for AccessKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deliberately not the value.
        f.write_str("AccessKey(<redacted>)")
    }
}

/// Why a value is not usable as an access key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotAKey {
    /// The value was `FFFFFFFFh`, which is free access and not a key.
    FreeAccessSentinel,
}

impl std::error::Error for NotAKey {}

impl fmt::Display for NotAKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotAKey::FreeAccessSentinel => write!(
                f,
                "FFFFFFFFh is the free-access sentinel, not a key: a client with \
                 no key must skip authorisation instead of sending it"
            ),
        }
    }
}

/// An access level, one octet, where **0 is maximum access**.
///
/// `[D]` AL §3.5.7: *"Access levels (unsigned8) between 0 (maximum level,
/// i.e., maximum access rights) and 3 (minimum level …) or 0 … and 15
/// (minimum level …) are allowed."*
///
/// The ordering implemented here is **access ordering, not numeric
/// ordering**: `AccessLevel(0) > AccessLevel(3)`, because level 0 has more
/// rights. Comparing two of these therefore reads the way the question is
/// asked ("is this level better?") rather than the way the octets sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessLevel(u8);

impl AccessLevel {
    /// Maximum access rights.
    pub const MAXIMUM: AccessLevel = AccessLevel(0);
    /// The minimum level of a 4-level device.
    pub const MINIMUM_OF_FOUR: AccessLevel = AccessLevel(3);
    /// The minimum level of a 16-level device.
    pub const MINIMUM_OF_SIXTEEN: AccessLevel = AccessLevel(15);

    /// Wraps a level octet as the device reported it.
    ///
    /// Any octet is accepted, because the number of levels is a device
    /// property and not a client choice: `[D]` spec §10.1 — *"The client
    /// must not assume 4."* Out-of-range values are the device's business;
    /// a client that rejected them would refuse to talk to a conforming
    /// 16-level device.
    pub const fn from_octet(level: u8) -> Self {
        Self(level)
    }

    /// The octet, for encoding and for reporting.
    pub fn octet(self) -> u8 {
        self.0
    }

    /// Whether this level has strictly more rights than `other`.
    pub fn is_more_powerful_than(self, other: AccessLevel) -> bool {
        self.0 < other.0
    }
}

impl PartialOrd for AccessLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccessLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed on purpose: fewer is more.
        other.0.cmp(&self.0)
    }
}

impl fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "level {}", self.0)
    }
}

/// What a connection's authorisation state actually is, as opposed to what
/// the client hoped.
///
/// `[D]` AL §3.5.7: a level *"shall be valid until the connection is
/// released"*, so this value dies with the Transport Layer connection and
/// is never cached across one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Authorisation {
    /// No key was supplied, so MP §3.5.1 was skipped entirely and the
    /// device granted whatever level `FFFFFFFFh` protects. The level is
    /// unknown because nobody asked, which is different from level 0.
    #[default]
    FreeLevelUnknown,
    /// A key was sent and the device answered with a level. Whether that
    /// level is any good is a separate question — see
    /// [`Authorisation::is_suspicious_for`].
    Granted {
        /// The level the device returned.
        level: AccessLevel,
    },
}

impl Authorisation {
    /// The level, if one was ever read.
    pub fn level(self) -> Option<AccessLevel> {
        match self {
            Authorisation::FreeLevelUnknown => None,
            Authorisation::Granted { level } => Some(level),
        }
    }

    /// Whether the granted level is the minimum of a device with
    /// `level_count` levels — the symptom `[D]` AL §3.5.7 describes for a
    /// wrong key: *"shall select the minimal access level (this is level 3
    /// or level 15)"*.
    ///
    /// It is a suspicion and not a diagnosis, because a correct key may
    /// legitimately map to the minimum level. The Standard provides no way
    /// to tell those apart, so neither does this.
    pub fn is_suspicious_for(self, level_count: LevelCount) -> bool {
        match self {
            Authorisation::FreeLevelUnknown => false,
            Authorisation::Granted { level } => level.octet() == level_count.minimum().octet(),
        }
    }
}

/// How many access levels a device has, which is a device property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LevelCount {
    /// Four levels, 0–3.
    Four,
    /// Sixteen levels, 0–15.
    Sixteen,
    /// Not known: the Profile for this device's mask was not consulted, or
    /// says `O`. `[D, corpus]` PROF §4.2's row contains `4`, `16` and `O`,
    /// and per-column attribution is not citable, so "unknown" is a real
    /// and common answer rather than a placeholder.
    #[default]
    Unknown,
}

impl LevelCount {
    /// The minimum (weakest) level for this count. An unknown count is
    /// treated as sixteen levels, because assuming four would call level
    /// 15 out of range on a device where it is the documented minimum.
    pub fn minimum(self) -> AccessLevel {
        match self {
            LevelCount::Four => AccessLevel::MINIMUM_OF_FOUR,
            LevelCount::Sixteen | LevelCount::Unknown => AccessLevel::MINIMUM_OF_SIXTEEN,
        }
    }
}

/// Whether MP §3.5.1 should run at all, and with what.
///
/// `[D]` spec §10.9 items 2 and 3: no key means skip the procedure and
/// record "free level, unknown value"; and never send a key the user did
/// not supply. Encoding that as a two-variant type means there is no third
/// state in which a guessed key could live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthorisationPlan {
    /// Do not authorise. The device will grant the free level.
    #[default]
    Skip,
    /// Authorise with this key, which came from the operator.
    WithKey(AccessKey),
}

impl AuthorisationPlan {
    /// Builds a plan from an optional operator-supplied key.
    pub fn from_operator_key(key: Option<AccessKey>) -> Self {
        match key {
            Some(key) => AuthorisationPlan::WithKey(key),
            None => AuthorisationPlan::Skip,
        }
    }

    /// What the connection's state is before any exchange happens.
    pub fn initial_authorisation(self) -> Authorisation {
        Authorisation::FreeLevelUnknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §14 item 15: lower is more powerful, and the ordering says so.
    #[test]
    fn level_zero_outranks_every_other_level() {
        for other in 1..=255u8 {
            let other = AccessLevel::from_octet(other);
            assert!(AccessLevel::MAXIMUM.is_more_powerful_than(other));
            assert!(
                AccessLevel::MAXIMUM > other,
                "{other} should rank below level 0"
            );
        }
    }

    #[test]
    fn the_ordering_is_access_ordering_and_not_numeric_ordering() {
        let strong = AccessLevel::from_octet(0);
        let weak = AccessLevel::from_octet(15);
        assert!(strong > weak);
        assert!(weak < strong);
        assert_eq!(strong.cmp(&weak), Ordering::Greater);
        // And the octets sort the other way round, which is the trap.
        assert!(strong.octet() < weak.octet());
    }

    #[test]
    fn the_two_documented_minima_are_three_and_fifteen() {
        assert_eq!(LevelCount::Four.minimum(), AccessLevel::from_octet(3));
        assert_eq!(LevelCount::Sixteen.minimum(), AccessLevel::from_octet(15));
    }

    #[test]
    fn an_unknown_level_count_assumes_sixteen_rather_than_four() {
        assert_eq!(LevelCount::Unknown.minimum(), AccessLevel::from_octet(15));
        assert_eq!(LevelCount::default(), LevelCount::Unknown);
    }

    #[test]
    fn the_free_access_sentinel_is_not_accepted_as_a_key() {
        let err = AccessKey::new(FREE_ACCESS_KEY).unwrap_err();
        assert_eq!(err, NotAKey::FreeAccessSentinel);
        assert!(err.to_string().contains("skip authorisation"), "{err}");
    }

    #[test]
    fn a_key_is_four_octets_most_significant_first() {
        let key = AccessKey::new(0x1234_5678).unwrap();
        assert_eq!(key.octets(), [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn a_key_never_prints_itself() {
        let key = AccessKey::new(0xDEAD_BEEF).unwrap();
        let rendered = format!("{key:?}");
        assert!(!rendered.contains("dead"), "{rendered}");
        assert!(!rendered.contains("DEAD"), "{rendered}");
        assert!(!rendered.contains("3735928559"), "{rendered}");
    }

    /// §10.9 items 2 and 3: no key means skip, and skipping is not
    /// "authorised at level 0".
    #[test]
    fn no_key_means_skip_and_an_unknown_free_level() {
        let plan = AuthorisationPlan::from_operator_key(None);
        assert_eq!(plan, AuthorisationPlan::Skip);
        let state = plan.initial_authorisation();
        assert_eq!(state, Authorisation::FreeLevelUnknown);
        assert_eq!(state.level(), None, "nobody asked, so nobody knows");
    }

    #[test]
    fn the_default_plan_sends_no_key_at_all() {
        assert_eq!(AuthorisationPlan::default(), AuthorisationPlan::Skip);
        assert_eq!(Authorisation::default(), Authorisation::FreeLevelUnknown);
    }

    #[test]
    fn an_operator_key_produces_a_plan_that_carries_exactly_it() {
        let key = AccessKey::new(7).unwrap();
        assert_eq!(
            AuthorisationPlan::from_operator_key(Some(key)),
            AuthorisationPlan::WithKey(key)
        );
    }

    /// §10.2: the minimum level is the symptom of a wrong key, and it is
    /// reported as a suspicion because the wire cannot prove it.
    #[test]
    fn the_minimum_level_is_suspicious_but_not_a_verdict() {
        let granted = Authorisation::Granted {
            level: AccessLevel::from_octet(15),
        };
        assert!(granted.is_suspicious_for(LevelCount::Sixteen));
        assert!(!granted.is_suspicious_for(LevelCount::Four));
        let good = Authorisation::Granted {
            level: AccessLevel::from_octet(0),
        };
        assert!(!good.is_suspicious_for(LevelCount::Sixteen));
    }

    #[test]
    fn not_authorising_is_never_flagged_as_a_wrong_key() {
        assert!(!Authorisation::FreeLevelUnknown.is_suspicious_for(LevelCount::Four));
        assert!(!Authorisation::FreeLevelUnknown.is_suspicious_for(LevelCount::Sixteen));
    }
}
