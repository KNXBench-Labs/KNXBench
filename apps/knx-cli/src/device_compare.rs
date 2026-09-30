//! `knx device compare`: what a download would change on a device, read only.
//!
//! The same plan as `knx device download` (the same project, product file
//! and `--partial` selection), then `knx_net`'s `compare_with_plan`: a
//! read-only session that reads exactly the regions the plan would write and the load
//! states of the machines it touches. It prints every run of octets where
//! the device and the plan differ. It has no phrase because it cannot write:
//! a read-only session has no authorisation, so every write path is refused
//! inside the session, and it sends no Verify Mode write on connect either.
//!
//! No access key is sent (`AuthorisationPlan::Skip`). A device that protects
//! its memory against reading at the free level refuses the read, and the
//! command says so; it never guesses a key.
//!
//! `[V]` 2026-09-29, the maintainer's house, read-only: this is the check
//! that found project drift on five devices and the missing instance flags
//! (RESEARCH §19.13), done then with a throwaway test.

use std::io::Write;

use knx_core::commissioning::device_backup::{DeviceBackup, OctetChange};
use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::ContactableAddress;
use knx_net::commissioning::memory_download::compare_with_plan;
use knx_net::{ManagementTransport, SessionTiming};
use knx_productdb::image::SegmentImage;

/// `knx device compare`'s arguments. No `--confirm`, `--key-file` or
/// `--backup-dir`: they belong to writing, and are refused as unknown.
#[derive(Debug, PartialEq, Eq)]
pub struct CompareArgs {
    /// The device address, unparsed.
    pub target: String,
    /// The project.
    pub project: String,
    /// The product database, if not the default one.
    pub product_db: Option<String>,
    /// Compare only what a partial download would write.
    pub partial: Option<knx_core::commissioning::partial_memory_download::PartialDownloadParts>,
    /// The gateway.
    pub gateway: std::net::SocketAddrV4,
}

/// Parses `knx device compare`'s arguments.
pub fn parse_compare_args(args: &[String]) -> Result<CompareArgs, String> {
    let mut target = None;
    let mut project = None;
    let mut product_db = None;
    let mut partial = None;
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
            "--partial" => {
                partial = Some(crate::device_download::parse_partial(&crate::take_value(
                    args,
                    i + 1,
                    "--partial",
                )?)?);
                i += 1;
            }
            "--gateway" => {
                gateway = Some(crate::take_value(args, i + 1, "--gateway")?);
                i += 1;
            }
            flag if flag.starts_with("--") => {
                return Err(format!(
                    "unknown flag {flag} (compare only reads; writing is `knx device download`)"
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
        .ok_or("--gateway <host:port> is required: compare reads the device")?
        .parse()
        .map_err(|_| "--gateway must be host:port, e.g. 192.0.2.1:3671".to_string())?;
    Ok(CompareArgs {
        target: target.ok_or("missing the device address, e.g. 1.1.67")?,
        project: project.ok_or("--project <path.knxdb> is required")?,
        product_db,
        partial,
        gateway,
    })
}

/// How a comparison ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compared {
    /// The device holds every octet the plan would write.
    Same,
    /// At least one octet differs.
    Different,
    /// Nothing was compared: the read failed or was refused.
    Failed,
}

/// Reads what `plan` would overwrite on `target` and prints the
/// differences to `out`. `segments` only names where a difference lies.
pub async fn compare<T: ManagementTransport>(
    transport: &T,
    target: ContactableAddress,
    timing: SessionTiming,
    plan: &MemoryDownloadPlan,
    segments: &[SegmentImage],
    out: &mut impl Write,
) -> Compared {
    let address = target.address();
    let _ = writeln!(
        out,
        "== compare device {address} with the project: read only, nothing is written =="
    );
    let compared = match compare_with_plan(transport, target, timing, plan).await {
        Ok(compared) => compared,
        Err(e) => {
            let _ = writeln!(out, "not compared: {e}");
            return Compared::Failed;
        }
    };
    let (held, changes) = (&compared.held, &compared.changes);
    print_device(out, held);
    print_changes(out, held, changes, segments);
    let _ = writeln!(out, "written to the device: no (read only)");
    if compared.is_same() {
        Compared::Same
    } else {
        Compared::Different
    }
}

fn print_device(out: &mut impl Write, held: &DeviceBackup) {
    let _ = writeln!(
        out,
        "device:  mask {:04X}h, manufacturer {:04X}h, as the plan expects",
        held.mask.0, held.manufacturer
    );
    for (machine, state) in &held.load_states {
        let _ = writeln!(out, "         {machine}: {state:?}");
    }
    if !held.was_loaded() {
        let _ = writeln!(
            out,
            "note:    not every part is Loaded; this compares memory, it does not say the \
             application runs"
        );
    }
}

fn print_changes(
    out: &mut impl Write,
    held: &DeviceBackup,
    changes: &[OctetChange],
    segments: &[SegmentImage],
) {
    let differing: usize = changes.iter().map(|change| change.planned.len()).sum();
    if changes.is_empty() {
        let _ = writeln!(
            out,
            "the device holds all {} octets a download would write",
            held.octets()
        );
        return;
    }
    let _ = writeln!(
        out,
        "a download would change {differing} of {} octets, in {} runs:",
        held.octets(),
        changes.len()
    );
    for change in changes {
        let segment = segments
            .iter()
            .find(|segment| {
                let start = segment.address;
                let end = start + segment.octets.len() as u32;
                (start..end).contains(&u32::from(change.address))
            })
            .map_or_else(String::new, |segment| format!(" ({})", segment.id));
        let _ = writeln!(
            out,
            "  {:04X}h+{}{segment}: device {} -> project {}",
            change.address,
            change.planned.len(),
            hex(&change.device),
            hex(&change.planned)
        );
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
    use knx_core::commissioning::load_control_memory::{event_record, MemoryLoadStateMachine};
    use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
    use knx_core::commissioning::memory_download::MemoryDownloadStep;
    use knx_core::commissioning::properties::{ObjectIndex, PID_MANUFACTURER_ID};
    use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
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

    /// Unload, write `4000h` and `4400h`, complete: the shape of a real
    /// plan, cut down to two runs.
    fn plan() -> MemoryDownloadPlan {
        let table = MemoryLoadStateMachine::AddressTable;
        let record = |event| {
            MemoryDownloadStep::LoadRecord(event_record(table, event).expect("an event record"))
        };
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0x0083,
            steps: vec![
                MemoryDownloadStep::Connect,
                record(LoadEvent::Unload),
                MemoryDownloadStep::WriteMemory {
                    address: 0x4000,
                    octets: vec![0x02, 0x11, 0x43],
                },
                MemoryDownloadStep::WriteMemory {
                    address: 0x4400,
                    octets: vec![1, 2, 3, 4],
                },
                record(LoadEvent::LoadCompleted),
                MemoryDownloadStep::Restart,
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    fn segments() -> Vec<SegmentImage> {
        vec![
            SegmentImage {
                id: "AS-4000".into(),
                address: 0x4000,
                octets: vec![0; 3],
                mask: None,
            },
            SegmentImage {
                id: "AS-4400".into(),
                address: 0x4400,
                octets: vec![0; 4],
                mask: None,
            },
        ]
    }

    fn mdt() -> SimulatedDevice {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..SimulatorConfig::default()
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
        device.preset_load_state(ObjectIndex::new(1), LoadState::Loaded);
        device
    }

    fn run(device: &SimulatedDevice) -> (Compared, String) {
        let target = ContactableAddress::new(device.address()).unwrap();
        let mut out = Vec::new();
        let compared = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(compare(
                device,
                target,
                fast(),
                &plan(),
                &segments(),
                &mut out,
            ));
        (compared, String::from_utf8(out).unwrap())
    }

    fn writes(device: &SimulatedDevice) -> Vec<Seen> {
        device
            .seen()
            .into_iter()
            .filter(|seen| {
                matches!(
                    seen,
                    Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
                )
            })
            .collect()
    }

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn the_arguments_need_a_gateway_and_refuse_every_write_flag() {
        let parsed =
            parse_compare_args(&args("1.1.5 --project p.knxdb --gateway 192.0.2.1:3671")).unwrap();
        assert_eq!(parsed.target, "1.1.5");
        assert_eq!(parsed.gateway.to_string(), "192.0.2.1:3671");
        assert_eq!(parsed.partial, None);
        assert!(parse_compare_args(&args("1.1.5 --project p.knxdb"))
            .unwrap_err()
            .contains("--gateway"));
        for flag in [
            "--confirm x",
            "--key-file k",
            "--backup-dir d",
            "--accept-untested x",
        ] {
            let error = parse_compare_args(&args(&format!(
                "1.1.5 --project p.knxdb --gateway 192.0.2.1:3671 {flag}"
            )))
            .unwrap_err();
            assert!(error.contains("compare only reads"), "{flag}: {error}");
        }
        assert!(parse_compare_args(&args("1.1.5 --project p --gateway nowhere")).is_err());
    }

    #[test]
    fn a_differing_device_is_listed_run_by_run_and_nothing_is_written() {
        let device = mdt();
        device.preset_memory(0x4000, &[0x02, 0x11, 0x44]);
        device.preset_memory(0x4400, &[9, 2, 3, 9]);
        let before = device.memory(0x4400, 4);
        let (compared, out) = run(&device);
        assert_eq!(compared, Compared::Different, "{out}");
        assert!(
            out.contains("a download would change 3 of 7 octets, in 3 runs"),
            "{out}"
        );
        assert!(
            out.contains("4002h+1 (AS-4000): device 44 -> project 43"),
            "{out}"
        );
        assert!(
            out.contains("4400h+1 (AS-4400): device 09 -> project 01"),
            "{out}"
        );
        assert!(
            out.contains("4403h+1 (AS-4400): device 09 -> project 04"),
            "{out}"
        );
        assert!(out.contains("address table: Loaded"), "{out}");
        assert!(
            out.contains("written to the device: no (read only)"),
            "{out}"
        );
        assert_eq!(writes(&device), vec![]);
        assert_eq!(device.memory(0x4400, 4), before);
    }

    #[test]
    fn a_device_that_holds_the_plan_says_so() {
        let device = mdt();
        device.preset_memory(0x4000, &[0x02, 0x11, 0x43]);
        device.preset_memory(0x4400, &[1, 2, 3, 4]);
        let (compared, out) = run(&device);
        assert_eq!(compared, Compared::Same, "{out}");
        assert!(out.contains("the device holds all 7 octets"), "{out}");
    }

    #[test]
    fn another_product_is_not_compared() {
        let device = mdt();
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x02]);
        let (compared, out) = run(&device);
        assert_eq!(compared, Compared::Failed, "{out}");
        assert!(
            out.contains("not compared: the device is from manufacturer 0002h"),
            "{out}"
        );
        assert!(!out.contains("a download would change"), "{out}");
        assert_eq!(writes(&device), vec![]);
    }

    #[test]
    fn a_device_not_fully_loaded_is_flagged() {
        let device = mdt();
        device.preset_load_state(ObjectIndex::new(1), LoadState::Unloaded);
        let (_, out) = run(&device);
        assert!(out.contains("address table: Unloaded"), "{out}");
        assert!(out.contains("not every part is Loaded"), "{out}");
    }
}
