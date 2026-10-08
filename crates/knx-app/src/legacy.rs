//! Opens legacy ETS3 EX-IM files with the user's password; the one place that decrypts them.
//!
//! `knx-productdb` reads the container and the grammar but never decrypts,
//! so no binary that links it without this crate links key material
//! (ADR-0090, ADR-0094). Here the password meets `knx-secure`'s single
//! ZipCrypto implementation, and only here. The password is never stored,
//! reported or logged, and nothing is ever guessed.

use std::fmt;

use knx_productdb::legacy::{
    inspect_payload, publish_legacy, read_legacy_member, LegacyError, LegacyInspection,
    LegacyPayload, LegacyPublishError, LegacyPublishReport,
};
use knx_productdb::Connection;
use knx_secure::zipcrypto::{self, CheckBytes, ZipCryptoError};

/// The password a user supplied for one legacy file.
///
/// `Debug` prints a placeholder; there is no `Display`, `Clone` or
/// serialisation, and the text is only reachable inside this module.
pub struct LegacyPassword(String);

impl LegacyPassword {
    pub fn new(password: impl Into<String>) -> Self {
        Self(password.into())
    }
}

impl fmt::Debug for LegacyPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LegacyPassword(<redacted>)")
    }
}

/// Opens a legacy file: decrypts with `password` when its member is
/// encrypted, inflates within the declared size and checks the CRC-32. An
/// empty password counts as none, so an always-sending client is asked for
/// one rather than told it is wrong. A password for an unencrypted file is
/// ignored.
pub fn open_legacy_file(
    bytes: &[u8],
    password: Option<&LegacyPassword>,
) -> Result<LegacyPayload, LegacyError> {
    let member = read_legacy_member(bytes)?;
    let Some(stream) = member.encrypted_stream() else {
        return member.open_unencrypted();
    };
    let password = password
        .filter(|p| !p.0.is_empty())
        .ok_or(LegacyError::PasswordRequired)?;
    let check = member.check_bytes();
    let compressed = zipcrypto::decrypt(
        password.0.as_bytes(),
        stream,
        CheckBytes {
            crc32_high_byte: check.crc32_high_byte,
            dos_time_high_byte: check.dos_time_high_byte,
        },
    )
    .map_err(|e| match e {
        ZipCryptoError::WrongPassword => LegacyError::WrongPassword,
        ZipCryptoError::TruncatedHeader { .. } => LegacyError::InvalidContainer {
            reason: e.to_string(),
        },
    })?;
    member.open_decrypted(&compressed)
}

/// Opens, parses and summarises a legacy file. Writes nothing.
pub fn inspect_legacy_file(
    bytes: &[u8],
    password: Option<&LegacyPassword>,
) -> Result<LegacyInspection, LegacyError> {
    inspect_payload(&open_legacy_file(bytes, password)?)
}

/// Opens a legacy product database with the user's password and publishes
/// every application program in it into the product database, in one
/// transaction (ADR-0094). The password is used for decryption only: it is
/// not stored, logged or returned.
pub fn import_legacy_file(
    products: &Connection,
    source_name: &str,
    bytes: &[u8],
    password: Option<&LegacyPassword>,
) -> Result<LegacyPublishReport, LegacyPublishError> {
    let payload = open_legacy_file(bytes, password)?;
    publish_legacy(products, source_name, bytes, &payload)
}
