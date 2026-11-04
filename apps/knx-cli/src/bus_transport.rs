//! Select exactly one validated bus endpoint and preserve shared CLI monitor/write behavior.

use knx_core::IndividualAddress;
use knx_net::{
    ApplicationService, BusConnection, BusError, Destination, LDataFrame, LDataMessageKind, Tpci,
    TpuartClient, TpuartMode, TunnelClient, TunnelEvent,
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
pub(crate) fn write_outcome(
    serial: bool,
    sent: Result<(), BusError>,
    cleanup: Result<(), BusError>,
    address: &str,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitCode {
    let cleanup_status = monitor_cleanup_outcome(serial, cleanup, errors);
    match sent {
        Ok(()) => {
            let _ = writeln!(output, "wrote to {address}");
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
}

impl EndpointArgs {
    pub(crate) fn take(&mut self, args: &[String], i: usize) -> Result<(), String> {
        let flag = args[i].as_str();
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

    pub(crate) fn finish(self) -> Result<BusEndpoint, String> {
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
                Ok(BusEndpoint::Gateway(gateway))
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
    Gateway(SocketAddrV4),
    Serial {
        path: String,
        source: IndividualAddress,
    },
}
impl std::fmt::Display for BusEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gateway(gateway) => gateway.fmt(f),
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
            Self::Gateway(gateway) => knx_net::KnxNetIpClient::new()
                .connect_tunnel(*gateway)
                .await
                .map(BusLink::Ip),
            Self::Serial { path, source } => TpuartClient::open(path, *source, mode)
                .await
                .map(BusLink::Serial),
        }
    }
}
pub(crate) enum BusLink {
    Ip(TunnelClient),
    Serial(TpuartClient),
}
impl BusLink {
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
            Self::Ip(_) => "gateway closed the tunnel".into(),
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
            BusEndpoint::Gateway(_)
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
}
