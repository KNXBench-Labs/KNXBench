//! Binds loopback TCP endpoints together with a same-port UDP trap, retrying on port collisions.

use std::net::{Ipv4Addr, SocketAddr, TcpListener, UdpSocket};

/// Bounded number of fresh candidates before the helper gives up.
const MAX_ATTEMPTS: usize = 64;

/// Binds a TCP endpoint from `bind_tcp` and a UDP trap on the same loopback port number.
///
/// The UDP port number of an ephemeral TCP port may already belong to another process or test,
/// so a taken UDP port drops that TCP candidate and tries a fresh one, at most [`MAX_ATTEMPTS`]
/// times before panicking.
pub fn with_udp_trap<T>(
    mut bind_tcp: impl FnMut() -> T,
    addr_of: impl Fn(&T) -> SocketAddr,
) -> (T, UdpSocket) {
    for _ in 0..MAX_ATTEMPTS {
        let tcp = bind_tcp();
        match UdpSocket::bind(addr_of(&tcp)) {
            Ok(udp) => return (tcp, udp),
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {}
            Err(error) => panic!("UDP trap bind failed: {error}"),
        }
    }
    panic!("no loopback port with both TCP and UDP free after {MAX_ATTEMPTS} attempts");
}

/// A loopback TCP listener on an ephemeral port plus a UDP trap on the same port number.
pub fn tcp_listener_with_udp_trap() -> (TcpListener, UdpSocket) {
    with_udp_trap(
        || TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap(),
        |listener| listener.local_addr().unwrap(),
    )
}

/// A loopback UDP socket on an ephemeral port plus a TCP trap on the same port number.
///
/// Mirror of [`tcp_listener_with_udp_trap`] for UDP-first gateways: a taken TCP port number
/// drops that UDP candidate and tries a fresh one instead of panicking with `AddrInUse`.
pub fn udp_socket_with_tcp_trap() -> (UdpSocket, TcpListener) {
    for _ in 0..MAX_ATTEMPTS {
        let udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        match TcpListener::bind(udp.local_addr().unwrap()) {
            Ok(tcp) => return (udp, tcp),
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {}
            Err(error) => panic!("TCP trap bind failed: {error}"),
        }
    }
    panic!("no loopback port with both UDP and TCP free after {MAX_ATTEMPTS} attempts");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free_udp_addr() -> SocketAddr {
        UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
    }

    #[test]
    fn taken_udp_port_is_retried_with_a_fresh_candidate() {
        let occupied = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let taken = occupied.local_addr().unwrap();
        let mut calls = 0;
        let (addr, trap) = with_udp_trap(
            || {
                calls += 1;
                if calls == 1 {
                    taken
                } else {
                    free_udp_addr()
                }
            },
            |addr| *addr,
        );
        assert!(calls >= 2, "first candidate must collide and be retried");
        assert_ne!(addr, taken);
        assert_eq!(trap.local_addr().unwrap(), addr);
    }

    #[test]
    #[should_panic(expected = "no loopback port with both TCP and UDP free")]
    fn persistent_collision_panics_with_a_clear_message() {
        let occupied = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let taken = occupied.local_addr().unwrap();
        let _ = with_udp_trap(|| taken, |addr| *addr);
    }

    #[test]
    fn udp_first_pair_shares_one_port_and_survives_held_tcp_ports() {
        // Many held TCP ports make a same-numbered TCP bind after a bare UDP bind collide often.
        let held: Vec<TcpListener> = (0..2000)
            .map_while(|_| TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).ok())
            .collect();
        for _ in 0..50 {
            let (udp, tcp) = udp_socket_with_tcp_trap();
            assert_eq!(udp.local_addr().unwrap(), tcp.local_addr().unwrap());
        }
        drop(held);
    }

    #[test]
    fn listener_and_trap_share_one_port() {
        let (listener, trap) = tcp_listener_with_udp_trap();
        assert_eq!(listener.local_addr().unwrap(), trap.local_addr().unwrap());
    }
}
