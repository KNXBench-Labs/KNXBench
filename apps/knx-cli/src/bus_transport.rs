//! Select exactly one validated bus endpoint and preserve shared CLI monitor/write behavior.

use knx_core::IndividualAddress;
use knx_net::{
    ApplicationService, BusConnection, BusError, Destination, LDataFrame, LDataMessageKind, Tpci,
    TpuartClient, TpuartMode, TunnelClient, TunnelEvent, TunnelTransport,
};
use std::net::SocketAddrV4;
use std::{io::Write, process::ExitCode};

/// Keep IP's best-effort disconnect policy; serial cleanup is part of completion.
pub(crate) fn monitor_cleanup_outcome(
    serial: bool,
    cleanup: Result<(), BusError>,
    errors: &mut impl Write,
) -> ExitCode {
    if let Err(error) = cleanup {
        let _ = writeln!(errors, "disconnect: {error}");
        if serial {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Report delivery independently of cleanup, without inviting a duplicate write.
/// Over TCP a successful send is the socket accepting the frame, so it says so.
pub(crate) fn write_outcome(
    serial: bool,
    tcp: bool,
    sent: Result<(), BusError>,
    cleanup: Result<(), BusError>,
    address: &str,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitCode {
    let cleanup_status = monitor_cleanup_outcome(serial, cleanup, errors);
    match sent {
        Ok(()) => {
            if tcp {
                let _ = writeln!(
                    output,
                    "sent to {address} over TCP; the TCP socket accepted it, device delivery is not confirmed"
                );
            } else {
                let _ = writeln!(output, "wrote to {address}");
            }
            if cleanup_status == ExitCode::FAILURE {
                let _ = writeln!(errors, "delivery confirmed, but serial cleanup failed; auto-ACK state unknown; do not retry automatically");
            }
            cleanup_status
        }
        Err(error) => {
            let _ = writeln!(errors, "write failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Default)]
pub(crate) struct EndpointArgs {
    gateway: Option<String>,
    serial: Option<String>,
    source: Option<String>,
    tunnel_transport: Option<TunnelTransport>,
}

impl EndpointArgs {
    pub(crate) fn take(&mut self, args: &[String], i: usize) -> Result<(), String> {
        let flag = args[i].as_str();
        if flag == "--tunnel-transport" {
            return self.take_tunnel_transport(args, i);
        }
        let slot = match flag {
            "--gateway" => &mut self.gateway,
            "--serial" => &mut self.serial,
            "--source" => &mut self.source,
            _ => return Err(format!("unrecognized endpoint argument: {flag}")),
        };
        if slot.is_some() {
            return Err(format!("duplicate {flag}"));
        }
        let value = crate::take_value(args, i + 1, flag)?;
        if value.is_empty() || value.starts_with("--") {
            return Err(format!("{flag} requires a value"));
        }
        *slot = Some(value);
        Ok(())
    }

    fn take_tunnel_transport(&mut self, args: &[String], i: usize) -> Result<(), String> {
        let flag = "--tunnel-transport";
        if self.tunnel_transport.is_some() {
            return Err(format!("duplicate {flag}"));
        }
        self.tunnel_transport = Some(match crate::take_value(args, i + 1, flag)?.as_str() {
            "udp" => TunnelTransport::Udp,
            "tcp" => TunnelTransport::Tcp,
            _ => return Err(format!("{flag} must be udp or tcp")),
        });
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<BusEndpoint, String> {
        if self.tunnel_transport.is_some() && self.serial.is_some() {
            return Err("--tunnel-transport is only valid with --gateway, not --serial".into());
        }
        let transport = self.tunnel_transport.unwrap_or(TunnelTransport::Udp);
        match (self.gateway, self.serial, self.source) {
            (Some(_), Some(_), _) => Err("--gateway and --serial are mutually exclusive".into()),
            (Some(_), None, Some(_)) => Err("--source is only valid with --serial".into()),
            (Some(gateway), None, None) => {
                let gateway: SocketAddrV4 = gateway
                    .parse()
                    .map_err(|_| "--gateway must be host:port, e.g. 192.0.2.1:3671")?;
                if gateway.port() == 0
                    || gateway.ip().is_unspecified()
                    || gateway.ip().is_broadcast()
                    || gateway.ip().is_multicast()
                {
                    return Err("--gateway requires a unicast IPv4 address and nonzero port".into());
                }
                Ok(BusEndpoint::Gateway { gateway, transport })
            }
            (None, Some(path), Some(source)) => {
                let source: IndividualAddress = source
                    .parse()
                    .map_err(|_| "--source must be area.line.device, e.g. 1.1.1")?;
                TpuartClient::validate_source(source).map_err(|e| e.to_string())?;
                Ok(BusEndpoint::Serial { path, source })
            }
            (None, Some(_), None) => Err("--serial requires --source <area.line.device>".into()),
            _ => Err("exactly one of --gateway <host:port> or --serial <tty> is required".into()),
        }
    }
}

pub(crate) enum BusEndpoint {
    Gateway {
        gateway: SocketAddrV4,
        transport: TunnelTransport,
    },
    Serial {
        path: String,
        source: IndividualAddress,
    },
}
impl std::fmt::Display for BusEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gateway { gateway, .. } => gateway.fmt(f),
            Self::Serial { path, .. } => write!(f, "serial {path}"),
        }
    }
}
impl BusEndpoint {
    pub(crate) fn validate_write(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Result<(), String> {
        if let Self::Serial { source, .. } = self {
            TpuartClient::validate_frame(&LDataFrame {
                kind: LDataMessageKind::Request,
                source: *source,
                destination,
                transport: Tpci::UnnumberedData,
                service,
                control: None,
            })
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub(crate) async fn connect(&self, mode: TpuartMode) -> Result<BusLink, BusError> {
        match self {
            Self::Gateway { gateway, transport } => {
                let client = match transport {
                    TunnelTransport::Udp => knx_net::KnxNetIpClient::new(),
                    TunnelTransport::Tcp => {
                        knx_net::KnxNetIpClient::new().with_tunnel_transport(*transport)
                    }
                };
                let link = client.connect_tunnel(*gateway).await?;
                // An explicit transport is either what runs or an error.
                let actual = link.transport();
                if actual != *transport {
                    let _ = link.disconnect().await;
                    return Err(BusError::Protocol(format!(
                        "requested a {} tunnel but the client opened {}",
                        transport_label(*transport),
                        transport_label(actual)
                    )));
                }
                Ok(BusLink::Ip(link))
            }
            Self::Serial { path, source } => TpuartClient::open(path, *source, mode)
                .await
                .map(BusLink::Serial),
        }
    }
}
/// The transport as the CLI words it in help and output.
pub(crate) fn transport_label(transport: TunnelTransport) -> &'static str {
    match transport {
        TunnelTransport::Udp => "UDP",
        TunnelTransport::Tcp => "TCP",
    }
}

/// One line for TCP frames the stream decoder skipped, or nothing when there were none (or the
/// link has no such counter). They are frames, not necessarily telegrams: an unknown service or
/// a frame above the local receive limit. Their content is not kept and they are not part of
/// any dropped-telegram count. The count is the latest one read before the disconnect request.
pub(crate) fn report_skipped_frames(skipped: Option<u64>, errors: &mut impl Write) {
    if let Some(count) = skipped.filter(|count| *count > 0) {
        let _ = writeln!(
            errors,
            "warning: {count} TCP frame(s) skipped before disconnecting (unknown service or larger than the local receive limit); their content was not kept"
        );
    }
}

/// Interrupt notifications, registered before the monitor announces itself, so a signal sent
/// afterwards is never missed.
pub(crate) struct Interrupt {
    #[cfg(unix)]
    signal: tokio::signal::unix::Signal,
}
impl Interrupt {
    #[cfg(unix)]
    pub(crate) fn register() -> std::io::Result<Self> {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .map(|signal| Self { signal })
    }
    #[cfg(not(unix))]
    pub(crate) fn register() -> std::io::Result<Self> {
        Ok(Self {})
    }
    #[cfg(unix)]
    pub(crate) async fn recv(&mut self) {
        self.signal.recv().await;
    }
    #[cfg(not(unix))]
    pub(crate) async fn recv(&mut self) {
        let _ = tokio::signal::ctrl_c().await;
    }
}

pub(crate) enum BusLink {
    Ip(TunnelClient),
    Serial(TpuartClient),
}
impl BusLink {
    /// The IP transport the tunnel really runs on; `None` for serial.
    pub(crate) fn ip_transport(&self) -> Option<TunnelTransport> {
        match self {
            Self::Ip(link) => Some(link.transport()),
            Self::Serial(_) => None,
        }
    }
    /// Cumulative skipped TCP frames; `None` unless this is a TCP tunnel.
    pub(crate) fn skipped_frames(&self) -> Option<u64> {
        match self {
            Self::Ip(link) if link.transport() == TunnelTransport::Tcp => {
                Some(link.skipped_frame_count())
            }
            Self::Ip(_) | Self::Serial(_) => None,
        }
    }
    pub(crate) fn assigned_address(&self) -> IndividualAddress {
        match self {
            Self::Ip(link) => link.assigned_address(),
            Self::Serial(link) => link.assigned_address(),
        }
    }
    pub(crate) fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        match self {
            Self::Ip(link) => link.subscribe(),
            Self::Serial(link) => link.subscribe(),
        }
    }
    pub(crate) async fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        match self {
            Self::Ip(link) => link.send(destination, service).await,
            Self::Serial(link) => link.send(destination, service).await,
        }
    }
    pub(crate) async fn disconnect(self) -> Result<(), BusError> {
        match self {
            Self::Ip(link) => link.disconnect().await,
            Self::Serial(link) => link.disconnect().await,
        }
    }
    pub(crate) fn closure_message(&self) -> String {
        match self {
            Self::Ip(link) => match link.transport() {
                TunnelTransport::Udp => "gateway closed the tunnel".into(),
                TunnelTransport::Tcp => match link.closure() {
                    Some(cause) => format!("TCP tunnel closed: {cause}"),
                    None => "TCP tunnel closed".into(),
                },
            },
            Self::Serial(link) => link
                .diagnostics()
                .closure
                .unwrap_or_else(|| "serial transport closed".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cleanup_error() -> BusError {
        BusError::Protocol("reset on close timed out (auto-ACK state unknown)".into())
    }

    #[test]
    fn write_consumer_preserves_delivery_but_fails_serial_cleanup_anomaly() {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        assert_eq!(
            write_outcome(
                true,
                false,
                Ok(()),
                Err(cleanup_error()),
                "1/1/1",
                &mut output,
                &mut errors
            ),
            ExitCode::FAILURE
        );
        assert_eq!(String::from_utf8(output).unwrap(), "wrote to 1/1/1\n");
        let errors = String::from_utf8(errors).unwrap();
        assert!(errors.contains("delivery confirmed, but serial cleanup failed"));
        assert!(errors.contains("auto-ACK state unknown"));
        assert!(errors.contains("do not retry automatically"));
    }

    #[test]
    fn write_consumer_outcome_matrix_preserves_ip_disconnect_policy() {
        for serial in [false, true] {
            for delivered in [false, true] {
                for cleaned in [false, true] {
                    let mut output = Vec::new();
                    let mut errors = Vec::new();
                    let result = write_outcome(
                        serial,
                        false,
                        if delivered {
                            Ok(())
                        } else {
                            Err(BusError::Timeout)
                        },
                        if cleaned {
                            Ok(())
                        } else {
                            Err(cleanup_error())
                        },
                        "1/1/1",
                        &mut output,
                        &mut errors,
                    );
                    assert_eq!(
                        result,
                        if delivered && (!serial || cleaned) {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::FAILURE
                        }
                    );
                    assert_eq!(!output.is_empty(), delivered);
                    assert_eq!(
                        String::from_utf8(errors).unwrap().contains("disconnect:"),
                        !cleaned
                    );
                }
            }
        }
    }

    #[test]
    fn monitor_ctrl_c_consumer_fails_serial_cleanup_but_keeps_ip_policy() {
        for serial in [false, true] {
            let mut errors = Vec::new();
            assert_eq!(
                monitor_cleanup_outcome(serial, Err(cleanup_error()), &mut errors),
                if serial {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            );
            assert!(String::from_utf8(errors)
                .unwrap()
                .contains("reset on close timed out"));
            assert_eq!(
                monitor_cleanup_outcome(serial, Ok(()), &mut Vec::new()),
                ExitCode::SUCCESS
            );
        }
    }
    fn parse(values: &[&str]) -> Result<BusEndpoint, String> {
        let args: Vec<_> = values.iter().map(|s| s.to_string()).collect();
        let mut endpoint = EndpointArgs::default();
        for i in (0..args.len()).step_by(2) {
            endpoint.take(&args, i)?;
        }
        endpoint.finish()
    }
    #[test]
    fn endpoint_selection_matrix() {
        assert!(matches!(
            parse(&["--gateway", "192.0.2.1:3671"]).unwrap(),
            BusEndpoint::Gateway { .. }
        ));
        assert!(matches!(
            parse(&["--serial", "missing-serial-test", "--source", "1.1.1"]).unwrap(),
            BusEndpoint::Serial { .. }
        ));
        for values in [
            vec![],
            vec!["--source", "1.1.1"],
            vec!["--serial", "missing"],
            vec!["--gateway", "192.0.2.1:3671", "--source", "1.1.1"],
            vec!["--gateway", "192.0.2.1:3671", "--serial", "missing"],
            vec!["--gateway", "192.0.2.1:3671", "--gateway", "192.0.2.2:3671"],
            vec![
                "--serial", "missing", "--serial", "other", "--source", "1.1.1",
            ],
            vec![
                "--serial", "missing", "--source", "1.1.1", "--source", "1.1.2",
            ],
            vec!["--gateway", "0.0.0.0:3671"],
            vec!["--gateway", "192.0.2.1:0"],
            vec!["--gateway", "224.0.23.12:3671"],
            vec!["--gateway", "255.255.255.255:3671"],
            vec!["--gateway", "nonsense"],
            vec!["--serial", ""],
            vec!["--serial"],
            vec!["--serial", "missing", "--source", "0.0.0"],
            vec!["--serial", "missing", "--source", "15.15.255"],
            vec!["--serial", "missing", "--source", "16.1.1"],
        ] {
            assert!(parse(&values).is_err(), "{values:?}");
        }
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
        assert!(parse(&["--serial", "missing", "--source", &excluded]).is_err());
    }

    fn gateway_transport(values: &[&str]) -> TunnelTransport {
        match parse(values).unwrap() {
            BusEndpoint::Gateway { transport, .. } => transport,
            BusEndpoint::Serial { .. } => panic!("expected a gateway endpoint"),
        }
    }

    #[test]
    fn tunnel_transport_defaults_to_udp_and_selects_tcp_only_when_asked() {
        let gateway = ["--gateway", "192.0.2.1:3671"];
        assert_eq!(gateway_transport(&gateway), TunnelTransport::Udp);
        for (value, expected) in [("udp", TunnelTransport::Udp), ("tcp", TunnelTransport::Tcp)] {
            let with = [gateway[0], gateway[1], "--tunnel-transport", value];
            assert_eq!(gateway_transport(&with), expected);
            // Order must not matter.
            let before = ["--tunnel-transport", value, gateway[0], gateway[1]];
            assert_eq!(gateway_transport(&before), expected);
        }
        // The endpoint text is the address only, so existing output keeps its shape.
        let tcp = parse(&["--gateway", "192.0.2.1:3671", "--tunnel-transport", "tcp"]).unwrap();
        assert_eq!(tcp.to_string(), "192.0.2.1:3671");
    }

    #[test]
    fn tunnel_transport_rejects_duplicates_invalid_values_and_serial() {
        let gateway = ["--gateway", "192.0.2.1:3671"];
        let serial = ["--serial", "missing", "--source", "1.1.1"];
        for (values, expected) in [
            (
                vec!["--tunnel-transport", "udp", "--tunnel-transport", "udp"],
                "duplicate",
            ),
            (
                vec!["--tunnel-transport", "tcp", "--tunnel-transport", "udp"],
                "duplicate",
            ),
            (vec!["--tunnel-transport", "TCP"], "must be udp or tcp"),
            (vec!["--tunnel-transport", "Udp"], "must be udp or tcp"),
            (vec!["--tunnel-transport", "tcp "], "must be udp or tcp"),
            (vec!["--tunnel-transport", "tls"], "must be udp or tcp"),
            (vec!["--tunnel-transport", ""], "must be udp or tcp"),
            (vec!["--tunnel-transport", "--gateway"], "needs a value"),
            (vec!["--tunnel-transport"], "needs a value"),
        ] {
            let mut all = gateway.to_vec();
            all.extend(&values);
            // A missing value shifts the pairs, so parse manually below.
            if values.len() == 1 {
                let args: Vec<String> = all.iter().map(|s| s.to_string()).collect();
                let mut endpoint = EndpointArgs::default();
                endpoint.take(&args, 0).unwrap();
                let error = endpoint.take(&args, 2).unwrap_err();
                assert!(error.contains(expected), "{values:?}: {error}");
                continue;
            }
            let error = parse(&all).err().unwrap_or_default();
            assert!(error.contains(expected), "{values:?}: {error}");
        }
        for transport in ["udp", "tcp"] {
            let mut all = serial.to_vec();
            all.extend(["--tunnel-transport", transport]);
            let error = parse(&all)
                .err()
                .expect("serial must refuse a tunnel transport");
            assert!(
                error.contains("only valid with --gateway"),
                "{transport}: {error}"
            );
        }
        // A transport alone selects no endpoint.
        assert!(parse(&["--tunnel-transport", "tcp"]).is_err());
        // A transport next to both endpoints is refused too.
        assert!(parse(&[
            "--gateway",
            "192.0.2.1:3671",
            "--serial",
            "missing",
            "--tunnel-transport",
            "tcp"
        ])
        .is_err());
    }

    #[test]
    fn tcp_write_says_the_socket_accepted_it_and_never_claims_device_delivery() {
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        assert_eq!(
            write_outcome(
                false,
                true,
                Ok(()),
                Ok(()),
                "1/1/1",
                &mut output,
                &mut errors
            ),
            ExitCode::SUCCESS
        );
        let line = String::from_utf8(output).unwrap();
        assert_eq!(
            line,
            "sent to 1/1/1 over TCP; the TCP socket accepted it, device delivery is not confirmed\n"
        );
        assert!(!line.contains("wrote to"));
        assert!(errors.is_empty());
        // The failure path keeps the shared wording and sends no success line.
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        assert_eq!(
            write_outcome(
                false,
                true,
                Err(BusError::Timeout),
                Ok(()),
                "1/1/1",
                &mut output,
                &mut errors
            ),
            ExitCode::FAILURE
        );
        assert!(output.is_empty());
        assert!(String::from_utf8(errors).unwrap().contains("write failed"));
    }

    #[test]
    fn skipped_frames_get_one_precise_line_only_when_nonzero() {
        for skipped in [None, Some(0)] {
            let mut errors = Vec::new();
            report_skipped_frames(skipped, &mut errors);
            assert!(errors.is_empty(), "{skipped:?}");
        }
        let mut errors = Vec::new();
        report_skipped_frames(Some(3), &mut errors);
        let line = String::from_utf8(errors).unwrap();
        assert_eq!(line.lines().count(), 1);
        assert!(line.contains("3 TCP frame(s) skipped before disconnecting"));
        assert!(line.contains("unknown service or larger than the local receive limit"));
        assert!(line.contains("content was not kept"));
        // Frames are not called lost telegrams, and the drop wording is not reused.
        assert!(!line.contains("dropped") && !line.contains("lost"));
    }
}
