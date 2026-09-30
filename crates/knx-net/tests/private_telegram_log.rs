//! K19: the private ETS telegram capture, decoded offline.
//!
//! An ETS `CommunicationLog` (`{http://knx.org/xml/telegrams/01}`) holds one
//! `Telegram` per captured frame, its cEMI message as hex `RawData`. This
//! test decodes every one with [`decode_l_data`] and re-encodes it with
//! [`encode_l_data`], then compares the two. It reads one private file; the
//! path comes from `KNXBENCH_TELEGRAM_LOG` only, because the capture belongs
//! to the maintainer's installation and never enters the repository.
//!
//! Only aggregate counts leave this test: per decode outcome, refusal
//! reason, destination kind, transport and application service. Addresses,
//! payloads, timestamps and connection names are never printed, not even on
//! failure. The re-encode comparison skips Ctrl1 and Ctrl2: the encoder
//! writes its own priority, repeat and hop-count bits (`encode_l_data`), so
//! those two octets say nothing about decoding. Everything after them (both
//! addresses, `L`, TPCI, APCI, data) has to come back octet for octet.
//!
//! Not an importer: KNXBench does not read this format for users
//! (`goal-commission.md` K19).

use std::collections::BTreeMap;

use knx_net::cemi::{decode_l_data, encode_l_data, ApplicationService, Destination, Tpci};
use quick_xml::events::Event;
use quick_xml::Reader;

const TELEGRAMS_NAMESPACE: &str = "http://knx.org/xml/telegrams/01";

/// The `Debug` spelling of an enum value, cut at its payload: `GroupValueWrite`
/// for `GroupValueWrite(..)`, so a count label never carries a value.
fn variant(value: &impl std::fmt::Debug) -> String {
    let text = format!("{value:?}");
    text.split(['(', ' ', '{'])
        .next()
        .unwrap_or_default()
        .to_string()
}

fn hex(raw: &str) -> Option<Vec<u8>> {
    if !raw.len().is_multiple_of(2) {
        return None;
    }
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(raw.get(i..i + 2)?, 16).ok())
        .collect()
}

#[derive(Default, Debug)]
struct Census {
    telegrams: usize,
    /// Per outcome: `decoded`, `refused`, `not-hex`, `no-raw-data`.
    outcome: BTreeMap<&'static str, usize>,
    refused: BTreeMap<String, usize>,
    destination: BTreeMap<&'static str, usize>,
    transport: BTreeMap<String, usize>,
    service: BTreeMap<String, usize>,
    /// Decoded frames `Tpci::Unknown` or `ApplicationService::Other` keep
    /// only as raw values: decoded, but not understood.
    unknown: usize,
    /// Per result of re-encoding a decoded frame: `identical` past
    /// Ctrl1/Ctrl2, `differs`, or the encoder's refusal.
    reencode: BTreeMap<String, usize>,
    /// Per Ctrl1/Ctrl2 field: frames whose captured value the decode →
    /// encode path did not bring back. `LDataFrame` has no field for these,
    /// so the encoder writes its own; a count here is information the
    /// decoder drops, not a decoding error.
    control_not_carried: BTreeMap<&'static str, usize>,
}

fn census(xml: &str) -> Census {
    let mut census = Census::default();
    let mut reader = Reader::from_str(xml);
    let mut namespace_seen = false;
    loop {
        let event = reader
            .read_event()
            // quick-xml's message can quote the document; it is not printed.
            .unwrap_or_else(|_| panic!("the capture is not well-formed XML"));
        let element = match event {
            Event::Start(element) | Event::Empty(element) => element,
            Event::Eof => break,
            _ => continue,
        };
        match element.local_name().as_ref() {
            "CommunicationLog" => {
                let namespace = element
                    .try_get_attribute("xmlns")
                    .ok()
                    .flatten()
                    .map(|a| a.value.into_owned());
                assert_eq!(
                    namespace.as_deref(),
                    Some(TELEGRAMS_NAMESPACE),
                    "not an ETS telegram capture"
                );
                namespace_seen = true;
            }
            "Telegram" => {
                census.telegrams += 1;
                let raw = element
                    .try_get_attribute("RawData")
                    .ok()
                    .flatten()
                    .map(|a| a.value.into_owned());
                let Some(raw) = raw else {
                    *census.outcome.entry("no-raw-data").or_default() += 1;
                    continue;
                };
                let Some(bytes) = hex(&raw) else {
                    *census.outcome.entry("not-hex").or_default() += 1;
                    continue;
                };
                count_frame(&mut census, &bytes);
            }
            _ => {}
        }
    }
    assert!(namespace_seen, "no CommunicationLog root element");
    census
}

fn count_frame(census: &mut Census, bytes: &[u8]) {
    let frame = match decode_l_data(bytes) {
        Ok(frame) => frame,
        Err(error) => {
            *census.outcome.entry("refused").or_default() += 1;
            *census.refused.entry(variant(&error)).or_default() += 1;
            return;
        }
    };
    *census.outcome.entry("decoded").or_default() += 1;
    let destination = match frame.destination {
        Destination::Group(_) => "group",
        Destination::Individual(_) => "individual",
        Destination::SystemBroadcast => "system-broadcast",
    };
    *census.destination.entry(destination).or_default() += 1;
    *census
        .transport
        .entry(variant(&frame.transport))
        .or_default() += 1;
    *census.service.entry(variant(&frame.service)).or_default() += 1;
    if matches!(frame.transport, Tpci::Unknown(_))
        || matches!(frame.service, ApplicationService::Other { .. })
    {
        census.unknown += 1;
    }
    let reencode = match encode_l_data(&frame) {
        Ok(again) => {
            for field in control_fields_not_carried(bytes, &again) {
                *census.control_not_carried.entry(field).or_default() += 1;
            }
            if same_past_control_field(bytes, &again) {
                "identical".to_string()
            } else {
                "differs".to_string()
            }
        }
        Err(error) => format!("encoder refused: {}", variant(&error)),
    };
    *census.reencode.entry(reencode).or_default() += 1;
}

/// Whether `again` repeats `captured` apart from Ctrl1 and Ctrl2: octet 0
/// is the message code, octet 1 AddIL, then AddIL octets, then Ctrl1 and
/// Ctrl2, then the addresses, `L` and the TPDU.
fn same_past_control_field(captured: &[u8], again: &[u8]) -> bool {
    let Some(&add_info_len) = captured.get(1) else {
        return false;
    };
    let skip = 2 + usize::from(add_info_len) + 2;
    captured.first() == again.first()
        && captured.get(1..skip - 2) == again.get(1..skip - 2)
        && captured.get(skip..) == again.get(skip..)
}

/// Ctrl1 and Ctrl2 fields whose value differs between the captured frame and
/// its re-encoding. Ctrl1 `FT r R SB P P A C`, Ctrl2 `AT hop hop hop EFF`
/// (EMI_IMI v01.04.02 AS §4.1.5.3.2); priority codes `00` system, `01`
/// normal, `10` urgent, `11` low (Data Link Layer General v01.03.02 AS
/// §2.2.3). Both frames were checked to carry no additional information
/// beyond octet 1's length before this is called.
fn control_fields_not_carried(captured: &[u8], again: &[u8]) -> Vec<&'static str> {
    const FIELDS: [(&str, usize, u8); 9] = [
        ("frame type", 0, 0b1000_0000),
        ("repeat", 0, 0b0010_0000),
        ("system broadcast", 0, 0b0001_0000),
        ("priority", 0, 0b0000_1100),
        ("ack request", 0, 0b0000_0010),
        ("confirm", 0, 0b0000_0001),
        ("address type", 1, 0b1000_0000),
        ("hop count", 1, 0b0111_0000),
        ("extended frame format", 1, 0b0000_1111),
    ];
    let (Some(&captured_add_info), Some(&again_add_info)) = (captured.get(1), again.get(1)) else {
        return vec!["frame too short"];
    };
    let ctrl = |frame: &[u8], add_info: u8| {
        let at = 2 + usize::from(add_info);
        Some([*frame.get(at)?, *frame.get(at + 1)?])
    };
    let (Some(captured), Some(again)) = (
        ctrl(captured, captured_add_info),
        ctrl(again, again_add_info),
    ) else {
        return vec!["frame too short"];
    };
    FIELDS
        .iter()
        .filter(|(_, octet, mask)| captured[*octet] & mask != again[*octet] & mask)
        .map(|(name, _, _)| *name)
        .collect()
}

#[test]
#[ignore = "requires a private ETS CommunicationLog; set KNXBENCH_TELEGRAM_LOG to its path"]
fn every_captured_telegram_decodes_and_survives_the_round_trip() {
    let path = std::env::var_os("KNXBENCH_TELEGRAM_LOG")
        .expect("set KNXBENCH_TELEGRAM_LOG to the ETS CommunicationLog XML");
    // The path is private; the message names the variable, not the file.
    let xml = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("KNXBENCH_TELEGRAM_LOG is not a readable UTF-8 file"));
    let census = census(&xml);
    println!("{census:#?}");

    assert!(census.telegrams > 0, "the capture holds no Telegram");
    assert_eq!(
        census.outcome.get("decoded"),
        Some(&census.telegrams),
        "every telegram decodes"
    );
    assert_eq!(census.unknown, 0, "every decoded telegram is understood");
    assert_eq!(
        census.reencode.get("identical"),
        Some(&census.telegrams),
        "every decoded telegram re-encodes to the captured octets"
    );
    // The capture's measured baseline (2026-09-30), aggregate only. Drift
    // means the capture or the decoder changed: explain which before
    // updating. `priority`: 16 telegrams were sent at normal priority, and
    // `LDataFrame` has no priority field, so they re-encode at low
    // (KNOWN_LIMITATIONS §147).
    assert_eq!(census.telegrams, 71);
    assert_eq!(
        census.service,
        BTreeMap::from([
            ("GroupValueRead".to_string(), 8),
            ("GroupValueResponse".to_string(), 8),
            ("GroupValueWrite".to_string(), 55),
        ])
    );
    assert_eq!(
        census.control_not_carried,
        BTreeMap::from([("priority", 16)])
    );
}

/// The census itself, on synthetic frames: the private run above can only
/// be trusted if these counts are.
#[test]
fn the_census_counts_outcomes_without_carrying_values() {
    // GroupValueWrite 1 from 1.1.1 to 1/0/1 (0x0801); the same with Ctrl1
    // 0xB0 and hop count 5, which must still count as identical; a
    // truncated frame; odd hex; a telegram without RawData.
    let xml = format!(
        r#"<CommunicationLog xmlns="{TELEGRAMS_NAMESPACE}">
            <Telegram RawData="2900BCE011010801010081" />
            <Telegram RawData="2900B0D011010801010081" />
            <Telegram RawData="2900BC" />
            <Telegram RawData="290" />
            <Telegram />
        </CommunicationLog>"#
    );
    let census = census(&xml);
    assert_eq!(census.telegrams, 5);
    assert_eq!(census.outcome["decoded"], 2);
    assert_eq!(census.outcome["refused"], 1);
    assert_eq!(census.outcome["not-hex"], 1);
    assert_eq!(census.outcome["no-raw-data"], 1);
    assert_eq!(census.refused["TooShort"], 1);
    assert_eq!(census.destination["group"], 2);
    assert_eq!(census.transport["UnnumberedData"], 2);
    assert_eq!(census.service["GroupValueWrite"], 2);
    assert_eq!(census.reencode["identical"], 2);
    assert_eq!(census.unknown, 0);
    assert_eq!(
        census.control_not_carried,
        BTreeMap::from([("priority", 1), ("hop count", 1)])
    );
    // No count label carries an address or a value.
    let labels = format!("{census:?}");
    assert!(
        !labels.contains("1.1.1") && !labels.contains("0801"),
        "{labels}"
    );
}

#[test]
fn only_ctrl1_and_ctrl2_may_differ() {
    let captured = hex("2900BCE011010801010081").unwrap();
    let mut control = captured.clone();
    control[2] = 0xB0;
    control[3] = 0xD0;
    assert!(same_past_control_field(&captured, &control));
    for octet in [0, 1, 4, 5, 6, 7, 8, 9, 10] {
        let mut changed = captured.clone();
        changed[octet] ^= 0x01;
        assert!(
            !same_past_control_field(&captured, &changed),
            "octet {octet}"
        );
    }
    assert!(!same_past_control_field(&captured, &captured[..10]));
    assert!(!same_past_control_field(&[], &captured));
}

#[test]
fn a_dropped_control_field_is_named() {
    let low = hex("2900BCE011010801010081").unwrap();
    // Ctrl1 1011_0100: normal priority; Ctrl2 1101_0000: hop count 5.
    let normal_hop5 = hex("2900B4D011010801010081").unwrap();
    assert_eq!(
        control_fields_not_carried(&normal_hop5, &low),
        ["priority", "hop count"]
    );
    assert!(control_fields_not_carried(&low, &low).is_empty());
    assert_eq!(
        control_fields_not_carried(&low[..3], &low),
        ["frame too short"]
    );

    let xml = format!(
        r#"<CommunicationLog xmlns="{TELEGRAMS_NAMESPACE}">
            <Telegram RawData="2900B4E011010801010081" />
            <Telegram RawData="2900BCE011010801010081" />
        </CommunicationLog>"#
    );
    let census = census(&xml);
    assert_eq!(
        census.control_not_carried,
        BTreeMap::from([("priority", 1)])
    );
    assert_eq!(census.reencode["identical"], 2);
}

#[test]
#[should_panic(expected = "not an ETS telegram capture")]
fn another_namespace_is_refused() {
    census(r#"<CommunicationLog xmlns="http://knx.org/xml/project/20"></CommunicationLog>"#);
}
