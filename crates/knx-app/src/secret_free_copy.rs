//! Secret-free copies of a native project and of an ETS import file for the diagnostic bundle.
//!
//! DIAG-03 owner decision 3: BCU keys, KNX Secure keys, tool keys and
//! tunnel/login passwords are always redacted, even with consent. The bundle
//! may carry the whole `.knxdb` and, for import problems, the original
//! `.knxproj` (decision 4, O1). Both keep secret material only as
//! uninterpreted source (KNOWN_LIMITATIONS §8), so this module produces
//! copies in which every such value is gone, never the original file.
//!
//! What counts as secret is a fixed list of exact, case-sensitive XML names
//! ([`SECRET_NAMES`]): an attribute with such a name is blanked, and so is the
//! text of an element with such a name. `BusAccess/@Parameter` connection
//! strings lose the values of password-, key- and code-like parts. Anything
//! that cannot be inspected (an encrypted or unreadable archive, malformed
//! XML, UTF-16 text, a carried product file that names a secret) is withheld
//! and listed, never copied unredacted. Every redaction is reported by
//! location and kind only; no value, digest or length of a secret is ever
//! written into a [`Redaction`], an error or `Debug` output.
//!
//! As a second net, every removed value of at least
//! [`MIN_RESIDUAL_BYTES`] bytes is searched for in what is about to be
//! copied: a member or retained row that still contains one is withheld,
//! and a native copy whose final bytes still contain one is refused.

mod archive;
mod native;
#[cfg(test)]
mod tests;
mod xml;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use serde::Serialize;

/// Exact XML names whose values are secret material. The first eleven are
/// the names `knx-etsproj` keeps out of every known-attribute table
/// (`no_known_table_lists_secure_key_material`); `BCUKey` is the device
/// access key (`access_key`); the `Loaded…` names are the last-downloaded
/// copies Schema 23 §1.2.5.3 declares next to them
/// (docs/research/secure-key-provenance.md). The two `Loaded…Hash` names are
/// inferred from that clause's "corresponding hash/loaded-hash attributes",
/// not seen in a sample; listing a name that never occurs costs nothing.
pub const SECRET_NAMES: &[&str] = &[
    "Key",
    "ToolKey",
    "FDSK",
    "IPRoutingBackboneKey",
    "Password",
    "PasswordHash",
    "ProjectPassword",
    "DeviceAuthenticationCode",
    "DeviceAuthenticationCodeHash",
    "DeviceManagementPassword",
    "DeviceManagementPasswordHash",
    "BCUKey",
    "LoadedToolKey",
    "LoadedIPRoutingBackboneKey",
    "LoadedDeviceAuthenticationCodeHash",
    "LoadedDeviceManagementPasswordHash",
];

/// `Installation/@BCUKey`'s schema default, the free-access sentinel: not a
/// key (AL §3.5.7, see `access_key`), so it is kept and not reported.
const FREE_ACCESS_KEY: &[u8] = b"4294967295";

/// Removed values shorter than this are not searched for in the copy: a
/// short decimal key would match unrelated numbers and withhold innocent
/// data. Those values are still removed where they were found.
pub const MIN_RESIDUAL_BYTES: usize = 6;

/// A copy with its secrets removed, plus what was removed or left out.
///
/// `bytes` is the payload the bundle stores as a file; it is not part of
/// the serialized note, which carries only `redactions` and `withheld`.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretFreeCopy {
    #[serde(skip)]
    pub bytes: Vec<u8>,
    /// Sorted by location, then kind; one row per place and kind.
    pub redactions: Vec<Redaction>,
    /// `"<location>: <reason>"`, in the order the copy met them.
    pub withheld: Vec<String>,
}

impl fmt::Debug for SecretFreeCopy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretFreeCopy")
            .field("bytes", &format_args!("{} bytes", self.bytes.len()))
            .field("redactions", &self.redactions)
            .field("withheld", &self.withheld)
            .finish()
    }
}

/// One redacted place. Never carries the value, its length or a digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Redaction {
    /// Member or row, then an element path (`P-0001/0.xml:/KNX/…/@BCUKey`).
    pub location: String,
    /// The secret's name (`BCUKey`, `ToolKey`, `BusAccess/@Parameter:Password`).
    pub kind: String,
    /// How many values were blanked at this location under this kind.
    pub count: usize,
}

/// Why no copy was produced. Carries no secret value.
#[derive(Debug)]
pub enum SecretFreeCopyError {
    /// Reading the source or the scratch copy failed.
    Io {
        stage: &'static str,
        kind: std::io::ErrorKind,
    },
    /// The source is larger than one retained file may be.
    TooLarge { bytes: u64, limit: u64 },
    /// The source is not a project store this build can open read-only.
    Store(knx_store::MigrationError),
    /// A query or update on the scratch copy failed.
    Sql(knx_store::SqlError),
    /// The redacted copy does not reopen as a project.
    Reopen(String),
    /// The source archive itself cannot be read; nothing could be copied.
    Archive(knx_etsproj::ContainerError),
    /// Writing the copied archive failed.
    Write(String),
    /// A removed value still occurs in the finished copy outside any
    /// retained row (for example in a device name), so no copy is given.
    ResidualSecret { occurrences: usize },
}

impl fmt::Display for SecretFreeCopyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { stage, kind } => write!(f, "secret-free copy: {stage} failed ({kind})"),
            Self::TooLarge { bytes, limit } => write!(
                f,
                "secret-free copy: the source has {bytes} bytes, more than the {limit}-byte limit"
            ),
            Self::Store(error) => write!(f, "secret-free copy: {error}"),
            Self::Sql(error) => write!(f, "secret-free copy: project storage: {error}"),
            Self::Reopen(reason) => write!(
                f,
                "secret-free copy: the redacted project does not reopen: {reason}"
            ),
            Self::Archive(error) => write!(f, "secret-free copy: {error}"),
            Self::Write(reason) => write!(f, "secret-free copy: writing the archive: {reason}"),
            Self::ResidualSecret { occurrences } => write!(
                f,
                "secret-free copy refused: {occurrences} removed secret value(s) still occur \
                 elsewhere in the project, so no copy is offered"
            ),
        }
    }
}

impl std::error::Error for SecretFreeCopyError {}

impl From<knx_store::SqlError> for SecretFreeCopyError {
    fn from(error: knx_store::SqlError) -> Self {
        Self::Sql(error)
    }
}

impl From<knx_store::MigrationError> for SecretFreeCopyError {
    fn from(error: knx_store::MigrationError) -> Self {
        Self::Store(error)
    }
}

fn io_error(stage: &'static str) -> impl Fn(std::io::Error) -> SecretFreeCopyError {
    move |error| SecretFreeCopyError::Io {
        stage,
        kind: error.kind(),
    }
}

/// The bytes of a `.knxdb` equal to `source` except for its secrets. The
/// source is opened read-only and never written. Project history (undo,
/// redo, saved versions) is left out, because its snapshots embed the
/// retained source; carried product files that name a secret are left out
/// too, because their bytes are their identity.
pub fn native_project_without_secrets(
    source: &Path,
) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    native::copy(source)
}

/// A `.knxproj` (any ZIP) equal to `source` except for its secrets: XML
/// members are redacted, nested archives are rewritten the same way, other
/// members keep their bytes, and every member that cannot be inspected
/// (encrypted, unreadable, malformed) is left out and listed.
pub fn import_source_without_secrets(source: &Path) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    archive::copy(source)
}

/// Whether `name` is one of [`SECRET_NAMES`], exactly.
fn is_secret_name(name: &[u8]) -> bool {
    SECRET_NAMES.iter().any(|secret| secret.as_bytes() == name)
}

/// Whether a value under a secret name is secret material: non-empty, and
/// not the `BCUKey` free-access sentinel.
fn is_secret_value(name: &[u8], value: &[u8]) -> bool {
    let trimmed = value.trim_ascii();
    !trimmed.is_empty() && !(name == b"BCUKey" && trimmed == FREE_ACCESS_KEY)
}

/// What one copy found: redactions, withheld locations and, in memory only,
/// the removed values for the residual search. Deliberately not `Debug`.
#[derive(Default)]
struct Findings {
    redactions: BTreeMap<(String, String), usize>,
    withheld: Vec<String>,
    values: BTreeSet<Vec<u8>>,
}

impl Findings {
    fn redact(&mut self, location: String, kind: String) {
        *self.redactions.entry((location, kind)).or_default() += 1;
    }

    fn remember(&mut self, value: &[u8]) {
        let value = value.trim_ascii();
        if value.len() >= MIN_RESIDUAL_BYTES {
            self.values.insert(value.to_vec());
        }
    }

    fn withhold(&mut self, location: &str, reason: &str) {
        self.withheld.push(format!("{location}: {reason}"));
    }

    /// Takes over a part's findings (a member or row that is kept).
    fn absorb(&mut self, mut part: Findings) {
        for (key, count) in part.redactions {
            *self.redactions.entry(key).or_default() += count;
        }
        self.withheld.append(&mut part.withheld);
        self.values.append(&mut part.values);
    }

    /// The residual search over every value remembered so far.
    fn needles(&self) -> Needles<'_> {
        Needles::new(&self.values)
    }

    fn finish(self, bytes: Vec<u8>) -> SecretFreeCopy {
        SecretFreeCopy {
            bytes,
            redactions: self
                .redactions
                .into_iter()
                .map(|((location, kind), count)| Redaction {
                    location,
                    kind,
                    count,
                })
                .collect(),
            withheld: self.withheld,
        }
    }
}

/// One linear pass finds any of many removed values: every value is at
/// least [`MIN_RESIDUAL_BYTES`] long, so its first six bytes, packed into an
/// integer, index it. A bit filter rejects almost every window before the
/// map is asked, so a large project costs one cheap step per byte, not one
/// comparison per byte and value.
struct Needles<'a> {
    filter: Vec<u64>,
    by_prefix: std::collections::HashMap<u64, Vec<&'a [u8]>>,
}

const PREFIX_MASK: u64 = (1 << (8 * MIN_RESIDUAL_BYTES)) - 1;
const FILTER_BITS: u32 = 20;

impl<'a> Needles<'a> {
    fn new(values: &'a BTreeSet<Vec<u8>>) -> Self {
        let mut filter = vec![0u64; (1usize << FILTER_BITS) / 64];
        let mut by_prefix = std::collections::HashMap::<u64, Vec<&[u8]>>::new();
        for value in values.iter().filter(|v| v.len() >= MIN_RESIDUAL_BYTES) {
            let prefix = value[..MIN_RESIDUAL_BYTES]
                .iter()
                .fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
            let slot = Self::slot(prefix);
            filter[slot / 64] |= 1 << (slot % 64);
            by_prefix.entry(prefix).or_default().push(value);
        }
        Self { filter, by_prefix }
    }

    fn slot(prefix: u64) -> usize {
        (prefix.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - FILTER_BITS)) as usize
    }

    /// How many distinct values occur in `haystack`; stops at `limit`.
    fn count_in(&self, haystack: &[u8], limit: usize) -> usize {
        if self.by_prefix.is_empty() || limit == 0 {
            return 0;
        }
        let mut found = BTreeSet::new();
        let mut window = 0u64;
        for (index, &byte) in haystack.iter().enumerate() {
            window = ((window << 8) | u64::from(byte)) & PREFIX_MASK;
            if index + 1 < MIN_RESIDUAL_BYTES {
                continue;
            }
            let slot = Self::slot(window);
            if self.filter[slot / 64] & (1 << (slot % 64)) == 0 {
                continue;
            }
            let start = index + 1 - MIN_RESIDUAL_BYTES;
            for value in self.by_prefix.get(&window).into_iter().flatten() {
                if haystack[start..].starts_with(value)
                    && found.insert(*value)
                    && found.len() >= limit
                {
                    return found.len();
                }
            }
        }
        found.len()
    }

    fn any_in(&self, haystack: &[u8]) -> bool {
        self.count_in(haystack, 1) > 0
    }
}
