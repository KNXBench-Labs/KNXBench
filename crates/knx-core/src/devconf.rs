//! One device's configuration profile for a device configuration module.
//!
//! The profile is project state (ADR-0175 decision 5): the module package
//! itself lives in the product-data layer, but the document a user edits
//! belongs to the project and is stored with the module identity it was
//! written for. A device holds at most one profile; `None` means the device
//! has no device configuration yet, not that it has an empty one.

use std::fmt;

use crate::ids::DeviceId;

/// Largest retained document, in bytes. The engine's own per-module
/// `max_document_bytes` ceiling (at most 1 MiB) applies on top at validation
/// time; this project-wide ceiling bounds what the store retains before a
/// module is even known.
pub const MAX_PROFILE_DOCUMENT_BYTES: usize = 1_048_576;

/// Largest retained module id, in bytes (the engine's own limit is 64).
pub const MAX_MODULE_ID_BYTES: usize = 64;

/// Largest retained module version string, in bytes (the engine's own
/// `MAJOR.MINOR.PATCH` limits are stricter).
pub const MAX_MODULE_VERSION_BYTES: usize = 24;

/// Largest retained module package SHA-256, in lowercase hex (64 characters).
pub const MAX_MODULE_SHA256_BYTES: usize = 64;

/// One device's configuration profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfigProfile {
    /// The device this profile belongs to.
    pub device: DeviceId,
    /// `id` of the module package the document was written for.
    pub module_id: String,
    /// `version` of that module package.
    pub module_version: String,
    /// SHA-256 (lowercase hex) of the exact module package bytes.
    pub module_sha256: String,
    /// The configuration document, as strict JSON bytes.
    pub document: Vec<u8>,
}

/// Why a profile is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// `device` is id 0, which is never a real device.
    InvalidDevice,
    /// `module_id` is empty, over 64 bytes, or not the engine's id syntax.
    InvalidModuleId,
    /// `module_version` is empty, over 24 bytes, or not `MAJOR.MINOR.PATCH`.
    InvalidModuleVersion,
    /// `module_sha256` is not 64 lowercase hex characters.
    InvalidModuleSha256,
    /// The document is empty or over [`MAX_PROFILE_DOCUMENT_BYTES`].
    DocumentBudget,
    /// A device may hold at most one profile.
    DuplicateDevice(DeviceId),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDevice => f.write_str("device id 0 is not a valid identity"),
            Self::InvalidModuleId => {
                f.write_str("module id is empty, over 64 bytes or not [a-z0-9.-]")
            }
            Self::InvalidModuleVersion => {
                f.write_str("module version is empty, over 24 bytes or not MAJOR.MINOR.PATCH")
            }
            Self::InvalidModuleSha256 => {
                f.write_str("module sha256 is not 64 lowercase hex characters")
            }
            Self::DocumentBudget => write!(
                f,
                "the document is empty or exceeds {MAX_PROFILE_DOCUMENT_BYTES} bytes"
            ),
            Self::DuplicateDevice(device) => {
                write!(f, "device {device} already holds a configuration profile")
            }
        }
    }
}

impl std::error::Error for ProfileError {}

fn valid_module_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (1..=MAX_MODULE_ID_BYTES).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'.' || *b == b'-')
}

fn valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
                && p.parse::<u32>().is_ok_and(|n| n <= 65535)
        })
}

fn valid_sha256(sha: &str) -> bool {
    sha.len() == MAX_MODULE_SHA256_BYTES
        && sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl DeviceConfigProfile {
    /// Validates one profile. Bounds are the project-side ceilings above; the
    /// engine applies the module's own, stricter ones when it validates.
    pub fn new(
        device: DeviceId,
        module_id: &str,
        module_version: &str,
        module_sha256: &str,
        document: Vec<u8>,
    ) -> Result<Self, ProfileError> {
        if device.0 == 0 {
            return Err(ProfileError::InvalidDevice);
        }
        if !valid_module_id(module_id) {
            return Err(ProfileError::InvalidModuleId);
        }
        if module_version.is_empty()
            || module_version.len() > MAX_MODULE_VERSION_BYTES
            || !valid_version(module_version)
        {
            return Err(ProfileError::InvalidModuleVersion);
        }
        if !valid_sha256(module_sha256) {
            return Err(ProfileError::InvalidModuleSha256);
        }
        if document.is_empty() || document.len() > MAX_PROFILE_DOCUMENT_BYTES {
            return Err(ProfileError::DocumentBudget);
        }
        Ok(Self {
            device,
            module_id: module_id.to_owned(),
            module_version: module_version.to_owned(),
            module_sha256: module_sha256.to_owned(),
            document,
        })
    }

    pub fn device(&self) -> DeviceId {
        self.device
    }
    pub fn module_id(&self) -> &str {
        &self.module_id
    }
    pub fn module_version(&self) -> &str {
        &self.module_version
    }
    pub fn module_sha256(&self) -> &str {
        &self.module_sha256
    }
    pub fn document(&self) -> &[u8] {
        &self.document
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> DeviceConfigProfile {
        DeviceConfigProfile::new(
            DeviceId(1),
            "example.demo-panel",
            "1.0.0",
            &"a".repeat(64),
            b"{}".to_vec(),
        )
        .unwrap()
    }

    #[test]
    fn valid_profile_is_accepted() {
        let p = profile();
        assert_eq!(p.device(), DeviceId(1));
        assert_eq!(p.module_id(), "example.demo-panel");
        assert_eq!(p.document(), b"{}");
    }

    #[test]
    fn identity_and_budget_bounds_are_enforced() {
        assert_eq!(
            DeviceConfigProfile::new(DeviceId(0), "m", "1.0.0", &"a".repeat(64), b"{}".to_vec()),
            Err(ProfileError::InvalidDevice)
        );
        for bad in ["", "A", ".a", "a_b", &"a".repeat(65)] {
            assert_eq!(
                DeviceConfigProfile::new(
                    DeviceId(1),
                    bad,
                    "1.0.0",
                    &"a".repeat(64),
                    b"{}".to_vec()
                ),
                Err(ProfileError::InvalidModuleId),
                "{bad:?}"
            );
        }
        for bad in ["", "1.2", "01.2.3", "1.2.65536", &"1.0.0".repeat(6)] {
            assert_eq!(
                DeviceConfigProfile::new(DeviceId(1), "m", bad, &"a".repeat(64), b"{}".to_vec()),
                Err(ProfileError::InvalidModuleVersion),
                "{bad:?}"
            );
        }
        for bad in [
            "",
            "A".repeat(64).as_str(),
            &"a".repeat(63),
            "g".repeat(64).as_str(),
        ] {
            assert_eq!(
                DeviceConfigProfile::new(DeviceId(1), "m", "1.0.0", bad, b"{}".to_vec()),
                Err(ProfileError::InvalidModuleSha256),
                "{bad:?}"
            );
        }
        assert_eq!(
            DeviceConfigProfile::new(DeviceId(1), "m", "1.0.0", &"a".repeat(64), vec![]),
            Err(ProfileError::DocumentBudget)
        );
        assert_eq!(
            DeviceConfigProfile::new(
                DeviceId(1),
                "m",
                "1.0.0",
                &"a".repeat(64),
                vec![0; MAX_PROFILE_DOCUMENT_BYTES + 1]
            ),
            Err(ProfileError::DocumentBudget)
        );
    }
}
