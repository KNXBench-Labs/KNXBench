//! Description Information Blocks carried in a `SEARCH_RESPONSE`: Device
//! Info (mandatory, Core v01.06.02 AS §7.5.4.2) and Supported Service
//! Families (optional, §7.5.4.3). Pure byte <-> struct, no IO.

use std::net::Ipv4Addr;

use knx_core::IndividualAddress;

pub const DEVICE_INFO: u8 = 0x01;
pub const SUPP_SVC_FAMILIES: u8 = 0x02;

/// KNXnet/IP Tunnelling service family ID (Core v01.06.02 AS §7.5.4.3,
/// Service Family IDs table) — the one `DiscoveredGateway::supports_tunnelling`
/// checks for.
pub const SERVICE_FAMILY_TUNNELLING: u8 = 0x04;

pub const DEVICE_INFO_LEN: u8 = 54;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub medium: u8,
    pub status: u8,
    pub individual_address: IndividualAddress,
    pub project_installation_id: u16,
    pub serial_number: [u8; 6],
    pub routing_multicast: Ipv4Addr,
    pub mac_address: [u8; 6],
    pub friendly_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFamily {
    pub id: u8,
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServiceFamilies(pub Vec<ServiceFamily>);

impl ServiceFamilies {
    pub fn supports(&self, family_id: u8) -> bool {
        self.0.iter().any(|f| f.id == family_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DibError {
    TooShort { needed: usize, got: usize },
    BadLength(u8),
    UnexpectedType { expected: u8, got: u8 },
    OddServiceFamiliesLength(u8),
}

impl std::fmt::Display for DibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DibError::TooShort { needed, got } => {
                write!(f, "DIB too short: needed {needed} octets, got {got}")
            }
            DibError::BadLength(len) => {
                write!(f, "unexpected DIB structure length {len:#04x}")
            }
            DibError::UnexpectedType { expected, got } => {
                write!(
                    f,
                    "unexpected DIB description type {got:#04x}, expected {expected:#04x}"
                )
            }
            DibError::OddServiceFamiliesLength(len) => {
                write!(
                    f,
                    "Supported Service Families DIB length {len:#04x} does not hold a whole number of (ID, version) pairs"
                )
            }
        }
    }
}

impl std::error::Error for DibError {}

/// Decodes a Device Info DIB (Core v01.06.02 AS §7.5.4.2, Figure 40) from
/// the start of `buf` — structure length, type code `DEVICE_INFO`, KNX
/// medium, device status, individual address, project-installation
/// identifier, serial number, routing multicast address, MAC address, and
/// a 30-octet friendly name (ISO 8859-1, zero-padded; trailing zeros are
/// trimmed). Returns the decoded struct along with whatever octets follow
/// it in `buf`.
pub fn decode_device_info(buf: &[u8]) -> Result<(DeviceInfo, &[u8]), DibError> {
    if buf.len() < DEVICE_INFO_LEN as usize {
        return Err(DibError::TooShort {
            needed: DEVICE_INFO_LEN as usize,
            got: buf.len(),
        });
    }
    let len = buf[0];
    if len != DEVICE_INFO_LEN {
        return Err(DibError::BadLength(len));
    }
    let type_code = buf[1];
    if type_code != DEVICE_INFO {
        return Err(DibError::UnexpectedType {
            expected: DEVICE_INFO,
            got: type_code,
        });
    }
    let medium = buf[2];
    let status = buf[3];
    let individual_address = IndividualAddress::from_raw(u16::from_be_bytes([buf[4], buf[5]]));
    let project_installation_id = u16::from_be_bytes([buf[6], buf[7]]);
    let serial_number: [u8; 6] = buf[8..14].try_into().unwrap();
    let routing_multicast = Ipv4Addr::new(buf[14], buf[15], buf[16], buf[17]);
    let mac_address: [u8; 6] = buf[18..24].try_into().unwrap();
    let name_bytes = &buf[24..54];
    let name_end = name_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_bytes.len());
    let friendly_name = name_bytes[..name_end].iter().map(|&b| b as char).collect();

    Ok((
        DeviceInfo {
            medium,
            status,
            individual_address,
            project_installation_id,
            serial_number,
            routing_multicast,
            mac_address,
            friendly_name,
        },
        &buf[54..],
    ))
}

/// Decodes a Supported Service Families DIB (§7.5.4.3, Figure 41):
/// structure length, type code `SUPP_SVC_FAMILIES`, then `(len - 2) / 2`
/// pairs of (Family ID, Family Version). Returns the decoded list along
/// with whatever octets follow it in `buf`.
pub fn decode_service_families(buf: &[u8]) -> Result<(ServiceFamilies, &[u8]), DibError> {
    if buf.len() < 2 {
        return Err(DibError::TooShort {
            needed: 2,
            got: buf.len(),
        });
    }
    let len = buf[0];
    if buf.len() < len as usize {
        return Err(DibError::TooShort {
            needed: len as usize,
            got: buf.len(),
        });
    }
    let type_code = buf[1];
    if type_code != SUPP_SVC_FAMILIES {
        return Err(DibError::UnexpectedType {
            expected: SUPP_SVC_FAMILIES,
            got: type_code,
        });
    }
    if len < 2 {
        return Err(DibError::BadLength(len));
    }
    let pairs_len = len - 2;
    if !pairs_len.is_multiple_of(2) {
        return Err(DibError::OddServiceFamiliesLength(len));
    }
    let pairs = &buf[2..len as usize];
    let families = pairs
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| ServiceFamily {
            id: pair[0],
            version: pair[1],
        })
        .collect();
    Ok((ServiceFamilies(families), &buf[len as usize..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn device_info_bytes() -> Vec<u8> {
        let mut b = vec![
            0x36,
            DEVICE_INFO, // structure length 54, type 0x01
            0x02,        // KNX medium: TP1
            0x00,        // device status
            0x11,
            0x01, // individual address 1.1.1
            0x00,
            0x00, // project-installation identifier
            0x00,
            0xFA,
            0x12,
            0x34,
            0x56,
            0x78, // serial number
            224,
            0,
            23,
            12, // routing multicast address
            0x00,
            0x01,
            0x02,
            0x03,
            0x04,
            0x05, // MAC address
        ];
        let mut name = b"KNX IP Gateway".to_vec();
        name.resize(30, 0); // zero-padded to 30 octets
        b.extend_from_slice(&name);
        assert_eq!(b.len(), 54);
        b
    }

    #[test]
    fn decode_device_info_reads_every_field() {
        let bytes = device_info_bytes();
        let (info, rest) = decode_device_info(&bytes).unwrap();
        assert_eq!(info.medium, 0x02);
        assert_eq!(info.status, 0x00);
        assert_eq!(info.individual_address, IndividualAddress::from_raw(0x1101));
        assert_eq!(info.project_installation_id, 0);
        assert_eq!(info.serial_number, [0x00, 0xFA, 0x12, 0x34, 0x56, 0x78]);
        assert_eq!(info.routing_multicast, Ipv4Addr::new(224, 0, 23, 12));
        assert_eq!(info.mac_address, [0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(info.friendly_name, "KNX IP Gateway");
        assert!(rest.is_empty());
    }

    #[test]
    fn decode_device_info_leaves_trailing_bytes_for_the_caller() {
        let mut bytes = device_info_bytes();
        bytes.extend_from_slice(&[0xFF, 0xEE]);
        let (_, rest) = decode_device_info(&bytes).unwrap();
        assert_eq!(rest, &[0xFF, 0xEE]);
    }

    #[test]
    fn decode_device_info_rejects_short_buffer() {
        let err = decode_device_info(&[0x36, DEVICE_INFO, 0x02]).unwrap_err();
        assert_eq!(err, DibError::TooShort { needed: 54, got: 3 });
    }

    #[test]
    fn decode_device_info_rejects_bad_length() {
        let mut bytes = device_info_bytes();
        bytes[0] = 0x37;
        let err = decode_device_info(&bytes).unwrap_err();
        assert_eq!(err, DibError::BadLength(0x37));
    }

    #[test]
    fn decode_device_info_rejects_wrong_type() {
        let mut bytes = device_info_bytes();
        bytes[1] = SUPP_SVC_FAMILIES;
        let err = decode_device_info(&bytes).unwrap_err();
        assert_eq!(
            err,
            DibError::UnexpectedType {
                expected: DEVICE_INFO,
                got: SUPP_SVC_FAMILIES
            }
        );
    }

    #[test]
    fn decode_service_families_reads_every_pair_and_reports_tunnelling_support() {
        let bytes = [
            0x06,
            SUPP_SVC_FAMILIES,
            0x02,
            0x01,
            SERVICE_FAMILY_TUNNELLING,
            0x01,
        ];
        let (families, rest) = decode_service_families(&bytes).unwrap();
        assert_eq!(
            families,
            ServiceFamilies(vec![
                ServiceFamily {
                    id: 0x02,
                    version: 0x01
                },
                ServiceFamily {
                    id: SERVICE_FAMILY_TUNNELLING,
                    version: 0x01
                },
            ])
        );
        assert!(families.supports(SERVICE_FAMILY_TUNNELLING));
        assert!(!families.supports(0x05));
        assert!(rest.is_empty());
    }

    #[test]
    fn decode_service_families_rejects_odd_length() {
        let bytes = [0x05, SUPP_SVC_FAMILIES, 0x02, 0x01, 0x00];
        let err = decode_service_families(&bytes).unwrap_err();
        assert_eq!(err, DibError::OddServiceFamiliesLength(0x05));
    }

    #[test]
    fn decode_service_families_rejects_wrong_type() {
        let bytes = [0x04, DEVICE_INFO, 0x02, 0x01];
        let err = decode_service_families(&bytes).unwrap_err();
        assert_eq!(
            err,
            DibError::UnexpectedType {
                expected: SUPP_SVC_FAMILIES,
                got: DEVICE_INFO
            }
        );
    }

    #[test]
    fn decode_service_families_rejects_len_zero() {
        let bytes = [0x00, SUPP_SVC_FAMILIES];
        let err = decode_service_families(&bytes).unwrap_err();
        assert_eq!(err, DibError::BadLength(0x00));
    }

    #[test]
    fn decode_service_families_rejects_len_one() {
        let bytes = [0x01, SUPP_SVC_FAMILIES];
        let err = decode_service_families(&bytes).unwrap_err();
        assert_eq!(err, DibError::BadLength(0x01));
    }
}
