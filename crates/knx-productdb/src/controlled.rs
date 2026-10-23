//! Controlled source and policy identity for the future preview/apply binding.
//!
//! One authenticated bounded borrow of a sealed source plus the immutable
//! source and policy identities the binding compares. Construction-only: no
//! production route is wired here. See
//! `docs/contracts/catalogue-jobs/managed-binding.md` §5 and
//! `controlled-source-increment.md`.

use sha2::{Digest, Sha256};

use knx_core::limits::MAX_RETAINED_BYTES;

use crate::package::PackageLimits;

/// Largest admitted source name in UTF-8 bytes (P3 display-name grammar).
const MAX_SOURCE_NAME_BYTES: usize = 512;
/// The versioned policy namespace for controlled admission (policy `wp043-v1`).
const POLICY_VERSION: u32 = 1;

/// The two effective limit profiles a controlled job may select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledPolicy {
    Standard,
    Large,
}

impl ControlledPolicy {
    /// The exact existing STANDARD/LARGE limits; no new constants.
    pub(crate) const fn limits(self) -> PackageLimits {
        match self {
            ControlledPolicy::Standard => PackageLimits::STANDARD,
            ControlledPolicy::Large => PackageLimits::LARGE,
        }
    }
}

/// One authenticated bounded borrow of a sealed source: the exact name, the
/// actual bytes, their single digest and the effective limits. Never a client
/// path.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ControlledSource<'a> {
    pub(crate) name: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) sha256: [u8; 32],
    pub(crate) limits: PackageLimits,
}

impl<'a> ControlledSource<'a> {
    /// Authenticates a sealed source: validates the P3 name grammar, bounds the
    /// actual bytes and computes their digest once. An empty source is
    /// syntactically admitted (the installer's package refusal stays separate).
    pub(crate) fn admit(
        name: &'a str,
        bytes: &'a [u8],
        policy: ControlledPolicy,
    ) -> Result<Self, ControlledError> {
        check_source_name(name)?;
        let length = bytes.len() as u64;
        if length > MAX_RETAINED_BYTES {
            return Err(ControlledError::SourceTooLarge(length));
        }
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let sha256: [u8; 32] = hasher.finalize().into();
        Ok(Self {
            name,
            bytes,
            sha256,
            limits: policy.limits(),
        })
    }
}

/// The immutable source identity: exact name bytes, actual byte length and the
/// digest of the actual bytes. Never a claimed hash or length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceIdentity {
    pub(crate) name: Vec<u8>,
    pub(crate) length: u64,
    pub(crate) sha256: [u8; 32],
}

impl SourceIdentity {
    /// An infallible projection of an already-admitted source; no second hash.
    pub(crate) fn from_source(source: &ControlledSource<'_>) -> Self {
        Self {
            name: source.name.as_bytes().to_vec(),
            length: source.bytes.len() as u64,
            sha256: source.sha256,
        }
    }
}

/// The effective policy identity: the version and the actual admitted limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PolicyIdentity {
    pub(crate) version: u32,
    pub(crate) max_member_size: u64,
    pub(crate) max_expanded_size: u64,
}

impl PolicyIdentity {
    /// An infallible projection of an already-admitted policy.
    pub(crate) const fn from_policy(policy: ControlledPolicy) -> Self {
        let limits = policy.limits();
        Self {
            version: POLICY_VERSION,
            max_member_size: limits.max_member_size,
            max_expanded_size: limits.max_expanded_size,
        }
    }
}

/// Typed failures of controlled source admission; never a truncated prefix.
#[derive(Debug, PartialEq, Eq)]
pub enum ControlledError {
    /// The source name violated one exact P3 grammar rule.
    InvalidName(NameRule),
    /// The actual source bytes exceed the retained-byte ceiling.
    SourceTooLarge(u64),
}

/// The exact P3 name-grammar rule a source name violated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameRule {
    Empty,
    TooLong,
    DotOrDotDot,
    LeadingWhitespace,
    TrailingWhitespace,
    SeparatorColonOrNul,
    Control,
}

/// Mirrors the P3 display-name grammar exactly (the same rules as the server's
/// wire `Create` validation): 1..=512 UTF-8 bytes, basename only, no path
/// separators, drive colon, NUL, controls, `.`/`..`, or leading/trailing ASCII
/// whitespace. Never silently basenames or renames.
fn check_source_name(name: &str) -> Result<(), ControlledError> {
    if name.is_empty() {
        return Err(ControlledError::InvalidName(NameRule::Empty));
    }
    if name.len() > MAX_SOURCE_NAME_BYTES {
        return Err(ControlledError::InvalidName(NameRule::TooLong));
    }
    if name == "." || name == ".." {
        return Err(ControlledError::InvalidName(NameRule::DotOrDotDot));
    }
    if name.starts_with([' ', '\t', '\r', '\n']) {
        return Err(ControlledError::InvalidName(NameRule::LeadingWhitespace));
    }
    if name.ends_with([' ', '\t', '\r', '\n']) {
        return Err(ControlledError::InvalidName(NameRule::TrailingWhitespace));
    }
    if name.contains(['/', '\\', ':', '\0']) {
        return Err(ControlledError::InvalidName(NameRule::SeparatorColonOrNul));
    }
    if name.chars().any(char::is_control) {
        return Err(ControlledError::InvalidName(NameRule::Control));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn digest(bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hasher.finalize().into()
    }

    fn admit<'a>(name: &'a str, bytes: &'a [u8]) -> Result<ControlledSource<'a>, ControlledError> {
        ControlledSource::admit(name, bytes, ControlledPolicy::Standard)
    }

    #[test]
    fn a_valid_basename_is_admitted_with_the_actual_digest_and_limits() {
        let source = admit("manufacturer.knxprod", b"\x00\x01\x02").unwrap();
        assert_eq!(source.name, "manufacturer.knxprod");
        assert_eq!(source.bytes, b"\x00\x01\x02");
        assert_eq!(source.sha256, digest(b"\x00\x01\x02"));
        assert_eq!(source.limits, PackageLimits::STANDARD);
    }

    #[test]
    fn the_digest_is_stable_across_identical_bytes() {
        let first = admit("a.knxprod", b"same bytes").unwrap();
        let second = admit("a.knxprod", b"same bytes").unwrap();
        assert_eq!(first.sha256, second.sha256);
        assert_ne!(first.sha256, digest(b"different bytes"));
    }

    #[test]
    fn each_name_rule_is_refused_with_its_exact_rule() {
        use NameRule::*;
        let cases = [
            ("", Empty),
            ("a/knxprod", SeparatorColonOrNul),
            ("a\\b", SeparatorColonOrNul),
            ("a:b", SeparatorColonOrNul),
            ("a\0b", SeparatorColonOrNul),
            (".", DotOrDotDot),
            ("..", DotOrDotDot),
            (" leading", LeadingWhitespace),
            ("trailing ", TrailingWhitespace),
            ("tab\tinside", Control),
        ];
        for (name, rule) in cases {
            assert_eq!(
                admit(name, b"x").unwrap_err(),
                ControlledError::InvalidName(rule),
                "name {name:?} should violate {rule:?}"
            );
        }
    }

    #[test]
    fn the_name_byte_bound_is_exact() {
        // 512 bytes admitted, 513 refused.
        let at = "a".repeat(MAX_SOURCE_NAME_BYTES);
        assert!(admit(&at, b"x").is_ok());
        let over = "a".repeat(MAX_SOURCE_NAME_BYTES + 1);
        assert_eq!(
            admit(&over, b"x").unwrap_err(),
            ControlledError::InvalidName(NameRule::TooLong)
        );
    }

    #[test]
    fn an_empty_source_is_syntactically_admitted_but_not_the_empty_name() {
        // Empty bytes are admitted; the package refusal is a later concern.
        assert!(admit("empty.knxprod", b"").is_ok());
        assert_eq!(
            admit("", b"").unwrap_err(),
            ControlledError::InvalidName(NameRule::Empty)
        );
    }

    #[test]
    fn the_source_identity_projects_the_actual_name_length_and_digest() {
        let source = admit("package.knxprod", b"\x10\x20\x30\x40").unwrap();
        let identity = SourceIdentity::from_source(&source);
        assert_eq!(identity.name, b"package.knxprod");
        assert_eq!(identity.length, 4);
        assert_eq!(identity.sha256, digest(b"\x10\x20\x30\x40"));
    }

    #[test]
    fn both_policies_project_the_exact_existing_limits() {
        let standard = PolicyIdentity::from_policy(ControlledPolicy::Standard);
        assert_eq!(standard.version, 1);
        assert_eq!(standard.max_member_size, 1_000_000_000);
        assert_eq!(standard.max_expanded_size, 4 * 1024 * 1024 * 1024);
        let large = PolicyIdentity::from_policy(ControlledPolicy::Large);
        assert_eq!(large.max_member_size, 1_000_000_000);
        assert_eq!(large.max_expanded_size, 16 * 1024 * 1024 * 1024);
    }

    #[test]
    fn the_retained_byte_ceiling_is_pinned() {
        const {
            assert!(MAX_RETAINED_BYTES == 1_000_000_000);
            assert!(MAX_SOURCE_NAME_BYTES == 512);
        }
    }
}
