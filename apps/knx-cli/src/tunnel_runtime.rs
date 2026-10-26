//! The async runtime for CLI paths that keep a KNXnet/IP tunnel open across durable writes.

use std::io;

/// A small fixed pool: spawned tasks do only short protocol work, and the
/// operation future runs on the calling thread, not on a worker.
const TUNNEL_WORKERS: usize = 2;

/// Builds the runtime for a CLI operation that holds a KNXnet/IP tunnel open
/// while it performs synchronous durable persistence (fsync, readback,
/// validation) between frames.
///
/// `KnxNetIpClient::connect_tunnel` spawns the tunnel's receive and heartbeat
/// tasks. They answer the gateway's protocol obligations, above all the
/// TUNNELLING_ACK for every TUNNELLING_REQUEST, which the gateway repeats once
/// after one second and then answers by tearing the connection down
/// (Tunnelling v01.07.01 AS §2.6.1). On a current-thread runtime those tasks
/// only run while the operation future yields, so a multi-second synchronous
/// checkpoint inside it starves them and the gateway drops the tunnel. Worker
/// threads keep them serviced; the operation future itself, and therefore
/// every persistence, barrier and confirmation step in it, still runs in
/// order on the calling thread.
pub fn tunnel_runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(TUNNEL_WORKERS)
        .enable_all()
        .build()
}

#[cfg(test)]
mod tests {
    use super::tunnel_runtime;
    use knx_net::{
        core::{hpai::Hpai, services},
        frame, tunnelling, BusConnection, KnxNetIpClient,
    };
    use std::{
        net::{Ipv4Addr, SocketAddr, UdpSocket},
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    const CHANNEL: u8 = 0x2a;
    const SEQUENCE: u8 = 0;
    /// TUNNELLING_REQUEST_TIMEOUT, Tunnelling v01.07.01 AS §2.6.1.
    const ACK_WINDOW: Duration = Duration::from_secs(1);
    /// Longer than the ACK window, like the observed multi-second checkpoint.
    const SYNCHRONOUS_PERSISTENCE: Duration = Duration::from_millis(1500);
    /// An L_Data.ind group write (1.1.1 -> 0/0/1, value 1). Its content is
    /// irrelevant to the ACK obligation; the test checks that it decodes.
    const L_DATA_IND: [u8; 11] = [
        0x29, 0x00, 0xbc, 0xe0, 0x11, 0x01, 0x00, 0x01, 0x01, 0x00, 0x81,
    ];

    /// A fake gateway: accept one tunnel, wait until the client's operation
    /// future is blocked, send one TUNNELLING_REQUEST and return how long the
    /// matching TUNNELLING_ACK took, or `None` if it missed the window.
    fn fake_gateway(server: UdpSocket, blocked: mpsc::Receiver<()>) -> Option<Duration> {
        let mut buf = [0u8; 256];
        let (n, client) = server.recv_from(&mut buf).expect("CONNECT_REQUEST");
        let (header, _) = frame::decode_frame(&buf[..n]).expect("a KNXnet/IP frame");
        assert_eq!(header.service_type, services::CONNECT_REQUEST);
        let SocketAddr::V4(local) = server.local_addr().unwrap() else {
            unreachable!("bound to IPv4")
        };
        let mut body = vec![CHANNEL, services::E_NO_ERROR];
        body.extend_from_slice(
            &Hpai {
                addr: *local.ip(),
                port: local.port(),
            }
            .encode(),
        );
        // CRD: length 04h, TUNNEL_CONNECTION, assigned IA 1.1.255
        // (Tunnelling v01.07.01 AS §5.4.4, printed page 30).
        body.extend_from_slice(&[0x04, tunnelling::TUNNEL_CONNECTION, 0x11, 0xff]);
        server
            .send_to(
                &frame::encode_frame(services::CONNECT_RESPONSE, &body),
                client,
            )
            .unwrap();

        blocked
            .recv()
            .expect("the operation future started blocking");
        let request = tunnelling::encode_tunnelling_request(CHANNEL, SEQUENCE, &L_DATA_IND);
        server
            .send_to(
                &frame::encode_frame(tunnelling::TUNNELLING_REQUEST, &request),
                client,
            )
            .unwrap();
        let sent = Instant::now();
        loop {
            let left = ACK_WINDOW.checked_sub(sent.elapsed())?;
            server
                .set_read_timeout(Some(left.max(Duration::from_millis(1))))
                .unwrap();
            let Ok(n) = server.recv(&mut buf) else {
                return None;
            };
            let Ok((header, body)) = frame::decode_frame(&buf[..n]) else {
                continue;
            };
            if header.service_type != tunnelling::TUNNELLING_ACK {
                continue;
            }
            let ack = tunnelling::decode_tunnelling_ack(body).expect("a TUNNELLING_ACK body");
            if ack.channel_id == CHANNEL && ack.sequence_counter == SEQUENCE {
                assert_eq!(ack.status, tunnelling::E_NO_ERROR);
                let took = sent.elapsed();
                println!("TUNNELLING_ACK after {took:?}");
                return (took <= ACK_WINDOW).then_some(took);
            }
        }
    }

    #[test]
    fn tunnel_acks_while_the_operation_future_blocks_in_synchronous_persistence() {
        assert!(knx_net::cemi::decode_l_data(&L_DATA_IND).is_ok());
        let server = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let SocketAddr::V4(gateway) = server.local_addr().unwrap() else {
            unreachable!("bound to IPv4")
        };
        let (blocking, blocked) = mpsc::channel();
        let fake = thread::spawn(move || fake_gateway(server, blocked));

        let runtime = tunnel_runtime().expect("a runtime");
        runtime.block_on(async {
            let tunnel = KnxNetIpClient::new()
                .connect_tunnel(gateway)
                .await
                .expect("the fake gateway accepts the tunnel");
            blocking.send(()).unwrap();
            // Stands in for a durable checkpoint: synchronous, no await.
            thread::sleep(SYNCHRONOUS_PERSISTENCE);
            drop(tunnel);
        });

        let acked = fake.join().expect("the fake gateway finished");
        assert!(
            acked.is_some(),
            "no TUNNELLING_ACK within {ACK_WINDOW:?} while the operation future blocked"
        );
    }
}
