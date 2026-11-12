//! Bound the multi-request TCP peer and assert every outgoing read/control service and cleanup.

use super::{
    super::{read_frame, CHANNEL, DISCONNECT_RESPONSE, TCP_CONNECT_REQUEST, TCP_CONNECT_RESPONSE},
    Joined, WAIT,
};
use knx_net::{
    cemi::{decode_l_data, encode_l_data},
    ApplicationService as A, Destination, LDataFrame, LDataMessageKind, Tpci,
};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket},
    thread,
    time::{Duration, Instant},
};

pub(super) type Wire = Vec<(bool, Vec<u8>)>;
pub(super) struct Peer {
    stream: TcpStream,
    wire: Wire,
    outer_seq: u8,
}

pub(super) fn gateway(
    assigned: u8,
    script: impl FnOnce(&mut Peer) + Send + 'static,
) -> (SocketAddr, Joined<Wire>, Trap) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let udp = UdpSocket::bind(addr).unwrap();
    udp.set_nonblocking(true).unwrap();
    listener.set_nonblocking(true).unwrap();
    let trap = Trap {
        udp,
        tcp: listener.try_clone().unwrap(),
    };
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + WAIT;
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "probe never dialed TCP");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("TCP accept: {e}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        let mut peer = Peer {
            stream,
            wire: Vec::new(),
            outer_seq: 0,
        };
        assert_eq!(peer.read(), TCP_CONNECT_REQUEST);
        let mut response = TCP_CONNECT_RESPONSE;
        response[19] = assigned;
        peer.send(&response);
        script(&mut peer);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "no reconnect/replay"
        );
        peer.wire
    });
    (addr, Joined::new(worker), trap)
}

/// The gateway's own sockets, kept for checks after the CLI has exited.
pub(super) struct Trap {
    udp: UdpSocket,
    tcp: TcpListener,
}

/// After the CLI exited: no UDP fallback and no late TCP reconnect or replay.
pub(super) fn no_retry(trap: &Trap) {
    no_udp(&trap.udp);
    assert_eq!(
        trap.tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "no reconnect/replay after exit"
    );
}

pub(super) fn no_udp(trap: &UdpSocket) {
    assert_eq!(
        trap.recv_from(&mut [0; 2048]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "explicit TCP must never dial UDP"
    );
}

impl Peer {
    fn read(&mut self) -> Vec<u8> {
        let frame = read_frame(&mut self.stream);
        self.wire.push((true, frame.clone()));
        frame
    }
    pub fn send(&mut self, frame: &[u8]) {
        self.stream.write_all(frame).unwrap();
        self.wire.push((false, frame.to_vec()));
    }
    fn expect(&mut self, tpci: Tpci, service: A) -> LDataFrame {
        let raw = self.read();
        assert_eq!(&raw[..4], [6, 0x10, 4, 0x20], "TCP has no UDP tunnel ACK");
        assert_eq!(&raw[6..10], [4, CHANNEL, self.outer_seq, 0]);
        self.outer_seq = self.outer_seq.wrapping_add(1);
        let frame = decode_l_data(&raw[10..]).unwrap();
        assert_eq!(frame.kind, LDataMessageKind::Request);
        assert_eq!(frame.source.to_string(), "0.0.0");
        assert_eq!(
            frame.destination,
            Destination::Individual("1.1.67".parse().unwrap())
        );
        assert_eq!(frame.transport, tpci);
        assert_eq!(frame.service, service, "only independently enumerated reads/control PDUs allowed; no writes/key/restart/Verify Mode");
        frame
    }
    fn frame(&mut self, frame: LDataFrame) {
        let cemi = encode_l_data(&frame).unwrap();
        let mut bytes = vec![6, 0x10, 4, 0x20, 0, 0, 4, CHANNEL, 7, 0];
        bytes.extend_from_slice(&cemi);
        let size = (bytes.len() as u16).to_be_bytes();
        bytes[4..6].copy_from_slice(&size);
        self.send(&bytes);
    }
    pub fn indicate(&mut self, tpci: Tpci, service: A) {
        self.frame(LDataFrame {
            kind: LDataMessageKind::Indication,
            source: "1.1.67".parse().unwrap(),
            destination: Destination::Individual("1.1.250".parse().unwrap()),
            transport: tpci,
            service,
            control: None,
        });
    }
    pub fn begin(&mut self, refuse: bool) {
        let mut frame = self.expect(Tpci::Connect, A::NoApplicationPdu);
        frame.kind = LDataMessageKind::Confirmation { error: refuse };
        frame.source = "1.1.250".parse().unwrap();
        self.frame(frame);
    }
    pub fn request(&mut self, seq: u8, service: A) {
        self.expect(Tpci::NumberedData { seq }, service);
    }
    pub fn answer(&mut self, seq: u8, service: A) {
        self.indicate(Tpci::NumberedData { seq }, service);
        self.expect(Tpci::Ack { seq }, A::NoApplicationPdu);
    }
    pub fn descriptor_request(&mut self) {
        self.request(0, A::DeviceDescriptorRead { descriptor_type: 0 });
    }
    pub fn descriptor_answer(&mut self, data: Vec<u8>) {
        self.answer(
            0,
            A::DeviceDescriptorResponse {
                descriptor_type: 0,
                data,
            },
        );
    }
    pub fn property_request(&mut self, seq: u8, pid: u8) {
        self.request(
            seq,
            A::PropertyValueRead {
                object_index: 0,
                property_id: pid,
                nr_of_elem: 1,
                start_index: 1,
            },
        );
    }
    pub fn property_answer(&mut self, seq: u8, pid: u8, nr: u8, data: Vec<u8>) {
        self.answer(
            seq,
            A::PropertyValueResponse {
                object_index: 0,
                property_id: pid,
                nr_of_elem: nr,
                start_index: 1,
                data,
            },
        );
    }
    pub fn identity(&mut self) {
        self.begin(false);
        self.descriptor_request();
        self.descriptor_answer(vec![7, 1]);
        self.property_request(1, 12);
        self.property_answer(1, 12, 1, vec![0, 2]);
        self.property_request(2, 56);
        self.property_answer(2, 56, 1, vec![0, 15]);
    }
    pub fn memory_request(&mut self) {
        self.request(
            3,
            A::MemoryRead {
                address: 0x4400,
                number: 4,
            },
        );
    }
    pub fn memory_answer(&mut self, data: Vec<u8>) {
        self.answer(
            3,
            A::MemoryResponse {
                address: 0x4400,
                data,
            },
        );
    }
    pub fn control(&mut self, byte: u8) {
        self.request(
            4,
            A::MemoryRead {
                address: 0x4408,
                number: 1,
            },
        );
        self.answer(
            4,
            A::MemoryResponse {
                address: 0x4408,
                data: vec![byte],
            },
        );
    }
    pub fn management_end(&mut self) {
        self.expect(Tpci::Disconnect, A::NoApplicationPdu);
    }
    pub fn finish(&mut self) {
        assert_eq!(
            self.read(),
            [6, 0x10, 2, 9, 0, 16, CHANNEL, 0, 8, 2, 0, 0, 0, 0, 0, 0]
        );
        self.send(&DISCONNECT_RESPONSE);
        self.eof();
    }
    pub fn eof(&mut self) {
        let mut tail = Vec::new();
        self.stream
            .read_to_end(&mut tail)
            .expect("actual peer EOF required, timeout is failure");
        assert!(tail.is_empty(), "unexpected tail/replay: {tail:02x?}");
    }
    pub fn close_write(&mut self) {
        self.stream.shutdown(Shutdown::Write).unwrap();
        self.eof();
    }
    pub fn skips(&mut self) {
        self.send(&[6, 0x10, 9, 0x99, 0, 10, 0xde, 0xad, 0xbe, 0xef]);
        let mut large = vec![6, 0x10, 4, 0x20, 5, 0xdc];
        large.resize(1500, 0xee);
        self.send(&large);
    }
}
