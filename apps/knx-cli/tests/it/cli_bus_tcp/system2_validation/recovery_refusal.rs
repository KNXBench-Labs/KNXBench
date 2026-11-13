//! Preserve the original recovery admission across pre-contact refusal and a failed TCP read.

use super::*;

#[test]
fn tcp_recovery_phrases_refuse_before_io_then_skip_eof_retains_original_backup() {
    use std::net::{Ipv4Addr, TcpListener, UdpSocket};
    let f = Fixture::new();
    let (failed, _) = f.run(false, false, true);
    exit(&failed, 1);
    let before = tree(&f.operation());
    let backup = f.store().recovery().unwrap().clone();
    let old = f.inputs.device.system2_snapshot();
    let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    tcp.set_nonblocking(true).unwrap();
    let udp = UdpSocket::bind(tcp.local_addr().unwrap()).unwrap();
    udp.set_nonblocking(true).unwrap();
    let command = f.command(&tcp.local_addr().unwrap().to_string(), Some("tcp"), true);
    for phrase in [
        f.bundle.restore_phrase(),
        f.bundle.acceptance_phrase(),
        knx_core::commissioning::mutation::required_confirmation_phrase(
            f.inputs.device.address(),
            knx_core::WriteScope::Download,
        ),
        knx_net::commissioning::hardware_release::required_release_phrase(
            f.inputs.device.address(),
        ),
    ] {
        let mut cmd = f.inputs.bare();
        let mut replaced = 0;
        for arg in command.get_args() {
            if arg == std::ffi::OsStr::new(&phrase) {
                cmd.arg("wrong");
                replaced += 1;
            } else {
                cmd.arg(arg);
            }
        }
        assert_eq!(replaced, 1);
        let out = collect(&mut cmd);
        exit(&out, 1);
        assert!(stderr(&out).contains("exact"));
        assert_eq!(tree(&f.operation()), before);
        assert_eq!(
            f.rows().len(),
            1,
            "no failed pre-contact operation admitted"
        );
    }
    assert_eq!(
        tcp.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    super::super::probe::peer::no_udp(&udp);
    let (addr, worker, trap) = gateway(250, |p| {
        p.target("1.2.52");
        p.begin(false);
        p.descriptor_request();
        p.skips();
        p.close_write();
    });
    let out = collect(&mut f.command(&addr.to_string(), Some("tcp"), true));
    worker.join().unwrap();
    exit(&out, 1);
    connected(&out, &addr.to_string(), true);
    no_retry(&trap);
    assert_eq!(f.inputs.device.system2_snapshot(), old);
    assert_eq!(f.store().recovery(), Some(&backup));
    assert_eq!(f.rows().last().unwrap()["state"], "notSent");
    for (name, bytes) in before.into_iter().filter(|(p, _)| p != "HEAD") {
        assert_eq!(fs::read(f.operation().join(name)).unwrap(), bytes);
    }
}
