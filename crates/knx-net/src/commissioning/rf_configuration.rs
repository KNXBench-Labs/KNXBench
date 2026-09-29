//! KNX-RF device configuration procedures, simulator-verified only.
//!
//! KNX RF device configuration, CP §3.6 and §3.7 (K17).
//!
//! `[D]` CP (`03_05_03`) §3.6–§3.7, pp. 57–60; MP (`03_05_02`) §2.6, p. 17,
//! §3.2.2 `DMP_Connect_RCl`, p. 69, §3.2.7 `DM_DeviceDescriptor_InfoReport`,
//! p. 71; AL §3.3.2 and §3.4.7; RES §4.1.3, §4.3.14, §4.3.16. The payload
//! shapes live in `knx_core::commissioning::rf_configuration`.
//!
//! **Simulator only.** No KNX-RF device has been on this project's bus; every
//! write here needs [`WriteScope::RfConfiguration`], which hardware refuses
//! (KNOWN_LIMITATIONS §144).
//!
//! | CP | Here |
//! |---|---|
//! | §3.6.2 identification, `DMP_Connect_RCl(IA, 2)` | [`read_device_descriptor`] |
//! | §3.6.3 individualisation, MP §2.6 | [`individual_address_serial_number_write2`] |
//! | §3.6.4 parameter download, `PID_PARAMETER` | [`write_parameter`], [`read_parameter`] |
//! | §3.6.5 device linking, `PID_OBJECTLINK` | [`write_object_link`] |
//! | §3.7.2 identification, `DM_DeviceDescriptor_InfoReport` | [`collect_info_reports`] |
//! | §3.7.2.3 group-address calculation | `knx_core`'s `pre_assigned_group_addresses` |
//! | §3.7.3 individualisation | none: always `05FFh` |
//!
//! Not here: MP §2.6's opening `CC_Config_Link` exchange is the PB-Mode
//! link sequence (Easy mode, a later goal); the function starts where the
//! Management Client *"shall know the KNX Serial Number"*. §3.7.4's
//! parameter view rides on that same link procedure. The RF-only `number
//! = 0` of the unidirectional frame's KNX Serial Number (MP §3.2.7) is in
//! the cEMI 'RF medium information', which `LDataFrame` does not carry
//! yet, so [`collect_info_reports`] reports the source address only.

use std::time::Duration;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::rf_configuration::{
    object_link_write, parameter_read, parameter_read_result, parameter_write, DeviceDescriptor2,
    LinkAddress, LinkChange, ObjectLinkReturn, ParameterReadResult, DEVICE_DESCRIPTOR_TYPE_2,
    DEVICE_OBJECT_INDEX, PID_OBJECTLINK, PID_PARAMETER,
};
use knx_core::commissioning::serial_number::SerialNumber;
use knx_core::IndividualAddress;

use super::domain_address::collect;
use super::{broadcast_serial_number_read, ManagementSession, SessionError, SessionTiming};
use crate::cemi::{ApplicationService, Destination, LDataFrame, LDataMessageKind};
use crate::management::ManagementTransport;

/// What `DMP_Connect_RCl` found (MP §3.2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceDescriptorAnswer {
    /// *"If no A_DeviceDescriptor_Response-PDU is received, the Management
    /// Server with the Individual Address does not exist or the network is
    /// not configured correctly."*
    Nobody,
    /// One answer. `descriptor_type` *"may be different from
    /// nm_desc_type_req"*; the procedure hands it back as it came.
    One {
        descriptor_type: u8,
        device_descriptor: Vec<u8>,
    },
    /// *"there is more than one device with the target address"*.
    Several(usize),
}

impl DeviceDescriptorAnswer {
    /// DD2, decoded, when the one answer is one.
    pub fn dd2(&self) -> Option<DeviceDescriptor2> {
        match self {
            Self::One {
                descriptor_type: DEVICE_DESCRIPTOR_TYPE_2,
                device_descriptor,
            } => DeviceDescriptor2::from_octets(device_descriptor).ok(),
            _ => None,
        }
    }
}

/// MP §3.2.2 `DMP_Connect_RCl`: `A_DeviceDescriptor_Read` point-to-point
/// connectionless, every answer collected for `window`. CP §3.6.2 calls it
/// with type 2. Read only, so no authorisation.
pub async fn read_device_descriptor<T: ManagementTransport>(
    transport: &T,
    address: IndividualAddress,
    descriptor_type: u8,
    window: Duration,
) -> Result<DeviceDescriptorAnswer, SessionError> {
    let mut answers = collect(
        transport,
        Destination::Individual(address),
        ApplicationService::DeviceDescriptorRead { descriptor_type },
        window,
        "A_DeviceDescriptor_Response (connectionless)",
        |frame| match &frame.service {
            ApplicationService::DeviceDescriptorResponse {
                descriptor_type,
                data,
            } if frame.source == address => Some((*descriptor_type, data.clone())),
            _ => None,
        },
    )
    .await?;
    Ok(match answers.len() {
        0 => DeviceDescriptorAnswer::Nobody,
        1 => {
            let (descriptor_type, device_descriptor) = answers.remove(0);
            DeviceDescriptorAnswer::One {
                descriptor_type,
                device_descriptor,
            }
        }
        n => DeviceDescriptorAnswer::Several(n),
    })
}

/// One `A_DeviceDescriptor_InfoReport` heard (AL §3.3.2 NOTE 5: the same
/// APCI as `A_DeviceDescriptor_Response`, sent on the system broadcast).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoReport {
    /// The sender; `05FFh` for a unidirectional device (CP §3.7.3).
    pub source: IndividualAddress,
    pub descriptor_type: u8,
    pub device_descriptor: Vec<u8>,
}

/// Whether `frame` is an InfoReport: a Device Descriptor response on the
/// system broadcast. A response addressed to someone is the answer to a
/// read, not a report.
pub fn info_report(frame: &LDataFrame) -> Option<InfoReport> {
    match (&frame.kind, frame.destination, &frame.service) {
        (
            LDataMessageKind::Indication,
            Destination::SystemBroadcast,
            ApplicationService::DeviceDescriptorResponse {
                descriptor_type,
                data,
            },
        ) => Some(InfoReport {
            source: frame.source,
            descriptor_type: *descriptor_type,
            device_descriptor: data.clone(),
        }),
        _ => None,
    }
}

/// CP §3.7.2: the Management Client in *"teaching mode"* collects the
/// InfoReports RF unidirectional devices send *"upon a manufacturer-specific
/// user action"*, for `window`. Sends nothing.
pub async fn collect_info_reports<T: ManagementTransport>(
    transport: &T,
    window: Duration,
) -> Result<Vec<InfoReport>, SessionError> {
    let mut events = transport.subscribe();
    let deadline = tokio::time::Instant::now() + window;
    let mut found = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(found);
        }
        match tokio::time::timeout(remaining, events.recv()).await {
            Ok(Ok(crate::client::TunnelEvent::Telegram(frame))) => {
                found.extend(info_report(&frame));
            }
            Ok(Ok(crate::client::TunnelEvent::Closed))
            | Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                return Err(SessionError::ConnectionLost {
                    during: "A_DeviceDescriptor_InfoReport",
                })
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                return Err(SessionError::Lagged {
                    waiting_for: "A_DeviceDescriptor_InfoReport",
                })
            }
            Err(_) => {}
        }
    }
}

/// Why an RF configuration step stopped.
#[derive(Debug)]
pub enum RfConfigurationError {
    /// A session, scope or transport failure in the named step.
    Session {
        step: &'static str,
        source: SessionError,
    },
    /// MP §2.6: the serial-number read-back did not find the device at the
    /// new address.
    NotAtNewAddress {
        answered_from: Option<IndividualAddress>,
    },
    /// MP §2.6 note a): the device did not answer the Device Descriptor
    /// read at its new address.
    NoDeviceDescriptor(DeviceDescriptorAnswer),
    /// The function property answered without a return code: AL §3.4.7.3,
    /// the property is not a function on this device.
    NotAFunction { property_id: u8 },
    /// Nothing answered the function property in time.
    NoAnswer { property_id: u8 },
    /// `PID_PARAMETER` said `FFh`, *"ERROR (invalid parameter)"*.
    InvalidParameter { channel: u8, parameter: u8 },
    /// A code RES does not give for `PID_PARAMETER`, or octets of the wrong
    /// shape, kept.
    UnexpectedParameterAnswer(ParameterReadResult),
    /// `PID_OBJECTLINK` did not return success.
    Link(ObjectLinkReturn),
}

impl std::fmt::Display for RfConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session { step, source } => write!(f, "{step}: {source}"),
            Self::NotAtNewAddress {
                answered_from: Some(at),
            } => write!(
                f,
                "the device answers its serial number from {at}, not the new address"
            ),
            Self::NotAtNewAddress {
                answered_from: None,
            } => f.write_str("no device answered its serial number after the write"),
            Self::NoDeviceDescriptor(answer) => write!(
                f,
                "the device did not answer at its new address (MP §2.6 note a): {answer:?}"
            ),
            Self::NotAFunction { property_id } => {
                write!(f, "property {property_id} is not a function on this device")
            }
            Self::NoAnswer { property_id } => {
                write!(
                    f,
                    "no A_FunctionPropertyState_Response for property {property_id}"
                )
            }
            Self::InvalidParameter { channel, parameter } => write!(
                f,
                "the device rejects parameter {parameter} of channel {channel} (FFh)"
            ),
            Self::UnexpectedParameterAnswer(answer) => {
                write!(f, "unexpected PID_PARAMETER answer: {answer:?}")
            }
            Self::Link(code) => write!(f, "PID_OBJECTLINK returned {code:?}"),
        }
    }
}

impl std::error::Error for RfConfigurationError {}

fn at(step: &'static str) -> impl Fn(SessionError) -> RfConfigurationError {
    move |source| RfConfigurationError::Session { step, source }
}

/// MP §2.6 `NM_IndividualAddress_SerialNumber_Write2` after the link
/// sequence: `A_IndividualAddressSerialNumber_Write`, the read-back by
/// serial number, then `A_DeviceDescriptor_Read` type 2 point-to-point,
/// where the client *"is only interested in whether it receives a response
/// or not"* (note a)). CP §3.6.3 lists it for RF bidirectional devices.
///
/// `authorisation` names the new address and
/// [`WriteScope::RfConfiguration`].
pub async fn individual_address_serial_number_write2<T: ManagementTransport>(
    transport: &T,
    timing: SessionTiming,
    serial_number: SerialNumber,
    authorisation: WriteAuthorisation,
) -> Result<IndividualAddress, RfConfigurationError> {
    let new_address = authorisation.target().address();
    let session =
        ManagementSession::authorised(transport, AuthorisationPlan::Skip, timing, authorisation)
            .map_err(at("authorisation"))?;
    session
        .broadcast_serial_number_write_as(serial_number, WriteScope::RfConfiguration)
        .await
        .map_err(at("A_IndividualAddressSerialNumber_Write"))?;
    let answered_from =
        broadcast_serial_number_read(transport, serial_number, timing.response_timeout)
            .await
            .map_err(at("A_IndividualAddressSerialNumber_Read"))?;
    if answered_from != Some(new_address) {
        return Err(RfConfigurationError::NotAtNewAddress { answered_from });
    }
    let answer = read_device_descriptor(
        transport,
        new_address,
        DEVICE_DESCRIPTOR_TYPE_2,
        timing.response_timeout,
    )
    .await
    .map_err(at("A_DeviceDescriptor_Read"))?;
    if !matches!(answer, DeviceDescriptorAnswer::One { .. }) {
        return Err(RfConfigurationError::NoDeviceDescriptor(answer));
    }
    Ok(new_address)
}

/// Sends one function-property request connectionless and waits for the
/// matching `A_FunctionPropertyState_Response` from `address`.
async fn call_function<T: ManagementTransport>(
    transport: &T,
    address: IndividualAddress,
    service: ApplicationService,
    property_id: u8,
    timeout: Duration,
) -> Result<(u8, Vec<u8>), RfConfigurationError> {
    let answers = collect(
        transport,
        Destination::Individual(address),
        service,
        timeout,
        "A_FunctionPropertyState_Response",
        |frame| match &frame.service {
            ApplicationService::FunctionPropertyStateResponse {
                object_index: DEVICE_OBJECT_INDEX,
                property_id: pid,
                return_code,
                data,
            } if frame.source == address && *pid == property_id => {
                Some((*return_code, data.clone()))
            }
            _ => None,
        },
    )
    .await
    .map_err(at("function property"))?;
    match answers.into_iter().next() {
        None => Err(RfConfigurationError::NoAnswer { property_id }),
        Some((None, _)) => Err(RfConfigurationError::NotAFunction { property_id }),
        Some((Some(code), data)) => Ok((code, data)),
    }
}

fn session_for<'t, T: ManagementTransport>(
    transport: &'t T,
    timing: SessionTiming,
    authorisation: WriteAuthorisation,
) -> Result<ManagementSession<'t, T>, RfConfigurationError> {
    let session =
        ManagementSession::authorised(transport, AuthorisationPlan::Skip, timing, authorisation)
            .map_err(at("authorisation"))?;
    session
        .authorise_scope(WriteScope::RfConfiguration)
        .map_err(at("authorisation"))?;
    Ok(session)
}

/// CP §3.6.4: Function Write Parameter through `PID_PARAMETER` (RES
/// §4.3.16 a). `authorisation` names the device and
/// [`WriteScope::RfConfiguration`].
pub async fn write_parameter<T: ManagementTransport>(
    transport: &T,
    timing: SessionTiming,
    authorisation: WriteAuthorisation,
    channel: u8,
    parameter: u8,
    value: &[u8],
) -> Result<(), RfConfigurationError> {
    let session = session_for(transport, timing, authorisation)?;
    let address = session.target();
    let (code, _) = call_function(
        transport,
        address,
        ApplicationService::FunctionPropertyCommand {
            object_index: DEVICE_OBJECT_INDEX,
            property_id: PID_PARAMETER,
            data: parameter_write(channel, parameter, value),
        },
        PID_PARAMETER,
        timing.response_timeout,
    )
    .await?;
    match code {
        0x00 => Ok(()),
        0xFF => Err(RfConfigurationError::InvalidParameter { channel, parameter }),
        other => Err(RfConfigurationError::UnexpectedParameterAnswer(
            ParameterReadResult::Malformed {
                return_code: other,
                data: Vec::new(),
            },
        )),
    }
}

/// CP §3.6.4: Function Read Parameter (RES §4.3.16 b). Read only.
pub async fn read_parameter<T: ManagementTransport>(
    transport: &T,
    address: IndividualAddress,
    timeout: Duration,
    channel: u8,
    parameter: u8,
) -> Result<Vec<u8>, RfConfigurationError> {
    let (code, data) = call_function(
        transport,
        address,
        ApplicationService::FunctionPropertyStateRead {
            object_index: DEVICE_OBJECT_INDEX,
            property_id: PID_PARAMETER,
            data: parameter_read(channel, parameter),
        },
        PID_PARAMETER,
        timeout,
    )
    .await?;
    match parameter_read_result(code, &data) {
        ParameterReadResult::Value(value) => Ok(value),
        ParameterReadResult::InvalidParameter => {
            Err(RfConfigurationError::InvalidParameter { channel, parameter })
        }
        other => Err(RfConfigurationError::UnexpectedParameterAnswer(other)),
    }
}

/// CP §3.6.5: set or delete one link through `PID_OBJECTLINK` (RES
/// §4.3.14.1). `authorisation` names the device and
/// [`WriteScope::RfConfiguration`].
pub async fn write_object_link<T: ManagementTransport>(
    transport: &T,
    timing: SessionTiming,
    authorisation: WriteAuthorisation,
    change: LinkChange,
    link: LinkAddress,
    group_object: u16,
) -> Result<(), RfConfigurationError> {
    let session = session_for(transport, timing, authorisation)?;
    let address = session.target();
    let (code, _) = call_function(
        transport,
        address,
        ApplicationService::FunctionPropertyCommand {
            object_index: DEVICE_OBJECT_INDEX,
            property_id: PID_OBJECTLINK,
            data: object_link_write(change, link, group_object),
        },
        PID_OBJECTLINK,
        timing.response_timeout,
    )
    .await?;
    match ObjectLinkReturn::from_octet(code) {
        ObjectLinkReturn::Success => Ok(()),
        other => Err(RfConfigurationError::Link(other)),
    }
}

#[cfg(test)]
mod tests {
    use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
    use knx_core::commissioning::rf_configuration::{
        pre_assigned_group_addresses, ManagementProfile, RF_UNIDIRECTIONAL_ADDRESS,
    };
    use knx_core::GroupAddress;

    use super::*;
    use crate::commissioning::simulator::{SimulatedDevice, SimulatorConfig};

    const SERIAL: SerialNumber = SerialNumber::from_octets([0x00, 0x09, 0x12, 0x34, 0x56, 0x78]);
    const OTHER_SERIAL: SerialNumber =
        SerialNumber::from_octets([0x00, 0x09, 0x12, 0x34, 0x56, 0x79]);
    /// CP §3.7.2.3 Example 16.
    const EXAMPLE_16: [u8; 14] = [
        0x00, 0x09, 0x30, 0x00, 0x10, 0x3F, 0x00, 0x08, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00,
    ];
    const WINDOW: Duration = Duration::from_millis(20);

    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(60),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).expect("a valid individual address")
    }

    fn rf(target: IndividualAddress) -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(target, WriteScope::RfConfiguration)
            .expect("not an excluded address")
    }

    fn bidirectional() -> SimulatedDevice {
        SimulatedDevice::with_config(SimulatorConfig {
            serial_number: Some(SERIAL.octets()),
            device_descriptor_2: Some(EXAMPLE_16),
            rf_function_properties: true,
            rf_group_objects: 5,
            ..SimulatorConfig::default()
        })
    }

    #[tokio::test]
    async fn dd2_is_read_connectionless_and_decodes() {
        let device = bidirectional();
        let answer = read_device_descriptor(&device, device.address(), 2, WINDOW)
            .await
            .unwrap();
        let dd2 = answer.dd2().expect("a DD2");
        assert_eq!(dd2.application_manufacturer, 0x0009);
        assert_eq!(dd2.management, ManagementProfile::Reserved);
    }

    #[tokio::test]
    async fn a_device_without_dd2_answers_with_the_type_it_has() {
        let device = SimulatedDevice::with_config(SimulatorConfig::default());
        let answer = read_device_descriptor(&device, device.address(), 2, WINDOW)
            .await
            .unwrap();
        assert!(matches!(
            answer,
            DeviceDescriptorAnswer::One {
                descriptor_type: 0,
                ..
            }
        ));
        assert_eq!(answer.dd2(), None, "MP §3.2.2: the type may differ");
        let nobody = read_device_descriptor(&device, addr(1, 1, 99), 2, WINDOW)
            .await
            .unwrap();
        assert_eq!(nobody, DeviceDescriptorAnswer::Nobody);
    }

    #[tokio::test]
    async fn the_serial_write2_moves_the_device_and_checks_it_answers() {
        let device = bidirectional();
        let new = addr(1, 1, 42);
        assert_eq!(
            individual_address_serial_number_write2(&device, fast(), SERIAL, rf(new))
                .await
                .unwrap(),
            new
        );
        assert_eq!(device.address(), new);
        assert!(!device.programming_mode(), "no programming mode needed");
    }

    #[tokio::test]
    async fn the_serial_write2_to_a_stranger_is_caught() {
        let device = bidirectional();
        let err = individual_address_serial_number_write2(
            &device,
            fast(),
            OTHER_SERIAL,
            rf(addr(1, 1, 42)),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(
                err,
                RfConfigurationError::NotAtNewAddress {
                    answered_from: None
                }
            ),
            "{err}"
        );
        assert_ne!(device.address(), addr(1, 1, 42));
    }

    #[tokio::test]
    async fn a_device_that_ignores_the_write2_is_caught_at_its_old_address() {
        // MP §2.6's read-back answers, but from where the device still is.
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number: Some(SERIAL.octets()),
            serial_number_write_enabled: false,
            device_descriptor_2: Some(EXAMPLE_16),
            ..SimulatorConfig::default()
        });
        let old = device.address();
        let err =
            individual_address_serial_number_write2(&device, fast(), SERIAL, rf(addr(1, 1, 42)))
                .await
                .unwrap_err();
        assert!(
            matches!(err, RfConfigurationError::NotAtNewAddress { answered_from: Some(at) } if at == old),
            "{err}"
        );
    }

    #[tokio::test]
    async fn another_devices_function_answer_is_not_taken() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            device_descriptor_2: Some(EXAMPLE_16),
            rf_function_properties: true,
            rf_group_objects: 5,
            foreign_function_answer: Some(addr(1, 1, 200)),
            ..SimulatorConfig::default()
        });
        device.preset_rf_parameter(1, 3, &[0x2A]);
        assert_eq!(
            read_parameter(&device, device.address(), WINDOW, 1, 3)
                .await
                .unwrap(),
            vec![0x2A],
            "not the stranger's EEh"
        );
    }

    #[tokio::test]
    async fn the_serial_write2_needs_the_rf_scope() {
        let device = bidirectional();
        let new = addr(1, 1, 42);
        let wrong =
            WriteAuthorisation::for_simulator(new, WriteScope::IndividualAddressProgramming)
                .unwrap();
        let err = individual_address_serial_number_write2(&device, fast(), SERIAL, wrong)
            .await
            .unwrap_err();
        assert!(matches!(err, RfConfigurationError::Session { .. }), "{err}");
        assert_ne!(device.address(), new);
    }

    #[tokio::test]
    async fn a_parameter_is_written_and_read_back() {
        let device = bidirectional();
        let at = device.address();
        write_parameter(&device, fast(), rf(at), 1, 3, &[0x2A])
            .await
            .unwrap();
        assert_eq!(device.rf_parameter(1, 3), Some(vec![0x2A]));
        assert_eq!(
            read_parameter(&device, at, WINDOW, 1, 3).await.unwrap(),
            vec![0x2A]
        );
        let err = read_parameter(&device, at, WINDOW, 1, 4).await.unwrap_err();
        assert!(matches!(
            err,
            RfConfigurationError::InvalidParameter {
                channel: 1,
                parameter: 4
            }
        ));
    }

    #[tokio::test]
    async fn a_device_without_function_properties_says_so() {
        let device = SimulatedDevice::with_config(SimulatorConfig::default());
        let err = read_parameter(&device, device.address(), WINDOW, 1, 3)
            .await
            .unwrap_err();
        assert!(
            matches!(err, RfConfigurationError::NotAFunction { property_id: 65 }),
            "{err}"
        );
    }

    #[tokio::test]
    async fn a_parameter_write_needs_the_rf_scope_and_the_right_device() {
        let device = bidirectional();
        let wrong =
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap();
        let err = write_parameter(&device, fast(), wrong, 1, 3, &[1])
            .await
            .unwrap_err();
        assert!(matches!(err, RfConfigurationError::Session { .. }), "{err}");
        assert_eq!(device.rf_parameter(1, 3), None);
        // Authorised for another address: the frame goes there, and nobody
        // answers.
        let elsewhere = write_parameter(&device, fast(), rf(addr(1, 1, 99)), 1, 3, &[1])
            .await
            .unwrap_err();
        assert!(
            matches!(elsewhere, RfConfigurationError::NoAnswer { .. }),
            "{elsewhere}"
        );
        assert_eq!(device.rf_parameter(1, 3), None);
    }

    #[tokio::test]
    async fn links_are_set_and_deleted_and_bad_ones_refused() {
        let device = bidirectional();
        let at = device.address();
        let extended = LinkAddress::Extended {
            serial_number: OTHER_SERIAL.octets(),
            group_address: GroupAddress::from_raw(0x0004),
        };
        write_object_link(
            &device,
            fast(),
            rf(at),
            LinkChange::Add { sending: false },
            extended,
            4,
        )
        .await
        .unwrap();
        assert_eq!(
            device.rf_links(),
            vec![(4, OTHER_SERIAL.octets(), 4, false)]
        );
        write_object_link(&device, fast(), rf(at), LinkChange::Delete, extended, 4)
            .await
            .unwrap();
        assert!(device.rf_links().is_empty());
        let err = write_object_link(
            &device,
            fast(),
            rf(at),
            LinkChange::Add { sending: true },
            extended,
            6,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            RfConfigurationError::Link(ObjectLinkReturn::GroupObject)
        ));
    }

    #[tokio::test]
    async fn a_unidirectional_device_is_heard_and_its_addresses_calculated() {
        let device = SimulatedDevice::with_config_at(
            RF_UNIDIRECTIONAL_ADDRESS,
            SimulatorConfig {
                device_descriptor_2: Some(EXAMPLE_16),
                ..SimulatorConfig::default()
            },
        );
        let listening = collect_info_reports(&device, Duration::from_millis(30));
        let pressing = async {
            tokio::time::sleep(Duration::from_millis(5)).await;
            device.press_info_report_button();
        };
        let (reports, ()) = tokio::join!(listening, pressing);
        let reports = reports.unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].source, RF_UNIDIRECTIONAL_ADDRESS);
        let dd2 = DeviceDescriptor2::from_octets(&reports[0].device_descriptor).unwrap();
        let gas = pre_assigned_group_addresses(&dd2, |code| match code {
            0x0008 => Some(2),
            0x000E => Some(3),
            _ => None,
        })
        .unwrap();
        assert_eq!(gas.len(), 5);
    }

    #[test]
    fn an_addressed_descriptor_response_is_not_an_info_report() {
        let answer = LDataFrame {
            kind: LDataMessageKind::Indication,
            source: RF_UNIDIRECTIONAL_ADDRESS,
            destination: Destination::Individual(addr(1, 1, 250)),
            transport: crate::cemi::Tpci::UnnumberedData,
            service: ApplicationService::DeviceDescriptorResponse {
                descriptor_type: 2,
                data: EXAMPLE_16.to_vec(),
            },
        };
        assert_eq!(info_report(&answer), None);
    }

    #[test]
    fn hardware_is_refused_the_rf_scope() {
        assert!(
            !knx_core::commissioning::mutation::hardware_write_is_authorised(
                WriteScope::RfConfiguration
            )
        );
    }
}
