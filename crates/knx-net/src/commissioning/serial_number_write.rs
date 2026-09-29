//! Runs MP §2.5 `NM_IndividualAddress_SerialNumber_Write`: a new address by serial number.
//!
//! `[D]` `03_05_02 Management Procedures` §2.4/§2.5, pp. 16–17, and AL
//! §3.2.4/§3.2.5, pp. 21–24. Profiles Table 4.4, p. 44 makes the procedure
//! mandatory for BIM M112 (mask `0701h`).
//!
//! Four steps, each a clause's own:
//!
//! 1. **Find the device** (MP §2.4): a broadcast
//!    `A_IndividualAddressSerialNumber_Read`. No answer means *"there is no
//!    device present in the network with the given KNX Serial Number"*;
//!    the procedure stops. The answer's source is the device's current
//!    address, which must pass the project's exclusion guard like every
//!    other target.
//! 2. **The new address must be free** (MP §2.5 *Use*: *"The procedure
//!    shall ensure that the assigned Individual Address is unique"*). The
//!    clause names no way to do it, so this borrows MP §2.3 step 1's probe.
//!    Occupied by the device itself is fine (nothing to write); occupied by
//!    anyone else stops the procedure.
//! 3. **Write** (MP §2.5 step 1): a broadcast
//!    `A_IndividualAddressSerialNumber_Write`. Skipped when the device
//!    already holds the address.
//! 4. **Verify** (MP §2.5 step 2): read by serial number again. *"Different
//!    or no answer received ⇒ Error"*.
//!
//! **No restart.** The NOTE under §2.5's sequence: *"this procedure does not
//! reset the device after assigning the Individual Address"*, unlike
//! `NM_IndividualAddress_Write`.
//!
//! **Time-outs are borrowed, not mandated.** MP §2.4/§2.5 give no figure
//! for the response. The read waits [`SessionTiming::response_timeout`]
//! (TL clause 4's 3 s, this crate's "how long to wait for one answer").
//! A silent first verify is read once more after
//! [`SessionTiming::restart_basic_t1`], for the same reason and on the
//! same borrowed figure as `individual_address_write`'s step 4
//! (`RESEARCH.md` §8.8.6).

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::WriteAuthorisation;
use knx_core::commissioning::serial_number::SerialNumber;
use knx_core::{ContactableAddress, IndividualAddress};

use super::individual_address_write::{probe_occupancy, Occupancy};
use super::{broadcast_serial_number_read, ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// What happened, running `NM_IndividualAddress_SerialNumber_Write`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialNumberWriteReport {
    /// The device.
    pub serial_number: SerialNumber,
    /// Where step 1 found it.
    pub previous_address: IndividualAddress,
    /// Step 2's finding for the new address.
    pub occupancy: Occupancy,
    /// Whether step 3 broadcast the write, or skipped it because the device
    /// already held the address.
    pub wrote: bool,
    /// The address step 4 read back. Equal to the requested one, or the
    /// procedure would have failed.
    pub verified_address: IndividualAddress,
}

/// Why `NM_IndividualAddress_SerialNumber_Write` stopped.
#[derive(Debug)]
pub enum SerialNumberWriteError {
    /// Step 1: nobody answered to this serial number (MP §2.4).
    NotFound(SerialNumber),
    /// Step 1: the device answered from an address the project excludes.
    Excluded(knx_core::ExcludedAddress),
    /// Step 2: another device already holds the new address.
    OccupiedByAnotherDevice {
        /// How step 2 found it occupied.
        occupancy: Occupancy,
        /// Where the serial-numbered device still is.
        device_address: IndividualAddress,
    },
    /// Step 4: the device did not answer from the new address.
    NotVerified {
        /// The address it answered from, or `None` for silence. *"Different
        /// or no answer received ⇒ Error"* (MP §2.5 p. 17).
        answered_from: Option<IndividualAddress>,
    },
    /// A session or transport failure in the named step.
    Session {
        /// 1 to 4, as in the module docs.
        step: u8,
        /// What went wrong.
        source: SessionError,
    },
}

impl std::fmt::Display for SerialNumberWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(serial_number) => write!(
                f,
                "no device answered to serial number {serial_number}; MP §2.4: none is \
                 present on this network"
            ),
            Self::Excluded(excluded) => write!(
                f,
                "the device answered from {}, which this project never addresses",
                excluded.0
            ),
            Self::OccupiedByAnotherDevice { device_address, .. } => write!(
                f,
                "the new address is held by another device; the serial-numbered device \
                 stays at {device_address} (MP §2.5: the address must be unique)"
            ),
            Self::NotVerified {
                answered_from: Some(address),
            } => write!(
                f,
                "MP §2.5 step 2: the device still answers from {address}, not from the \
                 new address; a device whose PID_SERVICE_CONTROL bit 2 is clear \
                 (RES §4.2.8: \"it shall not be possible to change the Individual \
                 Address\") ignores the write exactly like this"
            ),
            Self::NotVerified {
                answered_from: None,
            } => f.write_str(
                "MP §2.5 step 2: no answer to the read-back; the write may not have \
                 landed, or the device's address changes are disabled \
                 (PID_SERVICE_CONTROL bit 2)",
            ),
            Self::Session { step, source } => write!(f, "MP §2.5 step {step}: {source}"),
        }
    }
}

impl std::error::Error for SerialNumberWriteError {}

fn at(step: u8) -> impl Fn(SessionError) -> SerialNumberWriteError {
    move |source| SerialNumberWriteError::Session { step, source }
}

/// MP §2.4 `NM_IndividualAddress_SerialNumber_Read` on its own: where the
/// device with `serial_number` is. `None` when nobody answered. Read-only;
/// no authorisation, no connection.
pub async fn serial_number_read<T: ManagementTransport>(
    transport: &T,
    serial_number: SerialNumber,
    timing: SessionTiming,
) -> Result<Option<IndividualAddress>, SessionError> {
    broadcast_serial_number_read(transport, serial_number, timing.response_timeout).await
}

/// Runs MP §2.5 end to end. The new address is the one
/// `programming_authorisation` names: the operator confirms that address,
/// so no second parameter can disagree with it.
pub async fn serial_number_write<T: ManagementTransport>(
    transport: &T,
    timing: SessionTiming,
    serial_number: SerialNumber,
    programming_authorisation: WriteAuthorisation,
) -> Result<SerialNumberWriteReport, SerialNumberWriteError> {
    let new_address = programming_authorisation.target().address();
    let writer = ManagementSession::authorised(
        transport,
        AuthorisationPlan::Skip,
        timing,
        programming_authorisation,
    )
    .map_err(at(1))?;

    // ---- Step 1: find the device (MP §2.4) ----
    let previous_address = serial_number_read(transport, serial_number, timing)
        .await
        .map_err(at(1))?
        .ok_or(SerialNumberWriteError::NotFound(serial_number))?;
    ContactableAddress::new(previous_address).map_err(SerialNumberWriteError::Excluded)?;

    // ---- Step 2: the new address must be free (MP §2.5 Use) ----
    let occupancy = if previous_address == new_address {
        Occupancy::OccupiedWithResponse
    } else {
        probe_occupancy(transport, new_address, timing)
            .await
            .map_err(at(2))?
    };
    if occupancy.is_occupied() && previous_address != new_address {
        return Err(SerialNumberWriteError::OccupiedByAnotherDevice {
            occupancy,
            device_address: previous_address,
        });
    }

    // ---- Step 3: write (MP §2.5 step 1) ----
    let wrote = previous_address != new_address;
    if wrote {
        writer
            .broadcast_serial_number_write(serial_number)
            .await
            .map_err(at(3))?;
    }

    // ---- Step 4: verify (MP §2.5 step 2) ----
    let mut answered = serial_number_read(transport, serial_number, timing)
        .await
        .map_err(at(4))?;
    if answered.is_none() {
        tokio::time::sleep(timing.restart_basic_t1).await;
        answered = serial_number_read(transport, serial_number, timing)
            .await
            .map_err(at(4))?;
    }
    match answered {
        Some(address) if address == new_address => Ok(SerialNumberWriteReport {
            serial_number,
            previous_address,
            occupancy,
            wrote,
            verified_address: address,
        }),
        answered_from => Err(SerialNumberWriteError::NotVerified { answered_from }),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};

    use super::*;
    use crate::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};

    const SERIAL: SerialNumber = SerialNumber::from_octets([0x00, 0x83, 0x12, 0x34, 0x56, 0x78]);
    const OTHER_SERIAL: SerialNumber =
        SerialNumber::from_octets([0x00, 0x83, 0x12, 0x34, 0x56, 0x79]);

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

    fn programming(target: IndividualAddress) -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(target, WriteScope::IndividualAddressProgramming)
            .expect("the target is not an excluded address")
    }

    fn with_serial(config: SimulatorConfig) -> SimulatedDevice {
        SimulatedDevice::with_config(SimulatorConfig {
            serial_number: Some(SERIAL.octets()),
            // Serial-number addressing needs no button; a device in
            // programming mode would hide a write that wrongly went through
            // the programming-mode path.
            programming_mode: false,
            ..config
        })
    }

    #[tokio::test]
    async fn read_finds_the_device_by_serial_number_and_nobody_by_another() {
        let device = with_serial(SimulatorConfig::default());
        assert_eq!(
            serial_number_read(&device, SERIAL, fast()).await.unwrap(),
            Some(device.address())
        );
        assert_eq!(
            serial_number_read(&device, OTHER_SERIAL, fast())
                .await
                .unwrap(),
            None,
            "MP §2.4: no answer means no such device"
        );
    }

    /// RES §4.22.1.3 rule 1: the property and the services agree, so the
    /// serial number read over a connection finds the same device again.
    #[tokio::test]
    async fn the_property_read_and_the_broadcast_name_the_same_device() {
        let device = with_serial(SimulatorConfig::default());
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            fast(),
        )
        .unwrap();
        session.connect().await.unwrap();
        let read = session.read_serial_number().await.unwrap();
        session.disconnect().await;
        assert_eq!(read, SERIAL);
        assert_eq!(
            serial_number_read(&device, read, fast()).await.unwrap(),
            Some(device.address())
        );
    }

    /// Another device's answer to another question arrives first. It is not
    /// an answer to this one (AL §3.2.4: only the matching device answers).
    #[tokio::test]
    async fn an_answer_carrying_another_serial_number_is_not_taken() {
        let stranger = addr(1, 1, 99);
        let device = with_serial(SimulatorConfig {
            foreign_serial_number_answer: Some((OTHER_SERIAL.octets(), stranger)),
            ..Default::default()
        });
        assert_eq!(
            serial_number_read(&device, SERIAL, fast()).await.unwrap(),
            Some(device.address())
        );
        let without = SimulatedDevice::with_config(SimulatorConfig {
            serial_number: None,
            programming_mode: false,
            foreign_serial_number_answer: Some((OTHER_SERIAL.octets(), stranger)),
            ..Default::default()
        });
        assert_eq!(
            serial_number_read(&without, SERIAL, fast()).await.unwrap(),
            None,
            "a stranger's answer does not make the device present"
        );
    }

    /// MP §2.5 end to end: found, the new address is free, written, and read
    /// back from the new address. No programming button, no restart.
    #[tokio::test]
    async fn writes_and_verifies_a_new_address_without_the_button() {
        let device = with_serial(SimulatorConfig::default());
        let before = device.address();
        let new_address = addr(1, 1, 68);
        assert_ne!(before, new_address);

        let report = serial_number_write(&device, fast(), SERIAL, programming(new_address))
            .await
            .expect("MP §2.5 on a willing device");

        assert_eq!(report.previous_address, before);
        assert_eq!(report.occupancy, Occupancy::NotOccupied);
        assert!(report.wrote);
        assert_eq!(report.verified_address, new_address);
        assert_eq!(device.address(), new_address);
        assert_eq!(device.serial_number_writes(), 1);
        assert!(
            !device
                .seen()
                .iter()
                .any(|entry| matches!(entry, Seen::Restart { .. })),
            "MP §2.5 NOTE: no reset after the write"
        );
    }

    /// The device already holds the address: nothing to write, but still
    /// verified by a read.
    #[tokio::test]
    async fn already_there_writes_nothing_and_still_verifies() {
        let device = with_serial(SimulatorConfig::default());
        let here = device.address();

        let report = serial_number_write(&device, fast(), SERIAL, programming(here))
            .await
            .expect("an idempotent run succeeds");

        assert!(!report.wrote);
        assert_eq!(report.verified_address, here);
        assert_eq!(device.serial_number_writes(), 0);
        assert_eq!(device.serial_number_reads(), 2, "found, then verified");
    }

    #[tokio::test]
    async fn an_unknown_serial_number_stops_before_anything_is_sent() {
        let device = with_serial(SimulatorConfig::default());
        let new_address = addr(1, 1, 68);

        let err = serial_number_write(&device, fast(), OTHER_SERIAL, programming(new_address))
            .await
            .expect_err("nobody answers to this serial number");

        assert!(matches!(err, SerialNumberWriteError::NotFound(s) if s == OTHER_SERIAL));
        assert_eq!(device.serial_number_writes(), 0);
    }

    /// MP §2.5 Use: *"The procedure shall ensure that the assigned
    /// Individual Address is unique"*. The simulated device sits at the new
    /// address; the serial-numbered device is somebody else.
    #[tokio::test]
    async fn an_occupied_new_address_stops_before_the_write() {
        let holder = addr(1, 1, 90);
        let device = with_serial(SimulatorConfig {
            serial_number_holder: Some(holder),
            ..Default::default()
        });
        let occupied = device.address();

        let err = serial_number_write(&device, fast(), SERIAL, programming(occupied))
            .await
            .expect_err("somebody else holds the new address");

        match err {
            SerialNumberWriteError::OccupiedByAnotherDevice {
                occupancy,
                device_address,
            } => {
                assert!(occupancy.is_occupied());
                assert_eq!(device_address, holder);
            }
            other => panic!("expected OccupiedByAnotherDevice, got {other:?}"),
        }
        assert_eq!(
            device.serial_number_writes(),
            0,
            "no write over an occupant"
        );
        assert_eq!(device.serial_number_holder_address(), holder);
    }

    /// The holder is elsewhere, the new address is free: the holder moves,
    /// the device that was probed stays where it is.
    #[tokio::test]
    async fn moves_the_serial_numbered_device_not_whoever_was_probed() {
        let holder = addr(1, 1, 90);
        let device = with_serial(SimulatorConfig {
            serial_number_holder: Some(holder),
            ..Default::default()
        });
        let bystander = device.address();
        let new_address = addr(1, 1, 91);

        let report = serial_number_write(&device, fast(), SERIAL, programming(new_address))
            .await
            .expect("the holder moves");

        assert_eq!(report.previous_address, holder);
        assert_eq!(device.serial_number_holder_address(), new_address);
        assert_eq!(device.address(), bystander);
    }

    /// MP §2.5 step 2: *"Different or no answer received ⇒ Error"*. A device
    /// with address changes disabled keeps answering from the old address.
    #[tokio::test]
    async fn a_refused_write_is_caught_by_the_read_back() {
        let device = with_serial(SimulatorConfig {
            serial_number_write_enabled: false,
            ..Default::default()
        });
        let before = device.address();
        let new_address = addr(1, 1, 68);

        let err = serial_number_write(&device, fast(), SERIAL, programming(new_address))
            .await
            .expect_err("the device ignored the write");

        assert!(
            matches!(err, SerialNumberWriteError::NotVerified { answered_from: Some(a) } if a == before),
            "{err:?}"
        );
    }

    /// A device answering from an address the project never contacts is
    /// not written to, even though the broadcast write would reach it.
    #[tokio::test]
    async fn a_device_found_at_an_excluded_address_is_left_alone() {
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0];
        let device = with_serial(SimulatorConfig {
            serial_number_holder: Some(excluded),
            ..Default::default()
        });
        let new_address = addr(1, 1, 68);

        let err = serial_number_write(&device, fast(), SERIAL, programming(new_address))
            .await
            .expect_err("the exclusion guard holds for broadcast targets too");

        assert!(
            matches!(err, SerialNumberWriteError::Excluded(_)),
            "{err:?}"
        );
        assert_eq!(device.serial_number_writes(), 0);
    }
}
