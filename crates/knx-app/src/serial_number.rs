//! A device's KNX Serial Number, as the imported project records it.
//!
//! `[D]` Project Schema 23: `DeviceInstance/@SerialNumber`, `xs:base64Binary`,
//! *"The SerialNumber is used for DownloadIndividualAddressBySerialNumber.
//! This serial number must be provided base64 encoded."* The importer keeps
//! the attribute verbatim as a retained opaque row on the device's element,
//! so this reads that row and nothing else. A project that never recorded
//! the device's serial number yields `None`; the operator supplies it then.

use std::fmt;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use knx_core::commissioning::serial_number::SerialNumber;
use knx_store::StoredOpaqueEntry;

/// The opaque row kind of a retained attribute.
const RETAINED_ATTRIBUTE_KIND: &str = "RetainedAttribute";
/// The attribute's name on `DeviceInstance`.
const SERIAL_NUMBER_ATTRIBUTE: &str = "SerialNumber";

/// Why a recorded serial number cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSerialNumberError {
    /// Not base64.
    NotBase64,
    /// Base64, but not the six octets RES §4.22.1.2 makes a serial number.
    WrongLength(usize),
}

impl fmt::Display for ProjectSerialNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotBase64 => f.write_str(
                "the project's SerialNumber for this device is not base64 (Project Schema: \
                 xs:base64Binary)",
            ),
            Self::WrongLength(length) => write!(
                f,
                "the project's SerialNumber for this device is {length} octets, not six"
            ),
        }
    }
}

impl std::error::Error for ProjectSerialNumberError {}

/// The serial number the project records for the device whose ETS id is
/// `device_ets_id` (`DeviceInstance/@Id`). `Ok(None)` when it records none.
pub fn project_serial_number(
    opaque: &[StoredOpaqueEntry],
    device_ets_id: &str,
) -> Result<Option<SerialNumber>, ProjectSerialNumberError> {
    let element = format!("/DeviceInstance[@Id='{device_ets_id}']");
    let Some(entry) = opaque.iter().find(|entry| {
        entry.kind == RETAINED_ATTRIBUTE_KIND
            && entry.name == SERIAL_NUMBER_ATTRIBUTE
            && entry.xpath.ends_with(&element)
    }) else {
        return Ok(None);
    };
    let text =
        std::str::from_utf8(&entry.bytes).map_err(|_| ProjectSerialNumberError::NotBase64)?;
    let octets = STANDARD
        .decode(text.trim())
        .map_err(|_| ProjectSerialNumberError::NotBase64)?;
    let octets: [u8; 6] = octets
        .as_slice()
        .try_into()
        .map_err(|_| ProjectSerialNumberError::WrongLength(octets.len()))?;
    Ok(Some(SerialNumber::from_octets(octets)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICE_XPATH: &str =
        "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance";

    fn row(xpath: &str, name: &str, value: &str) -> StoredOpaqueEntry {
        StoredOpaqueEntry {
            source_path: "P-0001/0.xml".to_owned(),
            xpath: xpath.to_owned(),
            kind: RETAINED_ATTRIBUTE_KIND.to_owned(),
            name: name.to_owned(),
            bytes: value.as_bytes().to_vec(),
            sha256: String::new(),
        }
    }

    fn device(id: &str) -> String {
        format!("{DEVICE_XPATH}[@Id='{id}']")
    }

    #[test]
    fn reads_the_named_devices_serial_number_and_no_other() {
        // 00 83 12 34 56 78, base64.
        let opaque = vec![
            row(
                &device("P-0001-0_DI-1"),
                SERIAL_NUMBER_ATTRIBUTE,
                "AIMSNFZ4",
            ),
            row(
                &device("P-0001-0_DI-2"),
                SERIAL_NUMBER_ATTRIBUTE,
                "AIMSNFZ5",
            ),
            row(&device("P-0001-0_DI-3"), "Comment", "AIMSNFZ4"),
        ];
        assert_eq!(
            project_serial_number(&opaque, "P-0001-0_DI-1")
                .unwrap()
                .map(|s| s.to_string()),
            Some("0083:12345678".to_owned())
        );
        assert_eq!(
            project_serial_number(&opaque, "P-0001-0_DI-2")
                .unwrap()
                .map(|s| s.to_string()),
            Some("0083:12345679".to_owned())
        );
        assert_eq!(
            project_serial_number(&opaque, "P-0001-0_DI-3").unwrap(),
            None
        );
        // A prefix of another id is not that id.
        assert_eq!(project_serial_number(&opaque, "P-0001-0_DI").unwrap(), None);
    }

    #[test]
    fn a_malformed_value_is_an_error_not_a_guess() {
        let id = "P-0001-0_DI-1";
        let bad = |value: &str| {
            project_serial_number(&[row(&device(id), SERIAL_NUMBER_ATTRIBUTE, value)], id)
        };
        assert_eq!(bad("not base64!"), Err(ProjectSerialNumberError::NotBase64));
        assert_eq!(
            bad("AIMSNFY="),
            Err(ProjectSerialNumberError::WrongLength(5))
        );
        assert_eq!(
            bad("AIMSNFZ4AA=="),
            Err(ProjectSerialNumberError::WrongLength(7))
        );
    }
}
