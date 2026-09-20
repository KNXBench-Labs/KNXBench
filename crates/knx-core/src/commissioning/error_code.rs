//! `PID_ERROR_CODE`'s values: naming for `DPT_ErrorClass_System` 20.011.
//!
//! The octet is decoded by the shared DPT codec, which carries no
//! per-subtype table for main type 20 on purpose (`dpt/codec.rs`, and
//! `KNOWN_LIMITATIONS.md` §61). This module is the caller that codec
//! comment names: it takes the codec's `DptValue::Enum { code }` and
//! supplies the 20.011 names from design spec §5.6. Nothing here re-decodes an
//! octet the codec already decodes.

use std::fmt;

use crate::dpt::codec::{decode, DptCodecError, DptValue};
use crate::dpt::{DptRef, GroupValue};

/// The datapoint type `PID_ERROR_CODE` (PID 28, `PDT_ENUM8`) carries.
pub const ERROR_CLASS_SYSTEM: DptRef = DptRef {
    main: 20,
    sub: Some(11),
};

/// A `DPT_ErrorClass_System` 20.011 value, `[D]` DPT clause 3, ID 20.011,
/// `Range: [0 to 18]` (design spec §5.6).
///
/// The names are the Standard's own wording, shortened only by dropping
/// its parenthetical examples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SystemErrorClass {
    /// 0 — *"no fault"*.
    NoFault,
    /// 1 — *"general device fault (e.g. RAM, EEPROM, UI, watchdog, …)"*.
    GeneralDeviceFault,
    /// 2 — *"communication fault"*.
    CommunicationFault,
    /// 3 — *"configuration fault"*.
    ConfigurationFault,
    /// 4 — *"hardware fault"*.
    HardwareFault,
    /// 5 — *"software fault"*.
    SoftwareFault,
    /// 6 — *"insufficient non volatile memory"*. A download diagnostic:
    /// the device is too small for what was sent.
    InsufficientNonVolatileMemory,
    /// 7 — *"insufficient volatile memory"*.
    InsufficientVolatileMemory,
    /// 8 — *"memory allocation command with size 0 received"*. A download
    /// diagnostic: the client's own allocation was malformed.
    MemoryAllocationWithSizeZero,
    /// 9 — *"CRC-error"*. A download diagnostic: the data arrived
    /// corrupted.
    CrcError,
    /// 10 — *"watchdog reset detected"*.
    WatchdogResetDetected,
    /// 11 — *"invalid opcode detected"*.
    InvalidOpcodeDetected,
    /// 12 — *"general protection fault"*.
    GeneralProtectionFault,
    /// 13 — *"maximal table length exceeded"*. A download diagnostic: the
    /// table the client wrote is longer than the device allows.
    MaximalTableLengthExceeded,
    /// 14 — *"undefined load command received"*. A download diagnostic:
    /// the device does not know the load control that was sent.
    UndefinedLoadCommandReceived,
    /// 15 — *"Group Address Table is not sorted"*. RES §4.16.3 requires
    /// ascending order with increasing memory locations, so a client that
    /// writes an unsorted table produces this at runtime.
    GroupAddressTableNotSorted,
    /// 16 — *"invalid connection number (TSAP)"*.
    InvalidConnectionNumber,
    /// 17 — *"invalid Group Object number (ASAP)"*.
    InvalidGroupObjectNumber,
    /// 18 — *"Group Object Type exceeds (PID_MAX_APDU_LENGTH – 2)"*.
    GroupObjectTypeTooLarge,
}

impl SystemErrorClass {
    /// Names an octet, or reports it unknown. 19–255 are *"reserved,
    /// shall not be used"* and are surfaced rather than mapped onto a
    /// neighbouring name (design spec §5.6, §14 item 17).
    pub fn from_code(code: u8) -> Result<Self, ReservedErrorClass> {
        use SystemErrorClass::*;
        Ok(match code {
            0 => NoFault,
            1 => GeneralDeviceFault,
            2 => CommunicationFault,
            3 => ConfigurationFault,
            4 => HardwareFault,
            5 => SoftwareFault,
            6 => InsufficientNonVolatileMemory,
            7 => InsufficientVolatileMemory,
            8 => MemoryAllocationWithSizeZero,
            9 => CrcError,
            10 => WatchdogResetDetected,
            11 => InvalidOpcodeDetected,
            12 => GeneralProtectionFault,
            13 => MaximalTableLengthExceeded,
            14 => UndefinedLoadCommandReceived,
            15 => GroupAddressTableNotSorted,
            16 => InvalidConnectionNumber,
            17 => InvalidGroupObjectNumber,
            18 => GroupObjectTypeTooLarge,
            reserved => return Err(ReservedErrorClass(reserved)),
        })
    }

    /// The octet this class is encoded as.
    pub fn code(self) -> u8 {
        use SystemErrorClass::*;
        match self {
            NoFault => 0,
            GeneralDeviceFault => 1,
            CommunicationFault => 2,
            ConfigurationFault => 3,
            HardwareFault => 4,
            SoftwareFault => 5,
            InsufficientNonVolatileMemory => 6,
            InsufficientVolatileMemory => 7,
            MemoryAllocationWithSizeZero => 8,
            CrcError => 9,
            WatchdogResetDetected => 10,
            InvalidOpcodeDetected => 11,
            GeneralProtectionFault => 12,
            MaximalTableLengthExceeded => 13,
            UndefinedLoadCommandReceived => 14,
            GroupAddressTableNotSorted => 15,
            InvalidConnectionNumber => 16,
            InvalidGroupObjectNumber => 17,
            GroupObjectTypeTooLarge => 18,
        }
    }

    /// Whether this value tells the client that *its own* request was
    /// wrong or too large, rather than reporting a device fault. Spec
    /// §5.6: values 6, 8, 9, 13 and 14 are download diagnostics, and only
    /// 6 is the user's problem.
    pub fn is_download_diagnostic(self) -> bool {
        use SystemErrorClass::*;
        matches!(
            self,
            InsufficientNonVolatileMemory
                | MemoryAllocationWithSizeZero
                | CrcError
                | MaximalTableLengthExceeded
                | UndefinedLoadCommandReceived
        )
    }

    /// Every named value, in code order.
    pub const ALL: [SystemErrorClass; 19] = [
        SystemErrorClass::NoFault,
        SystemErrorClass::GeneralDeviceFault,
        SystemErrorClass::CommunicationFault,
        SystemErrorClass::ConfigurationFault,
        SystemErrorClass::HardwareFault,
        SystemErrorClass::SoftwareFault,
        SystemErrorClass::InsufficientNonVolatileMemory,
        SystemErrorClass::InsufficientVolatileMemory,
        SystemErrorClass::MemoryAllocationWithSizeZero,
        SystemErrorClass::CrcError,
        SystemErrorClass::WatchdogResetDetected,
        SystemErrorClass::InvalidOpcodeDetected,
        SystemErrorClass::GeneralProtectionFault,
        SystemErrorClass::MaximalTableLengthExceeded,
        SystemErrorClass::UndefinedLoadCommandReceived,
        SystemErrorClass::GroupAddressTableNotSorted,
        SystemErrorClass::InvalidConnectionNumber,
        SystemErrorClass::InvalidGroupObjectNumber,
        SystemErrorClass::GroupObjectTypeTooLarge,
    ];
}

impl fmt::Display for SystemErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SystemErrorClass::*;
        let text = match self {
            NoFault => "no fault",
            GeneralDeviceFault => "general device fault",
            CommunicationFault => "communication fault",
            ConfigurationFault => "configuration fault",
            HardwareFault => "hardware fault",
            SoftwareFault => "software fault",
            InsufficientNonVolatileMemory => "insufficient non volatile memory",
            InsufficientVolatileMemory => "insufficient volatile memory",
            MemoryAllocationWithSizeZero => "memory allocation command with size 0 received",
            CrcError => "CRC-error",
            WatchdogResetDetected => "watchdog reset detected",
            InvalidOpcodeDetected => "invalid opcode detected",
            GeneralProtectionFault => "general protection fault",
            MaximalTableLengthExceeded => "maximal table length exceeded",
            UndefinedLoadCommandReceived => "undefined load command received",
            GroupAddressTableNotSorted => "Group Address Table is not sorted",
            InvalidConnectionNumber => "invalid connection number (TSAP)",
            InvalidGroupObjectNumber => "invalid Group Object number (ASAP)",
            GroupObjectTypeTooLarge => "Group Object Type exceeds (PID_MAX_APDU_LENGTH - 2)",
        };
        f.write_str(text)
    }
}

/// A `PID_ERROR_CODE` octet in 19–255, which DPT 20.011 calls *"reserved,
/// shall not be used"*. Reported as itself; the reserved boundary differs
/// per subtype, so there is no neighbouring name to fall back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReservedErrorClass(pub u8);

impl std::error::Error for ReservedErrorClass {}

impl fmt::Display for ReservedErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error class {} is reserved in DPT 20.011, whose range is [0 to 18]",
            self.0
        )
    }
}

/// Why a `PID_ERROR_CODE` payload could not be named.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCodeReadError {
    /// The shared DPT codec refused the payload.
    Codec(DptCodecError),
    /// The codec produced something other than an enumeration, which
    /// would mean 20.011 stopped being an `N8`.
    NotAnEnumeration,
    /// The octet decoded, but 20.011 reserves that value.
    Reserved(ReservedErrorClass),
}

impl std::error::Error for ErrorCodeReadError {}

impl fmt::Display for ErrorCodeReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCodeReadError::Codec(err) => write!(f, "{err}"),
            ErrorCodeReadError::NotAnEnumeration => {
                write!(f, "DPT 20.011 did not decode to an enumeration")
            }
            ErrorCodeReadError::Reserved(err) => write!(f, "{err}"),
        }
    }
}

/// Decodes and names a `PID_ERROR_CODE` payload.
///
/// The decoding half is the shared codec's `decode(DPST-20-11, …)`, not a
/// second copy of it: §14 item 17 asks for exactly that, and the codec's
/// own comment nominates this module as the caller that supplies the
/// names it deliberately does not carry.
pub fn read_error_code(payload: &GroupValue) -> Result<SystemErrorClass, ErrorCodeReadError> {
    let value = decode(ERROR_CLASS_SYSTEM, payload).map_err(ErrorCodeReadError::Codec)?;
    let DptValue::Enum { code } = value else {
        return Err(ErrorCodeReadError::NotAnEnumeration);
    };
    SystemErrorClass::from_code(code).map_err(ErrorCodeReadError::Reserved)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §14 item 17: 0–18 decode to their names, 19–255 are surfaced as
    /// unknown rather than mapped, and the decoder is the shared one.
    #[test]
    fn codes_0_to_18_round_trip_through_the_shared_codec() {
        for code in 0..=18u8 {
            let named = SystemErrorClass::from_code(code).unwrap();
            assert_eq!(named.code(), code);
            let from_wire = read_error_code(&GroupValue::Bytes(vec![code])).unwrap();
            assert_eq!(from_wire, named);
            assert!(!named.to_string().is_empty());
        }
        assert_eq!(SystemErrorClass::ALL.len(), 19);
    }

    #[test]
    fn codes_19_to_255_are_surfaced_as_reserved_not_mapped_to_a_name() {
        for code in 19..=255u8 {
            assert_eq!(
                SystemErrorClass::from_code(code),
                Err(ReservedErrorClass(code))
            );
            assert_eq!(
                read_error_code(&GroupValue::Bytes(vec![code])),
                Err(ErrorCodeReadError::Reserved(ReservedErrorClass(code)))
            );
        }
    }

    #[test]
    fn the_decoder_is_the_shared_dpt_codec_and_not_a_second_copy() {
        // Proven by behaviour rather than by inspection: a payload the
        // shared codec rejects for length must be rejected here too, with
        // the codec's own error. A private re-implementation would have
        // to reproduce that, and would not.
        let too_long = GroupValue::Bytes(vec![1, 2]);
        let direct = decode(ERROR_CLASS_SYSTEM, &too_long).unwrap_err();
        assert_eq!(
            read_error_code(&too_long),
            Err(ErrorCodeReadError::Codec(direct))
        );
    }

    #[test]
    fn the_five_download_diagnostics_are_exactly_the_ones_section_5_6_names() {
        let diagnostics: Vec<u8> = SystemErrorClass::ALL
            .iter()
            .filter(|c| c.is_download_diagnostic())
            .map(|c| c.code())
            .collect();
        assert_eq!(diagnostics, vec![6, 8, 9, 13, 14]);
    }

    #[test]
    fn error_15_is_the_unsorted_group_address_table() {
        assert_eq!(
            SystemErrorClass::from_code(15),
            Ok(SystemErrorClass::GroupAddressTableNotSorted)
        );
        // Not a download diagnostic: it is a validity rule the download
        // must obey before writing, not a report about the request.
        assert!(!SystemErrorClass::GroupAddressTableNotSorted.is_download_diagnostic());
    }

    #[test]
    fn no_fault_is_zero_because_unloading_clears_the_property() {
        // RES §4.2.28: leaving Error sets the code to 0. A reader that
        // sees 0 after an unload is seeing the erasure, not the cause,
        // which is why §9.1 reads the property before unloading.
        assert_eq!(SystemErrorClass::NoFault.code(), 0);
    }
}
