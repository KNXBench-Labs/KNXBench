//! Master Reset requests and answers, as MP §3.7.1.2 Tables 4 and 5 define them.
//!
//! `[D]` MP §3.7.1.2.3.1, pp. 81–83 (Table 4, Erase Code and Channel Number;
//! Table 5, Error Code) and RES "Master Reset" under `PID_DOWNLOAD_COUNTER`
//! (the counter table). The frame itself is `A_Restart` with `restart_type`
//! 1; `knx_net` sends it.
//!
//! A [`MasterResetRequest`] is only constructible with a code Table 4 lets
//! a Management Client use (not `00h`, not `09h`–`FFh`: *"The Management
//! Client shall not use these Erase Codes"*) and a Channel Number the code
//! allows (`00h` where Table 4 says *"Fixed: 00h"*).

use std::fmt;

/// Table 4's Erase Codes a Management Client may send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EraseCode {
    /// `01h`: no Resource is reset; a confirmed Basic Restart.
    ConfirmedRestart,
    /// `02h`: ex-factory state, the individual address included.
    FactoryReset,
    /// `03h`: the individual address goes to the medium's default.
    ResetIndividualAddress,
    /// `04h`: the application program goes to the default application.
    ResetApplicationProgram,
    /// `05h`: application parameters to their defaults.
    ResetParameters,
    /// `06h`: group address and association tables to their defaults.
    ResetLinks,
    /// `07h`: ex-factory state, but the individual address is kept.
    FactoryResetWithoutIndividualAddress,
    /// `08h`: persistently stored application data becomes invalid.
    ErasePersistentApplicationData,
}

impl EraseCode {
    pub const ALL: [EraseCode; 8] = [
        EraseCode::ConfirmedRestart,
        EraseCode::FactoryReset,
        EraseCode::ResetIndividualAddress,
        EraseCode::ResetApplicationProgram,
        EraseCode::ResetParameters,
        EraseCode::ResetLinks,
        EraseCode::FactoryResetWithoutIndividualAddress,
        EraseCode::ErasePersistentApplicationData,
    ];

    pub fn octet(self) -> u8 {
        match self {
            EraseCode::ConfirmedRestart => 0x01,
            EraseCode::FactoryReset => 0x02,
            EraseCode::ResetIndividualAddress => 0x03,
            EraseCode::ResetApplicationProgram => 0x04,
            EraseCode::ResetParameters => 0x05,
            EraseCode::ResetLinks => 0x06,
            EraseCode::FactoryResetWithoutIndividualAddress => 0x07,
            EraseCode::ErasePersistentApplicationData => 0x08,
        }
    }

    /// `None` for `00h` and `09h`–`FFh`, which Table 4 reserves.
    pub fn from_octet(octet: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|code| code.octet() == octet)
    }

    /// Table 4 *"Channel Number: Fixed: 00h"*: codes `01h`, `03h`, `04h`.
    pub fn channel_number_fixed(self) -> bool {
        matches!(
            self,
            EraseCode::ConfirmedRestart
                | EraseCode::ResetIndividualAddress
                | EraseCode::ResetApplicationProgram
        )
    }

    /// Whether the device leaves its individual address: `03h`, and `02h`
    /// (MP Table 6, p. 83: Factory Reset *"shall reset"* the IA; `07h`
    /// *"shall NOT"*).
    pub fn resets_individual_address(self) -> bool {
        matches!(
            self,
            EraseCode::FactoryReset | EraseCode::ResetIndividualAddress
        )
    }

    /// Whether anything is erased at all. Only `01h` erases nothing.
    pub fn erases(self) -> bool {
        self != EraseCode::ConfirmedRestart
    }

    /// RES, `PID_DOWNLOAD_COUNTER` "Master Reset": what the Device Object's
    /// counter does. `None` where the table has no row (`03h`, `04h`,
    /// `08h`): nothing is claimed.
    pub fn download_counter_effect(self) -> Option<DownloadCounterEffect> {
        match self {
            EraseCode::ConfirmedRestart => Some(DownloadCounterEffect::Unchanged),
            EraseCode::FactoryReset
            | EraseCode::ResetParameters
            | EraseCode::ResetLinks
            | EraseCode::FactoryResetWithoutIndividualAddress => {
                Some(DownloadCounterEffect::Increments)
            }
            EraseCode::ResetIndividualAddress
            | EraseCode::ResetApplicationProgram
            | EraseCode::ErasePersistentApplicationData => None,
        }
    }
}

impl fmt::Display for EraseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            EraseCode::ConfirmedRestart => "Confirmed Restart",
            EraseCode::FactoryReset => "Factory Reset",
            EraseCode::ResetIndividualAddress => "ResetIA",
            EraseCode::ResetApplicationProgram => "ResetAP",
            EraseCode::ResetParameters => "ResetParam",
            EraseCode::ResetLinks => "ResetLinks",
            EraseCode::FactoryResetWithoutIndividualAddress => "Factory Reset without IA",
            EraseCode::ErasePersistentApplicationData => {
                "Erase persistently stored application data"
            }
        };
        write!(f, "{name} ({:02X}h)", self.octet())
    }
}

/// RES's table: how a Master Reset moves the Device Object's download counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadCounterEffect {
    /// *"not influenced: no change."*
    Unchanged,
    /// *"recalculate: increment."*
    Increments,
}

/// An Erase Code with a Channel Number Table 4 allows for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterResetRequest {
    erase_code: EraseCode,
    channel_number: u8,
}

/// Why a request was not built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterResetRequestError {
    /// Table 4 reserves this octet; a Management Client shall not send it.
    ReservedEraseCode(u8),
    /// Table 4 fixes the Channel Number at `00h` for this code.
    ChannelNumberFixed {
        erase_code: EraseCode,
        channel_number: u8,
    },
}

impl fmt::Display for MasterResetRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReservedEraseCode(octet) => write!(
                f,
                "Erase Code {octet:02X}h is reserved (MP Table 4): a Management Client \
                 shall not use it"
            ),
            Self::ChannelNumberFixed {
                erase_code,
                channel_number,
            } => write!(
                f,
                "{erase_code} fixes the Channel Number at 00h (MP Table 4), not \
                 {channel_number:02X}h"
            ),
        }
    }
}

impl std::error::Error for MasterResetRequestError {}

impl MasterResetRequest {
    pub fn new(erase_code: EraseCode, channel_number: u8) -> Result<Self, MasterResetRequestError> {
        if erase_code.channel_number_fixed() && channel_number != 0 {
            return Err(MasterResetRequestError::ChannelNumberFixed {
                erase_code,
                channel_number,
            });
        }
        Ok(Self {
            erase_code,
            channel_number,
        })
    }

    /// From the two octets as an operator gives them.
    pub fn from_octets(
        erase_code: u8,
        channel_number: u8,
    ) -> Result<Self, MasterResetRequestError> {
        let code = EraseCode::from_octet(erase_code)
            .ok_or(MasterResetRequestError::ReservedEraseCode(erase_code))?;
        Self::new(code, channel_number)
    }

    /// Erase Code `01h`, Channel `00h`.
    pub fn confirmed_restart() -> Self {
        Self {
            erase_code: EraseCode::ConfirmedRestart,
            channel_number: 0,
        }
    }

    pub fn erase_code(self) -> EraseCode {
        self.erase_code
    }

    pub fn channel_number(self) -> u8 {
        self.channel_number
    }
}

/// Table 5: the Error Code of an `A_Restart_Response`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartErrorCode {
    /// `00h`: received, and it will be executed.
    NoError,
    /// `01h`: Master Reset is protected and the client is not authorised.
    AccessDenied,
    /// `02h`: the Erase Code is not supported.
    UnsupportedEraseCode,
    /// `03h`: the Channel Number is wrong for the code or the device.
    InvalidChannelNumber,
    /// `04h`–`FFh`: reserved; a Management Server shall not use them.
    Reserved(u8),
}

impl RestartErrorCode {
    pub fn from_octet(octet: u8) -> Self {
        match octet {
            0x00 => Self::NoError,
            0x01 => Self::AccessDenied,
            0x02 => Self::UnsupportedEraseCode,
            0x03 => Self::InvalidChannelNumber,
            other => Self::Reserved(other),
        }
    }
}

impl fmt::Display for RestartErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoError => f.write_str("no error (00h)"),
            Self::AccessDenied => f.write_str("access denied (01h)"),
            Self::UnsupportedEraseCode => f.write_str("unsupported Erase Code (02h)"),
            Self::InvalidChannelNumber => f.write_str("invalid Channel Number (03h)"),
            Self::Reserved(octet) => write!(f, "reserved Error Code {octet:02X}h"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_4_octets_round_trip_and_reserved_ones_are_refused() {
        for code in EraseCode::ALL {
            assert_eq!(EraseCode::from_octet(code.octet()), Some(code));
        }
        for reserved in [0x00, 0x09, 0x7F, 0xFF] {
            assert_eq!(EraseCode::from_octet(reserved), None);
            assert_eq!(
                MasterResetRequest::from_octets(reserved, 0),
                Err(MasterResetRequestError::ReservedEraseCode(reserved))
            );
        }
    }

    #[test]
    fn a_fixed_channel_code_refuses_another_channel_and_a_free_one_takes_it() {
        for code in [
            EraseCode::ConfirmedRestart,
            EraseCode::ResetIndividualAddress,
            EraseCode::ResetApplicationProgram,
        ] {
            assert!(MasterResetRequest::new(code, 0).is_ok());
            assert_eq!(
                MasterResetRequest::new(code, 3),
                Err(MasterResetRequestError::ChannelNumberFixed {
                    erase_code: code,
                    channel_number: 3
                })
            );
        }
        for code in [
            EraseCode::FactoryReset,
            EraseCode::ResetParameters,
            EraseCode::ResetLinks,
            EraseCode::FactoryResetWithoutIndividualAddress,
            EraseCode::ErasePersistentApplicationData,
        ] {
            assert_eq!(
                MasterResetRequest::new(code, 3).unwrap().channel_number(),
                3
            );
        }
    }

    #[test]
    fn only_factory_reset_and_reset_ia_move_the_address() {
        let moving: Vec<_> = EraseCode::ALL
            .into_iter()
            .filter(|code| code.resets_individual_address())
            .collect();
        assert_eq!(
            moving,
            vec![EraseCode::FactoryReset, EraseCode::ResetIndividualAddress]
        );
    }

    #[test]
    fn the_download_counter_table_is_the_res_table() {
        use DownloadCounterEffect::*;
        let table: Vec<_> = EraseCode::ALL
            .into_iter()
            .map(|code| (code.octet(), code.download_counter_effect()))
            .collect();
        assert_eq!(
            table,
            vec![
                (0x01, Some(Unchanged)),
                (0x02, Some(Increments)),
                (0x03, None),
                (0x04, None),
                (0x05, Some(Increments)),
                (0x06, Some(Increments)),
                (0x07, Some(Increments)),
                (0x08, None),
            ]
        );
    }

    #[test]
    fn table_5_error_codes() {
        assert_eq!(RestartErrorCode::from_octet(0), RestartErrorCode::NoError);
        assert_eq!(
            RestartErrorCode::from_octet(1),
            RestartErrorCode::AccessDenied
        );
        assert_eq!(
            RestartErrorCode::from_octet(2),
            RestartErrorCode::UnsupportedEraseCode
        );
        assert_eq!(
            RestartErrorCode::from_octet(3),
            RestartErrorCode::InvalidChannelNumber
        );
        assert_eq!(
            RestartErrorCode::from_octet(4),
            RestartErrorCode::Reserved(4)
        );
    }
}
