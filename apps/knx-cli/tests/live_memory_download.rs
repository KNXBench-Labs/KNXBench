//! Downloads option C into one real mask-`0701h` device, named by the operator.
//!
//! ```text
//! KNX_GATEWAY=<host>:3671 KNX_DOWNLOAD_ADDRESS=1.1.67 \
//! KNX_DOWNLOAD_CONFIRM="I confirm download to 1.1.67" \
//! KNXBENCH_PRODUCT_CORPUS=<dir with MDT_KP_BE_01_Push_Button_V15a.knxprod> \
//!     cargo test -p knx-cli --test live_memory_download -- --ignored --nocapture
//! ```
//!
//! **This rewrites the device's application, tables and parameters.** It is
//! gated three times: `#[ignore]`; `KNX_DOWNLOAD_CONFIRM` must be the exact
//! phrase `required_confirmation_phrase` gives for this device and
//! `WriteScope::Download`, so an `--ignored` sweep cannot write anything;
//! and `WriteAuthorisation::for_hardware` checks that phrase again.
//!
//! Option C, for `A-0027-15-0BAC` (MDT push button, 2026-09-28): button 1
//! toggles `2/0/53`, button 2 inactive, every other parameter at the
//! product's default, no other group address. The same image, plan and
//! executor as `memory_download_simulated.rs`; nothing is inlined here.
//!
//! After the download, a fresh read-only session reads every segment back
//! and compares it with the image. The masked octets (the individual
//! address at `4001h`–`4002h`) must hold the device's own address, and all
//! three load states must read `Loaded`. Every step and every read is
//! printed.

use std::net::SocketAddrV4;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{
    required_confirmation_phrase, WriteAuthorisation, WriteScope,
};
use knx_core::{GroupAddress, IndividualAddress, EXCLUDED_INDIVIDUAL_ADDRESSES};
use knx_net::commissioning::memory_download::{run_memory_download_observed, Progress};
use knx_net::{BusConnection, KnxNetIpClient, ManagementSession, SessionTiming};
use knx_productdb::download_plan::plan_memory_download;
use knx_productdb::image::{build_download_image, DownloadImage, ImageRequest, Link};
use knx_productdb::{install_package, open_and_migrate};

const FILE: &str = "MDT_KP_BE_01_Push_Button_V15a.knxprod";
const PROGRAM: &str = "M-0083_A-0027-15-0BAC";
/// Octets per verifying `A_Memory_Read`.
const CHUNK: u8 = 8;

fn env(name: &str, example: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name}, e.g. {example}"))
}

fn option_c(target: IndividualAddress) -> DownloadImage {
    let root = std::path::PathBuf::from(env(
        "KNXBENCH_PRODUCT_CORPUS",
        "OriginalData/ProductDatabases",
    ));
    let path = knx_testsupport::find_corpus_file(&root, FILE)
        .unwrap_or_else(|| panic!("{FILE} not under {}", root.display()));
    let bytes = std::fs::read(path).expect("readable");
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).expect("database");
    install_package(&conn, FILE, &bytes).expect("installs");
    let request = ImageRequest {
        program_id: PROGRAM.to_string(),
        individual_address: target,
        values: [
            ("P-1007_R-1007", "2"),
            ("UP-5500_R-5500", "0"),
            ("UP-5501_R-5501", "1"),
        ]
        .into_iter()
        .map(|(short, value)| (format!("{PROGRAM}_{short}"), value.to_string()))
        .collect(),
        links: vec![Link {
            object: 0,
            group_address: GroupAddress::from_raw(0x1035), // 2/0/53
            sending: true,
        }],
    };
    build_download_image(&conn, &request).expect("builds")
}

#[test]
#[ignore = "rewrites a real device; needs KNX_GATEWAY, KNX_DOWNLOAD_ADDRESS, KNX_DOWNLOAD_CONFIRM"]
fn downloads_option_c_and_reads_it_back() {
    let target: IndividualAddress = env("KNX_DOWNLOAD_ADDRESS", "1.1.67")
        .parse()
        .expect("KNX_DOWNLOAD_ADDRESS must be an individual address");
    assert!(
        !EXCLUDED_INDIVIDUAL_ADDRESSES.contains(&target),
        "refusing to contact the project-excluded address {target}"
    );
    let gateway: SocketAddrV4 = env("KNX_GATEWAY", "192.0.2.1:3671")
        .parse()
        .expect("KNX_GATEWAY must be host:port");
    let confirmation = env("KNX_DOWNLOAD_CONFIRM", "I confirm download to 1.1.67");
    assert_eq!(
        confirmation,
        required_confirmation_phrase(target, WriteScope::Download),
        "KNX_DOWNLOAD_CONFIRM must be the exact phrase for this device"
    );
    let authorisation =
        WriteAuthorisation::for_hardware(target, WriteScope::Download, &confirmation)
            .expect("the confirmation phrase is the required one");

    let image = option_c(target);
    let plan = plan_memory_download(&image).expect("plans");
    println!(
        "== plan for {target}: {} steps, {} data octets ==",
        plan.steps.len(),
        plan.data_octets()
    );
    for (index, step) in plan.steps.iter().enumerate() {
        println!("  {index:2}: {step}");
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let tunnel = KnxNetIpClient::new()
            .connect_tunnel(gateway)
            .await
            .expect("connect_tunnel against the real gateway");

        // ---- The download ----
        let outcome = {
            let mut session = ManagementSession::authorised(
                &tunnel,
                AuthorisationPlan::Skip,
                SessionTiming::default(),
                authorisation,
            )
            .expect("an operator-authorised download session");
            // Shown while it happens: a three-minute download that prints
            // nothing until the end looks exactly like a hung one.
            run_memory_download_observed(&mut session, &plan, print_progress).await
        };
        match &outcome {
            Ok(report) => {
                println!("== download finished ==");
                for step in &report.steps {
                    match &step.observed {
                        Some(observed) => {
                            println!("  {:2}: {} -> {observed}", step.index, step.step)
                        }
                        None => println!("  {:2}: {}", step.index, step.step),
                    }
                }
                println!("  final states: {:?}", report.final_states);
                // Loud on purpose: an unconfirmed restart is an Ok, and an
                // operator skimming for FAILED must still not miss it.
                println!("  restart: {}", report.restart);
                println!(
                    "  data octets written and read back: {}",
                    report.data_octets
                );
            }
            Err(err) => println!("== download FAILED: {err} =="),
        }

        // ---- Independent read-back, whatever the outcome ----
        let mut check = ManagementSession::read_only(
            &tunnel,
            target,
            AuthorisationPlan::Skip,
            SessionTiming::default(),
        )
        .expect("a read-only session");
        check
            .connect()
            .await
            .expect("the device answers after the download");
        let mut differences = 0usize;
        for segment in &image.segments {
            let mut stored = Vec::with_capacity(segment.octets.len());
            let length = segment.octets.len() as u32;
            let mut offset = 0u32;
            while offset < length {
                let count = u8::try_from((length - offset).min(u32::from(CHUNK)))
                    .expect("at most CHUNK octets");
                let bytes = check
                    .read_memory(segment.address + offset, count)
                    .await
                    .unwrap_or_else(|err| panic!("read {:04X}h: {err}", segment.address + offset));
                stored.extend(bytes);
                offset += u32::from(count);
            }
            for (index, (&found, &wanted)) in stored.iter().zip(&segment.octets).enumerate() {
                let masked = segment
                    .mask
                    .as_ref()
                    .is_some_and(|mask| mask.get(index) != Some(&0xFF));
                if !masked && found != wanted {
                    differences += 1;
                    println!(
                        "  DIFF {:04X}h: device {found:02X}, image {wanted:02X}",
                        segment.address + index as u32
                    );
                }
            }
            println!(
                "  segment {} at {:04X}h, {} octets read back",
                segment.id,
                segment.address,
                stored.len()
            );
        }
        let address = check.read_memory(0x4001, 2).await.expect("read 4001h");
        println!("  individual address octets 4001h-4002h: {address:02X?}");
        let states = check.read_memory(0xB6EA, 4).await.expect("read B6EAh");
        println!("  load states B6EA..B6ED: {states:02X?} (01 = Loaded)");
        check.disconnect().await;
        tunnel.disconnect().await.expect("clean tunnel disconnect");

        let report = outcome.expect("the download completes");
        if !report.restart.is_confirmed() {
            println!(
                "== restart NOT confirmed: the new program runs only after a \
                 power cycle or a restart the operator sends on purpose =="
            );
        }
        assert_eq!(differences, 0, "every unmasked octet reads back as written");
        let raw = target.raw();
        assert_eq!(
            address,
            raw.to_be_bytes(),
            "the individual address is untouched"
        );
        assert_eq!(
            &states[..3],
            &[0x01, 0x01, 0x01],
            "all three machines Loaded"
        );
    });
}

/// Everything below goes from KNXBench *to the device*.
fn print_progress(progress: Progress) {
    match progress {
        Progress::Started {
            target,
            steps,
            data_octets,
        } => println!(
            "== download to device {target}: {steps} steps, {data_octets} octets of segment data =="
        ),
        Progress::StepStarted { index, of, step } => {
            println!("  [{:2}/{of}] {step}", index + 1)
        }
        Progress::DataWritten {
            address,
            octets,
            written,
            of,
            ..
        } => println!(
            "        -> {address:04X}h {} ({written}/{of} octets, read back OK)",
            octets
                .iter()
                .map(|octet| format!("{octet:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Progress::StepDone(done) => {
            if let Some(observed) = done.observed {
                println!("        done: {observed}");
            }
        }
        Progress::Authorised {
            authorisation,
            suspicious,
        } => println!("        access: {authorisation:?}, suspicious: {suspicious}"),
    }
}
