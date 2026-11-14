//! Bridge the existing management simulator through bounded loopback TCP/cEMI for HTTP tests.
use knx_net::{
    cemi::{decode_l_data, encode_l_data},
    Destination, LDataFrame, LDataMessageKind, Tpci,
};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    thread,
    time::{Duration, Instant},
};
const CHANNEL: u8 = 0x21;
const TCP_CONNECT_REQUEST: [u8; 26] = [
    6, 0x10, 2, 5, 0, 26, 8, 2, 0, 0, 0, 0, 0, 0, 8, 2, 0, 0, 0, 0, 0, 0, 4, 4, 2, 0,
];
const TCP_CONNECT_RESPONSE: [u8; 20] = [
    6, 0x10, 2, 6, 0, 20, CHANNEL, 0, 8, 2, 0, 0, 0, 0, 0, 0, 4, 4, 0x11, 250,
];
const DISCONNECT_RESPONSE: [u8; 8] = [6, 0x10, 2, 10, 0, 8, CHANNEL, 0];
const WAIT: Duration = Duration::from_secs(15);
fn read_frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut header = [0; 6];
    stream.read_exact(&mut header).unwrap();
    let size = u16::from_be_bytes([header[4], header[5]]) as usize;
    assert!((6..=4096).contains(&size));
    let mut frame = vec![0; size];
    frame[..6].copy_from_slice(&header);
    stream.read_exact(&mut frame[6..]).unwrap();
    frame
}
pub(super) struct Joined<T>(Option<thread::JoinHandle<T>>);
impl<T> Joined<T> {
    fn new(handle: thread::JoinHandle<T>) -> Self {
        Self(Some(handle))
    }
    pub(super) fn join(mut self) -> thread::Result<T> {
        self.0.take().unwrap().join()
    }
}
impl<T> Drop for Joined<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            let _ = handle.join();
        }
    }
}
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
    /// Bridge the maintained management simulator through real TCP/cEMI, without new encoders.
    pub(super) fn simulate(
        &mut self,
        device: &knx_net::commissioning::simulator::SimulatedDevice,
        skips: bool,
        mut observe: impl FnMut(&LDataFrame) -> bool,
    ) {
        use knx_net::{ManagementTransport, TunnelEvent};
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut events = device.subscribe();
        let mut injected = false;
        let mut management_open = false;
        let mut management_sessions = 0;
        loop {
            let raw = self.read();
            if raw[2..4] == [2, 9] {
                assert!(
                    management_sessions > 0,
                    "completed management acquisition required"
                );
                assert!(
                    !management_open,
                    "management disconnect must precede tunnel disconnect"
                );
                assert_eq!(
                    raw,
                    [6, 0x10, 2, 9, 0, 16, CHANNEL, 0, 8, 2, 0, 0, 0, 0, 0, 0]
                );
                self.send(&DISCONNECT_RESPONSE);
                self.eof();
                break;
            }
            assert_eq!(&raw[..4], [6, 0x10, 4, 0x20]);
            assert_eq!(&raw[6..10], [4, CHANNEL, self.outer_seq, 0]);
            self.outer_seq = self.outer_seq.wrapping_add(1);
            let frame = decode_l_data(&raw[10..]).unwrap();
            assert_eq!(frame.kind, LDataMessageKind::Request);
            assert_eq!(frame.destination, Destination::Individual(device.address()));
            match frame.transport {
                Tpci::Connect => {
                    assert!(!management_open, "no nested/replayed management connect");
                    management_open = true;
                    management_sessions += 1;
                }
                Tpci::Disconnect => {
                    assert!(
                        management_open,
                        "disconnect belongs to an opened management session"
                    );
                    management_open = false;
                }
                _ => assert!(management_open, "no management traffic outside the session"),
            }
            if observe(&frame) {
                self.skips();
                self.stream.shutdown(std::net::Shutdown::Write).unwrap();
                self.eof();
                break;
            }
            if skips && !injected && matches!(frame.transport, Tpci::NumberedData { .. }) {
                self.skips();
                injected = true;
            }
            runtime
                .block_on(device.send_frame(frame.destination, frame.transport, frame.service))
                .unwrap();
            while let Ok(event) = events.try_recv() {
                let TunnelEvent::Telegram(mut reply) = event else {
                    panic!("unexpected simulator event: {event:?}");
                };
                if matches!(reply.kind, LDataMessageKind::Confirmation { .. }) {
                    reply.source = "1.1.250".parse().unwrap();
                } else {
                    reply.destination = Destination::Individual("1.1.250".parse().unwrap());
                }
                self.frame(reply);
            }
        }
    }

    fn read(&mut self) -> Vec<u8> {
        let frame = read_frame(&mut self.stream);
        self.wire.push((true, frame.clone()));
        frame
    }
    pub fn send(&mut self, frame: &[u8]) {
        self.stream.write_all(frame).unwrap();
        self.wire.push((false, frame.to_vec()));
    }
    fn frame(&mut self, frame: LDataFrame) {
        let cemi = encode_l_data(&frame).unwrap();
        let mut bytes = vec![6, 0x10, 4, 0x20, 0, 0, 4, CHANNEL, 7, 0];
        bytes.extend_from_slice(&cemi);
        let size = (bytes.len() as u16).to_be_bytes();
        bytes[4..6].copy_from_slice(&size);
        self.send(&bytes);
    }
    pub fn eof(&mut self) {
        let mut tail = Vec::new();
        self.stream
            .read_to_end(&mut tail)
            .expect("actual peer EOF required, timeout is failure");
        assert!(tail.is_empty(), "unexpected tail/replay: {tail:02x?}");
    }
    pub fn skips(&mut self) {
        self.send(&[6, 0x10, 9, 0x99, 0, 10, 0xde, 0xad, 0xbe, 0xef]);
        let mut large = vec![6, 0x10, 4, 0x20, 5, 0xdc];
        large.resize(1500, 0xee);
        self.send(&large);
    }
}
