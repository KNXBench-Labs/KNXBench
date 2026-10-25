//! A memory response of the other service is no answer to a read or an active-Verify write.

use super::*;
use crate::cemi::{
    decode_l_data, encode_l_data, ApplicationService, Destination, LDataMessageKind, Tpci,
};
use crate::client::BusError;
use knx_core::commissioning::memory::MemoryService;
use knx_core::commissioning::mutation::TargetKind;
use std::sync::Mutex;

use MemoryService::{Memory, UserMemory};

/// Synthetic nonconforming peer: AL v02.01.01 §3.5.3 and §3.5.6.2 answer a read, and §3.5.4 and
/// §3.5.6.3 an active-Verify write, with the response of the request's own service only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Case {
    /// Unchanged device answer of the requested service.
    SameService,
    /// The answer carries the other service, same numeric address, same octets.
    WrongOnly,
    /// The other service first with inverted octets, the device's own answer one sequence later.
    WrongThenCorrect,
    /// The other service, same address, no octets.
    WrongEmpty,
    /// The requested service, one address above the request, the device's octets.
    WrongAddress,
    /// Control: the requested service refuses, as the simulator's protected region does.
    SameEmpty,
}

/// Where the region starts: standard memory below the 16-bit limit, user memory across it.
#[derive(Clone, Copy, Debug)]
enum Space {
    Standard,
    User,
}

const LENGTH: usize = 8;

impl Space {
    fn base(self) -> u32 {
        match self {
            Space::Standard => 0x4000,
            Space::User => 0xFFFC,
        }
    }

    fn service(self) -> MemoryService {
        match self {
            Space::Standard => Memory,
            Space::User => UserMemory,
        }
    }

    fn other(self) -> MemoryService {
        match self {
            Space::Standard => UserMemory,
            Space::User => Memory,
        }
    }

    fn protected(self) -> (u32, u32) {
        (self.base(), self.base() + 0x1000)
    }
}

fn pattern() -> Vec<u8> {
    (0..LENGTH as u8).map(|i| 0x11 + 0x22 * i).collect()
}

fn inverted(octets: &[u8]) -> Vec<u8> {
    octets.iter().map(|octet| !octet).collect()
}

/// What reached the session for a memory response, read back from the decoded frame.
#[derive(Debug, PartialEq, Eq)]
struct Delivered {
    service: MemoryService,
    address: u32,
    seq: u8,
    data: Vec<u8>,
}

#[derive(Default)]
struct Log {
    /// Added to the sequence of every later device frame, after a frame was injected.
    offset: u8,
    injected: Option<u8>,
    fired: bool,
    /// Device `NumberedData` frames seen before the first memory response.
    numbered_before: usize,
    numbered: usize,
    requests: Vec<ApplicationService>,
    acks_from_session: Vec<u8>,
    acks_to_device: Vec<u8>,
    delivered: Vec<Delivered>,
}

/// Wraps the simulator: requests run through the cEMI encoder, and the memory response is
/// rewritten into a real frame that goes through `encode_l_data`/`decode_l_data` before the
/// session's subscription sees it.
struct Pairing<'a> {
    device: &'a SimulatedDevice,
    tx: broadcast::Sender<TunnelEvent>,
    case: Case,
    log: Mutex<Log>,
}

impl<'a> Pairing<'a> {
    fn new(device: &'a SimulatedDevice, case: Case) -> Self {
        Self {
            device,
            tx: broadcast::channel(64).0,
            case,
            log: Mutex::new(Log::default()),
        }
    }

    /// The same service one address above, so it still fits 16 bit for standard memory.
    fn next_address(service: &ApplicationService) -> ApplicationService {
        match service {
            ApplicationService::MemoryResponse { address, data } => {
                ApplicationService::MemoryResponse {
                    address: address
                        .checked_add(1)
                        .expect("the next address fits 16 bit"),
                    data: data.clone(),
                }
            }
            ApplicationService::UserMemoryResponse { address, data } => {
                ApplicationService::UserMemoryResponse {
                    address: address
                        .checked_add(1)
                        .expect("the next address fits 32 bit"),
                    data: data.clone(),
                }
            }
            other => panic!("not a memory response: {other:?}"),
        }
    }

    fn other_family(service: &ApplicationService, data: Vec<u8>) -> ApplicationService {
        match service {
            ApplicationService::MemoryResponse { address, .. } => {
                ApplicationService::UserMemoryResponse {
                    address: u32::from(*address),
                    data,
                }
            }
            ApplicationService::UserMemoryResponse { address, .. } => {
                ApplicationService::MemoryResponse {
                    address: u16::try_from(*address).expect("the wrong address fits 16 bit"),
                    data,
                }
            }
            other => panic!("not a memory response: {other:?}"),
        }
    }

    /// Encodes, decodes and checks the frame is unchanged, records what decoded, publishes it.
    fn deliver(&self, frame: LDataFrame, seq: u8, log: &mut Log) {
        let bytes = encode_l_data(&frame).expect("the response encodes");
        let decoded = decode_l_data(&bytes).expect("the response decodes");
        assert_eq!(decoded, frame, "the cEMI round trip preserves the frame");
        assert_eq!(decoded.kind, LDataMessageKind::Indication);
        let (service, address, data) = match &decoded.service {
            ApplicationService::MemoryResponse { address, data } => {
                (Memory, u32::from(*address), data.clone())
            }
            ApplicationService::UserMemoryResponse { address, data } => {
                (UserMemory, *address, data.clone())
            }
            other => panic!("decoded as {other:?}"),
        };
        log.delivered.push(Delivered {
            service,
            address,
            seq,
            data,
        });
        let _ = self.tx.send(TunnelEvent::Telegram(decoded));
    }

    fn relay(&self, event: TunnelEvent) {
        let TunnelEvent::Telegram(mut frame) = event else {
            let _ = self.tx.send(event);
            return;
        };
        let mut log = self.log.lock().unwrap();
        let Tpci::NumberedData { seq } = frame.transport else {
            drop(log);
            let _ = self.tx.send(TunnelEvent::Telegram(frame));
            return;
        };
        let is_memory = matches!(
            frame.service,
            ApplicationService::MemoryResponse { .. }
                | ApplicationService::UserMemoryResponse { .. }
        );
        if is_memory && !log.fired {
            log.fired = true;
            log.numbered_before = log.numbered;
        }
        log.numbered += 1;
        let out = seq.wrapping_add(log.offset) & 0x0F;
        frame.transport = Tpci::NumberedData { seq: out };
        if !is_memory {
            drop(log);
            let _ = self.tx.send(TunnelEvent::Telegram(frame));
            return;
        }
        let true_data = match &frame.service {
            ApplicationService::MemoryResponse { data, .. }
            | ApplicationService::UserMemoryResponse { data, .. } => data.clone(),
            _ => unreachable!(),
        };
        let wrong = |data: Vec<u8>, frame: &LDataFrame| LDataFrame {
            service: Self::other_family(&frame.service, data),
            ..frame.clone()
        };
        match self.case {
            Case::SameService | Case::SameEmpty => self.deliver(frame, out, &mut log),
            Case::WrongOnly => {
                let replaced = wrong(true_data, &frame);
                self.deliver(replaced, out, &mut log);
            }
            Case::WrongEmpty => {
                let replaced = wrong(Vec::new(), &frame);
                self.deliver(replaced, out, &mut log);
            }
            Case::WrongAddress => {
                let replaced = LDataFrame {
                    service: Self::next_address(&frame.service),
                    ..frame.clone()
                };
                self.deliver(replaced, out, &mut log);
            }
            Case::WrongThenCorrect => {
                let replaced = wrong(inverted(&true_data), &frame);
                self.deliver(replaced, out, &mut log);
                log.injected = Some(out);
                log.offset = log.offset.wrapping_add(1);
                let next = out.wrapping_add(1) & 0x0F;
                frame.transport = Tpci::NumberedData { seq: next };
                self.deliver(frame, next, &mut log);
            }
        }
    }
}

impl ManagementTransport for Pairing<'_> {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.tx.subscribe()
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let request = LDataFrame {
            kind: LDataMessageKind::Request,
            source: knx_core::IndividualAddress::from_raw(0),
            destination,
            transport,
            service: service.clone(),
            control: None,
        };
        encode_l_data(&request).map_err(|e| BusError::Protocol(e.to_string()))?;
        let to_device = {
            let mut log = self.log.lock().unwrap();
            match transport {
                Tpci::Ack { seq } => {
                    log.acks_from_session.push(seq);
                    if log.injected == Some(seq) {
                        return Ok(());
                    }
                    let original = seq.wrapping_sub(log.offset) & 0x0F;
                    log.acks_to_device.push(original);
                    Tpci::Ack { seq: original }
                }
                other => {
                    log.requests.push(service.clone());
                    other
                }
            }
        };
        let mut events = self.device.subscribe();
        self.device
            .send_frame(destination, to_device, service)
            .await?;
        while let Ok(event) = events.try_recv() {
            self.relay(event);
        }
        Ok(())
    }

    fn target_kind(&self) -> TargetKind {
        ManagementTransport::target_kind(self.device)
    }
}

fn device_for(space: Space, case: Case) -> SimulatedDevice {
    device_with_verify(space, case, true)
}

fn device_with_verify(space: Space, case: Case, verify_mode_supported: bool) -> SimulatedDevice {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        protected_memory: (case == Case::SameEmpty).then(|| space.protected()),
        verify_mode_supported,
        ..SimulatorConfig::default()
    });
    device.preset_memory(space.base(), &pattern());
    device
}

fn timing() -> SessionTiming {
    fast()
}

fn memory_operations(device: &SimulatedDevice) -> Vec<Seen> {
    device
        .seen()
        .into_iter()
        .filter(|seen| matches!(seen, Seen::MemoryRead { .. } | Seen::MemoryWrite { .. }))
        .collect()
}

/// The frames the session must have been handed, checked before the result so setup cannot pass.
fn check_frames(space: Space, case: Case, wire: &Pairing<'_>) {
    let log = wire.log.lock().unwrap();
    let s = (log.numbered_before & 0x0F) as u8;
    let next = (s + 1) & 0x0F;
    let at = |service, seq, data: &[u8]| Delivered {
        service,
        address: space.base(),
        seq,
        data: data.to_vec(),
    };
    let expected = match case {
        Case::SameService => vec![at(space.service(), s, &pattern())],
        Case::WrongOnly => vec![at(space.other(), s, &pattern())],
        Case::WrongThenCorrect => vec![
            at(space.other(), s, &inverted(&pattern())),
            at(space.service(), next, &pattern()),
        ],
        Case::WrongEmpty => vec![at(space.other(), s, &[])],
        Case::WrongAddress => vec![Delivered {
            address: space.base() + 1,
            ..at(space.service(), s, &pattern())
        }],
        Case::SameEmpty => vec![at(space.service(), s, &[])],
    };
    assert_eq!(log.delivered, expected, "frames the session received");
}

/// The acknowledgements, checked after the result so a wrong result is named first.
fn check_acks(case: Case, wire: &Pairing<'_>) {
    let log = wire.log.lock().unwrap();
    let s = (log.numbered_before & 0x0F) as u8;
    let next = (s + 1) & 0x0F;
    let (from_session, to_device) = match case {
        Case::WrongThenCorrect => (vec![s, next], vec![s]),
        _ => (vec![s], vec![s]),
    };
    let tail = |acks: &[u8]| acks[log.numbered_before..].to_vec();
    assert_eq!(tail(&log.acks_from_session), from_session, "T_ACKs sent");
    assert_eq!(
        tail(&log.acks_to_device),
        to_device,
        "T_ACKs the device got"
    );
}

fn read_request(space: Space) -> ApplicationService {
    match space.service() {
        Memory => ApplicationService::MemoryRead {
            number: LENGTH as u8,
            address: space.base() as u16,
        },
        UserMemory => ApplicationService::UserMemoryRead {
            number: LENGTH as u8,
            address: space.base(),
        },
    }
}

fn is_no_answer(result: &Result<impl std::fmt::Debug, SessionError>) -> bool {
    matches!(result, Err(SessionError::NoAnswer { .. }))
}

async fn read_case(space: Space, case: Case) {
    let device = device_for(space, case);
    let wire = Pairing::new(&device, case);
    let mut session =
        ManagementSession::read_only(&wire, device.address(), AuthorisationPlan::Skip, timing())
            .expect("the simulated device is contactable");
    session.connect().await.expect("connect");
    assert!(memory_operations(&device).is_empty(), "nothing read yet");

    let result = session.read_memory(space.base(), LENGTH as u8).await;

    let request = match space.service() {
        Memory => ApplicationService::MemoryRead {
            number: LENGTH as u8,
            address: space.base() as u16,
        },
        UserMemory => ApplicationService::UserMemoryRead {
            number: LENGTH as u8,
            address: space.base(),
        },
    };
    let sent: Vec<_> = wire
        .log
        .lock()
        .unwrap()
        .requests
        .iter()
        .filter(|s| {
            matches!(
                s,
                ApplicationService::MemoryRead { .. } | ApplicationService::UserMemoryRead { .. }
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        sent,
        vec![request],
        "exactly one read of the requested service"
    );
    assert_eq!(
        memory_operations(&device),
        vec![Seen::MemoryRead {
            address: space.base(),
            number: LENGTH as u8,
            service: space.service()
        }]
    );
    check_frames(space, case, &wire);

    match case {
        Case::SameService | Case::WrongThenCorrect => {
            let bytes = result.expect("the requested service's answer is the read");
            assert_eq!(bytes, pattern(), "the device's own octets");
        }
        Case::WrongOnly => assert!(
            is_no_answer(&result),
            "an answer of the other service must not complete the read, got {result:?}"
        ),
        Case::WrongAddress => assert!(
            is_no_answer(&result),
            "an answer at another address must not complete the read, got {result:?}"
        ),
        Case::WrongEmpty => assert!(
            is_no_answer(&result),
            "an empty answer of the other service is no refusal, got {result:?}"
        ),
        Case::SameEmpty => assert!(
            matches!(result, Err(SessionError::MemoryRefused { address }) if address == space.base()),
            "control: a refusal of the requested service, got {result:?}"
        ),
    }
    check_acks(case, &wire);
}

async fn write_case(space: Space, case: Case) {
    write_case_with(space, case, VerifyMode::Active).await;
}

/// With `Unavailable` the simulator answers no write, so the session reads the chunk back and the
/// response under test is that read's.
async fn write_case_with(space: Space, case: Case, verify: VerifyMode) {
    let device = device_with_verify(space, case, verify == VerifyMode::Active);
    let wire = Pairing::new(&device, case);
    let mut session = ManagementSession::authorised(
        &wire,
        AuthorisationPlan::Skip,
        timing(),
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download)
            .expect("the simulated device is not an excluded address"),
    )
    .expect("a simulator authorisation against a simulator transport is accepted");
    session.connect().await.expect("connect");
    assert_eq!(session.verify_mode(), Some(verify));
    assert!(memory_operations(&device).is_empty(), "nothing written yet");

    let limit = write_limit(ApduLengthSource::Absent);
    assert_eq!(limit.max_octets(), 12);
    let result = session
        .write_memory_region(space.base(), &pattern(), limit)
        .await;

    let request = match space.service() {
        Memory => ApplicationService::MemoryWrite {
            address: space.base() as u16,
            data: pattern(),
        },
        UserMemory => ApplicationService::UserMemoryWrite {
            address: space.base(),
            data: pattern(),
        },
    };
    let sent: Vec<_> = wire
        .log
        .lock()
        .unwrap()
        .requests
        .iter()
        .filter(|s| {
            matches!(
                s,
                ApplicationService::MemoryWrite { .. } | ApplicationService::UserMemoryWrite { .. }
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        sent,
        vec![request],
        "exactly one write of the requested service"
    );
    let written = Seen::MemoryWrite {
        address: space.base(),
        data: pattern(),
        service: space.service(),
    };
    let expected_operations = match verify {
        VerifyMode::Active => vec![written],
        VerifyMode::Unavailable => {
            let read_back = read_request(space);
            let sent_reads: Vec<_> = wire
                .log
                .lock()
                .unwrap()
                .requests
                .iter()
                .filter(|s| {
                    matches!(
                        s,
                        ApplicationService::MemoryRead { .. }
                            | ApplicationService::UserMemoryRead { .. }
                    )
                })
                .cloned()
                .collect();
            assert_eq!(
                sent_reads,
                vec![read_back],
                "exactly one read-back of the written service"
            );
            vec![
                written,
                Seen::MemoryRead {
                    address: space.base(),
                    number: LENGTH as u8,
                    service: space.service(),
                },
            ]
        }
    };
    assert_eq!(memory_operations(&device), expected_operations);
    check_frames(space, case, &wire);

    match case {
        Case::SameService | Case::WrongThenCorrect => {
            assert_eq!(result.expect("the requested service's answer verifies"), 1);
        }
        Case::WrongOnly => assert!(
            is_no_answer(&result),
            "an answer of the other service must not verify the write, got {result:?}"
        ),
        Case::WrongAddress => assert!(
            is_no_answer(&result),
            "an answer at another address must not verify the write, got {result:?}"
        ),
        Case::WrongEmpty => assert!(
            is_no_answer(&result),
            "an empty answer of the other service is no refusal, got {result:?}"
        ),
        Case::SameEmpty => assert!(
            matches!(result, Err(SessionError::MemoryRefused { address }) if address == space.base()),
            "control: a refusal of the requested service, got {result:?}"
        ),
    }
    check_acks(case, &wire);
}

#[tokio::test]
async fn standard_read_same_service_answer_is_the_result() {
    read_case(Space::Standard, Case::SameService).await;
}

#[tokio::test]
async fn standard_read_wrong_service_only_is_no_answer() {
    read_case(Space::Standard, Case::WrongOnly).await;
}

#[tokio::test]
async fn standard_read_wrong_service_then_correct_answer_uses_the_correct_one() {
    read_case(Space::Standard, Case::WrongThenCorrect).await;
}

#[tokio::test]
async fn standard_read_wrong_service_empty_is_no_answer() {
    read_case(Space::Standard, Case::WrongEmpty).await;
}

#[tokio::test]
async fn standard_read_same_service_empty_is_refused_control() {
    read_case(Space::Standard, Case::SameEmpty).await;
}

#[tokio::test]
async fn user_read_same_service_answer_is_the_result() {
    read_case(Space::User, Case::SameService).await;
}

#[tokio::test]
async fn user_read_wrong_service_only_is_no_answer() {
    read_case(Space::User, Case::WrongOnly).await;
}

#[tokio::test]
async fn user_read_wrong_service_then_correct_answer_uses_the_correct_one() {
    read_case(Space::User, Case::WrongThenCorrect).await;
}

#[tokio::test]
async fn user_read_wrong_service_empty_is_no_answer() {
    read_case(Space::User, Case::WrongEmpty).await;
}

#[tokio::test]
async fn user_read_same_service_empty_is_refused_control() {
    read_case(Space::User, Case::SameEmpty).await;
}

#[tokio::test]
async fn standard_write_same_service_answer_is_the_result() {
    write_case(Space::Standard, Case::SameService).await;
}

#[tokio::test]
async fn standard_write_wrong_service_only_is_no_answer() {
    write_case(Space::Standard, Case::WrongOnly).await;
}

#[tokio::test]
async fn standard_write_wrong_service_then_correct_answer_uses_the_correct_one() {
    write_case(Space::Standard, Case::WrongThenCorrect).await;
}

#[tokio::test]
async fn standard_write_wrong_service_empty_is_no_answer() {
    write_case(Space::Standard, Case::WrongEmpty).await;
}

#[tokio::test]
async fn standard_write_same_service_empty_is_refused_control() {
    write_case(Space::Standard, Case::SameEmpty).await;
}

#[tokio::test]
async fn user_write_same_service_answer_is_the_result() {
    write_case(Space::User, Case::SameService).await;
}

#[tokio::test]
async fn user_write_wrong_service_only_is_no_answer() {
    write_case(Space::User, Case::WrongOnly).await;
}

#[tokio::test]
async fn user_write_wrong_service_then_correct_answer_uses_the_correct_one() {
    write_case(Space::User, Case::WrongThenCorrect).await;
}

#[tokio::test]
async fn user_write_wrong_service_empty_is_no_answer() {
    write_case(Space::User, Case::WrongEmpty).await;
}

#[tokio::test]
async fn user_write_same_service_empty_is_refused_control() {
    write_case(Space::User, Case::SameEmpty).await;
}

#[tokio::test]
async fn standard_read_same_service_wrong_address_is_no_answer() {
    read_case(Space::Standard, Case::WrongAddress).await;
}

#[tokio::test]
async fn user_read_same_service_wrong_address_is_no_answer() {
    read_case(Space::User, Case::WrongAddress).await;
}

#[tokio::test]
async fn standard_write_same_service_wrong_address_is_no_answer() {
    write_case(Space::Standard, Case::WrongAddress).await;
}

#[tokio::test]
async fn user_write_same_service_wrong_address_is_no_answer() {
    write_case(Space::User, Case::WrongAddress).await;
}

#[tokio::test]
async fn standard_unverified_write_read_back_same_service_is_the_result() {
    write_case_with(Space::Standard, Case::SameService, VerifyMode::Unavailable).await;
}

#[tokio::test]
async fn standard_unverified_write_read_back_wrong_service_is_no_answer() {
    write_case_with(Space::Standard, Case::WrongOnly, VerifyMode::Unavailable).await;
}

#[tokio::test]
async fn user_unverified_write_read_back_same_service_is_the_result() {
    write_case_with(Space::User, Case::SameService, VerifyMode::Unavailable).await;
}

#[tokio::test]
async fn user_unverified_write_read_back_wrong_service_is_no_answer() {
    write_case_with(Space::User, Case::WrongOnly, VerifyMode::Unavailable).await;
}
