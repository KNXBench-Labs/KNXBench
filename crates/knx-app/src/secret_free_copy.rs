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
//! emitted in a [`Redaction`] or in error `Display`/`Debug` output.
//!
//! As a second net, every removed value of at least
//! [`MIN_RESIDUAL_BYTES`] bytes is searched for in what is about to be
//! copied: a member or retained row that still contains one is withheld,
//! and a native copy whose final bytes still contain one is refused.
//!
//! This exports a quiescent **saved root**, not unsaved editor/recovery state.
//! Native input is read once into a bounded private snapshot before any SQLite
//! opener runs. WAL-mode and journal/sidecar-bearing sources are refused, not
//! checkpointed or opened through a reader that could write shared-memory state.
//! History
//! omission is reported. Manufacturer identities are never forged: a withheld
//! carried file can leave a manifest reference unavailable, visibly listed.
//!
//! Diagnostic-only ceilings (shared across discovery/enforcement and children):
//! source ZIP 256 MiB; native snapshot 512 MiB and 262,144 retained rows;
//! decoded expansion/output 1 GiB each; XML depth 256, path/name 8 KiB,
//! 4,096 attributes per tag and 128 MiB of explicitly charged XML work;
//! complete secret element value 1 MiB; learned values 16,384 / 8 MiB;
//! findings 32,768 / 8 MiB; residual matching 16 MiB of input/comparison work.
//! Budget exhaustion refuses the whole artifact, never means no secret matched.
//! Reports additionally require bounded JSON objects (depth 64, conservative
//! 16 MiB serialized-size allowance). Uninspectable encodings remain withheld.
//! Residual matching is literal, not arbitrary encoding/decryption analysis;
//! the short-value exception never exempts directly identified credential fields.

mod archive;
mod budget;
mod native;
mod report;
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

/// Why no copy was produced. Display and Debug never echo untrusted error text.
pub enum SecretFreeCopyError {
    /// A source cannot be snapshotted without touching live journal state.
    SourceUnavailable { reason: &'static str },
    /// A bounded inspection cannot complete safely.
    BudgetExceeded { resource: &'static str },
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
            Self::SourceUnavailable { reason } => {
                write!(f, "secret-free copy: source unavailable ({reason})")
            }
            Self::BudgetExceeded { resource } => write!(f, "secret-free copy refused: {resource}"),
            Self::Io { stage, kind } => write!(f, "secret-free copy: {stage} failed ({kind})"),
            Self::TooLarge { bytes, limit } => write!(
                f,
                "secret-free copy: the source has {bytes} bytes, more than the {limit}-byte limit"
            ),
            Self::Store(_) => f.write_str("secret-free copy: project store unavailable"),
            Self::Sql(_) => f.write_str("secret-free copy: project storage refused"),
            Self::Reopen(_) => f.write_str("secret-free copy: redacted project does not reopen"),
            Self::Archive(_) => f.write_str("secret-free copy: archive unavailable"),
            Self::Write(_) => f.write_str("secret-free copy: output refused"),
            Self::ResidualSecret { occurrences } => write!(
                f,
                "secret-free copy refused: {occurrences} removed secret value(s) still occur \
                 elsewhere in the project, so no copy is offered"
            ),
        }
    }
}

impl fmt::Debug for SecretFreeCopyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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

/// Sanitized historical import report JSON; the caller's evidence is never changed.
/// Row identities/counts stay intact. A secret-bearing identity makes the report
/// unavailable instead of silently regrouping historical findings.
pub fn import_report_without_secrets(
    report: &serde_json::Value,
) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    report::copy(report)
}

/// Owned body and notes admitted for one diagnostic-bundle source instance.
///
/// The boundary deliberately implements neither `Serialize` nor `Debug`:
///
/// ```compile_fail
/// fn require_serialize<T: serde::Serialize>() {}
/// require_serialize::<knx_app::secret_free_copy::BundleAdmittedCopy>();
/// ```
///
/// ```compile_fail
/// fn require_debug<T: std::fmt::Debug>() {}
/// require_debug::<knx_app::secret_free_copy::BundleAdmittedCopy>();
/// ```
pub struct BundleAdmittedCopy {
    bytes: Vec<u8>,
    notes: Vec<String>,
}
impl BundleAdmittedCopy {
    /// Consumes the admitted instance without exposing an outer serializable wrapper.
    pub fn into_parts(self) -> (Vec<u8>, Vec<String>) {
        (self.bytes, self.notes)
    }
}

/// One generic bundle-copy refusal with no source-derived payload.
///
/// The refusal deliberately implements neither `Serialize` nor `Debug`:
///
/// ```compile_fail
/// fn require_serialize<T: serde::Serialize>() {}
/// require_serialize::<knx_app::secret_free_copy::BundleCopyUnavailable>();
/// ```
///
/// ```compile_fail
/// fn require_debug<T: std::fmt::Debug>() {}
/// require_debug::<knx_app::secret_free_copy::BundleCopyUnavailable>();
/// ```
pub struct BundleCopyUnavailable;

/// Admits one historical import-report instance for diagnostic-bundle use.
pub fn import_report_for_bundle(
    report: &serde_json::Value,
) -> Result<BundleAdmittedCopy, BundleCopyUnavailable> {
    admit_bundle_copy(report::copy_for_bundle(report))
}

/// Admits one saved native-project instance for diagnostic-bundle use.
pub fn native_project_for_bundle(
    source: &Path,
) -> Result<BundleAdmittedCopy, BundleCopyUnavailable> {
    admit_bundle_copy(native::copy_for_bundle(source))
}

fn admit_bundle_copy(
    result: Result<ObservedCopy, SecretFreeCopyError>,
) -> Result<BundleAdmittedCopy, BundleCopyUnavailable> {
    let observed = result.map_err(|_| BundleCopyUnavailable)?;
    if observed.credential_material_observed {
        return Err(BundleCopyUnavailable);
    }
    let SecretFreeCopy {
        bytes,
        redactions,
        withheld,
    } = observed.copy;
    let mut notes = Vec::with_capacity(redactions.len().saturating_add(withheld.len()));
    for redaction in redactions {
        notes.push(format!(
            "removed {} at {}",
            redaction.kind, redaction.location
        ));
    }
    for withheld in withheld {
        notes.push(format!("left out: {withheld}"));
    }
    Ok(BundleAdmittedCopy { bytes, notes })
}

/// Whether `name` is one of [`SECRET_NAMES`], exactly.
fn is_secret_name(name: &[u8]) -> bool {
    let name = local_name(name);
    SECRET_NAMES.iter().any(|secret| secret.as_bytes() == name)
}

/// The namespace-local part of an XML name.
fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

/// Whether decoded bytes are exactly the documented free-access sentinel.
fn is_free_access_key(name: &[u8], value: &[u8]) -> bool {
    local_name(name) == b"BCUKey" && value == FREE_ACCESS_KEY
}

/// The historical ordinary-copy redaction rule, including ASCII trimming.
fn is_secret_value(name: &[u8], value: &[u8]) -> bool {
    let trimmed = value.trim_ascii();
    !trimmed.is_empty() && !is_free_access_key(name, trimmed)
}

/// What one copy found: redactions, withheld locations and, in memory only,
/// the removed values for the residual search. Deliberately not `Debug`.
#[derive(Default)]
struct Findings {
    budget: budget::Budget,
    redactions: BTreeMap<(String, String), usize>,
    withheld: Vec<String>,
    values: BTreeSet<Vec<u8>>,
    credential_material_observed: bool,
    credential_observation_epoch: u64,
}

/// One completed copy plus its private source-instance admission witness.
struct ObservedCopy {
    copy: SecretFreeCopy,
    credential_material_observed: bool,
}

impl Findings {
    fn child(&self) -> Self {
        Self {
            budget: self.budget.clone(),
            credential_material_observed: self.credential_material_observed,
            credential_observation_epoch: self.credential_observation_epoch,
            ..Self::default()
        }
    }
    fn ensure(&self) -> Result<(), SecretFreeCopyError> {
        self.budget
            .check()
            .map_err(|resource| SecretFreeCopyError::BudgetExceeded { resource })
    }
    fn redact(&mut self, location: String, kind: String) {
        if self
            .budget
            .finding(location.len().saturating_add(kind.len()))
            .is_err()
        {
            return;
        }
        *self.redactions.entry((location, kind)).or_default() += 1;
    }

    fn remember(&mut self, value: &[u8]) {
        let value = value.trim_ascii();
        if value.len() >= MIN_RESIDUAL_BYTES && !self.values.contains(value) {
            if self.budget.value(value.len()).is_err() {
                return;
            }
            self.values.insert(value.to_vec());
        }
    }

    fn observe_credential(&mut self, name: &[u8], value: &[u8]) {
        if !value.is_empty() && !is_free_access_key(name, value) {
            self.observe_credential_material();
        }
    }

    fn observe_credential_material(&mut self) {
        self.credential_material_observed = true;
        self.credential_observation_epoch = self.credential_observation_epoch.wrapping_add(1);
    }

    fn begin_tentative_credential_observation(&mut self) -> (bool, u64) {
        let prior = (
            self.credential_material_observed,
            self.credential_observation_epoch,
        );
        self.credential_material_observed = true;
        prior
    }

    fn resolve_tentative_credential_observation(
        &mut self,
        prior: (bool, u64),
        free_access_sentinel: bool,
    ) {
        if free_access_sentinel {
            if self.credential_observation_epoch == prior.1 {
                self.credential_material_observed = prior.0;
            }
        } else {
            self.observe_credential_material();
        }
    }

    fn withhold(&mut self, location: &str, reason: &str) {
        if self
            .budget
            .finding(location.len().saturating_add(reason.len()))
            .is_err()
        {
            return;
        }
        self.withheld.push(format!("{location}: {reason}"));
    }

    /// Takes over a part's findings (a member or row that is kept).
    fn absorb(&mut self, mut part: Findings) {
        self.retain_private(&mut part);
        for (key, count) in part.redactions {
            *self.redactions.entry(key).or_default() += count;
        }
        self.withheld.append(&mut part.withheld);
    }

    /// Retains source-instance observations when a child itself is withheld.
    fn retain_private(&mut self, part: &mut Findings) {
        if part.credential_material_observed {
            self.observe_credential_material();
        }
        self.values.append(&mut part.values);
    }

    /// The residual search over every value remembered so far.
    fn needles(&self) -> Needles<'_> {
        Needles::with_budget(&self.values, self.budget.clone())
    }

    fn finish(mut self, bytes: Vec<u8>) -> Result<SecretFreeCopy, SecretFreeCopyError> {
        self.ensure()?;
        let needles = Needles::with_budget(&self.values, self.budget.clone());
        for text in &mut self.withheld {
            if needles.any_in(text.as_bytes()) {
                *text = "withheld entry: secret in metadata".into();
            }
        }
        let copy = SecretFreeCopy {
            bytes,
            redactions: self
                .redactions
                .into_iter()
                .map(|((location, kind), count)| Redaction {
                    location: if needles.any_in(location.as_bytes()) {
                        "redacted location".into()
                    } else {
                        location
                    },
                    kind: if needles.any_in(kind.as_bytes()) {
                        "redacted kind".into()
                    } else {
                        kind
                    },
                    count,
                })
                .collect(),
            withheld: self.withheld,
        };
        let note = serde_json::to_vec(&copy)
            .map_err(|_| SecretFreeCopyError::Write("copy metadata".into()))?;
        let residual = needles.any_in(&note);
        self.budget
            .check()
            .map_err(|resource| SecretFreeCopyError::BudgetExceeded { resource })?;
        if residual {
            return Err(SecretFreeCopyError::ResidualSecret { occurrences: 1 });
        }
        Ok(copy)
    }

    fn finish_observed(self, bytes: Vec<u8>) -> Result<ObservedCopy, SecretFreeCopyError> {
        let credential_material_observed = self.credential_material_observed;
        let copy = self.finish(bytes)?;
        Ok(ObservedCopy {
            copy,
            credential_material_observed,
        })
    }
}

/// Prefix-indexed matching with a cumulative worst-case comparison budget.
/// Every window and candidate comparison is charged before work. A sticky
/// exhaustion error forbids publication, including after metadata scanning.
struct Needles<'a> {
    budget: budget::Budget,
    filter: Vec<u64>,
    by_prefix: std::collections::HashMap<u64, Vec<&'a [u8]>>,
}

const PREFIX_MASK: u64 = (1 << (8 * MIN_RESIDUAL_BYTES)) - 1;
const FILTER_BITS: u32 = 20;

impl<'a> Needles<'a> {
    #[cfg(test)]
    fn new(values: &'a BTreeSet<Vec<u8>>) -> Self {
        Self::with_budget(values, budget::Budget::default())
    }

    fn with_budget(values: &'a BTreeSet<Vec<u8>>, budget: budget::Budget) -> Self {
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
        Self {
            filter,
            by_prefix,
            budget,
        }
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
        // Charge bytes once, and candidate comparison upper bounds before each
        // comparison. Exhaustion is sticky and publication must return its error.
        if self.budget.matching(haystack.len()).is_err() {
            return 1;
        }
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
                if self.budget.matching(value.len()).is_err() {
                    return 1;
                }
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
