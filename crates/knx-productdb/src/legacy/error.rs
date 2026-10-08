//! Typed refusals of the legacy EX-IM path; none of them ever carries a password.

use std::fmt;

/// Why a legacy file could not be opened or parsed.
///
/// Every variant names the format. Wrong password, damaged data and an
/// unsupported layout are distinct, except where ZipCrypto cannot tell the
/// first two apart ([`LegacyError::WrongPasswordOrCorrupt`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyError {
    /// Not a ZIP with exactly one `ets.vd_`/`ets2.vd_`/`ets.pr_` member.
    NotLegacyContainer { reason: String },
    /// The single legacy member is there, but the archive around it does not
    /// have the observed layout (local header at offset 0, nothing between
    /// the member and the central directory).
    InvalidContainer { reason: String },
    /// Strong encryption or AES, which no observed legacy file uses.
    UnsupportedEncryption { reason: &'static str },
    /// A compression method other than stored (0) or deflate (8).
    UnsupportedCompression { method: u16 },
    /// The member is encrypted and no (or an empty) password was given.
    PasswordRequired,
    /// The encryption header's check byte rejected the password.
    WrongPassword,
    /// The check byte accepted the password, but the data did not inflate
    /// or its CRC-32 did not match. ZipCrypto's one-byte check accepts about
    /// one wrong password in 128, so this cannot be told apart from damage.
    WrongPasswordOrCorrupt,
    /// An unencrypted member that does not inflate to its declared size and
    /// CRC-32.
    Corrupt { reason: String },
    /// A resource bound was exceeded before or during parsing.
    SizeLimit { what: &'static str, limit: u64 },
    /// The EX-IM text does not follow the observed grammar. `line` is
    /// 1-based.
    Syntax { line: usize, reason: String },
    /// The file parsed, but cannot be published (not a product database,
    /// or nothing to identify it by).
    Mapping { reason: String },
}

impl fmt::Display for LegacyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLegacyContainer { reason } => {
                write!(
                    f,
                    "not a legacy EX-IM container (.vd3–.vd5/.pr3–.pr5): {reason}"
                )
            }
            Self::InvalidContainer { reason } => {
                write!(f, "unexpected legacy EX-IM container layout: {reason}")
            }
            Self::UnsupportedEncryption { reason } => {
                write!(f, "unsupported encryption in legacy EX-IM file: {reason}")
            }
            Self::UnsupportedCompression { method } => write!(
                f,
                "unsupported compression method {method} in legacy EX-IM file (only stored and \
                 deflate occur)"
            ),
            Self::PasswordRequired => f.write_str(
                "this legacy ETS3 file is password-protected; enter the password for this file",
            ),
            Self::WrongPassword => f.write_str("wrong password for this legacy ETS3 file"),
            Self::WrongPasswordOrCorrupt => f.write_str(
                "wrong password, or the encrypted legacy ETS3 file is damaged (ZipCrypto cannot \
                 tell the two apart)",
            ),
            Self::Corrupt { reason } => write!(f, "damaged legacy ETS3 file: {reason}"),
            Self::SizeLimit { what, limit } => {
                write!(f, "legacy EX-IM {what} exceeds the limit of {limit}")
            }
            Self::Syntax { line, reason } => {
                write!(f, "legacy EX-IM text, line {line}: {reason}")
            }
            Self::Mapping { reason } => write!(f, "legacy product database: {reason}"),
        }
    }
}

impl std::error::Error for LegacyError {}
