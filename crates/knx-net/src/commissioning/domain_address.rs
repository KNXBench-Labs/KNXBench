//! The domain-address procedures of MP §2.7–§2.14, for KNX-RF (K16).
//!
//! `[D]` `03_05_02 Management Procedures v02.01.02 AS` §2.7–§2.14,
//! pp. 18–27; the PDUs of AL §3.3.3–§3.3.7, pp. 34–42; CP §2.3.1, pp.
//! 17–21, for how RF uses them. **Simulator only**: no KNX-RF or PL110
//! device has ever been on this project's bus, and
//! [`WriteScope::DomainAddressProgramming`] is refused on hardware
//! (KNOWN_LIMITATIONS §143).
//!
//! | Procedure | Here |
//! |---|---|
//! | §2.7 `NM_DomainAddress_Read` | [`domain_address_read`] |
//! | §2.8 `NM_DomainAndIndividualAddress_Read` | [`domain_and_individual_address_read`] |
//! | §2.9 `NM_DomainAndIndividualAddress_Write` | [`domain_and_individual_address_write`] |
//! | §2.10 `NM_DomainAndIndividualAddress_Write2` | [`domain_and_individual_address_write2`] |
//! | §2.11 `…_Write3` | *"not yet specified"* — nothing to implement |
//! | §2.12 `NM_DomainAddressSerialNumber_Write` | [`domain_address_serial_number_write`] |
//! | §2.13 `…_Secure_Write` | not implemented: needs KNX Data Security |
//! | §2.14 `A_DomainAddressSelective_Read` | not implemented: 2-octet (PL110) only, AL NOTE 6 |
//!
//! **Which broadcast.** AL §3.3.3–§3.3.7 send every domain-address PDU
//! with `T_Data_SystemBroadcast`. CP §2.3.1.1 lets a client use the plain
//! broadcast instead when a TP1/RF media coupler forwards it
//! (§2.3.1.4); each function takes the [`Destination`] so the caller
//! decides, and every test uses the system broadcast unless it says so.
//!
//! **Time-outs.** §2.7 names 3 s for collecting responses and §2.9 step 2
//! 1 s; both are windows the client always waits out (*"The Management
//! Client shall always wait until the time-out has elapsed"*). §2.12 step 3
//! repeats its read *"until the MaS responds … or the timeout elapses"*
//! and points at KNX IP §4.3.5.3.4, whose 1 s / 60 s are for IP multicast
//! addresses, not RF; this module takes the caller's [`SessionTiming`]
//! windows and a bounded number of rounds ([`SERIAL_WRITE_VERIFY_ROUNDS`],
//! ours) and says so.

use std::time::Duration;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::domain_address::DomainAddress;
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::serial_number::SerialNumber;
use knx_core::{ContactableAddress, IndividualAddress};
use tokio::sync::broadcast;

use super::individual_address_write::{
    probe_occupancy, verify_before_restart, AddressRestart, Occupancy,
};
use super::memory_download::restart_may_have_gone_out;
use super::{ManagementSession, SessionError, SessionTiming};
use crate::cemi::{ApplicationService, Destination, LDataMessageKind, Tpci};
use crate::client::TunnelEvent;
use crate::management::ManagementTransport;

/// MP §2.7's collection window: *"time-out: 3 s"*.
pub const DOMAIN_ADDRESS_READ_WINDOW: Duration = Duration::from_secs(3);

/// How often §2.12 step 3's verify read is sent before the procedure gives
/// up. Ours: the clause bounds it by a time-out it borrows from KNX IP.
pub const SERIAL_WRITE_VERIFY_ROUNDS: u32 = 3;

/// One device's answer to a domain-address read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainAddressHolder {
    /// The answering frame's source (MP §2.7: *"source_address = IAn"*).
    pub individual_address: IndividualAddress,
    /// What it reported.
    pub domain_address: DomainAddress,
}

/// MP §2.7's verdict, in its own four cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAddressReadReport {
    /// Every response, in arrival order, duplicates included: *"If two or
    /// more responses with the same Domain Address and Individual Addresses
    /// are received, there is more than one device with the same Domain
    /// Address and the same Individual Addresses."*
    pub responses: Vec<DomainAddressHolder>,
}

impl DomainAddressReadReport {
    /// Whether two responses carried the same pair.
    pub fn has_duplicates(&self) -> bool {
        self.responses
            .iter()
            .enumerate()
            .any(|(i, a)| self.responses[..i].contains(a))
    }
}

/// Sends `service` on `destination` and collects, for the whole `window`,
/// every indication `matcher` accepts. The window is always waited out.
async fn collect<T: ManagementTransport, R>(
    transport: &T,
    destination: Destination,
    service: ApplicationService,
    window: Duration,
    waiting_for: &'static str,
    mut matcher: impl FnMut(&crate::cemi::LDataFrame) -> Option<R>,
) -> Result<Vec<R>, SessionError> {
    let mut events = transport.subscribe();
    transport
        .send_frame(destination, Tpci::UnnumberedData, service)
        .await
        .map_err(SessionError::Transport)?;
    let deadline = tokio::time::Instant::now() + window;
    let mut found = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(found);
        }
        match tokio::time::timeout(remaining, events.recv()).await {
            Ok(Ok(TunnelEvent::Telegram(frame))) => {
                // An `L_Data.con` is this client's own frame coming back
                // (MP §2.7: *"The Management Client shall not evaluate
                // Layer-2 repetitions"*).
                if frame.kind == LDataMessageKind::Indication {
                    if let Some(hit) = matcher(&frame) {
                        found.push(hit);
                    }
                }
            }
            Ok(Ok(TunnelEvent::Closed)) | Ok(Err(broadcast::error::RecvError::Closed)) => {
                return Err(SessionError::ConnectionLost {
                    during: waiting_for,
                });
            }
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                return Err(SessionError::Lagged { waiting_for });
            }
            Err(_) => {}
        }
    }
}

fn domain_response(frame: &crate::cemi::LDataFrame) -> Option<DomainAddressHolder> {
    match frame.service {
        ApplicationService::DomainAddressResponse { domain_address } => Some(DomainAddressHolder {
            individual_address: frame.source,
            domain_address,
        }),
        _ => None,
    }
}

/// MP §2.7 `NM_DomainAddress_Read`: the domain addresses of every device in
/// programming mode. No write, so no authorisation.
pub async fn domain_address_read<T: ManagementTransport>(
    transport: &T,
    destination: Destination,
    window: Duration,
) -> Result<DomainAddressReadReport, SessionError> {
    let responses = collect(
        transport,
        destination,
        ApplicationService::DomainAddressRead,
        window,
        "A_DomainAddress_Response",
        domain_response,
    )
    .await?;
    Ok(DomainAddressReadReport { responses })
}

/// What MP §2.8 found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAndIndividualAddressReadReport {
    /// The `A_DomainAddress_Response`s.
    pub domain_addresses: Vec<DomainAddressHolder>,
    /// The sources of the `A_IndividualAddress_Response`s.
    pub individual_addresses: Vec<IndividualAddress>,
}

/// MP §2.8 `NM_DomainAndIndividualAddress_Read`: the domain-address read,
/// then an `A_IndividualAddress_Read`, both on `destination` (the sequence
/// draws both as *"comm_mode = system broadcast"*).
pub async fn domain_and_individual_address_read<T: ManagementTransport>(
    transport: &T,
    destination: Destination,
    window: Duration,
) -> Result<DomainAndIndividualAddressReadReport, SessionError> {
    let domain_addresses = domain_address_read(transport, destination, window)
        .await?
        .responses;
    let individual_addresses = collect(
        transport,
        destination,
        ApplicationService::IndividualAddressRead,
        window,
        "A_IndividualAddress_Response",
        |frame| {
            (frame.service == ApplicationService::IndividualAddressResponse).then_some(frame.source)
        },
    )
    .await?;
    Ok(DomainAndIndividualAddressReadReport {
        domain_addresses,
        individual_addresses,
    })
}

/// Why a domain-address procedure stopped.
#[derive(Debug)]
pub enum DomainAddressError {
    /// Not exactly one device in programming mode (§2.9 "to 2.", §2.10
    /// step 1): *"The user of the Management Client should get an
    /// information, how many devices are in Programming Mode"*.
    ProgrammingModeCount {
        /// Who answered.
        devices: Vec<IndividualAddress>,
    },
    /// §2.9 step 1: the new individual address belongs to a device other
    /// than the one in programming mode (*"The Management Client shall not
    /// continue"*).
    OccupiedByAnotherDevice {
        occupancy: Occupancy,
        in_programming_mode: IndividualAddress,
    },
    /// The device answered in another domain-address format than the one
    /// to be written. AL §3.3.3: the address is encoded *"according to the
    /// Domain Address format used on the medium"*.
    WrongFormat {
        device: DomainAddress,
        requested: DomainAddress,
    },
    /// The verify step did not see the new values.
    NotVerified { detail: String },
    /// A session or transport failure in the named step.
    Session {
        step: &'static str,
        source: SessionError,
    },
}

impl std::fmt::Display for DomainAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProgrammingModeCount { devices } if devices.is_empty() => {
                f.write_str("no device is in programming mode")
            }
            Self::ProgrammingModeCount { devices } => {
                let list: Vec<_> = devices.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "several devices are in programming mode: {}",
                    list.join(", ")
                )
            }
            Self::OccupiedByAnotherDevice {
                in_programming_mode,
                ..
            } => write!(
                f,
                "the new individual address belongs to a device other than the one in \
                 programming mode ({in_programming_mode}); MP §2.9 stops here"
            ),
            Self::WrongFormat { device, requested } => write!(
                f,
                "the device reports domain address {device}, another medium's format than \
                 {requested}; nothing was written"
            ),
            Self::NotVerified { detail } => write!(f, "not verified: {detail}"),
            Self::Session { step, source } => write!(f, "{step}: {source}"),
        }
    }
}

impl std::error::Error for DomainAddressError {}

fn at(step: &'static str) -> impl Fn(SessionError) -> DomainAddressError {
    move |source| DomainAddressError::Session { step, source }
}

/// What §2.9/§2.10 did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAndIndividualAddressWriteReport {
    /// The device in programming mode, by the address it had.
    pub previous_address: IndividualAddress,
    /// Its domain address before, where the procedure read one (§2.9).
    pub previous_domain_address: Option<DomainAddress>,
    /// Whether the domain-address write went out (§2.9 skips it when the
    /// device already holds the value; §2.10 always sends it).
    pub wrote_domain_address: bool,
    /// Whether the individual-address write went out.
    pub wrote_individual_address: bool,
    /// §2.9 step 1's finding; `None` for §2.10, which does not look.
    pub occupancy: Option<Occupancy>,
    /// §2.9's closing restart; §2.10 sends its restart connectionless and
    /// has nothing to acknowledge it.
    pub restart: AddressRestart,
}

fn checked_authorisation(
    session: &ManagementSession<'_, impl ManagementTransport>,
    new_address: IndividualAddress,
) -> Result<(), DomainAddressError> {
    // The individual-address write sends the session's target, so an
    // authorisation for another address would program that one (the K13
    // lesson).
    if session.target() != new_address {
        return Err(at("authorisation")(SessionError::Refused(
            knx_core::commissioning::mutation::AuthorisationRefused::WrongTarget {
                authorised: session.target(),
                attempted: new_address,
            },
        )));
    }
    Ok(())
}

/// The one device in programming mode, or the count error. §2.10 step 1
/// reads with `A_IndividualAddress_Read`; §2.9 step 2 with
/// `A_DomainAddress_Read`, which also yields the current domain address.
fn exactly_one<T: Copy>(
    found: Vec<T>,
    address: impl Fn(&T) -> IndividualAddress,
) -> Result<T, DomainAddressError> {
    let mut devices: Vec<IndividualAddress> = found.iter().map(&address).collect();
    devices.sort_unstable_by_key(|a| a.raw());
    devices.dedup();
    match (found.first(), devices.len()) {
        (Some(one), 1) => Ok(*one),
        _ => Err(DomainAddressError::ProgrammingModeCount { devices }),
    }
}

/// MP §2.10 `NM_DomainAndIndividualAddress_Write2`, the procedure CP
/// §2.3.1.3 step 3 uses to give an RF device its domain and individual
/// address.
///
/// Four steps, as p. 24 draws them: (1) broadcast
/// `A_IndividualAddress_Read`, exactly one answer; (2) broadcast
/// `A_DomainAddress_Write`; (3) broadcast `A_IndividualAddress_Write` if the
/// address differs; (4) `A_DeviceDescriptor_Read` connectionless at the new
/// address — *"only interested in whether it receives a response or not"*
/// (note a)) — then `A_Restart` connectionless, *"The device shall quit
/// Programming Mode"*. It does not check that the new address is free
/// (*"It shall not check whether the Individual Address … is already
/// present"*): that is the procedure's, not ours, and the report says so by
/// leaving `occupancy` empty.
///
/// `domain_authorisation` names `new_address` and
/// [`WriteScope::DomainAddressProgramming`]; `address_authorisation` names
/// `new_address` and [`WriteScope::IndividualAddressProgramming`].
pub async fn domain_and_individual_address_write2<T: ManagementTransport>(
    transport: &T,
    destination: Destination,
    timing: SessionTiming,
    new_domain_address: DomainAddress,
    new_address: IndividualAddress,
    domain_authorisation: WriteAuthorisation,
    address_authorisation: WriteAuthorisation,
) -> Result<DomainAndIndividualAddressWriteReport, DomainAddressError> {
    let domain = ManagementSession::authorised(
        transport,
        AuthorisationPlan::Skip,
        timing,
        domain_authorisation,
    )
    .map_err(at("authorisation"))?;
    let addressing = ManagementSession::authorised(
        transport,
        AuthorisationPlan::Skip,
        timing,
        address_authorisation,
    )
    .map_err(at("authorisation"))?;
    checked_authorisation(&domain, new_address)?;
    checked_authorisation(&addressing, new_address)?;

    // Step 1.
    let found = addressing
        .broadcast_individual_address_read_to(
            destination,
            timing.programming_mode_broadcast_timeout,
        )
        .await
        .map_err(at("MP §2.10 step 1"))?;
    let previous_address = exactly_one(found.devices().collect(), |a| *a)?;
    ContactableAddress::new(previous_address)
        .map_err(|excluded| at("MP §2.10 step 1")(SessionError::Excluded(excluded)))?;

    // Step 2: always written; the procedure reads no domain address first.
    domain
        .broadcast_domain_address_write(destination, new_domain_address)
        .await
        .map_err(at("MP §2.10 step 2"))?;

    // Step 3.
    let wrote_individual_address = previous_address != new_address;
    if wrote_individual_address {
        addressing
            .broadcast_individual_address_write_to(
                destination,
                WriteScope::IndividualAddressProgramming,
            )
            .await
            .map_err(at("MP §2.10 step 3"))?;
    }

    // Step 4: connectionless, point-to-point.
    let answered = collect(
        transport,
        Destination::Individual(new_address),
        ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
        timing.response_timeout,
        "A_DeviceDescriptor_Response (connectionless)",
        |frame| {
            (frame.source == new_address
                && matches!(
                    frame.service,
                    ApplicationService::DeviceDescriptorResponse { .. }
                ))
            .then_some(())
        },
    )
    .await
    .map_err(at("MP §2.10 step 4"))?;
    if answered.is_empty() {
        return Err(DomainAddressError::NotVerified {
            detail: format!("no A_DeviceDescriptor_Response from {new_address} (MP §2.10 step 4)"),
        });
    }
    transport
        .send_frame(
            Destination::Individual(new_address),
            Tpci::UnnumberedData,
            ApplicationService::Restart {
                response: false,
                restart_type: 0,
                data: Vec::new(),
            },
        )
        .await
        .map_err(|e| at("MP §2.10 step 4 restart")(SessionError::Transport(e)))?;

    Ok(DomainAndIndividualAddressWriteReport {
        previous_address,
        previous_domain_address: None,
        wrote_domain_address: true,
        wrote_individual_address,
        occupancy: None,
        restart: AddressRestart::Unconfirmed {
            error: "a connectionless A_Restart has no acknowledgement (MP §2.10 step 4)"
                .to_string(),
        },
    })
}

/// MP §2.9 `NM_DomainAndIndividualAddress_Write`, the connection-oriented
/// variant: (1) the new address must be free or the device's own — MP §2.3
/// step 1's probe; (2) broadcast `A_DomainAddress_Read`, 1 s window,
/// exactly one answer; (3) write the domain address if it differs, the
/// individual address if it differs; (4) connect to the new address, read
/// the Device Descriptor, `A_Restart`.
///
/// `restart_authorisation` names `new_address` and [`WriteScope::Restart`].
#[allow(clippy::too_many_arguments)]
pub async fn domain_and_individual_address_write<T: ManagementTransport>(
    transport: &T,
    destination: Destination,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    new_domain_address: DomainAddress,
    new_address: IndividualAddress,
    domain_authorisation: WriteAuthorisation,
    address_authorisation: WriteAuthorisation,
    restart_authorisation: WriteAuthorisation,
) -> Result<DomainAndIndividualAddressWriteReport, DomainAddressError> {
    let domain = ManagementSession::authorised(
        transport,
        AuthorisationPlan::Skip,
        timing,
        domain_authorisation,
    )
    .map_err(at("authorisation"))?;
    let addressing = ManagementSession::authorised(
        transport,
        AuthorisationPlan::Skip,
        timing,
        address_authorisation,
    )
    .map_err(at("authorisation"))?;
    checked_authorisation(&domain, new_address)?;
    checked_authorisation(&addressing, new_address)?;

    // Step 1.
    let occupancy = probe_occupancy(transport, new_address, timing)
        .await
        .map_err(at("MP §2.9 step 1"))?;

    // Step 2: *"time-out: 1 s"*, the programming-mode window.
    let found = domain_address_read(
        transport,
        destination,
        timing.programming_mode_broadcast_timeout,
    )
    .await
    .map_err(at("MP §2.9 step 2"))?;
    let holder = exactly_one(found.responses, |h| h.individual_address)?;
    ContactableAddress::new(holder.individual_address)
        .map_err(|excluded| at("MP §2.9 step 2")(SessionError::Excluded(excluded)))?;
    if occupancy.is_occupied() && holder.individual_address != new_address {
        return Err(DomainAddressError::OccupiedByAnotherDevice {
            occupancy,
            in_programming_mode: holder.individual_address,
        });
    }
    if !holder.domain_address.same_format(&new_domain_address) {
        return Err(DomainAddressError::WrongFormat {
            device: holder.domain_address,
            requested: new_domain_address,
        });
    }

    // Step 3.
    let wrote_domain_address = holder.domain_address != new_domain_address;
    if wrote_domain_address {
        domain
            .broadcast_domain_address_write(destination, new_domain_address)
            .await
            .map_err(at("MP §2.9 step 3"))?;
    }
    let wrote_individual_address = holder.individual_address != new_address;
    if wrote_individual_address {
        addressing
            .broadcast_individual_address_write_to(
                destination,
                WriteScope::IndividualAddressProgramming,
            )
            .await
            .map_err(at("MP §2.9 step 3"))?;
    }

    // Step 4.
    let mut finishing =
        ManagementSession::authorised(transport, plan, timing, restart_authorisation)
            .map_err(at("MP §2.9 step 4"))?;
    if let Err(err) = verify_before_restart(&mut finishing, timing).await {
        finishing.disconnect().await;
        return Err(at("MP §2.9 step 4")(err));
    }
    let restart = match finishing.restart_basic().await {
        Ok(()) => AddressRestart::Acknowledged,
        Err(err) if restart_may_have_gone_out(&err) => AddressRestart::Unconfirmed {
            error: err.to_string(),
        },
        Err(err) => return Err(at("MP §2.9 step 4")(err)),
    };

    Ok(DomainAndIndividualAddressWriteReport {
        previous_address: holder.individual_address,
        previous_domain_address: Some(holder.domain_address),
        wrote_domain_address,
        wrote_individual_address,
        occupancy: Some(occupancy),
        restart,
    })
}

/// AL §3.3.6 `A_DomainAddressSerialNumber_Read`: the domain address of the
/// device with `serial_number`, or `None` when nobody answers in `window`.
/// Only an answer naming this serial number counts.
pub async fn domain_address_serial_number_read<T: ManagementTransport>(
    transport: &T,
    destination: Destination,
    serial_number: SerialNumber,
    window: Duration,
) -> Result<Option<DomainAddressHolder>, SessionError> {
    let wanted = serial_number.octets();
    let found = collect(
        transport,
        destination,
        ApplicationService::DomainAddressSerialNumberRead {
            serial_number: wanted,
        },
        window,
        "A_DomainAddressSerialNumber_Response",
        |frame| match frame.service {
            ApplicationService::DomainAddressSerialNumberResponse {
                serial_number,
                domain_address,
            } if serial_number == wanted => Some(DomainAddressHolder {
                individual_address: frame.source,
                domain_address,
            }),
            _ => None,
        },
    )
    .await?;
    Ok(found.into_iter().next())
}

/// What §2.12 did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAddressSerialNumberWriteReport {
    pub serial_number: SerialNumber,
    /// How many write-and-verify rounds it took.
    pub rounds: u32,
    /// Where the device answered the verify read from.
    pub answered_from: IndividualAddress,
}

/// MP §2.12 `NM_DomainAddressSerialNumber_Write`, for RF/PL domain
/// addresses: write by serial number, wait, then read back by serial number.
///
/// Step 3 verifies with `A_IndividualAddressSerialNumber_Read`, whose
/// response carries only a two-octet domain address — enough to prove the
/// device is there, not to prove a six-octet RF domain address landed. This
/// function therefore also reads the domain address back with
/// `A_DomainAddressSerialNumber_Read` and compares it; that second read is
/// ours, not the procedure's. Steps 1 and 4 (a KNXnet/IP router's
/// system-broadcast routing mode) are the caller's: no router exists in the
/// simulator. *"If this entire Management Procedure fails, the MaC (ETS)
/// shall not automatically repeat it."*
pub async fn domain_address_serial_number_write<T: ManagementTransport>(
    transport: &T,
    destination: Destination,
    timing: SessionTiming,
    serial_number: SerialNumber,
    new_domain_address: DomainAddress,
    authorisation: WriteAuthorisation,
) -> Result<DomainAddressSerialNumberWriteReport, DomainAddressError> {
    let session =
        ManagementSession::authorised(transport, AuthorisationPlan::Skip, timing, authorisation)
            .map_err(at("authorisation"))?;
    let mut last = String::from("nothing was read");
    for round in 1..=SERIAL_WRITE_VERIFY_ROUNDS {
        // Step 2.
        session
            .broadcast_domain_address_serial_number_write(
                destination,
                serial_number,
                new_domain_address,
            )
            .await
            .map_err(at("MP §2.12 step 2"))?;
        // Step 3: *"the MaC waits 1 second"* — `restart_basic_t1` is that
        // second in this crate's timing set.
        tokio::time::sleep(timing.restart_basic_t1).await;
        let present =
            super::broadcast_serial_number_read(transport, serial_number, timing.response_timeout)
                .await
                .map_err(at("MP §2.12 step 3"))?;
        let Some(answered_from) = present else {
            last = format!("no A_IndividualAddressSerialNumber_Response in round {round}");
            // *"the MaC first repeats from 2 after a delay of 1 second"*.
            tokio::time::sleep(timing.restart_basic_t1).await;
            continue;
        };
        let read_back = domain_address_serial_number_read(
            transport,
            destination,
            serial_number,
            timing.response_timeout,
        )
        .await
        .map_err(at("domain-address read-back"))?;
        match read_back {
            Some(holder) if holder.domain_address == new_domain_address => {
                return Ok(DomainAddressSerialNumberWriteReport {
                    serial_number,
                    rounds: round,
                    answered_from,
                });
            }
            Some(holder) => {
                last = format!(
                    "the device reports domain address {} in round {round}",
                    holder.domain_address
                );
            }
            None => last = format!("no A_DomainAddressSerialNumber_Response in round {round}"),
        }
        tokio::time::sleep(timing.restart_basic_t1).await;
    }
    Err(DomainAddressError::NotVerified { detail: last })
}

#[cfg(test)]
mod tests {
    use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};

    use super::*;
    use crate::cemi::SYSTEM_BROADCAST_DESTINATION;
    use crate::commissioning::simulator::{SimulatedDevice, SimulatorConfig};

    const SB: Destination = SYSTEM_BROADCAST_DESTINATION;
    const OLD_DOA: DomainAddress = DomainAddress::Rf([0x00, 0xFA, 0x00, 0x00, 0x00, 0x01]);
    const NEW_DOA: DomainAddress = DomainAddress::Rf([0x00, 0xFA, 0x12, 0x34, 0x56, 0x78]);
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

    fn auth(target: IndividualAddress, scope: WriteScope) -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(target, scope).expect("not an excluded address")
    }

    /// An RF device, in programming mode unless the test says otherwise
    /// with [`SimulatedDevice::set_programming_mode`].
    fn rf_device(config: SimulatorConfig) -> SimulatedDevice {
        SimulatedDevice::with_config(SimulatorConfig {
            domain_address: Some(OLD_DOA),
            serial_number: Some(SERIAL.octets()),
            programming_mode: true,
            ..config
        })
    }

    async fn write2(
        device: &SimulatedDevice,
        doa: DomainAddress,
        new: IndividualAddress,
    ) -> Result<DomainAndIndividualAddressWriteReport, DomainAddressError> {
        domain_and_individual_address_write2(
            device,
            SB,
            fast(),
            doa,
            new,
            auth(new, WriteScope::DomainAddressProgramming),
            auth(new, WriteScope::IndividualAddressProgramming),
        )
        .await
    }

    async fn write1(
        device: &SimulatedDevice,
        doa: DomainAddress,
        new: IndividualAddress,
    ) -> Result<DomainAndIndividualAddressWriteReport, DomainAddressError> {
        domain_and_individual_address_write(
            device,
            SB,
            AuthorisationPlan::Skip,
            fast(),
            doa,
            new,
            auth(new, WriteScope::DomainAddressProgramming),
            auth(new, WriteScope::IndividualAddressProgramming),
            auth(new, WriteScope::Restart),
        )
        .await
    }

    #[tokio::test]
    async fn the_read_reports_every_device_in_programming_mode_and_only_those() {
        let device = rf_device(SimulatorConfig::default());
        let report = domain_address_read(&device, SB, Duration::from_millis(20))
            .await
            .unwrap();
        assert_eq!(
            report.responses,
            vec![DomainAddressHolder {
                individual_address: device.address(),
                domain_address: OLD_DOA,
            }]
        );
        assert!(!report.has_duplicates());
        device.set_programming_mode(false);
        let silent = domain_address_read(&device, SB, Duration::from_millis(20))
            .await
            .unwrap();
        assert!(
            silent.responses.is_empty(),
            "AL §3.3.4: programming mode only"
        );
    }

    #[tokio::test]
    async fn a_tp_device_has_no_domain_address_to_report() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..SimulatorConfig::default()
        });
        let report = domain_and_individual_address_read(&device, SB, Duration::from_millis(20))
            .await
            .unwrap();
        assert!(report.domain_addresses.is_empty());
        assert_eq!(report.individual_addresses, vec![device.address()]);
    }

    #[test]
    fn two_equal_pairs_are_reported_as_duplicates() {
        let holder = DomainAddressHolder {
            individual_address: addr(1, 1, 5),
            domain_address: OLD_DOA,
        };
        let report = DomainAddressReadReport {
            responses: vec![holder, holder],
        };
        assert!(report.has_duplicates(), "MP §2.7 case 4");
    }

    #[tokio::test]
    async fn write2_gives_the_device_both_addresses_and_ends_programming_mode() {
        let device = rf_device(SimulatorConfig::default());
        let new = addr(1, 1, 42);
        let report = write2(&device, NEW_DOA, new).await.unwrap();
        assert_eq!(device.domain_address(), Some(NEW_DOA));
        assert_eq!(device.address(), new);
        assert!(!device.programming_mode(), "MP §2.10 step 4");
        assert!(report.wrote_domain_address && report.wrote_individual_address);
        assert_eq!(report.occupancy, None, "§2.10 does not check occupancy");
        assert!(matches!(report.restart, AddressRestart::Unconfirmed { .. }));
    }

    #[tokio::test]
    async fn write2_keeps_an_unchanged_individual_address_and_still_writes_the_domain() {
        let device = rf_device(SimulatorConfig::default());
        let same = device.address();
        let report = write2(&device, NEW_DOA, same).await.unwrap();
        assert!(!report.wrote_individual_address);
        assert!(report.wrote_domain_address);
        assert_eq!(device.domain_address_writes(), 1);
    }

    #[tokio::test]
    async fn write2_refuses_nobody_and_crowds() {
        let device = rf_device(SimulatorConfig::default());
        device.set_programming_mode(false);
        let err = write2(&device, NEW_DOA, addr(1, 1, 42)).await.unwrap_err();
        assert!(
            matches!(err, DomainAddressError::ProgrammingModeCount { ref devices } if devices.is_empty())
        );
        assert_eq!(device.domain_address(), Some(OLD_DOA));

        let crowd = rf_device(SimulatorConfig {
            other_programming_mode_devices: vec![addr(1, 1, 77)],
            ..SimulatorConfig::default()
        });
        let err = write2(&crowd, NEW_DOA, addr(1, 1, 42)).await.unwrap_err();
        assert!(
            matches!(err, DomainAddressError::ProgrammingModeCount { ref devices } if devices.len() == 2)
        );
        assert_eq!(crowd.domain_address(), Some(OLD_DOA), "nothing written");
    }

    /// §2.10 does not read the domain address back, so a device that took
    /// only the individual address (here: a TP device, which has no domain
    /// address) still passes step 4. The report says *wrote*, never
    /// *verified*; §2.12's serial-number path is the one that reads back.
    #[tokio::test]
    async fn write2_reports_the_domain_write_as_sent_not_as_verified() {
        let tp = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..SimulatorConfig::default()
        });
        let new = addr(1, 1, 42);
        let report = write2(&tp, NEW_DOA, new).await.unwrap();
        assert!(report.wrote_domain_address);
        assert_eq!(tp.domain_address(), None);
        assert_eq!(tp.address(), new);
    }

    #[tokio::test]
    async fn write2_fails_when_the_device_is_silent_at_the_new_address() {
        // Programming mode ends right after step 1's count, so the device
        // ignores the individual-address write (AL §3.2.2) and step 4's
        // connectionless read at the new address goes unanswered.
        let device = rf_device(SimulatorConfig::default());
        let new = addr(1, 1, 42);
        let original = device.address();
        let err = domain_and_individual_address_write2(
            &DropsProgrammingModeAfterCount(&device),
            SB,
            fast(),
            NEW_DOA,
            new,
            auth(new, WriteScope::DomainAddressProgramming),
            auth(new, WriteScope::IndividualAddressProgramming),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainAddressError::NotVerified { .. }),
            "{err}"
        );
        assert_eq!(device.address(), original);
    }

    /// A device whose programming mode ends right after it answered the
    /// step 1 count — the operator's button timing out, say.
    struct DropsProgrammingModeAfterCount<'a>(&'a SimulatedDevice);

    impl ManagementTransport for DropsProgrammingModeAfterCount<'_> {
        fn subscribe(&self) -> tokio::sync::broadcast::Receiver<crate::client::TunnelEvent> {
            self.0.subscribe()
        }

        async fn send_frame(
            &self,
            destination: Destination,
            transport: Tpci,
            service: ApplicationService,
        ) -> Result<(), crate::client::BusError> {
            let count = service == ApplicationService::IndividualAddressRead;
            let result = self.0.send_frame(destination, transport, service).await;
            if count {
                self.0.set_programming_mode(false);
            }
            result
        }

        fn target_kind(&self) -> knx_core::commissioning::mutation::TargetKind {
            self.0.target_kind()
        }

        fn assigned_address(&self) -> IndividualAddress {
            self.0.assigned_address()
        }
    }

    #[tokio::test]
    async fn every_write_is_refused_without_the_domain_scope() {
        let device = rf_device(SimulatorConfig::default());
        let new = addr(1, 1, 42);
        let err = domain_and_individual_address_write2(
            &device,
            SB,
            fast(),
            NEW_DOA,
            new,
            auth(new, WriteScope::IndividualAddressProgramming),
            auth(new, WriteScope::IndividualAddressProgramming),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainAddressError::Session { .. }), "{err}");
        assert_eq!(device.domain_address(), Some(OLD_DOA));
        assert_ne!(device.address(), new);
    }

    #[tokio::test]
    async fn an_authorisation_for_another_address_is_refused_before_any_frame() {
        let device = rf_device(SimulatorConfig::default());
        let err = domain_and_individual_address_write2(
            &device,
            SB,
            fast(),
            NEW_DOA,
            addr(1, 1, 42),
            auth(addr(1, 1, 43), WriteScope::DomainAddressProgramming),
            auth(addr(1, 1, 43), WriteScope::IndividualAddressProgramming),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            DomainAddressError::Session {
                step: "authorisation",
                ..
            }
        ));
        assert_eq!(device.individual_address_read_broadcasts(), 0);
    }

    #[tokio::test]
    async fn write1_reads_first_and_writes_only_what_differs() {
        let device = rf_device(SimulatorConfig::default());
        let new = addr(1, 1, 42);
        let report = write1(&device, NEW_DOA, new).await.unwrap();
        assert_eq!(report.previous_domain_address, Some(OLD_DOA));
        assert!(report.wrote_domain_address && report.wrote_individual_address);
        assert_eq!(device.domain_address(), Some(NEW_DOA));
        assert_eq!(device.address(), new);
        assert_eq!(report.occupancy, Some(Occupancy::NotOccupied));
        assert!(!device.programming_mode());

        // Again, nothing differs: nothing is written.
        device.set_programming_mode(true);
        let again = write1(&device, NEW_DOA, new).await.unwrap();
        assert!(!again.wrote_domain_address && !again.wrote_individual_address);
        assert_eq!(device.domain_address_writes(), 1);
    }

    #[tokio::test]
    async fn write1_refuses_a_domain_address_of_another_medium() {
        let device = rf_device(SimulatorConfig::default());
        let err = write1(&device, DomainAddress::Powerline(0x1234), addr(1, 1, 42))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DomainAddressError::WrongFormat { .. }),
            "{err}"
        );
        assert_eq!(device.domain_address(), Some(OLD_DOA));
    }

    #[tokio::test]
    async fn serial_write_sets_the_domain_without_programming_mode_and_reads_it_back() {
        let device = rf_device(SimulatorConfig::default());
        device.set_programming_mode(false);
        let report = domain_address_serial_number_write(
            &device,
            SB,
            fast(),
            SERIAL,
            NEW_DOA,
            auth(device.address(), WriteScope::DomainAddressProgramming),
        )
        .await
        .unwrap();
        assert_eq!(report.rounds, 1);
        assert_eq!(report.answered_from, device.address());
        assert_eq!(device.domain_address(), Some(NEW_DOA));
    }

    #[tokio::test]
    async fn serial_write_to_another_serial_changes_nothing_and_says_so() {
        let device = rf_device(SimulatorConfig::default());
        let err = domain_address_serial_number_write(
            &device,
            SB,
            fast(),
            OTHER_SERIAL,
            NEW_DOA,
            auth(device.address(), WriteScope::DomainAddressProgramming),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainAddressError::NotVerified { .. }),
            "{err}"
        );
        assert_eq!(device.domain_address(), Some(OLD_DOA));
    }

    #[tokio::test]
    async fn serial_write_catches_a_device_that_ignores_it() {
        let device = rf_device(SimulatorConfig {
            domain_address_write_enabled: false,
            ..SimulatorConfig::default()
        });
        let err = domain_address_serial_number_write(
            &device,
            SB,
            fast(),
            SERIAL,
            NEW_DOA,
            auth(device.address(), WriteScope::DomainAddressProgramming),
        )
        .await
        .unwrap_err();
        match err {
            DomainAddressError::NotVerified { detail } => {
                assert!(detail.contains("reports domain address"), "{detail}")
            }
            other => panic!("{other}"),
        }
    }

    #[tokio::test]
    async fn the_serial_read_takes_only_the_matching_answer() {
        let device = rf_device(SimulatorConfig::default());
        assert_eq!(
            domain_address_serial_number_read(&device, SB, SERIAL, Duration::from_millis(20))
                .await
                .unwrap()
                .map(|h| h.domain_address),
            Some(OLD_DOA)
        );
        assert_eq!(
            domain_address_serial_number_read(&device, SB, OTHER_SERIAL, Duration::from_millis(20))
                .await
                .unwrap(),
            None
        );
    }

    /// Another device's answer to another serial number arrives first and
    /// must not be taken (AL §3.3.6: only the matching device answers).
    #[tokio::test]
    async fn the_serial_read_ignores_a_foreign_answer_that_arrives_first() {
        let device = rf_device(SimulatorConfig {
            foreign_serial_number_answer: Some((OTHER_SERIAL.octets(), addr(1, 1, 99))),
            ..SimulatorConfig::default()
        });
        let found =
            domain_address_serial_number_read(&device, SB, SERIAL, Duration::from_millis(20))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(found.individual_address, device.address());
    }

    #[tokio::test]
    async fn hardware_is_refused_the_domain_scope() {
        assert!(
            !knx_core::commissioning::mutation::hardware_write_is_authorised(
                WriteScope::DomainAddressProgramming
            ),
            "KNOWN_LIMITATIONS §143"
        );
    }
}
