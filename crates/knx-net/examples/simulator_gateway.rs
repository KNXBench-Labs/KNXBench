//! Serves one simulated device as a KNXnet/IP tunnelling gateway and records every frame.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use knx_core::commissioning::properties::{
    PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
use knx_core::IndividualAddress;
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::simulator_gateway::{serve, GatewayConfig, GatewayEvent};
use socket2::{Domain, Socket, Type};
use tokio::net::UdpSocket;

const USAGE: &str = "usage: simulator_gateway --advertise IP[:PORT] --device A.L.D [--mask HEX] \
[--serial 12HEX] [--manufacturer HEX] [--hardware-type 12HEX] [--program-version 10HEX] \
[--max-apdu N] [--gateway-property OBJTYPE:INSTANCE:PID=HEX]... [--capture FILE] [--datagrams]";
const MULTICAST: Ipv4Addr = Ipv4Addr::new(224, 0, 23, 12);

fn hex(text: &str, octets: usize) -> Result<Vec<u8>, String> {
    if text.len() != octets * 2 || !text.is_ascii() {
        return Err(format!("{text:?} is not {octets} octets of hex"));
    }
    (0..octets)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// Object type, instance and property id, as `GatewayConfig::properties` keys them.
type PropertyKey = (u16, u8, u8);

/// One `--gateway-property OBJTYPE:INSTANCE:PID=HEX`; odd-length or non-ASCII hex is refused.
fn gateway_property(spec: &str) -> Result<(PropertyKey, Vec<u8>), String> {
    let (key, data) = spec.split_once('=').ok_or(USAGE)?;
    let parts: Vec<&str> = key.split(':').collect();
    let [object, instance, pid] = parts[..] else {
        return Err(USAGE.into());
    };
    if data.len() % 2 != 0 {
        return Err(format!(
            "--gateway-property: {data:?} has an odd number of hex digits"
        ));
    }
    let key = (
        u16::from_str_radix(object, 16).map_err(|e| e.to_string())?,
        instance.parse().map_err(|_| USAGE)?,
        pid.parse().map_err(|_| USAGE)?,
    );
    Ok((key, hex(data, data.len() / 2)?))
}

fn dump(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut advertise, mut device, mut capture) = (None, None, None);
    let (mut mask, mut serial, mut manufacturer) = (0x07B0u16, None, None);
    let (mut hardware, mut program, mut datagrams) = (None, None, false);
    let mut gateway_properties = Vec::new();
    let mut max_apdu = SimulatorConfig::default().max_apdu_length;
    while let Some(flag) = args.next() {
        if flag == "--datagrams" {
            datagrams = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))?;
        match flag.as_str() {
            "--advertise" => advertise = Some(value),
            "--device" => device = Some(value),
            "--capture" => capture = Some(value),
            "--mask" => mask = u16::from_str_radix(&value, 16).map_err(|e| e.to_string())?,
            "--serial" => serial = Some(hex(&value, 6)?),
            "--manufacturer" => {
                manufacturer = Some(u16::from_str_radix(&value, 16).map_err(|e| e.to_string())?)
            }
            "--hardware-type" => hardware = Some(hex(&value, 6)?),
            "--gateway-property" => gateway_properties.push(value),
            "--program-version" => program = Some(hex(&value, 5)?),
            "--max-apdu" => max_apdu = Some(value.parse().map_err(|_| USAGE)?),
            _ => return Err(USAGE.into()),
        }
    }
    let advertise = advertise.ok_or(USAGE)?;
    let advertised: SocketAddrV4 = if advertise.contains(':') {
        advertise.parse().map_err(|e| format!("--advertise: {e}"))?
    } else {
        SocketAddrV4::new(
            advertise.parse().map_err(|e| format!("--advertise: {e}"))?,
            3671,
        )
    };
    let address: IndividualAddress = device.ok_or(USAGE)?.parse().map_err(|e| format!("{e:?}"))?;
    let gateway_properties = gateway_properties
        .iter()
        .map(|spec| gateway_property(spec))
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let simulated = SimulatedDevice::with_config_at(
        address,
        SimulatorConfig {
            mask_version: mask,
            serial_number: serial.map(|s| s.try_into().expect("six octets")),
            max_apdu_length: max_apdu,
            ..Default::default()
        },
    );
    if let Some(manufacturer) = manufacturer {
        simulated.preset_property(0, PID_MANUFACTURER_ID, &manufacturer.to_be_bytes());
    }
    if let Some(hardware) = hardware {
        simulated.preset_property(0, PID_HARDWARE_TYPE, &hardware);
    }
    if let Some(program) = program {
        simulated.preset_property(3, PID_PROGRAM_VERSION, &program);
    }
    let device = Arc::new(simulated);

    let log: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(match capture {
        Some(path) => Box::new(File::create(path).map_err(|e| e.to_string())?),
        None => Box::new(std::io::stdout()),
    }));
    let started = Instant::now();
    let sink = log.clone();
    let observer = Arc::new(move |event: GatewayEvent<'_>| {
        let line = match event {
            GatewayEvent::Datagram { from_tool, bytes } if datagrams => {
                format!("{} datagram {}", direction(from_tool), dump(bytes))
            }
            GatewayEvent::Datagram { .. } => return,
            GatewayEvent::Frame {
                from_tool,
                cemi,
                frame,
            } => {
                format!("{} cemi {} | {frame:?}", direction(from_tool), dump(cemi))
            }
            GatewayEvent::Management {
                from_tool,
                cemi,
                summary,
            } => format!("{} mgmt {} | {summary}", direction(from_tool), dump(cemi)),
            GatewayEvent::Undecodable { cemi, reason } => {
                format!("TOOL-> UNDECODABLE cemi {} | {reason}", dump(cemi))
            }
            GatewayEvent::Note(text) => format!("note {text}"),
        };
        let mut out = sink.lock().unwrap_or_else(|p| p.into_inner());
        let _ = writeln!(out, "{:>9}ms {line}", started.elapsed().as_millis());
        let _ = out.flush();
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async move {
        // Bound to the wildcard so the multicast search reaches it as well as unicast.
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).map_err(|e| e.to_string())?;
        socket.set_reuse_address(true).map_err(|e| e.to_string())?;
        socket
            .bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, advertised.port()).into())
            .map_err(|e| e.to_string())?;
        socket
            .join_multicast_v4(&MULTICAST, advertised.ip())
            .map_err(|e| e.to_string())?;
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        let socket = UdpSocket::from_std(socket.into()).map_err(|e| e.to_string())?;
        let mut config = GatewayConfig::new(
            advertised,
            IndividualAddress::new(0, 0, 250).expect("valid"),
        );
        config.properties = gateway_properties;
        config.observer = Some(observer);
        eprintln!("serving {address} on {advertised}; mask {mask:04x}; max APDU {max_apdu:?}");
        serve(device, socket, config)
            .await
            .map_err(|e| e.to_string())
    })
}

fn direction(from_tool: bool) -> &'static str {
    if from_tool {
        "TOOL->"
    } else {
        "->TOOL"
    }
}

#[cfg(test)]
mod tests {
    use super::gateway_property;

    #[test]
    fn a_gateway_property_parses_into_key_and_octets() {
        assert_eq!(
            gateway_property("B:0:52=12ab"),
            Ok(((0x000B, 0, 52), vec![0x12, 0xAB]))
        );
        assert_eq!(gateway_property("B:0:52="), Ok(((0x000B, 0, 52), vec![])));
    }

    #[test]
    fn odd_or_non_ascii_hex_is_refused_not_cut_short() {
        assert!(gateway_property("B:0:52=ABC").is_err());
        // Four octets, so the length check passes; the old slicing split `é` and panicked.
        assert!(gateway_property("B:0:52=aéb").is_err());
        assert!(gateway_property("B:0:52=zz").is_err());
        assert!(gateway_property("B:0=12").is_err());
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
