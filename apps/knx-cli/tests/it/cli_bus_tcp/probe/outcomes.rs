//! Verify completed classifications, refusal exits, sampled skips and actual UDP/TCP cleanup.

use super::{
    collect, failed,
    fixture::Fixture,
    peer::{gateway, no_udp},
    result, stderr,
};
use knx_net::{ApplicationService as A, Tpci};

#[test]
fn tcp_reports_low_high_contradictory_and_neither_without_turning_completed_reads_into_errors() {
    for (bytes, control, counts, controls) in [
        (
            [0x2c, 1, 2, 1],
            7,
            "2 low octet first, 0 high octet first, 0 neither",
            "1 agree, 0 disagree",
        ),
        (
            [1, 0x2c, 1, 2],
            7,
            "0 low octet first, 2 high octet first, 0 neither",
            "1 agree, 0 disagree",
        ),
        (
            [0x2c, 1, 1, 2],
            7,
            "1 low octet first, 1 high octet first, 0 neither",
            "1 agree, 0 disagree",
        ),
        (
            [9, 9, 8, 8],
            6,
            "0 low octet first, 0 high octet first, 2 neither",
            "0 agree, 1 disagree",
        ),
    ] {
        let fixture = Fixture::new();
        let (addr, worker, trap) = gateway(250, move |peer| {
            peer.identity();
            peer.memory_request();
            peer.memory_answer(bytes.to_vec());
            peer.control(control);
            peer.management_end();
            peer.finish();
        });
        let output = collect(&mut fixture.command(&addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        result(&output, counts, controls, "TCP");
        assert!(!stderr(&output).contains("skipped"));
        no_udp(&trap);
        fixture.intact();
    }
}

#[test]
fn ordinary_tcp_success_without_recording_discloses_two_sampled_skips() {
    let fixture = Fixture::new();
    let (addr, worker, trap) = gateway(250, |peer| {
        peer.identity();
        peer.memory_request();
        peer.skips();
        peer.memory_answer(vec![0x2c, 1, 2, 1]);
        peer.control(7);
        peer.management_end();
        peer.finish();
    });
    let output = collect(&mut fixture.command(&addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    result(
        &output,
        "2 low octet first, 0 high octet first, 0 neither",
        "1 agree, 0 disagree",
        "TCP",
    );
    assert_eq!(stderr(&output).matches("warning: 2 TCP frame(s) skipped before disconnecting (unknown service or larger than the local receive limit); their content was not kept").count(), 1);
    no_udp(&trap);
    fixture.intact();
    assert_eq!(
        std::fs::read_dir(fixture.dir.path()).unwrap().count(),
        1,
        "no recording without consent"
    );
}

#[test]
fn ordinary_tcp_eof_without_recording_still_discloses_two_sampled_skips_and_no_tally() {
    let fixture = Fixture::new();
    let (addr, worker, trap) = gateway(250, |peer| {
        peer.identity();
        peer.memory_request();
        peer.skips();
        peer.close_write();
    });
    let output = collect(&mut fixture.command(&addr.to_string(), Some("tcp")));
    worker.join().unwrap();
    failed(&output, "not probed:");
    assert_eq!(stderr(&output).matches("warning: 2 TCP frame(s) skipped before disconnecting (unknown service or larger than the local receive limit); their content was not kept").count(), 1);
    no_udp(&trap);
    fixture.intact();
}

#[test]
fn identity_refusals_stop_before_memory_reads_and_settle_both_protocol_ends() {
    for case in 0..4 {
        let fixture = Fixture::new();
        let (addr, worker, trap) = gateway(250, move |peer| {
            peer.begin(false);
            peer.descriptor_request();
            peer.descriptor_answer(if case == 0 {
                vec![7, 5]
            } else if case == 1 {
                vec![7]
            } else {
                vec![7, 1]
            });
            if case >= 2 {
                peer.property_request(1, 12);
                peer.property_answer(
                    1,
                    12,
                    u8::from(case == 2),
                    if case == 2 { vec![0, 3] } else { vec![] },
                );
            }
            peer.management_end();
            peer.finish();
        });
        let output = collect(&mut fixture.command(&addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        failed(&output, "not probed:");
        assert!(!stderr(&output).contains("skipped"));
        no_udp(&trap);
        fixture.intact();
    }
}

#[test]
fn read_refusal_short_mismatched_malformed_and_eof_never_emit_a_completed_tally_or_replay() {
    for case in 0..5 {
        let fixture = Fixture::new();
        let (addr, worker, trap) = gateway(250, move |peer| {
            peer.identity();
            peer.memory_request();
            match case {
                0 | 1 => {
                    peer.memory_answer(if case == 0 { vec![] } else { vec![0x2c, 1] });
                    peer.management_end();
                    peer.finish();
                }
                2 => {
                    // Wrong memory address is ACKed at TL, never accepted by the application.
                    peer.answer(
                        3,
                        A::MemoryResponse {
                            address: 0x4410,
                            data: vec![0x2c, 1, 2, 1],
                        },
                    );
                    peer.indicate(Tpci::Disconnect, A::NoApplicationPdu);
                    peer.management_end();
                    peer.finish();
                }
                3 => {
                    peer.send(&[5, 0x10, 4, 0x20, 0, 6]);
                    peer.eof();
                }
                4 => peer.close_write(),
                _ => unreachable!(),
            }
        });
        let output = collect(&mut fixture.command(&addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        failed(&output, "not probed:");
        no_udp(&trap);
        fixture.intact();
    }
}

#[test]
fn assigned_self_address_and_negative_management_confirmation_are_not_preconnect_refusals() {
    for self_address in [true, false] {
        let fixture = Fixture::new();
        let (addr, worker, trap) = gateway(if self_address { 67 } else { 250 }, move |peer| {
            if !self_address {
                peer.begin(true);
            }
            peer.finish();
        });
        let output = collect(&mut fixture.command(&addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        failed(&output, "not probed:");
        assert!(stderr(&output).contains("over TCP"));
        no_udp(&trap);
        fixture.intact();
    }
}

#[test]
fn omitted_and_explicit_udp_complete_the_same_saved_real_plan_without_contacting_tcp() {
    use knx_net::commissioning::{
        simulator::{SimulatedDevice, SimulatorConfig},
        simulator_gateway::{serve, GatewayConfig, GatewayEvent},
    };
    use std::{
        net::{TcpListener, UdpSocket},
        sync::{Arc, Mutex},
        thread,
    };
    for selector in [None, Some("udp")] {
        let fixture = Fixture::new();
        let device = Arc::new(SimulatedDevice::with_config_at(
            "1.1.67".parse().unwrap(),
            SimulatorConfig {
                mask_version: 0x0701,
                max_apdu_length: Some(15),
                ..Default::default()
            },
        ));
        device.preset_memory(0x4400, &[0x2c, 1, 2, 1]);
        device.preset_memory(0x4408, &[7]);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();
        socket.set_nonblocking(true).unwrap();
        let tcp = TcpListener::bind(addr).unwrap();
        tcp.set_nonblocking(true).unwrap();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let frames = observed.clone();
        let mut config = GatewayConfig::new(
            match addr {
                std::net::SocketAddr::V4(addr) => addr,
                _ => unreachable!(),
            },
            "1.1.250".parse().unwrap(),
        );
        config.observer = Some(Arc::new(move |event| {
            if let GatewayEvent::Frame {
                from_tool: true,
                frame,
                ..
            } = event
            {
                frames.lock().unwrap().push(frame.clone());
            }
        }));
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let serving = device.clone();
        let worker = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let socket = tokio::net::UdpSocket::from_std(socket).unwrap();
                tokio::select! {
                    result = serve(serving, socket, config) => panic!("UDP simulator ended early: {result:?}"),
                    _ = stopped => {}
                }
            });
        });
        // Stop and join even if child collection or an assertion panics.
        struct UdpOwner(
            Option<tokio::sync::oneshot::Sender<()>>,
            Option<thread::JoinHandle<()>>,
        );
        impl UdpOwner {
            fn stop(mut self) {
                self.0.take().unwrap().send(()).unwrap();
                self.1.take().unwrap().join().unwrap();
            }
        }
        impl Drop for UdpOwner {
            fn drop(&mut self) {
                if let Some(stop) = self.0.take() {
                    let _ = stop.send(());
                }
                if let Some(worker) = self.1.take() {
                    let _ = worker.join();
                }
            }
        }
        let worker = UdpOwner(Some(stop), Some(worker));
        let output = collect(&mut fixture.command(&addr.to_string(), selector));
        worker.stop();
        result(
            &output,
            "2 low octet first, 0 high octet first, 0 neither",
            "1 agree, 0 disagree",
            "UDP",
        );
        assert!(!stderr(&output).contains("skipped"));
        assert_eq!(
            tcp.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let frames = observed.lock().unwrap();
        let application: Vec<_> = frames
            .iter()
            .filter(|f| matches!(f.transport, Tpci::NumberedData { .. }))
            .map(|f| f.service.clone())
            .collect();
        assert_eq!(
            application,
            [
                A::DeviceDescriptorRead { descriptor_type: 0 },
                A::PropertyValueRead {
                    object_index: 0,
                    property_id: 12,
                    nr_of_elem: 1,
                    start_index: 1
                },
                A::PropertyValueRead {
                    object_index: 0,
                    property_id: 56,
                    nr_of_elem: 1,
                    start_index: 1
                },
                A::MemoryRead {
                    number: 4,
                    address: 0x4400
                },
                A::MemoryRead {
                    number: 1,
                    address: 0x4408
                }
            ]
        );
        assert_eq!(frames.first().unwrap().transport, Tpci::Connect);
        assert_eq!(frames.last().unwrap().transport, Tpci::Disconnect);
        assert!(!device.memory_was_written());
        assert_eq!(
            device.memory(0x4400, 4),
            [Some(0x2c), Some(1), Some(2), Some(1)]
        );
        assert_eq!(fixture.plan.runs(), [(0x4400, 4), (0x4408, 1)]);
        fixture.intact();
    }
}

#[cfg(unix)]
#[test]
fn sigint_releases_the_actual_tcp_socket_without_claiming_graceful_protocol_cleanup() {
    use super::{KillOnDrop, WAIT};
    use std::{
        os::unix::process::ExitStatusExt,
        process::{Command, Stdio},
    };
    let fixture = Fixture::new();
    let (ready, started) = std::sync::mpsc::channel();
    let (addr, worker, trap) = gateway(250, move |peer| {
        peer.identity();
        peer.memory_request();
        ready.send(()).unwrap();
        peer.eof();
    });
    let mut cmd = fixture.command(&addr.to_string(), Some("tcp"));
    let argv = std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut child = KillOnDrop(
        cmd.stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    started.recv_timeout(WAIT).unwrap();
    assert!(Command::new("kill")
        .args(["-INT", &child.0.id().to_string()])
        .status()
        .unwrap()
        .success());
    let status = super::super::wait_for(&mut child.0);
    worker.join().unwrap();
    eprintln!(
        "probe-sigint-receipt {}",
        serde_json::json!({"argv": argv, "signal": status.signal(), "peer_eof": true})
    );
    assert_eq!(status.signal(), Some(2));
    no_udp(&trap);
    fixture.intact();
}
