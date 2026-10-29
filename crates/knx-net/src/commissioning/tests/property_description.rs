//! Exercises named-description correlation through encoded, sequenced connected peer replies.

use super::*;
use crate::cemi::{decode_l_data, encode_l_data, PropertyDescription};
use std::num::NonZeroU8;
use std::sync::Mutex;

struct Peer {
    events: broadcast::Sender<TunnelEvent>,
    replies: Vec<(u16, Vec<u8>)>,
    sent: Mutex<Vec<Vec<u8>>>,
}

impl Peer {
    fn new(replies: Vec<(u16, Vec<u8>)>) -> Self {
        Self {
            events: broadcast::channel(64).0,
            replies,
            sent: Mutex::new(vec![]),
        }
    }
    fn emit(&self, kind: LDataMessageKind, transport: Tpci, service: ApplicationService) {
        let frame = LDataFrame {
            kind,
            source: addr(1, 1, 67),
            destination: Destination::Individual(self.assigned_address()),
            transport,
            service,
            control: None,
        };
        let wire = encode_l_data(&frame).unwrap();
        let decoded = decode_l_data(&wire).unwrap();
        self.events.send(TunnelEvent::Telegram(decoded)).unwrap();
    }
}

impl ManagementTransport for Peer {
    fn assigned_address(&self) -> IndividualAddress {
        addr(1, 1, 254)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.events.subscribe()
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: self.assigned_address(),
            destination,
            transport,
            service: service.clone(),
            control: None,
        };
        self.sent
            .lock()
            .unwrap()
            .push(encode_l_data(&frame).unwrap());
        match transport {
            Tpci::Connect => {
                let mut confirmation = frame;
                confirmation.kind = LDataMessageKind::Confirmation { error: false };
                self.events
                    .send(TunnelEvent::Telegram(confirmation))
                    .unwrap();
            }
            Tpci::NumberedData { seq } => {
                self.emit(
                    LDataMessageKind::Indication,
                    Tpci::Ack { seq },
                    ApplicationService::NoApplicationPdu,
                );
                for (seq, (apci, data)) in self.replies.iter().enumerate() {
                    self.emit(
                        LDataMessageKind::Indication,
                        Tpci::NumberedData { seq: seq as u8 },
                        ApplicationService::Other {
                            apci: *apci,
                            data: data.clone(),
                        },
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
}

async fn request(peer: &Peer) -> Result<(u8, PropertyDescription), SessionError> {
    let mut session =
        ManagementSession::read_only(peer, addr(1, 1, 67), AuthorisationPlan::Skip, fast())
            .unwrap();
    session.connect().await.unwrap();
    let result = session
        .read_property_description(ObjectIndex::new(3), NonZeroU8::new(6).unwrap())
        .await;
    session.disconnect().await;
    result
}

#[tokio::test]
async fn named_description_accepts_response_index_zero_and_nonzero() {
    for index in [0, 0x37] {
        let peer = Peer::new(vec![(0x3d9, vec![3, 6, index, 0xc0, 0, 1, 0x89])]);
        assert_eq!(
            request(&peer).await.unwrap(),
            (index, PropertyDescription::from_raw([0xc0, 0, 1, 0x89]))
        );
        let sent = peer.sent.lock().unwrap();
        assert_eq!(&sent[1][9..], &[0x43, 0xd8, 3, 6, 0]);
        assert_eq!(&sent[2][9..], &[0xc2]);
    }
}

#[tokio::test]
async fn simulator_description_is_independent_of_value_presence_width_and_mask() {
    for mask in [0x0700, 0x0701, 0x0705] {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: mask,
            ..SimulatorConfig::default()
        });
        device.preset_property(3, 6, &[1]);
        let mut session = read_only(&device);
        session.connect().await.unwrap();
        assert!(matches!(
            session
                .read_property_description(ObjectIndex::new(3), NonZeroU8::new(6).unwrap())
                .await,
            Err(SessionError::NoAnswer { .. })
        ));
        device.preset_property_description(3, 6, 0x27, [0x42, 0x13, 0x57, 0x98]);
        assert_eq!(
            session
                .read_property_description(ObjectIndex::new(3), NonZeroU8::new(6).unwrap())
                .await
                .unwrap(),
            (
                0x27,
                PropertyDescription::from_raw([0x42, 0x13, 0x57, 0x98])
            )
        );
        device.preset_property(3, 6, &[1, 2, 3]);
        assert_eq!(
            session
                .read_property_description(ObjectIndex::new(3), NonZeroU8::new(6).unwrap())
                .await
                .unwrap(),
            (
                0x27,
                PropertyDescription::from_raw([0x42, 0x13, 0x57, 0x98])
            )
        );
        session.disconnect().await;
    }
}

fn wrong() -> Vec<(u16, Vec<u8>)> {
    vec![
        (0x3d9, vec![4, 6, 7, 0, 0, 1, 0]),
        (0x3d9, vec![3, 7, 7, 0, 0, 1, 0]),
        (0x3d9, vec![3, 0, 7, 0, 0, 1, 0]),
        (0x3d9, vec![3, 6, 7, 0, 0, 1]),
        (0x3d9, vec![3, 6, 7, 0, 0, 1, 0, 0]),
        (0x3d6, vec![3, 6, 0x10, 1, 1]),
    ]
}

#[tokio::test]
async fn named_description_ignores_wrong_replies_then_accepts_later_correct() {
    let mut replies = wrong();
    replies.push((0x3d9, vec![3, 6, 19, 0, 0, 1, 0]));
    assert_eq!(
        request(&Peer::new(replies)).await.unwrap(),
        (19, PropertyDescription::from_raw([0, 0, 1, 0]))
    );
}

#[tokio::test]
async fn named_description_wrong_only_or_silence_is_no_answer() {
    for replies in [wrong(), vec![]] {
        assert!(matches!(
            request(&Peer::new(replies)).await,
            Err(SessionError::NoAnswer {
                waiting_for: "A_PropertyDescription_Response",
                ..
            })
        ));
    }
}
