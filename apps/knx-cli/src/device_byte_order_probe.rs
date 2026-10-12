//! `knx device byte-order-probe`: reads which octet order a device holds values in.
//!
//! Read only (KL-170, ADR-0119).
//!
//! The project device at the given address is planned exactly like a
//! download (`knx_app::device_download::prepare_byte_order_probe`), then a
//! read-only session reads only the octets of the fields the plan names
//! (`knx_net`'s `read_memory_runs`) and each field is classified
//! (`knx_core::commissioning::byte_order_evidence`). It has no phrase
//! because it cannot write: a read-only session has no authorisation, and no
//! access key is sent.
//!
//! The answer is evidence about one device, valid only if ETS last
//! programmed it with this project's values; the controls say how far that
//! holds. Nothing it prints changes what a download writes.

use std::io::Write;

use knx_core::commissioning::byte_order_evidence::{
    covered_octets, read_field, FieldReading, Tally, Verdict,
};
use knx_core::ContactableAddress;
use knx_net::commissioning::memory_download::read_memory_runs;
use knx_net::{ManagementTransport, SessionTiming};
use knx_productdb::byte_order_probe::{ProbeField, ProbePlan};

/// `knx device byte-order-probe`'s arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct ProbeArgs {
    /// The device address, unparsed.
    pub target: String,
    /// The project.
    pub project: String,
    /// The product database, if not the default one.
    pub product_db: Option<String>,
    /// The gateway.
    pub gateway: std::net::SocketAddrV4,
}

/// Parses `knx device byte-order-probe`'s arguments.
pub fn parse_probe_args(args: &[String]) -> Result<ProbeArgs, String> {
    let mut target = None;
    let mut project = None;
    let mut product_db = None;
    let mut gateway = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--project" => {
                project = Some(crate::take_value(args, i + 1, "--project")?);
                i += 1;
            }
            "--product-db" => {
                product_db = Some(crate::take_value(args, i + 1, "--product-db")?);
                i += 1;
            }
            "--gateway" => {
                gateway = Some(crate::take_value(args, i + 1, "--gateway")?);
                i += 1;
            }
            flag if flag.starts_with("--") => {
                return Err(format!(
                    "unknown flag {flag} (the probe only reads; writing is `knx device download`)"
                ))
            }
            positional => {
                if target.replace(positional.to_string()).is_some() {
                    return Err("give exactly one device address".to_string());
                }
            }
        }
        i += 1;
    }
    let gateway = gateway
        .ok_or("--gateway <host:port> is required: the probe reads the device")?
        .parse()
        .map_err(|_| "--gateway must be host:port, e.g. 192.0.2.1:3671".to_string())?;
    Ok(ProbeArgs {
        target: target.ok_or("missing the device address, e.g. 1.1.67")?,
        project: project.ok_or("--project <path.knxdb> is required")?,
        product_db,
        gateway,
    })
}

/// How a probe ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probed {
    /// The device was read; the tally of its fields.
    Read(Tally),
    /// Nothing to read, or the read failed; nothing was written.
    Failed,
}

/// Reads and classifies the fields of `plan` on `target`, printing a report
/// to `out`.
pub async fn probe<T: ManagementTransport>(
    transport: &T,
    target: ContactableAddress,
    timing: SessionTiming,
    plan: &ProbePlan,
    out: &mut impl Write,
) -> Probed {
    let address = target.address();
    let _ = writeln!(
        out,
        "== byte-order probe of device {address}: read only, nothing is written =="
    );
    let declared = plan.declared_order.as_deref().map_or_else(
        || "nothing (BigEndian by schema default)".to_string(),
        |o| o.to_string(),
    );
    let _ = writeln!(
        out,
        "program: {}, mask {:04X}h, manufacturer {:04X}h; declares ParameterByteOrder {declared}",
        plan.program_id, plan.mask.0, plan.manufacturer
    );
    let runs = plan.runs();
    let octets: usize = runs.iter().map(|(_, length)| length).sum();
    let discriminating = plan.fields.iter().filter(|f| f.discriminates()).count();
    let _ = writeln!(
        out,
        "reading {} fields ({discriminating} that tell the orders apart, {} controls inside one octet): \
         {octets} octets in {} runs",
        plan.fields.len(),
        plan.fields.len() - discriminating,
        runs.len()
    );
    if plan.fields.is_empty() {
        let _ = writeln!(
            out,
            "nothing to read: no field of this device can be probed"
        );
        print_skipped(out, plan);
        return Probed::Failed;
    }
    let held = match read_memory_runs(
        transport,
        target,
        timing,
        plan.mask,
        plan.manufacturer,
        &runs,
    )
    .await
    {
        Ok(held) => held,
        Err(e) => {
            let _ = writeln!(out, "not probed: {e}");
            return Probed::Failed;
        }
    };
    let mut readings = Vec::with_capacity(plan.fields.len());
    for field in &plan.fields {
        let Some(octets) = octets_of(&held, field) else {
            // Every field lies inside a run it was planned with.
            let _ = writeln!(out, "not probed: {} was not read", field.parameter_ref);
            return Probed::Failed;
        };
        let reading = match read_field(field.field, field.value, octets) {
            Ok(reading) => reading,
            Err(e) => {
                let _ = writeln!(out, "not probed: {}: {e}", field.parameter_ref);
                return Probed::Failed;
            }
        };
        let _ = writeln!(
            out,
            "  {:04X}h {:>2} bit {:<22} project {:<10} device {:<11} {}",
            field.address,
            field.field.size_in_bit,
            field.parameter_ref,
            field.value,
            hex(octets),
            describe(reading)
        );
        readings.push(reading);
    }
    let tally = Tally::of(&readings);
    print_tally(out, &tally);
    print_skipped(out, plan);
    let _ = writeln!(out, "written to the device: no (read only)");
    let _ = writeln!(
        out,
        "This describes one device ETS programmed, if it was programmed with this project's \
         values; it is evidence for KL-170, not a download rule, and changes nothing KNXBench \
         writes."
    );
    Probed::Read(tally)
}

/// The octets `field` covers, from the runs read.
fn octets_of<'a>(held: &'a [(u32, Vec<u8>)], field: &ProbeField) -> Option<&'a [u8]> {
    let length = covered_octets(field.field);
    held.iter().find_map(|(start, octets)| {
        let from = field.address.checked_sub(*start)? as usize;
        octets.get(from..from + length)
    })
}

fn describe(reading: FieldReading) -> String {
    match reading {
        FieldReading::Holds(order) => order.to_string(),
        FieldReading::EitherOrder => "either order".to_string(),
        FieldReading::Neither => "NEITHER order: device and project disagree".to_string(),
        FieldReading::SingleOctet { agrees: true } => "control: agrees".to_string(),
        FieldReading::SingleOctet { agrees: false } => "control: DISAGREES".to_string(),
        FieldReading::NotClassified => "not classified".to_string(),
    }
}

fn print_tally(out: &mut impl Write, tally: &Tally) {
    let _ = writeln!(
        out,
        "fields that tell the orders apart: {} low octet first, {} high octet first, {} neither",
        tally.low_first, tally.high_first, tally.neither
    );
    let _ = writeln!(
        out,
        "controls inside one octet: {} agree, {} disagree",
        tally.single_octet_agree, tally.single_octet_disagree
    );
    let _ = writeln!(out, "verdict: {}", tally.verdict());
    let disagreeing = tally.neither + tally.single_octet_disagree;
    if disagreeing > 0 {
        let _ = writeln!(
            out,
            "caution: {disagreeing} fields hold values the project does not; the device may \
             not hold this project's configuration, so weigh the verdict with that"
        );
    }
    if tally.verdict() == Verdict::Contradictory {
        let _ = writeln!(
            out,
            "caution: a contradictory device does not follow one order; report it as it is"
        );
    }
}

fn print_skipped(out: &mut impl Write, plan: &ProbePlan) {
    if plan.skipped.is_empty() {
        return;
    }
    let _ = writeln!(out, "not read, {} parameters:", plan.skipped.len());
    for skip in &plan.skipped {
        let _ = writeln!(out, "  {}: {}", skip.parameter_ref, skip.reason);
    }
}

fn hex(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|octet| format!("{octet:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::commissioning::byte_order_evidence::OctetOrder;
    use knx_core::commissioning::load_state::MaskVersion;
    use knx_core::commissioning::parameter_image::ParameterField;
    use knx_core::commissioning::properties::PID_MANUFACTURER_ID;
    use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
    use knx_productdb::byte_order_probe::{ProbeSkip, SkipReason};
    use std::time::Duration;

    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(5),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn field(name: &str, address: u32, bit_offset: u8, size_in_bit: u32, value: u64) -> ProbeField {
        ProbeField {
            parameter_ref: name.into(),
            segment: "AS-4400".into(),
            address,
            field: ParameterField {
                offset: address - 0x4400,
                bit_offset,
                size_in_bit,
            },
            value,
        }
    }

    fn plan() -> ProbePlan {
        ProbePlan {
            program_id: "M-0002_A-0001-01-0000".into(),
            declared_order: Some("LittleEndian".into()),
            mask: MaskVersion(0x0701),
            manufacturer: 0x0002,
            fields: vec![
                field("P-1_R-1", 0x4400, 0, 16, 300),
                field("P-2_R-1", 0x4402, 0, 32, 0x0102_0304),
                field("P-3_R-1", 0x4410, 2, 3, 5),
            ],
            skipped: vec![ProbeSkip {
                parameter_ref: "P-9_R-1".into(),
                reason: SkipReason::EitherOrder,
            }],
        }
    }

    fn device(memory: &[u8]) -> SimulatedDevice {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..SimulatorConfig::default()
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x02]);
        device.preset_memory(0x4400, memory);
        device
    }

    /// The device memory with the plan's values in `order`; the 3-bit
    /// control `101b` at bit offset 2 of `4410h`.
    fn memory(order: OctetOrder) -> Vec<u8> {
        let mut memory = vec![0u8; 0x11];
        let (a, b): (&[u8], &[u8]) = match order {
            OctetOrder::HighFirst => (&[0x01, 0x2C], &[1, 2, 3, 4]),
            OctetOrder::LowFirst => (&[0x2C, 0x01], &[4, 3, 2, 1]),
        };
        memory[0..2].copy_from_slice(a);
        memory[2..6].copy_from_slice(b);
        memory[0x10] = 0b0010_1000;
        memory
    }

    fn run(device: &SimulatedDevice, plan: &ProbePlan) -> (Probed, String) {
        let target = ContactableAddress::new(device.address()).unwrap();
        let mut out = Vec::new();
        let probed = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(probe(device, target, fast(), plan, &mut out));
        (probed, String::from_utf8(out).unwrap())
    }

    fn no_writes(device: &SimulatedDevice) -> bool {
        !device.seen().iter().any(|seen| {
            matches!(
                seen,
                Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
            )
        })
    }

    #[test]
    fn a_low_first_device_is_reported_as_such_and_nothing_is_written() {
        let device = device(&memory(OctetOrder::LowFirst));
        let (probed, out) = run(&device, &plan());
        let Probed::Read(tally) = probed else {
            panic!("read: {out}")
        };
        assert_eq!(tally.verdict(), Verdict::Consistent(OctetOrder::LowFirst));
        assert_eq!((tally.low_first, tally.single_octet_agree), (2, 1));
        assert!(
            out.contains("declares ParameterByteOrder LittleEndian\n"),
            "{out}"
        );
        assert!(out.contains("4400h 16 bit P-1_R-1"), "{out}");
        assert!(out.contains("device 2C 01"), "{out}");
        assert!(
            out.contains("verdict: every discriminating field holds its value low octet first"),
            "{out}"
        );
        // The parameters left out are named, not dropped.
        assert!(
            out.contains("P-9_R-1: its value reads the same in both orders"),
            "{out}"
        );
        assert!(
            out.contains("written to the device: no (read only)"),
            "{out}"
        );
        assert!(!out.contains("caution"), "{out}");
        assert!(no_writes(&device));
        assert_eq!(device.seen().last(), Some(&Seen::Disconnect));
    }

    #[test]
    fn a_high_first_device_is_reported_as_such() {
        let device = device(&memory(OctetOrder::HighFirst));
        let (probed, out) = run(&device, &plan());
        assert!(
            matches!(probed, Probed::Read(tally) if tally.verdict() == Verdict::Consistent(OctetOrder::HighFirst)),
            "{out}"
        );
        assert!(no_writes(&device));
    }

    #[test]
    fn a_device_with_other_values_is_flagged_not_hidden() {
        let mut other = memory(OctetOrder::LowFirst);
        other[2] = 9; // 32-bit field now matches neither order
        other[0x10] = 0; // control disagrees
        let device = device(&other);
        let (probed, out) = run(&device, &plan());
        let Probed::Read(tally) = probed else {
            panic!("read: {out}")
        };
        assert_eq!((tally.low_first, tally.neither), (1, 1));
        assert_eq!(tally.single_octet_disagree, 1);
        assert!(out.contains("NEITHER order"), "{out}");
        assert!(out.contains("control: DISAGREES"), "{out}");
        assert!(
            out.contains("caution: 2 fields hold values the project does not"),
            "{out}"
        );
    }

    #[test]
    fn a_failed_read_writes_nothing_and_says_so() {
        // Another manufacturer: refused before any memory read.
        let device = device(&memory(OctetOrder::LowFirst));
        let mut other = plan();
        other.manufacturer = 0x0083;
        let (probed, out) = run(&device, &other);
        assert_eq!(probed, Probed::Failed);
        assert!(out.contains("not probed:"), "{out}");
        assert!(no_writes(&device));
    }

    #[test]
    fn nothing_to_read_is_a_failure_that_lists_why() {
        let device = device(&memory(OctetOrder::LowFirst));
        let mut empty = plan();
        empty.fields.clear();
        let (probed, out) = run(&device, &empty);
        assert_eq!(probed, Probed::Failed);
        assert!(out.contains("nothing to read"), "{out}");
        assert!(out.contains("P-9_R-1"), "{out}");
        assert!(device.seen().is_empty(), "no session for nothing");
    }

    #[test]
    fn arguments_are_parsed_and_writing_flags_refused() {
        let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let parsed = parse_probe_args(&args(&[
            "1.1.5",
            "--project",
            "p.knxdb",
            "--gateway",
            "192.0.2.1:3671",
        ]))
        .expect("parses");
        assert_eq!(parsed.target, "1.1.5");
        assert_eq!(parsed.project, "p.knxdb");
        assert_eq!(parsed.product_db, None);
        assert!(parse_probe_args(&args(&["1.1.5", "--project", "p", "--confirm", "x"])).is_err());
        assert!(parse_probe_args(&args(&["1.1.5", "--project", "p"]))
            .unwrap_err()
            .contains("--gateway"));
        assert!(
            parse_probe_args(&args(&["--project", "p", "--gateway", "192.0.2.1:3671"]))
                .unwrap_err()
                .contains("device address")
        );
        assert!(parse_probe_args(&args(&[
            "1.1.5",
            "1.1.6",
            "--project",
            "p",
            "--gateway",
            "192.0.2.1:3671"
        ]))
        .is_err());
    }
}
