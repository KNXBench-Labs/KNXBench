//! The access key a download authorises with: the project's, the operator's, or none.
//!
//! `[D]` KNX Project Schema 23 v01.00.00, element
//! `Project_t/Installations/Installation`, attribute `BCUKey`:
//! `xs:unsignedLong`, optional, default `4294967295`, *"The key used to lock
//! devices supporting authentication."* The importer keeps it verbatim as a
//! retained attribute (`knx-etsproj`'s known-attribute table lists it; the
//! domain model does not carry it), so it reaches this module as a stored
//! opaque row and nowhere else.
//!
//! `[D]` AL §3.5.7 makes a key four octets, `unsigned32`, and `4294967295`
//! (`FFFFFFFFh`) the free-access sentinel, which is **not** a key. A project
//! that leaves the attribute at its default therefore has no key, and this
//! module says so instead of handing the sentinel on.
//!
//! No key is ever made up here: a value that does not parse, does not fit
//! four octets, or disagrees between installations is an error the operator
//! sees, never a fallback to "try something".

use std::fmt;

use knx_core::commissioning::authorisation::{
    AccessKey, AuthorisationPlan, LevelCount, FREE_ACCESS_KEY,
};
use knx_core::commissioning::load_control_memory::loads_through_memory;
use knx_core::commissioning::load_state::MaskVersion;
use knx_store::StoredOpaqueEntry;

/// The attribute name, as the schema spells it.
const BCU_KEY_ATTRIBUTE: &str = "BCUKey";
/// The element that carries it; the stored xpath ends with this.
const INSTALLATION_ELEMENT: &str = "/Installation";
/// How `knx-app::import` stores `OpaqueKind::RetainedAttribute`.
const RETAINED_ATTRIBUTE_KIND: &str = "RetainedAttribute";

/// Where the key a download will use came from. Carries no value, so it
/// can be printed, logged and sent to a UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    /// No key: authorisation is skipped and the device grants its free
    /// level (MP §3.5.1's `key != FFFFFFFFh` guard).
    None,
    /// The project's `Installation/@BCUKey`.
    Project,
    /// Supplied by the operator for this one run.
    Operator,
}

impl fmt::Display for KeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            KeySource::None => "none (the device's free access level)",
            KeySource::Project => "from the project (Installation/@BCUKey)",
            KeySource::Operator => "supplied by the operator",
        })
    }
}

/// Why no key could be taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// The value is not a non-negative decimal (project) or a decimal /
    /// `0x` hexadecimal number (operator).
    NotANumber,
    /// The value does not fit AL §3.5.7's four octets.
    TooLarge,
    /// Two installations of the project name different keys, and a
    /// download has no way to know which one locked this device.
    Conflicting,
    /// The operator supplied `FFFFFFFFh`, which is the free-access sentinel
    /// and not a key.
    FreeAccessSentinel,
}

impl std::error::Error for KeyError {}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deliberately never the value: a key in an error is a key in a log.
        f.write_str(match self {
            KeyError::NotANumber => "the access key is not a number",
            KeyError::TooLarge => "the access key does not fit four octets (unsigned32)",
            KeyError::Conflicting => {
                "the project's installations name different access keys; supply the key \
                 for this device explicitly"
            }
            KeyError::FreeAccessSentinel => {
                "4294967295 (FFFFFFFFh) is the free-access sentinel, not a key; leave the key \
                 out to use the free access level"
            }
        })
    }
}

/// The project's access key, from the stored opaque rows.
///
/// `Ok(None)` when no installation carries a `BCUKey` other than the
/// schema default. Every installation that carries a real key must carry
/// the same one.
pub fn project_access_key(opaque: &[StoredOpaqueEntry]) -> Result<Option<AccessKey>, KeyError> {
    let mut found: Option<AccessKey> = None;
    for entry in opaque.iter().filter(|entry| {
        entry.kind == RETAINED_ATTRIBUTE_KIND
            && entry.name == BCU_KEY_ATTRIBUTE
            && entry.xpath.ends_with(INSTALLATION_ELEMENT)
    }) {
        let text = std::str::from_utf8(&entry.bytes).map_err(|_| KeyError::NotANumber)?;
        let value = parse_decimal(text)?;
        if value == FREE_ACCESS_KEY {
            continue;
        }
        let key = AccessKey::new(value).map_err(|_| KeyError::FreeAccessSentinel)?;
        match found {
            Some(earlier) if earlier != key => return Err(KeyError::Conflicting),
            _ => found = Some(key),
        }
    }
    Ok(found)
}

/// An operator-supplied key: decimal, or hexadecimal with a `0x` prefix,
/// surrounding whitespace ignored (a key file usually ends in a newline).
pub fn parse_operator_key(text: &str) -> Result<AccessKey, KeyError> {
    let text = text.trim();
    let value = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => {
            if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(KeyError::NotANumber);
            }
            u64::from_str_radix(hex, 16).map_err(|_| KeyError::TooLarge)?
        }
        None => parse_wide_decimal(text)?,
    };
    let value = u32::try_from(value).map_err(|_| KeyError::TooLarge)?;
    AccessKey::new(value).map_err(|_| KeyError::FreeAccessSentinel)
}

/// The key a download uses: the operator's wins over the project's,
/// because it is the more specific statement about this one device.
pub fn choose_key(
    operator: Option<AccessKey>,
    project: Option<AccessKey>,
) -> (Option<AccessKey>, KeySource) {
    match (operator, project) {
        (Some(key), _) => (Some(key), KeySource::Operator),
        (None, Some(key)) => (Some(key), KeySource::Project),
        (None, None) => (None, KeySource::None),
    }
}

/// How a download session authorises, decided offline from the plan's mask
/// and the chosen key. Carries no key value in its `Debug`
/// ([`AccessKey`]'s own is redacted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadKeying {
    /// What the session sends on connect.
    pub plan: AuthorisationPlan,
    /// Where the key came from.
    pub source: KeySource,
    /// Whether to run MP §3.5.2 `DM_Authorize2_RCo` instead of §3.5.1.
    pub two_key: bool,
    /// The device's level count, for the wrong-key suspicion.
    pub level_count: LevelCount,
}

/// The keying for a download to a device with `mask`.
///
/// `[D]` MP (`03_05_02` v02.01.02) §3.5.2, p. 76: `DM_Authorize2_RCo` is
/// scoped by its *Use* row to *"System 2, BIM M112"*, for *"Write access"*
/// when *"A key must be available"*, and exists for exactly the case
/// *"when the ETS User enters a key to be used to lock the devices and
/// uninitialised devices fresh from the factory are used"*: it authorises
/// with `FFFFFFFFh` and with the key and keeps the better level. The only
/// masks this project downloads are `070nh`, BIM M112 (ADR-0048), so with a
/// key the procedure scoped to that profile is the one that runs; without a
/// key it *"shall not be executed"*, and §3.5.1's own guard skips
/// authorisation too.
///
/// `[D]` PROF (`06 Profiles` v02.01.01) Table 4.2, p. 37, row
/// *"Authorization, nr of access levels"*: `16` for mask `0701h`. Other
/// masks stay [`LevelCount::Unknown`] rather than borrowing that value.
pub fn download_keying(
    mask: MaskVersion,
    key: Option<AccessKey>,
    source: KeySource,
) -> DownloadKeying {
    let level_count = if mask == MaskVersion(0x0701) {
        LevelCount::Sixteen
    } else {
        LevelCount::Unknown
    };
    DownloadKeying {
        plan: AuthorisationPlan::from_operator_key(key),
        source,
        two_key: key.is_some() && loads_through_memory(mask),
        level_count,
    }
}

fn parse_decimal(text: &str) -> Result<u32, KeyError> {
    u32::try_from(parse_wide_decimal(text.trim())?).map_err(|_| KeyError::TooLarge)
}

/// `xs:unsignedLong` is 64 bits wide, so a schema-valid value can overflow
/// four octets; that is reported as too large, not as garbage.
fn parse_wide_decimal(text: &str) -> Result<u64, KeyError> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(KeyError::NotANumber);
    }
    text.parse::<u64>().map_err(|_| KeyError::TooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bcu(xpath: &str, value: &str) -> StoredOpaqueEntry {
        StoredOpaqueEntry {
            source_path: "P-0001/0.xml".into(),
            xpath: xpath.into(),
            kind: RETAINED_ATTRIBUTE_KIND.into(),
            name: BCU_KEY_ATTRIBUTE.into(),
            bytes: value.as_bytes().to_vec(),
            sha256: String::new(),
        }
    }

    const INSTALLATION: &str = "/KNX/Project/Installations/Installation";

    #[test]
    fn a_project_without_the_attribute_has_no_key() {
        assert_eq!(project_access_key(&[]), Ok(None));
    }

    #[test]
    fn the_schema_default_is_the_sentinel_and_therefore_no_key() {
        assert_eq!(
            project_access_key(&[bcu(INSTALLATION, "4294967295")]),
            Ok(None)
        );
    }

    #[test]
    fn a_real_key_is_taken_from_the_installation() {
        let key = project_access_key(&[bcu(INSTALLATION, "305419896")])
            .unwrap()
            .expect("a key");
        assert_eq!(key.raw(), 0x1234_5678);
    }

    #[test]
    fn only_the_installation_attribute_counts() {
        let mut elsewhere = bcu("/KNX/Project/Installations/Installation/Topology", "7");
        elsewhere.name = BCU_KEY_ATTRIBUTE.into();
        let mut other_kind = bcu(INSTALLATION, "8");
        other_kind.kind = "RetainedElement".into();
        let mut other_name = bcu(INSTALLATION, "9");
        other_name.name = "InstallationId".into();
        assert_eq!(
            project_access_key(&[elsewhere, other_kind, other_name]),
            Ok(None)
        );
    }

    #[test]
    fn installations_that_agree_give_one_key_and_disagreeing_ones_give_none() {
        let agree = [bcu(INSTALLATION, "42"), bcu(INSTALLATION, "42")];
        assert_eq!(project_access_key(&agree).unwrap().unwrap().raw(), 42);
        let default_and_key = [bcu(INSTALLATION, "4294967295"), bcu(INSTALLATION, "42")];
        assert_eq!(
            project_access_key(&default_and_key).unwrap().unwrap().raw(),
            42
        );
        let disagree = [bcu(INSTALLATION, "42"), bcu(INSTALLATION, "43")];
        assert_eq!(project_access_key(&disagree), Err(KeyError::Conflicting));
    }

    #[test]
    fn a_malformed_project_key_is_an_error_not_a_guess() {
        for bad in ["", "abc", "-1", "0x10", "1 2"] {
            assert_eq!(
                project_access_key(&[bcu(INSTALLATION, bad)]),
                Err(KeyError::NotANumber),
                "{bad:?}"
            );
        }
        assert_eq!(
            project_access_key(&[bcu(INSTALLATION, "4294967296")]),
            Err(KeyError::TooLarge)
        );
    }

    #[test]
    fn operator_keys_are_decimal_or_hex_and_never_the_sentinel() {
        assert_eq!(
            parse_operator_key("305419896\n").unwrap().raw(),
            0x1234_5678
        );
        assert_eq!(
            parse_operator_key(" 0x12345678 ").unwrap().raw(),
            0x1234_5678
        );
        assert_eq!(parse_operator_key("0X0").unwrap().raw(), 0);
        assert_eq!(
            parse_operator_key("0xFFFFFFFF"),
            Err(KeyError::FreeAccessSentinel)
        );
        assert_eq!(
            parse_operator_key("4294967295"),
            Err(KeyError::FreeAccessSentinel)
        );
        assert_eq!(parse_operator_key("0x100000000"), Err(KeyError::TooLarge));
        for bad in ["", "0x", "key", "0xG1", "+5"] {
            assert_eq!(
                parse_operator_key(bad),
                Err(KeyError::NotANumber),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn the_operator_wins_and_no_key_means_the_free_level() {
        let project = AccessKey::new(1).unwrap();
        let operator = AccessKey::new(2).unwrap();
        assert_eq!(
            choose_key(Some(operator), Some(project)),
            (Some(operator), KeySource::Operator)
        );
        assert_eq!(
            choose_key(None, Some(project)),
            (Some(project), KeySource::Project)
        );
        assert_eq!(choose_key(None, None), (None, KeySource::None));
    }

    #[test]
    fn a_bim_m112_with_a_key_runs_the_two_key_procedure_and_without_one_skips() {
        let key = AccessKey::new(0x55).unwrap();
        let keyed = download_keying(MaskVersion(0x0701), Some(key), KeySource::Project);
        assert_eq!(keyed.plan, AuthorisationPlan::WithKey(key));
        assert!(keyed.two_key);
        assert_eq!(keyed.level_count, LevelCount::Sixteen);

        let free = download_keying(MaskVersion(0x0701), None, KeySource::None);
        assert_eq!(free.plan, AuthorisationPlan::Skip);
        assert!(
            !free.two_key,
            "MP §3.5.2: without a key it shall not be executed"
        );

        let other = download_keying(MaskVersion(0x07B0), Some(key), KeySource::Operator);
        assert!(!other.two_key, "scoped to BIM M112, not System B");
        assert_eq!(other.level_count, LevelCount::Unknown);
        let sibling = download_keying(MaskVersion(0x0705), Some(key), KeySource::Operator);
        assert!(sibling.two_key);
        assert_eq!(sibling.level_count, LevelCount::Unknown);
    }

    #[test]
    fn no_error_message_carries_the_value() {
        let secret = "305419897";
        let errors = [
            KeyError::NotANumber,
            KeyError::TooLarge,
            KeyError::Conflicting,
            KeyError::FreeAccessSentinel,
        ];
        for error in errors {
            assert!(!error.to_string().contains(secret));
        }
        let key = parse_operator_key(secret).unwrap();
        assert!(!format!("{key:?}").contains(secret));
    }
}
