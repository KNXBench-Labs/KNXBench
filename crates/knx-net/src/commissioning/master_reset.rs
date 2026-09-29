//! Runs MP §3.7.3 `DM_Restart_RCo` with a Master Reset, support check first.
//!
//! `[D]` MP §3.7.3, pp. 88–90: *"The Management Client shall prior to calling
//! this Management Procedure with a Master Reset verify that this feature is
//! effectively supported by the Management Server. If not, the procedure
//! shall only be called with a Basic Restart."* Footnote 11: older devices
//! *"may ignore the service entirely, only perform a Basic Restart if a
//! Master Reset is called or exhibit another behaviour."*
//!
//! The Standard names no way to verify support. Ours, said so in the report:
//! a **Confirmed Restart** (Erase Code `01h`) first. It erases nothing (MP
//! Table 4), and only a device that implements the Master Reset form of
//! `A_Restart` answers it with an `A_Restart_Response`. No answer, or a
//! negative one, and the erasing request is not sent.
//!
//! Then, as the procedure has it:
//!
//! 1. `A_Restart` Master Reset with the requested Erase Code and Channel;
//! 2. the answer's Error Code (Table 5) and Process Time;
//! 3. `T_Disconnect` and MP §3.7.3 exception (5)'s 6 s wait (the session
//!    does this on every path);
//! 4. MP §3.7.1.2.2, p. 81: after the Process Time the next service is
//!    tried, and *"one last time"* before the procedure counts as failed.
//!    Here that service is a Device Descriptor read at the address the
//!    device should now have: `FFFFh` for `02h`/`03h`, else unchanged.

use knx_core::commissioning::load_state::MaskVersion;
use knx_core::commissioning::master_reset::{EraseCode, MasterResetRequest, RestartErrorCode};
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::IndividualAddress;

use crate::commissioning::{
    AuthorisationPlan, ManagementSession, MasterResetResponse, SessionError, SessionTiming,
};
use crate::management::ManagementTransport;

/// Where a device with Erase Code `02h`/`03h` goes on TP (MP Table 4:
/// *"the medium specific default IA"*; MP §2.18 names `FFFFh`).
pub const TP_DEFAULT_INDIVIDUAL_ADDRESS: IndividualAddress = IndividualAddress::from_raw(0xFFFF);

/// What a Master Reset did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterResetReport {
    pub request: MasterResetRequest,
    /// The support probe's answer (always Error Code `00h` here).
    pub probe: MasterResetResponse,
    /// The erasing request's answer. `None` when the request was the
    /// Confirmed Restart itself: the probe was the whole job.
    pub answer: Option<MasterResetResponse>,
    /// Where the device answered afterwards.
    pub address_after: IndividualAddress,
    /// Its mask version there.
    pub mask_after: MaskVersion,
}

/// Why a Master Reset stopped.
#[derive(Debug)]
pub enum MasterResetError {
    /// The probe went unanswered: MP §3.7.3 forbids the Master Reset. The
    /// device may have done a Basic Restart (footnote 11).
    SupportNotVerified { source: SessionError },
    /// The device answered the probe or the request negatively (Table 5).
    Refused {
        erase_code: EraseCode,
        error_code: RestartErrorCode,
    },
    /// After the Process Time and one last try, nothing answered where the
    /// device should be.
    NotBack {
        expected_at: IndividualAddress,
        source: SessionError,
    },
    /// A session failure in the named step.
    Session {
        step: &'static str,
        source: SessionError,
    },
}

impl std::fmt::Display for MasterResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SupportNotVerified { source } => write!(
                f,
                "MP §3.7.3: Master Reset support not verified (the Confirmed Restart \
                 probe got no A_Restart_Response: {source}); nothing was erased, the \
                 device may have restarted"
            ),
            Self::Refused {
                erase_code,
                error_code,
            } => write!(f, "the device refused {erase_code}: {error_code}"),
            Self::NotBack {
                expected_at,
                source,
            } => write!(
                f,
                "MP §3.7.1.2.2: no answer at {expected_at} after the Process Time and one \
                 last try: {source}"
            ),
            Self::Session { step, source } => write!(f, "Master Reset, {step}: {source}"),
        }
    }
}

impl std::error::Error for MasterResetError {}

fn at(step: &'static str) -> impl Fn(SessionError) -> MasterResetError {
    move |source| MasterResetError::Session { step, source }
}

fn refused(erase_code: EraseCode, answer: MasterResetResponse) -> Option<MasterResetError> {
    match RestartErrorCode::from_octet(answer.error_code) {
        RestartErrorCode::NoError => None,
        error_code => Some(MasterResetError::Refused {
            erase_code,
            error_code,
        }),
    }
}

/// Runs the probe, the request and the check afterwards.
///
/// `restart` authorises the probe ([`WriteScope::Restart`] on the device).
/// `erase` authorises the erasing request ([`WriteScope::MasterReset`] on
/// the same device) and must be `Some` unless the request is the Confirmed
/// Restart. Hardware refuses [`WriteScope::MasterReset`].
pub async fn master_reset<T: ManagementTransport>(
    transport: &T,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    request: MasterResetRequest,
    restart: WriteAuthorisation,
    erase: Option<WriteAuthorisation>,
) -> Result<MasterResetReport, MasterResetError> {
    let device = restart.target().address();
    let erasing = request.erase_code().erases();
    // Refuse a missing or mismatched erase authorisation before anything
    // reaches the bus, the probe included.
    if erasing {
        let Some(erase) = &erase else {
            return Err(at("authorisation")(SessionError::NoAuthorisation {
                scope: WriteScope::MasterReset,
            }));
        };
        erase
            .authorise(device, WriteScope::MasterReset)
            .map_err(|refused| at("authorisation")(SessionError::Refused(refused)))?;
    }

    // ---- Probe: a Confirmed Restart ----
    let mut session = ManagementSession::authorised(transport, plan, timing, restart)
        .map_err(at("authorisation"))?;
    session
        .connect()
        .await
        .map_err(at("connect for the probe"))?;
    let probe = session
        .restart_master_reset(EraseCode::ConfirmedRestart.octet(), 0)
        .await
        .map_err(|source| MasterResetError::SupportNotVerified { source })?;
    if let Some(err) = refused(EraseCode::ConfirmedRestart, probe) {
        return Err(err);
    }
    tokio::time::sleep(probe.recovery_wait(&timing)).await;

    // ---- The erasing request ----
    let answer = match (erasing, erase) {
        (true, Some(erase)) => {
            let mut session = ManagementSession::authorised(transport, plan, timing, erase)
                .map_err(at("authorisation"))?;
            session
                .connect()
                .await
                .map_err(at("connect for the Master Reset"))?;
            let answer = session
                .restart_master_reset(request.erase_code().octet(), request.channel_number())
                .await
                .map_err(at("A_Restart Master Reset"))?;
            if let Some(err) = refused(request.erase_code(), answer) {
                return Err(err);
            }
            tokio::time::sleep(answer.recovery_wait(&timing)).await;
            Some(answer)
        }
        _ => None,
    };

    // ---- Back? MP §3.7.1.2.2: try, and one last time ----
    let address_after = if request.erase_code().resets_individual_address() {
        TP_DEFAULT_INDIVIDUAL_ADDRESS
    } else {
        device
    };
    let mask_after = match read_mask_at(transport, plan, timing, address_after).await {
        Ok(mask) => mask,
        Err(_) => {
            tokio::time::sleep(timing.restart_responsive_again).await;
            read_mask_at(transport, plan, timing, address_after)
                .await
                .map_err(|source| MasterResetError::NotBack {
                    expected_at: address_after,
                    source,
                })?
        }
    };
    Ok(MasterResetReport {
        request,
        probe,
        answer,
        address_after,
        mask_after,
    })
}

async fn read_mask_at<T: ManagementTransport>(
    transport: &T,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    address: IndividualAddress,
) -> Result<MaskVersion, SessionError> {
    let mut session = ManagementSession::read_only(transport, address, plan, timing)?;
    session.connect().await?;
    let mask = session.read_mask_version().await;
    session.disconnect().await;
    mask
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};

    use super::*;
    use crate::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};

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

    fn auth(device: &SimulatedDevice, scope: WriteScope) -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(device.address(), scope).unwrap()
    }

    fn restarts(device: &SimulatedDevice) -> Vec<(u8, Vec<u8>)> {
        device
            .seen()
            .into_iter()
            .filter_map(|seen| match seen {
                Seen::Restart {
                    restart_type, data, ..
                } => Some((restart_type, data)),
                _ => None,
            })
            .collect()
    }

    async fn run(
        device: &SimulatedDevice,
        request: MasterResetRequest,
    ) -> Result<MasterResetReport, MasterResetError> {
        master_reset(
            device,
            AuthorisationPlan::Skip,
            fast(),
            request,
            auth(device, WriteScope::Restart),
            Some(auth(device, WriteScope::MasterReset)),
        )
        .await
    }

    #[tokio::test]
    async fn reset_links_probes_then_erases_and_the_counter_moves() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            download_counter: Some(7),
            programming_mode: true,
            ..Default::default()
        });
        let before = device.address();
        let request = MasterResetRequest::new(EraseCode::ResetLinks, 0).unwrap();
        let report = run(&device, request).await.unwrap();
        assert_eq!(
            restarts(&device),
            vec![(1, vec![0x01, 0x00]), (1, vec![0x06, 0x00])],
            "the Confirmed Restart probe first, then the request"
        );
        assert_eq!(report.address_after, before);
        assert!(report.answer.is_some());
        assert_eq!(
            device.master_resets(),
            vec![EraseCode::ConfirmedRestart, EraseCode::ResetLinks]
        );
        assert_eq!(
            device.download_counter(),
            Some(8),
            "RES: ResetLinks increments"
        );
        assert!(
            !device.programming_mode(),
            "every Master Reset ends in a Basic Restart"
        );
    }

    #[tokio::test]
    async fn a_factory_reset_is_found_again_at_ffff() {
        let device = SimulatedDevice::new();
        let request = MasterResetRequest::new(EraseCode::FactoryReset, 0).unwrap();
        let report = run(&device, request).await.unwrap();
        assert_eq!(report.address_after, TP_DEFAULT_INDIVIDUAL_ADDRESS);
        assert_eq!(device.address(), TP_DEFAULT_INDIVIDUAL_ADDRESS);
    }

    #[tokio::test]
    async fn the_confirmed_restart_alone_needs_no_erase_authorisation() {
        let device = SimulatedDevice::new();
        let report = master_reset(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            MasterResetRequest::confirmed_restart(),
            auth(&device, WriteScope::Restart),
            None,
        )
        .await
        .unwrap();
        assert_eq!(report.answer, None);
        assert_eq!(restarts(&device), vec![(1, vec![0x01, 0x00])]);
    }

    #[tokio::test]
    async fn an_unanswered_probe_sends_nothing_erasing() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            restart_unanswered: true,
            ..Default::default()
        });
        let request = MasterResetRequest::new(EraseCode::FactoryReset, 0).unwrap();
        let before = device.address();
        let err = run(&device, request).await.unwrap_err();
        assert!(
            matches!(err, MasterResetError::SupportNotVerified { .. }),
            "{err}"
        );
        assert!(device.master_resets().is_empty());
        assert_eq!(device.address(), before);
    }

    #[tokio::test]
    async fn a_negative_probe_answer_stops_before_the_request() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            restart_error_code: 0x01,
            ..Default::default()
        });
        let request = MasterResetRequest::new(EraseCode::ResetParameters, 0).unwrap();
        let err = run(&device, request).await.unwrap_err();
        match err {
            MasterResetError::Refused {
                erase_code,
                error_code,
            } => {
                assert_eq!(erase_code, EraseCode::ConfirmedRestart);
                assert_eq!(error_code, RestartErrorCode::AccessDenied);
            }
            other => panic!("expected a refusal, got {other}"),
        }
        assert_eq!(restarts(&device).len(), 1, "only the probe went out");
    }

    #[tokio::test]
    async fn a_missing_or_foreign_erase_authorisation_sends_nothing() {
        let device = SimulatedDevice::new();
        let request = MasterResetRequest::new(EraseCode::FactoryReset, 0).unwrap();
        let other = WriteAuthorisation::for_simulator(
            IndividualAddress::new(1, 1, 99).unwrap(),
            WriteScope::MasterReset,
        )
        .unwrap();
        for erase in [None, Some(other), Some(auth(&device, WriteScope::Restart))] {
            let err = master_reset(
                &device,
                AuthorisationPlan::Skip,
                fast(),
                request,
                auth(&device, WriteScope::Restart),
                erase,
            )
            .await
            .unwrap_err();
            assert!(matches!(err, MasterResetError::Session { .. }), "{err}");
        }
        assert!(device.seen().is_empty(), "{:?}", device.seen());
    }

    #[test]
    fn the_erasing_scope_is_refused_on_hardware() {
        assert!(
            !knx_core::commissioning::mutation::hardware_write_is_authorised(
                WriteScope::MasterReset
            )
        );
    }
}
