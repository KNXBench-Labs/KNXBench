//! `PID_SERVICE_CONTROL` bit 2, *"Individual Address Write Enable"* (K12).
//!
//! `[D]` `03_05_01 Resources` §4.2.8, Table 10: *"This bit shall control the
//! possibility to set the Individual Address via programming mode or KNX
//! Serial Number services. If this bit is cleared, it shall not be possible
//! to change the Individual Address of the device."* Profiles A.2.3.1 makes
//! the property optional for mask `0701h`.
//!
//! No Management Procedure asks a client to write this bit. KNXBench never
//! sets it on its own: a serial-number write that a device ignores is
//! reported as such (KNOWN_LIMITATIONS §139). This module is the explicit
//! operator action the user asked for on 2026-09-30 (ADR-0051):
//!
//! * [`read_service_control`] reads the property, read-only;
//! * [`set_individual_address_write_enable`] reads it, changes **only bit 2**,
//!   writes the two octets back and checks them by reading back through the
//!   session's exact-octet comparison. The other fifteen bits go back as
//!   read.
//!
//! Refused by name, before anything is written:
//!
//! * mask `0021h`, whose bit 2 is coded inversely (RES §4.2.8);
//! * a device without the property (`nr_of_elem = 0`), because there is
//!   nothing to enable;
//! * a value that is not two octets long.
//!
//! The write itself needs [`WriteScope::IndividualAddressWriteEnable`], its
//! own confirmation phrase: a download or address confirmation never
//! covers it.

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_state::MaskVersion;
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::properties::{
    ObjectIndex, PID_SERVICE_CONTROL, SERVICE_CONTROL_IA_WRITE_ENABLE,
    SERVICE_CONTROL_INVERTED_MASK,
};
use knx_core::IndividualAddress;

use super::{ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// What the device's `PID_SERVICE_CONTROL` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceControl {
    /// The property as read, big-endian (`PDT_UNSIGNED_INT`).
    pub raw: u16,
    /// The device's mask version, read in the same session.
    pub mask: MaskVersion,
}

impl ServiceControl {
    /// Whether bit 2 allows the Individual Address to be changed. Only
    /// meaningful for a mask other than `0021h`: that mask is refused
    /// before a value is ever returned.
    pub fn individual_address_write_enabled(self) -> bool {
        self.raw & SERVICE_CONTROL_IA_WRITE_ENABLE != 0
    }
}

/// What a change of bit 2 did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceControlChange {
    pub before: ServiceControl,
    /// The value read back after the write. Equal to `before.raw` when bit
    /// 2 already had the requested value: nothing was written.
    pub after: u16,
    /// Whether a write went out.
    pub written: bool,
}

/// Why reading or changing bit 2 stopped.
#[derive(Debug)]
pub enum ServiceControlError {
    /// Mask `0021h` codes bit 2 inversely (RES §4.2.8). Refused rather than
    /// guessed at.
    InvertedMask,
    /// The device has no `PID_SERVICE_CONTROL` (AL §3.4.4.2: answered with
    /// `nr_of_elem = 0`). Profiles A.2.3.1 makes it optional.
    NotPresent,
    /// The property was not two octets long.
    Malformed { octets: usize },
    /// A session step failed.
    Session {
        step: &'static str,
        error: SessionError,
    },
}

impl std::fmt::Display for ServiceControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvertedMask => write!(
                f,
                "mask 0021h codes Individual Address Write Enable inversely (RES §4.2.8); \
                 refused rather than guessed"
            ),
            Self::NotPresent => write!(
                f,
                "the device has no PID_SERVICE_CONTROL (optional for mask 0701h, \
                 Profiles A.2.3.1); there is no bit to set"
            ),
            Self::Malformed { octets } => write!(
                f,
                "PID_SERVICE_CONTROL answered with {octets} octet(s), expected 2"
            ),
            Self::Session { step, error } => write!(f, "{step}: {error}"),
        }
    }
}

impl std::error::Error for ServiceControlError {}

fn at(step: &'static str) -> impl Fn(SessionError) -> ServiceControlError {
    move |error| match error {
        SessionError::PropertyRefused { .. } => ServiceControlError::NotPresent,
        error => ServiceControlError::Session { step, error },
    }
}

async fn read_in<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
) -> Result<ServiceControl, ServiceControlError> {
    let mask = session
        .read_mask_version()
        .await
        .map_err(at("A_DeviceDescriptor_Read"))?;
    if mask.0 == SERVICE_CONTROL_INVERTED_MASK {
        return Err(ServiceControlError::InvertedMask);
    }
    let octets = session
        .read_property(ObjectIndex::DEVICE, PID_SERVICE_CONTROL, 1, 1)
        .await
        .map_err(at("A_PropertyValue_Read PID_SERVICE_CONTROL"))?;
    let bits: [u8; 2] =
        octets
            .as_slice()
            .try_into()
            .map_err(|_| ServiceControlError::Malformed {
                octets: octets.len(),
            })?;
    Ok(ServiceControl {
        raw: u16::from_be_bytes(bits),
        mask,
    })
}

/// Reads `PID_SERVICE_CONTROL` and the mask, read-only.
pub async fn read_service_control<T: ManagementTransport>(
    transport: &T,
    address: IndividualAddress,
    plan: AuthorisationPlan,
    timing: SessionTiming,
) -> Result<ServiceControl, ServiceControlError> {
    let mut session =
        ManagementSession::read_only(transport, address, plan, timing).map_err(at("session"))?;
    session.connect().await.map_err(at("T_Connect"))?;
    let result = read_in(&mut session).await;
    session.disconnect().await;
    result
}

/// Sets bit 2 to `enable`, leaving the other fifteen bits as read.
///
/// `authorisation` must name the device and
/// [`WriteScope::IndividualAddressWriteEnable`]. When the bit already has
/// the requested value nothing is written, and the report says so.
pub async fn set_individual_address_write_enable<T: ManagementTransport>(
    transport: &T,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    authorisation: WriteAuthorisation,
    enable: bool,
) -> Result<ServiceControlChange, ServiceControlError> {
    let mut session = ManagementSession::authorised(transport, plan, timing, authorisation)
        .map_err(at("authorisation"))?;
    session.connect().await.map_err(at("T_Connect"))?;
    let result = change_in(&mut session, enable).await;
    session.disconnect().await;
    result
}

async fn change_in<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    enable: bool,
) -> Result<ServiceControlChange, ServiceControlError> {
    let before = read_in(session).await?;
    if before.individual_address_write_enabled() == enable {
        return Ok(ServiceControlChange {
            before,
            after: before.raw,
            written: false,
        });
    }
    let wanted = if enable {
        before.raw | SERVICE_CONTROL_IA_WRITE_ENABLE
    } else {
        before.raw & !SERVICE_CONTROL_IA_WRITE_ENABLE
    };
    // `write_property` compares the answer's octets with the written ones
    // (`Comparison::ExactOctets`); a device that ignores the write fails
    // there with `PropertyReadBackMismatch`.
    let read = session
        .write_property(
            ObjectIndex::DEVICE,
            PID_SERVICE_CONTROL,
            wanted.to_be_bytes().to_vec(),
            WriteScope::IndividualAddressWriteEnable,
        )
        .await
        .map_err(at("A_PropertyValue_Write PID_SERVICE_CONTROL"))?;
    let bits: [u8; 2] = read
        .as_slice()
        .try_into()
        .map_err(|_| ServiceControlError::Malformed { octets: read.len() })?;
    Ok(ServiceControlChange {
        before,
        after: u16::from_be_bytes(bits),
        written: true,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use knx_core::commissioning::mutation::{
        hardware_write_is_authorised, required_confirmation_phrase, WriteAuthorisation, WriteScope,
    };
    use knx_core::commissioning::serial_number::SerialNumber;

    use super::*;
    use crate::commissioning::serial_number_write::serial_number_write;
    use crate::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};

    const SERIAL: SerialNumber = SerialNumber::from_octets([0x00, 0x83, 0x12, 0x34, 0x56, 0x78]);

    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(5),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn auth(device: &SimulatedDevice) -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(
            device.address(),
            WriteScope::IndividualAddressWriteEnable,
        )
        .expect("simulator authorisation")
    }

    fn service_control_writes(device: &SimulatedDevice) -> Vec<Vec<u8>> {
        device
            .seen()
            .into_iter()
            .filter_map(|seen| match seen {
                Seen::PropertyWrite {
                    object_index: 0,
                    property_id: PID_SERVICE_CONTROL,
                    data,
                } => Some(data),
                _ => None,
            })
            .collect()
    }

    async fn read(device: &SimulatedDevice) -> ServiceControl {
        read_service_control(device, device.address(), AuthorisationPlan::Skip, fast())
            .await
            .expect("read")
    }

    async fn set(
        device: &SimulatedDevice,
        authorisation: WriteAuthorisation,
        enable: bool,
    ) -> Result<ServiceControlChange, ServiceControlError> {
        set_individual_address_write_enable(
            device,
            AuthorisationPlan::Skip,
            fast(),
            authorisation,
            enable,
        )
        .await
    }

    #[tokio::test]
    async fn reading_reports_the_bit_and_writes_nothing() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number_write_enabled: false,
            ..Default::default()
        });
        assert!(!read(&device).await.individual_address_write_enabled());
        assert!(service_control_writes(&device).is_empty());
    }

    #[tokio::test]
    async fn enabling_sets_only_bit_2_and_the_serial_write_then_takes() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number: Some(SERIAL.octets()),
            serial_number_write_enabled: false,
            programming_mode: false,
            ..Default::default()
        });
        // Another bit set, to see it survive.
        device.preset_property(0, PID_SERVICE_CONTROL, &[0x01, 0x00]);

        let change = set(&device, auth(&device), true).await.expect("enable");
        assert!(change.written);
        assert_eq!(change.before.raw, 0x0100);
        assert_eq!(change.after, 0x0104);
        assert_eq!(service_control_writes(&device), vec![vec![0x01, 0x04]]);
        assert_eq!(read(&device).await.raw, 0x0104);

        let new_address = IndividualAddress::new(1, 1, 99).expect("address");
        let written = serial_number_write(
            &device,
            fast(),
            SERIAL,
            WriteAuthorisation::for_simulator(
                new_address,
                WriteScope::IndividualAddressProgramming,
            )
            .expect("authorisation"),
        )
        .await;
        assert!(written.is_ok(), "{written:?}");
        assert_eq!(device.serial_number_writes(), 1);
    }

    #[tokio::test]
    async fn without_the_bit_the_serial_write_is_ignored() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number: Some(SERIAL.octets()),
            serial_number_write_enabled: false,
            programming_mode: false,
            ..Default::default()
        });
        let new_address = IndividualAddress::new(1, 1, 99).expect("address");
        let written = serial_number_write(
            &device,
            fast(),
            SERIAL,
            WriteAuthorisation::for_simulator(
                new_address,
                WriteScope::IndividualAddressProgramming,
            )
            .expect("authorisation"),
        )
        .await;
        assert!(written.is_err());
        assert_eq!(device.serial_number_writes(), 0);
    }

    #[tokio::test]
    async fn disabling_clears_bit_2_again() {
        let device = SimulatedDevice::with_config(SimulatorConfig::default());
        let change = set(&device, auth(&device), false).await.expect("disable");
        assert!(change.written);
        assert_eq!(change.after & SERVICE_CONTROL_IA_WRITE_ENABLE, 0);
        assert!(!read(&device).await.individual_address_write_enabled());
    }

    #[tokio::test]
    async fn an_already_set_bit_is_not_written_again() {
        let device = SimulatedDevice::with_config(SimulatorConfig::default());
        let change = set(&device, auth(&device), true).await.expect("enable");
        assert!(!change.written);
        assert_eq!(change.after, change.before.raw);
        assert!(service_control_writes(&device).is_empty());
    }

    #[tokio::test]
    async fn a_device_without_the_property_is_refused_by_name() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            service_control_present: false,
            ..Default::default()
        });
        let error = set(&device, auth(&device), true)
            .await
            .expect_err("no property");
        assert!(matches!(error, ServiceControlError::NotPresent), "{error}");
        assert!(service_control_writes(&device).is_empty());
    }

    #[tokio::test]
    async fn mask_0021h_is_refused_before_anything_is_written() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: SERVICE_CONTROL_INVERTED_MASK,
            ..Default::default()
        });
        let error = set(&device, auth(&device), true)
            .await
            .expect_err("inverted mask");
        assert!(
            matches!(error, ServiceControlError::InvertedMask),
            "{error}"
        );
        assert!(service_control_writes(&device).is_empty());
    }

    #[tokio::test]
    async fn another_scope_does_not_cover_the_write() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number_write_enabled: false,
            ..Default::default()
        });
        let download = WriteAuthorisation::for_simulator(device.address(), WriteScope::Download)
            .expect("auth");
        let error = set(&device, download, true).await.expect_err("wrong scope");
        assert!(
            matches!(error, ServiceControlError::Session { .. }),
            "{error}"
        );
        assert!(service_control_writes(&device).is_empty());
        assert!(!read(&device).await.individual_address_write_enabled());
    }

    #[test]
    fn the_scope_has_its_own_phrase_and_is_allowed_on_hardware() {
        let address = IndividualAddress::new(1, 1, 67).expect("address");
        let phrase =
            required_confirmation_phrase(address, WriteScope::IndividualAddressWriteEnable);
        assert!(phrase.contains("1.1.67"), "{phrase}");
        assert_ne!(
            phrase,
            required_confirmation_phrase(address, WriteScope::Download)
        );
        assert!(hardware_write_is_authorised(
            WriteScope::IndividualAddressWriteEnable
        ));
    }
}
