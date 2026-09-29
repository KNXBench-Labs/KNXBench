//! KNX-RF device configuration payloads, from CP §3.6–§3.7 and RES §4.
//!
//! KNX RF device configuration: the S-Mode interface of CP §3.6
//! (bidirectional) and §3.7 (unidirectional), K17.
//!
//! `[D]` `03_05_03 Configuration Procedures` §3.6–§3.7, pp. 57–60;
//! `03_05_01 Resources v01.10.01` §4.1.3 (Device Descriptor Type 2, pp.
//! 26–29), §4.3.14 (`PID_OBJECTLINK`, pp. 61–63) and §4.3.16
//! (`PID_PARAMETER`, pp. 67–68, read as rendered pages). No KNX-RF device
//! has been on this project's bus; everything here is simulator-tested
//! only (KNOWN_LIMITATIONS §144).
//!
//! What this module does **not** do, and why:
//! - It holds no table of E-Mode Channel codes or of the group objects each
//!   channel has. Those are defined per application in Volume 7 and belong
//!   in product data, not in code. [`pre_assigned_group_addresses`] takes
//!   them as input.
//! - It does not guess how several instances of one channel code number
//!   their group objects. CP §3.7.2.3's only example has one instance of
//!   each; a Channel Info with `number > 0` is refused.

use crate::{GroupAddress, IndividualAddress};

/// The Individual Address every RF unidirectional device uses: CP §3.7.3,
/// *"Unidirectional devices shall always use the Individual Address
/// 05FFh"* (`05h` the RF default subnetwork, `FFh` the device address).
pub const RF_UNIDIRECTIONAL_ADDRESS: IndividualAddress = IndividualAddress::from_raw(0x05FF);

/// The Device Descriptor Type (AL `descriptor_type`) of DD2.
pub const DEVICE_DESCRIPTOR_TYPE_2: u8 = 2;

/// RES Figure 2: DD2 is fourteen octets, *"All fields shall always be
/// provided, even if unused."*
pub const DD2_LENGTH: usize = 14;

/// `PID_OBJECTLINK` (RES §4.3.14), a function property of the Device
/// Object.
pub const PID_OBJECTLINK: u8 = 63;

/// `PID_PARAMETER` (RES §4.3.16), a function property of the Device
/// Object, *"Used by: mask 2010h, RF bidirectional devices"*.
pub const PID_PARAMETER: u8 = 65;

/// The Device Object's index. The function properties above live there
/// (CP §3.6.5: *"object_type = Device Object"*).
pub const DEVICE_OBJECT_INDEX: u8 = 0;

/// Octet 5 of DD2, RES Table 8: Management Profile (bits 7-4) and the
/// reserved bits (3-0), judged as one value because the table does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagementProfile {
    /// `00000000`: Easy Ctrl fixed/reloc DMA, no link services.
    EasyCtrl,
    /// `00111111`: the reserved Management Profile. CP §3.7.2.3's Example
    /// 16 carries it (its `Link mode 00` and `LT_Base 3Fh` are this octet).
    Reserved,
    /// `01000000`: Easy Ctrl with link management services.
    EasyCtrlWithLinks,
    /// `10000000`: a Ctrl FEC or PB FEC device; its Channel Infos are all
    /// `0000h`.
    Fec,
    /// Any other value. RES: *"If these bits do not have the specified
    /// value then the Management Client should not change the device."*
    Invalid(u8),
}

impl ManagementProfile {
    pub fn from_octet(octet: u8) -> Self {
        match octet {
            0x00 => Self::EasyCtrl,
            0x3F => Self::Reserved,
            0x40 => Self::EasyCtrlWithLinks,
            0x80 => Self::Fec,
            other => Self::Invalid(other),
        }
    }

    /// Whether RES allows a Management Client to change this device.
    pub fn may_be_changed(self) -> bool {
        !matches!(self, Self::Invalid(_))
    }
}

/// One Channel Info of DD2 (RES §4.1.3, U3U13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelInfo {
    /// How many E-Mode Channels of this code the device has: the 3-bit
    /// field plus one (*"Value 0 shall mean 1 such E-Mode Channel"*).
    pub count: u8,
    /// The E-Mode Channel Code (13 bits). `0000h` means unused, *"then no
    /// more valid data afterwards"* (RES Table 9).
    pub code: u16,
}

impl ChannelInfo {
    pub fn from_u16(raw: u16) -> Self {
        Self {
            count: (raw >> 13) as u8 + 1,
            code: raw & 0x1FFF,
        }
    }
}

/// A decoded Device Descriptor Type 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDescriptor2 {
    pub application_manufacturer: u16,
    /// *"This field may be named 'Device Type' in other publications."*
    pub application_identification: u16,
    pub application_version: u8,
    pub management: ManagementProfile,
    pub channels: [ChannelInfo; 4],
}

/// Why DD2 octets were not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dd2Error {
    /// Not the fourteen octets RES Figure 2 requires.
    Length(usize),
}

impl DeviceDescriptor2 {
    pub fn from_octets(octets: &[u8]) -> Result<Self, Dd2Error> {
        if octets.len() != DD2_LENGTH {
            return Err(Dd2Error::Length(octets.len()));
        }
        let be = |i: usize| u16::from_be_bytes([octets[i], octets[i + 1]]);
        Ok(Self {
            application_manufacturer: be(0),
            application_identification: be(2),
            application_version: octets[4],
            management: ManagementProfile::from_octet(octets[5]),
            channels: [
                ChannelInfo::from_u16(be(6)),
                ChannelInfo::from_u16(be(8)),
                ChannelInfo::from_u16(be(10)),
                ChannelInfo::from_u16(be(12)),
            ],
        })
    }

    /// The channels in use: those before the first `0000h` code.
    pub fn used_channels(&self) -> impl Iterator<Item = ChannelInfo> + '_ {
        self.channels.iter().copied().take_while(|c| c.code != 0)
    }
}

/// One pre-assigned group address of an RF unidirectional device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreAssignedGroupAddress {
    /// The group object number, from 1 (CP Example 16's *"GO number"*).
    pub group_object: u16,
    /// Which Channel Info (0–3) it comes from.
    pub channel_info: usize,
    /// The E-Mode Channel Code.
    pub channel_code: u16,
    /// The GA part of the Extended Group Address; the device's KNX Serial
    /// Number is the other part (CP §3.7.5.4.3).
    pub group_address: GroupAddress,
}

/// Why [`pre_assigned_group_addresses`] could not calculate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupAddressCalculationError {
    /// A FEC device has no E-Mode Channels to calculate from.
    NoChannels(ManagementProfile),
    /// The caller's channel definitions do not know this code.
    UnknownChannelCode(u16),
    /// Several instances of one channel code: CP §3.7.2.3 does not show how
    /// their group objects are numbered, so KNXBench does not guess.
    SeveralInstances { code: u16, count: u8 },
    /// More group objects than a 16-bit group address can number.
    Overflow,
}

/// CP §3.7.2.3: the pre-assigned group addresses, *"starting from 0001h
/// using the object sequence defined in the E-Mode Channels … and the
/// E-Mode Channel number in the DD2"*. Every group object gets one, *"Even
/// if an input Group Object is defined in a unidirectional sensor"*.
///
/// `objects_in_channel(code)` is the number of group objects the channel
/// definition gives that code (Volume 7), or `None` when unknown.
pub fn pre_assigned_group_addresses(
    dd2: &DeviceDescriptor2,
    objects_in_channel: impl Fn(u16) -> Option<u16>,
) -> Result<Vec<PreAssignedGroupAddress>, GroupAddressCalculationError> {
    if dd2.management == ManagementProfile::Fec {
        return Err(GroupAddressCalculationError::NoChannels(dd2.management));
    }
    let mut out = Vec::new();
    let mut next: u16 = 1;
    for (index, channel) in dd2.used_channels().enumerate() {
        if channel.count != 1 {
            return Err(GroupAddressCalculationError::SeveralInstances {
                code: channel.code,
                count: channel.count,
            });
        }
        let objects = objects_in_channel(channel.code).ok_or(
            GroupAddressCalculationError::UnknownChannelCode(channel.code),
        )?;
        for _ in 0..objects {
            out.push(PreAssignedGroupAddress {
                group_object: next,
                channel_info: index,
                channel_code: channel.code,
                group_address: GroupAddress::from_raw(next),
            });
            next = next
                .checked_add(1)
                .ok_or(GroupAddressCalculationError::Overflow)?;
        }
    }
    Ok(out)
}

/// What a link names: RES §4.3.14.1.2 field `aet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkAddress {
    /// `aet = 0`: an Extended Group Address, the KNX Serial Number of its
    /// sender plus a group address (*"useful for RF with SN"*).
    Extended {
        serial_number: [u8; 6],
        group_address: GroupAddress,
    },
    /// `aet = 1`: a plain group address; the SN field is then all zero
    /// (*"useful on TP, on PL with DoA and on RF with DoA"*).
    Group(GroupAddress),
}

/// Add or delete, RES flag `d`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkChange {
    /// Add; `sending` is flag `s`.
    Add { sending: bool },
    /// Delete; flag `s` is *"don't care"* and sent as 0.
    Delete,
}

/// The data of a Function Write Object Link, RES Figure 4, octets 10–21:
/// Flags, `00h`, SN (6), GA (2), group object index (2).
pub fn object_link_write(change: LinkChange, address: LinkAddress, group_object: u16) -> Vec<u8> {
    let (sending, delete) = match change {
        LinkChange::Add { sending } => (sending, false),
        LinkChange::Delete => (false, true),
    };
    let (aet, serial, group) = match address {
        LinkAddress::Extended {
            serial_number,
            group_address,
        } => (false, serial_number, group_address),
        LinkAddress::Group(group_address) => (true, [0; 6], group_address),
    };
    let flags = u8::from(sending) | (u8::from(delete) << 1) | (u8::from(aet) << 2);
    let mut data = vec![flags, 0x00];
    data.extend_from_slice(&serial);
    data.extend_from_slice(&group.raw().to_be_bytes());
    data.extend_from_slice(&group_object.to_be_bytes());
    data
}

/// The Return Code of a Function Write Object Link, RES Figure 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectLinkReturn {
    /// `00h`: *"Link established or broken or already established or
    /// broken before."*
    Success,
    /// `FFh`: no memory for another link.
    TableFull,
    /// `FEh`: *"Group Object does not exist or is out of range"*.
    GroupObject,
    /// `FDh`: SN not zero with `aet = 1`.
    Aet,
    /// `FCh`: any other error.
    Generic,
    /// A code RES does not list.
    Other(u8),
}

impl ObjectLinkReturn {
    pub fn from_octet(octet: u8) -> Self {
        match octet {
            0x00 => Self::Success,
            0xFF => Self::TableFull,
            0xFE => Self::GroupObject,
            0xFD => Self::Aet,
            0xFC => Self::Generic,
            other => Self::Other(other),
        }
    }
}

/// The data of a Function Write Parameter (RES §4.3.16 a): E-Mode Channel
/// Number, Parameter Number, value octets. Both numbers are one octet.
pub fn parameter_write(channel: u8, parameter: u8, value: &[u8]) -> Vec<u8> {
    let mut data = vec![channel, parameter];
    data.extend_from_slice(value);
    data
}

/// The data of a Function Read Parameter (RES §4.3.16 b).
pub fn parameter_read(channel: u8, parameter: u8) -> Vec<u8> {
    vec![channel, parameter]
}

/// A Read Parameter response, RES §4.3.16 b): Return Code, `00h`, value.
/// On `FFh` *"the … Response-PDU shall end at octet 10"*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterReadResult {
    Value(Vec<u8>),
    /// `FFh`, *"ERROR (invalid parameter)"*.
    InvalidParameter,
    /// Octets that fit neither shape, kept.
    Malformed {
        return_code: u8,
        data: Vec<u8>,
    },
}

/// Reads the `data` after the return code of a Read Parameter response.
pub fn parameter_read_result(return_code: u8, data: &[u8]) -> ParameterReadResult {
    match (return_code, data) {
        (0x00, [0x00, value @ ..]) if !value.is_empty() => {
            ParameterReadResult::Value(value.to_vec())
        }
        (0xFF, []) => ParameterReadResult::InvalidParameter,
        _ => ParameterReadResult::Malformed {
            return_code,
            data: data.to_vec(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CP §3.7.2.3 Example 16, with manufacturer 09h as two octets and
    /// `Link mode 00` + `LT_Base 3Fh` as octet 5.
    const EXAMPLE_16: [u8; 14] = [
        0x00, 0x09, 0x30, 0x00, 0x10, 0x3F, 0x00, 0x08, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn example_16_decodes_to_its_table() {
        let dd2 = DeviceDescriptor2::from_octets(&EXAMPLE_16).unwrap();
        assert_eq!(dd2.application_manufacturer, 0x0009);
        assert_eq!(dd2.application_identification, 0x3000);
        assert_eq!(dd2.application_version, 0x10);
        assert_eq!(dd2.management, ManagementProfile::Reserved);
        let used: Vec<_> = dd2.used_channels().collect();
        assert_eq!(
            used,
            vec![
                ChannelInfo {
                    count: 1,
                    code: 0x0008
                },
                ChannelInfo {
                    count: 1,
                    code: 0x000E
                },
            ]
        );
    }

    #[test]
    fn example_16_calculates_its_five_group_addresses() {
        let dd2 = DeviceDescriptor2::from_octets(&EXAMPLE_16).unwrap();
        // The example's own object lists: CH_PB_Scene has Scene Activate
        // and Scene Learn, CH_Switch_Dimmer_Toggle has Info OnOff, OnOff
        // and DimmingCtrl.
        let gas = pre_assigned_group_addresses(&dd2, |code| match code {
            0x0008 => Some(2),
            0x000E => Some(3),
            _ => None,
        })
        .unwrap();
        let raw: Vec<u16> = gas.iter().map(|g| g.group_address.raw()).collect();
        assert_eq!(raw, vec![1, 2, 3, 4, 5]);
        assert_eq!(gas[2].channel_code, 0x000E, "Info OnOff, GO 3");
        assert_eq!(gas[2].group_object, 3);
    }

    #[test]
    fn nothing_is_guessed() {
        let dd2 = DeviceDescriptor2::from_octets(&EXAMPLE_16).unwrap();
        assert_eq!(
            pre_assigned_group_addresses(&dd2, |_| None),
            Err(GroupAddressCalculationError::UnknownChannelCode(0x0008))
        );
        let mut twice = EXAMPLE_16;
        twice[6] = 0x20; // number = 1 → two instances
        let dd2 = DeviceDescriptor2::from_octets(&twice).unwrap();
        assert_eq!(
            pre_assigned_group_addresses(&dd2, |_| Some(2)),
            Err(GroupAddressCalculationError::SeveralInstances {
                code: 0x0008,
                count: 2
            })
        );
        let mut fec = EXAMPLE_16;
        fec[5] = 0x80;
        let dd2 = DeviceDescriptor2::from_octets(&fec).unwrap();
        assert!(matches!(
            pre_assigned_group_addresses(&dd2, |_| Some(1)),
            Err(GroupAddressCalculationError::NoChannels(_))
        ));
    }

    #[test]
    fn a_wrong_length_and_bad_reserved_bits_are_caught() {
        assert_eq!(
            DeviceDescriptor2::from_octets(&EXAMPLE_16[..13]),
            Err(Dd2Error::Length(13))
        );
        assert!(!ManagementProfile::from_octet(0x41).may_be_changed());
        for ok in [0x00, 0x3F, 0x40, 0x80] {
            assert!(ManagementProfile::from_octet(ok).may_be_changed());
        }
    }

    #[test]
    fn the_unidirectional_address_is_05ffh() {
        assert_eq!(RF_UNIDIRECTIONAL_ADDRESS.raw(), 0x05FF);
    }

    #[test]
    fn object_link_writes_have_figure_4s_shape() {
        let serial = [0x00, 0x09, 0x01, 0x02, 0x03, 0x04];
        let add = object_link_write(
            LinkChange::Add { sending: true },
            LinkAddress::Extended {
                serial_number: serial,
                group_address: GroupAddress::from_raw(0x0004),
            },
            4,
        );
        assert_eq!(add.len(), 12);
        assert_eq!(add[0], 0b001, "s = 1, d = 0, aet = 0");
        assert_eq!(add[1], 0x00);
        assert_eq!(&add[2..8], &serial);
        assert_eq!(&add[8..], &[0x00, 0x04, 0x00, 0x04]);

        let delete = object_link_write(
            LinkChange::Delete,
            LinkAddress::Group(GroupAddress::from_raw(0x0A01)),
            1,
        );
        assert_eq!(delete[0], 0b110, "s = 0, d = 1, aet = 1");
        assert_eq!(&delete[2..8], &[0; 6], "SN zero with aet = 1");
        assert_eq!(&delete[8..10], &[0x0A, 0x01]);
    }

    #[test]
    fn object_link_return_codes_are_the_table() {
        assert_eq!(
            ObjectLinkReturn::from_octet(0x00),
            ObjectLinkReturn::Success
        );
        assert_eq!(
            ObjectLinkReturn::from_octet(0xFF),
            ObjectLinkReturn::TableFull
        );
        assert_eq!(
            ObjectLinkReturn::from_octet(0xFE),
            ObjectLinkReturn::GroupObject
        );
        assert_eq!(ObjectLinkReturn::from_octet(0xFD), ObjectLinkReturn::Aet);
        assert_eq!(
            ObjectLinkReturn::from_octet(0xFC),
            ObjectLinkReturn::Generic
        );
        assert_eq!(
            ObjectLinkReturn::from_octet(0x01),
            ObjectLinkReturn::Other(0x01)
        );
    }

    #[test]
    fn parameter_payloads_and_answers() {
        assert_eq!(parameter_write(2, 7, &[0x12, 0x34]), vec![2, 7, 0x12, 0x34]);
        assert_eq!(parameter_read(2, 7), vec![2, 7]);
        assert_eq!(
            parameter_read_result(0x00, &[0x00, 0x12]),
            ParameterReadResult::Value(vec![0x12])
        );
        assert_eq!(
            parameter_read_result(0xFF, &[]),
            ParameterReadResult::InvalidParameter
        );
        // An error code with a value after it is not the shape RES gives.
        assert!(matches!(
            parameter_read_result(0xFF, &[0x00, 0x01]),
            ParameterReadResult::Malformed { .. }
        ));
        assert!(matches!(
            parameter_read_result(0x00, &[0x01, 0x12]),
            ParameterReadResult::Malformed { .. }
        ));
    }
}
