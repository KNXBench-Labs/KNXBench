//! Strict wire primitives for the catalogue job protocol (decimal U64, hex identities, capability).

use std::fmt;
use std::str::FromStr;

/// A canonical ASCII-decimal `u64` following the wire grammar `0|[1-9][0-9]{0,19}`.
///
/// The grammar check lives at the `FromStr` (wire) boundary; the field is
/// public for checked internal arithmetic, not to bypass wire validation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct U64(pub u64);

/// A strict-decimal parse failure: wrong grammar, length or overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseU64;

/// A strict lowercase-hex parse failure: wrong length or a non-hex character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseHex;

/// 16 CSPRNG bytes rendered as 32 lowercase hex; a non-secret job identity.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId([u8; 16]);

/// 16 CSPRNG bytes rendered as 32 lowercase hex; the job-protocol server incarnation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Incarnation([u8; 16]);

/// 32 bytes rendered as 64 lowercase hex; a SHA-256 digest.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256([u8; 32]);

/// 32 CSPRNG bytes rendered as 64 lowercase hex; the unlogged selection capability.
#[derive(Clone, PartialEq, Eq)]
pub struct Capability([u8; 32]);

impl U64 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);
    pub const MAX: Self = Self(u64::MAX);

    /// The raw value; the wire form is the canonical decimal string.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Checked addition, matching the checked-arithmetic policy of P2.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    /// Checked subtraction; never saturates.
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    /// Checked conversion from `usize`; `None` when it exceeds `u64`.
    pub fn try_from_usize(value: usize) -> Option<Self> {
        u64::try_from(value).ok().map(Self)
    }
}

impl FromStr for U64 {
    type Err = ParseU64;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_u64(s).map(Self).ok_or(ParseU64)
    }
}

impl fmt::Display for U64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for U64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "U64({})", self.0)
    }
}

/// Parses the strict decimal grammar; `None` on any violation or overflow.
fn parse_u64(s: &str) -> Option<u64> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes.len() > 20 {
        return None;
    }
    if bytes[0] == b'0' {
        return if bytes.len() == 1 { Some(0) } else { None };
    }
    if !(b'1'..=b'9').contains(&bytes[0]) {
        return None;
    }
    if !bytes[1..].iter().all(u8::is_ascii_digit) {
        return None;
    }
    s.parse::<u64>().ok()
}

impl JobId {
    pub fn try_mint() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    pub fn mint() -> Self {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).expect("CSPRNG unavailable for job identity");
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Incarnation {
    pub fn mint() -> Self {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).expect("CSPRNG unavailable for server incarnation");
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Sha256 {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Capability {
    pub fn mint() -> Self {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).expect("CSPRNG unavailable for job capability");
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Constant-time comparison; the capability is a secret and is never logged.
    pub fn verify(&self, other: &Capability) -> bool {
        constant_time_eq(&self.0, &other.0)
    }
}

impl FromStr for Capability {
    type Err = ParseHex;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        decode_hex::<32>(s).map(Self).ok_or(ParseHex)
    }
}

impl fmt::Debug for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Capability(redacted)")
    }
}

macro_rules! hex_identity {
    ($t:ty, $n:literal) => {
        impl FromStr for $t {
            type Err = ParseHex;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                decode_hex::<$n>(s).map(Self).ok_or(ParseHex)
            }
        }

        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&encode_hex(&self.0))
            }
        }

        impl fmt::Debug for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($t), encode_hex(&self.0))
            }
        }
    };
}

hex_identity!(JobId, 16);
hex_identity!(Incarnation, 16);
hex_identity!(Sha256, 32);

/// Constant-time byte equality: XOR accumulator, no short-circuit on bytes.
///
/// The length is compared first; callers compare fixed-length values (the
/// capability is always 32 bytes), so no secret length is observable.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc = 0_u8;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

/// Decodes exactly `N` bytes of strict lowercase hex; `None` on length/character errors.
fn decode_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let bytes = s.as_bytes();
    if bytes.len() != N * 2 {
        return None;
    }
    let mut out = [0_u8; N];
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = hex_value(bytes[2 * i])?;
        let lo = hex_value(bytes[2 * i + 1])?;
        *slot = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{constant_time_eq, Capability, Incarnation, JobId, Sha256, U64};

    #[test]
    fn u64_accepts_zero_and_maximum() {
        assert_eq!("0".parse::<U64>().unwrap(), U64::ZERO);
        assert_eq!("1".parse::<U64>().unwrap().get(), 1);
        assert_eq!("18446744073709551615".parse::<U64>().unwrap(), U64::MAX);
    }

    #[test]
    fn u64_rejects_malformed_decimal() {
        for bad in [
            "",
            " ",
            " 1",
            "1 ",
            "+1",
            "-1",
            "01",
            "00",
            "000",
            "1.0",
            "1e3",
            "1_000",
            "18446744073709551616",
            "99999999999999999999",
            "１２３",
            "123456789012345678901",
        ] {
            assert!(bad.parse::<U64>().is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn u64_roundtrips_canonical_decimal() {
        for value in [0_u64, 1, 42, 100, u64::MAX, u64::MAX - 1] {
            let wire = U64(value).to_string();
            assert_eq!(wire, value.to_string());
            assert_eq!(wire.parse::<U64>().unwrap(), U64(value));
        }
    }

    #[test]
    fn u64_checked_add_never_wraps() {
        assert_eq!(U64::MAX.checked_add(U64::ONE), None);
        assert_eq!(U64::MAX.checked_add(U64::ZERO), Some(U64::MAX));
        assert_eq!(U64::ONE.checked_add(U64::ONE), Some(U64(2)));
        assert_eq!(U64(10).checked_sub(U64(11)), None);
        assert_eq!(U64(10).checked_sub(U64(10)), Some(U64::ZERO));
    }

    #[test]
    fn hex_ids_accept_only_lowercase_hex() {
        assert!("00000000000000000000000000000000".parse::<JobId>().is_ok());
        assert!("ffffffffffffffffffffffffffffffff".parse::<JobId>().is_ok());
        assert!("0123456789abcdef0123456789abcdef".parse::<JobId>().is_ok());
        assert!("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".parse::<JobId>().is_err());
        assert!("0000000000000000000000000000000".parse::<JobId>().is_err());
        assert!("000000000000000000000000000000000"
            .parse::<JobId>()
            .is_err());
        assert!("gggggggggggggggggggggggggggggggg".parse::<JobId>().is_err());
        assert!("".parse::<JobId>().is_err());
    }

    #[test]
    fn hex_ids_roundtrip_bytes_through_hex() {
        let bytes = [0xabu8; 16];
        let job = JobId::from_bytes(bytes);
        let wire = job.to_string();
        assert_eq!(wire.len(), 32);
        assert_eq!(wire.parse::<JobId>().unwrap(), job);
        assert_eq!(
            wire.parse::<Incarnation>().unwrap(),
            Incarnation::from_bytes(bytes)
        );
    }

    #[test]
    fn sha256_roundtrips_64_hex() {
        let digest = Sha256::from_bytes([0x5a_u8; 32]);
        let wire = digest.to_string();
        assert_eq!(wire.len(), 64);
        assert_eq!(wire.parse::<Sha256>().unwrap(), digest);
        assert!("5a".repeat(31).parse::<Sha256>().is_err());
        assert!("5a".repeat(33).parse::<Sha256>().is_err());
    }

    #[test]
    fn capability_debug_never_reveals_the_secret() {
        let capability = Capability::from_bytes([0xde_u8; 32]);
        let rendered = format!("{capability:?}");
        assert_eq!(rendered, "Capability(redacted)");
        assert!(!rendered.contains("dedede"));
    }

    #[test]
    fn capability_verify_is_constant_time_and_exact() {
        let a = Capability::from_bytes([0x11_u8; 32]);
        let b = Capability::from_bytes([0x11_u8; 32]);
        let c = Capability::from_bytes([0x22_u8; 32]);
        assert!(a.verify(&b));
        assert!(!a.verify(&c));
        assert!(a.verify(&Capability::from_bytes([0x11_u8; 32])));
    }

    #[test]
    fn constant_time_eq_rejects_length_and_mismatch() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn minting_is_fresh_and_distinct() {
        let a = JobId::mint();
        let b = JobId::mint();
        let c = Incarnation::mint();
        let d = Capability::mint();
        assert_ne!(a, b);
        assert_ne!(a.to_string(), b.to_string());
        assert_ne!(c.to_string(), Incarnation::mint().to_string());
        assert_ne!(d, Capability::mint());
    }
}
