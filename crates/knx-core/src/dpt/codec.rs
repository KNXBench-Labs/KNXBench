//! Encode/decode between engineering values (text) and `GroupValue` wire
//! payloads, for main types 1-19: the fourteen the original design doc's
//! §4.1 lists (`docs/superpowers/specs/2026-09-11-dpt-codec-design.md` —
//! eleven marked in its table, and 6, 12 and 16 named in the paragraph
//! below it), plus 4, 10, 11, 15, and 19, added by task E4 (2026-09-13) — see
//! `docs/KNOWN_LIMITATIONS.md` §61 for that extension's per-type citations
//! and judgment calls (03_07_02 Datapoint Types v02.02.01 AS, hereafter
//! "DPT-AS").
//!
//! Pure: no I/O, no logging, no clock. Every fact this module states about
//! bit layout, range or rounding is cited to a DPT-AS section or, for the
//! inline-vs-own-octet threshold, to the Application Layer specification
//! (03_03_07 Application Layer v02.01.01 AS, hereafter "AL-AS").

use std::fmt;

use super::{DptRef, GroupValue};

/// A decoded datapoint value. Variants mirror the Standard's own format
/// families (DPT-AS §1.3.1's notation), not per-subtype semantic types —
/// see spec E4-D4. A `DptValue` only carries the meaning its format
/// encodes; a subtype whose *wording* differs from the generic field name
/// used here (3.008's Up/Down vs. `Step::increase`, 1.008's Up/Down vs.
/// `Bool`) is not modelled by this slice, and is never invented by it.
#[derive(Debug, Clone, PartialEq)]
pub enum DptValue {
    /// B1 — main type 1 (DPT-AS §3.1).
    Bool(bool),
    /// B2 — main type 2 (DPT-AS §3.2). `control` is field `c`; `value` is
    /// field `v`, meaningful only when `control` is set (§3.2: `c=0` is
    /// "no control", `v` don't-care).
    ControlBool { control: bool, value: bool },
    /// B1U3 — main type 3 (DPT-AS §3.3). `increase` is field `c`, named
    /// generically: 3.007 calls it Increase/Decrease, 3.008 calls it
    /// Up/Down (subtype-specific wording is out of scope for this slice —
    /// see the brief). `step_code` is the raw 3-bit field as transmitted:
    /// `0` is the dedicated "Break" code, `1..=7` denote an interval count
    /// of `2^(step_code-1)`; this codec stores the raw code, not the
    /// derived count, because the count is a display concern.
    Step { increase: bool, step_code: u8 },
    /// Raw unsigned counts: U8 (main type 5, except its scaled subtypes),
    /// U16 (main type 7), and U32 (main type 12).
    Unsigned(u32),
    /// Two's-complement signed counts: V8 (main type 6, except 6.020),
    /// V16 (main type 8, except 8.010), and V32 (main type 13, except its
    /// one scaled subtype, 13.002 DPT_FlowRate_m3/h).
    Signed(i32),
    /// Engineering values that need a fractional/scaled representation:
    /// main type 5's scaled subtypes (5.001 DPT_Scaling, 5.003 DPT_Angle),
    /// main type 8's 8.010 DPT_Percent_V16, main type 13's 13.002
    /// DPT_FlowRate_m3/h, and the F16/F32 float families (main types 9
    /// and 14, both always this variant regardless of subtype). The
    /// codec's job is the number, not its storage width, so one variant
    /// serves all of them.
    Float(f64),
    /// A single character (A8, main type 4, DPT-AS §3.4) or an A[14]
    /// fixed-length character string (main type 16, DPT-AS §3.17) — both
    /// share the same two character sets (7-bit ASCII / ISO 8859-1) and
    /// this one variant, since a `String` holds one character exactly as
    /// well as fourteen.
    Text(String),
    /// N3U5r2U6r2U6 — main type 10, time of day plus optional weekday
    /// (DPT-AS §3.11). `day` is `None` for wire code `0` ("no day",
    /// §3.11's own table row for the Day field) and `Some(1..=7)` for
    /// Monday(1)..Sunday(7); see `decode_time_of_day`'s doc comment for
    /// why this codec chose `Option<u8>` rather than the raw 0-7 code.
    TimeOfDay {
        day: Option<u8>,
        hour: u8,
        minute: u8,
        second: u8,
    },
    /// r3U5r4U4r1U7 — main type 11, date (DPT-AS §3.12). `year` is
    /// already century-resolved to `1990..=2089` per §3.12's "Century
    /// Encoding" rule (EXAMPLE 5), not the raw 7-bit octet — see
    /// `decode_date`'s doc comment.
    Date { day: u8, month: u8, year: u16 },
    /// U4U4U4U4U4U4B4N4 — main type 15, access data (DPT-AS §3.16).
    /// `code` holds the six BCD digits D6..D1 (most significant first,
    /// each `0..=9`), matching EXAMPLE 6/7's digit numbering. `accepted`
    /// is field `P`, `right_to_left` is field `D` (`false` = left to
    /// right, the Standard's own `0` encoding), `error` is field `E`,
    /// `encrypted` is field `C`, `index` is the 4-bit `Index` field
    /// (`0..=15`).
    AccessData {
        code: [u8; 6],
        error: bool,
        accepted: bool,
        right_to_left: bool,
        encrypted: bool,
        index: u8,
    },
    /// U8[r4U4][r3U5][U3U5][r2U6][r2U6]B16 — main type 19, date and time
    /// (DPT-AS §3.20/§3.20.1). Every field the Standard's encoding row
    /// and Note 15 assign a wire position to is carried here — see
    /// `decode_datetime`'s doc comment for `SRC`, the one bit the
    /// Standard's own diagram is internally inconsistent about (named in
    /// the field-names row, reserved in the encoding row and Note 15);
    /// this codec follows the encoding row, a stated ruling, not a
    /// missing-bit default.
    ///
    /// `year` is already offset-resolved (`1900 + raw`, §3.20's own
    /// table). `month`/`day_of_month`/`hour`/`minute`/`second` are the
    /// raw field values, stored even when their matching `_invalid` flag
    /// is set (a "field not valid" flag is a *meaning* the Standard
    /// attaches to the octet, not a licence for this codec to discard the
    /// octet's bits — see the brief's "never silently discard
    /// information"). `day_of_week` is the raw 3-bit code (`0..=7`);
    /// `0` together with `day_of_week_invalid == false` is DPT-AS §3.20.1
    /// NOTE 14's "any day" wildcard, deliberately kept distinct from
    /// `day_of_week_invalid == true` ("the field is not valid, ignore
    /// it") by using two separate fields rather than collapsing both into
    /// one `Option`.
    DateTime {
        year: u16,
        year_invalid: bool,
        month: u8,
        day_of_month: u8,
        date_invalid: bool,
        day_of_week: u8,
        day_of_week_invalid: bool,
        hour: u8,
        minute: u8,
        second: u8,
        time_invalid: bool,
        fault: bool,
        working_day: bool,
        working_day_unknown: bool,
        summer_time: bool,
        externally_synchronized: bool,
    },
    /// Scene number — main type 17. Holds the wire value 0-63 exactly;
    /// see `decode_scene`'s doc comment for the off-by-one ruling this
    /// deliberately does not apply.
    Scene { number: u8 },
    /// Scene control (learn/activate + scene number) — main type 18.
    /// `number` is the same undecorated wire value as `Scene`'s, 0-63,
    /// even though — unlike main type 17 — this type's own section
    /// (DPT-AS §3.19 NOTE 9) does carry a +1 display recommendation;
    /// see `decode_scene_control`'s doc comment for why this codec
    /// still hands back the untouched wire value regardless.
    SceneControl { learn: bool, number: u8 },
}

impl DptValue {
    /// The short human-readable form the CLI prints (spec E4-D9), e.g.
    /// `on`, `off`, `-3.4`, `50 %`, `increase step 3`. Needs `dpt` because
    /// a unit is only ever attached where the *subtype's scaling* makes it
    /// part of the conversion (DPT-AS §3.5's scaled-vs-non-scaled split):
    /// 5.001/5.003 and 8.010 print a unit; every raw count — including an
    /// unscaled main type 5 read with `sub: None` — prints the bare
    /// number, because that number is a raw octet count, not yet a
    /// percentage or an angle.
    ///
    /// Float formatting rule: Rust's `f64` `Display` already produces the
    /// shortest decimal representation that reads back to the same value,
    /// and never switches to exponent notation for `{}` (only `{:e}`
    /// does) — exactly "enough digits to round-trip visually, without
    /// exponent noise". This module relies on that guarantee rather than
    /// hand-rolling a digit count.
    pub fn format(&self, dpt: DptRef) -> String {
        match self {
            DptValue::Float(v) => match (dpt.main, dpt.sub) {
                (5, Some(1)) => format!("{v} %"),
                (5, Some(3)) => format!("{v} \u{b0}"), // °
                (8, Some(10)) => format!("{v} %"),
                // 13.002 DPT_FlowRate_m3/h — main type 13's one scaled
                // subtype (DPT-AS §3.14.1), same treatment as the U8/V16
                // scaled subtypes above.
                (13, Some(2)) => format!("{v} m\u{b3}/h"), // m³/h
                _ => v.to_string(),
            },
            other => other.to_string(),
        }
    }
}

impl fmt::Display for DptValue {
    /// Context-free rendering (no `DptRef`, so no unit — see `format`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DptValue::Bool(b) => write!(f, "{}", if *b { "on" } else { "off" }),
            DptValue::ControlBool { control, value } => {
                if !control {
                    write!(f, "no control")
                } else {
                    write!(f, "control {}", if *value { "on" } else { "off" })
                }
            }
            DptValue::Step {
                increase,
                step_code,
            } => {
                let dir = if *increase { "increase" } else { "decrease" };
                if *step_code == 0 {
                    write!(f, "{dir} break")
                } else {
                    write!(f, "{dir} step {step_code}")
                }
            }
            DptValue::Unsigned(v) => write!(f, "{v}"),
            DptValue::Signed(v) => write!(f, "{v}"),
            DptValue::Float(v) => write!(f, "{v}"),
            DptValue::Text(s) => write!(f, "{s}"),
            DptValue::Scene { number } => write!(f, "scene {number}"),
            DptValue::SceneControl { learn, number } => {
                write!(
                    f,
                    "{} scene {number}",
                    if *learn { "learn" } else { "activate" }
                )
            }
            DptValue::TimeOfDay {
                day,
                hour,
                minute,
                second,
            } => {
                let day_word = match day {
                    Some(d) => weekday_name(*d),
                    None => "none",
                };
                write!(f, "{day_word} {hour:02}:{minute:02}:{second:02}")
            }
            DptValue::Date { day, month, year } => write!(f, "{year:04}-{month:02}-{day:02}"),
            DptValue::AccessData {
                code,
                error,
                accepted,
                right_to_left,
                encrypted,
                index,
            } => {
                let code_str: String = code.iter().map(u8::to_string).collect();
                write!(
                    f,
                    "{code_str} {} {} {} {} {index}",
                    bool_word(*error),
                    bool_word(*accepted),
                    if *right_to_left { "right" } else { "left" },
                    bool_word(*encrypted)
                )
            }
            DptValue::DateTime {
                year,
                year_invalid,
                month,
                day_of_month,
                date_invalid,
                day_of_week,
                day_of_week_invalid,
                hour,
                minute,
                second,
                time_invalid,
                fault,
                working_day,
                working_day_unknown,
                summer_time,
                externally_synchronized,
            } => {
                write!(
                    f,
                    "year={year} month={month} day={day_of_month} dow={day_of_week} \
                     hour={hour} minute={minute} second={second} fault={} workday={} \
                     no-workday={} no-year={} no-date={} no-dow={} no-time={} \
                     summer-time={} synced={}",
                    bool_word(*fault),
                    bool_word(*working_day),
                    bool_word(*working_day_unknown),
                    bool_word(*year_invalid),
                    bool_word(*date_invalid),
                    bool_word(*day_of_week_invalid),
                    bool_word(*time_invalid),
                    bool_word(*summer_time),
                    bool_word(*externally_synchronized)
                )
            }
        }
    }
}

/// Everything the codec can refuse, instead of guessing (spec E4-D5). No
/// variant means "the codec invented an answer".
#[derive(Debug, Clone, PartialEq)]
pub enum DptCodecError {
    /// `dpt.main` is not one of the main types this slice implements —
    /// including `6.020`, whose `B5N3` layout is a different format from
    /// the rest of main type 6 and has no `DptValue` variant yet.
    UnsupportedDpt(DptRef),
    /// The payload's length does not match what `dpt` requires.
    /// `expected_bits`/`got` are the DPT's own significant-bit width and
    /// the payload's, e.g. `WrongLength { expected_bits: 8, got: 6 }` for
    /// an 8-bit DPT handed a `GroupValue::Short`.
    WrongLength {
        dpt: DptRef,
        expected_bits: u32,
        got: u32,
    },
    /// A parsed engineering value is outside `dpt`'s representable range.
    /// `value` is the offending value's own text form.
    OutOfRange { dpt: DptRef, value: String },
    /// `encode`'s text input is not a value of `dpt`.
    Unparsable { dpt: DptRef, input: String },
    /// The payload carries `dpt`'s own documented "this means invalid
    /// data" code (DPT-AS states one for some DPTs, e.g. 5.006's `255` or
    /// 8.010's `7FFFh`; see the per-type comments below for the citation).
    InvalidData { dpt: DptRef },
}

impl std::error::Error for DptCodecError {}

impl fmt::Display for DptCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DptCodecError::UnsupportedDpt(dpt) => write!(f, "unsupported datapoint type: {dpt}"),
            DptCodecError::WrongLength {
                dpt,
                expected_bits,
                got,
            } => write!(
                f,
                "{dpt}: wrong payload length: expected {expected_bits} bit(s), got {got}"
            ),
            DptCodecError::OutOfRange { dpt, value } => {
                write!(f, "{dpt}: value out of range: {value:?}")
            }
            DptCodecError::Unparsable { dpt, input } => {
                write!(f, "{dpt}: not a value of this datapoint type: {input:?}")
            }
            DptCodecError::InvalidData { dpt } => {
                write!(f, "{dpt}: payload is the DPT's own \"invalid data\" code")
            }
        }
    }
}

/// Decodes a wire payload into an engineering value.
///
/// Tolerance (documented once, applies to every ≤6-bit type below): the
/// Standard's inline six-bit `A_GroupValue_*`-PDU form (AL-AS §3.1.2/
/// §3.1.3: "Values that only consist of 6 bits or less have the following
/// optimized [...] PDU format") is what `encode_group_value` in
/// `knx-net::cemi` actually produces as `GroupValue::Short`. Some payloads
/// this codec is handed did not come from that path (hand-built test
/// fixtures, a future manufacturer product database, ...), so `decode`
/// also accepts a single-octet `GroupValue::Bytes([v])` for a DPT of ≤6
/// bits and treats it the same as `Short(v)`. `encode` does not do the
/// reverse: it always produces the form the Standard prescribes.
pub fn decode(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    match dpt.main {
        1 => decode_b1(dpt, payload),
        2 => decode_b2(dpt, payload),
        3 => decode_b1u3(dpt, payload),
        4 => decode_a8(dpt, payload),
        5 => decode_u8(dpt, payload),
        6 => decode_v8(dpt, payload),
        7 => decode_u16(dpt, payload),
        8 => decode_v16(dpt, payload),
        9 => decode_f16(dpt, payload),
        10 => decode_time_of_day(dpt, payload),
        11 => decode_date(dpt, payload),
        12 => decode_u32(dpt, payload),
        13 => decode_v32(dpt, payload),
        14 => decode_f32(dpt, payload),
        15 => decode_access_data(dpt, payload),
        16 => decode_a14(dpt, payload),
        17 => decode_scene(dpt, payload),
        18 => decode_scene_control(dpt, payload),
        19 => decode_datetime(dpt, payload),
        _ => Err(DptCodecError::UnsupportedDpt(dpt)),
    }
}

/// Encodes engineering-value text into a wire payload. See each `encode_*`
/// helper for that main type's text grammar; grammars not dictated by the
/// Standard (which specifies wire encoding, not human text) are this
/// module's own design choice, documented at the helper.
pub fn encode(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    match dpt.main {
        1 => encode_b1(dpt, input),
        2 => encode_b2(dpt, input),
        3 => encode_b1u3(dpt, input),
        4 => encode_a8(dpt, input),
        5 => encode_u8(dpt, input),
        6 => encode_v8(dpt, input),
        7 => encode_u16(dpt, input),
        8 => encode_v16(dpt, input),
        9 => encode_f16(dpt, input),
        10 => encode_time_of_day(dpt, input),
        11 => encode_date(dpt, input),
        12 => encode_u32(dpt, input),
        13 => encode_v32(dpt, input),
        14 => encode_f32(dpt, input),
        15 => encode_access_data(dpt, input),
        16 => encode_a14(dpt, input),
        17 => encode_scene(dpt, input),
        18 => encode_scene_control(dpt, input),
        19 => encode_datetime(dpt, input),
        _ => Err(DptCodecError::UnsupportedDpt(dpt)),
    }
}

// ---------------------------------------------------------------------
// Shared payload helpers
// ---------------------------------------------------------------------

/// Extracts the raw value of a ≤6-bit DPT from either payload form (see
/// `decode`'s doc comment), and checks DPT-AS §1.3.1's "Datapoint Types
/// shorter than 1 octet are transmitted in the data-field of the frame on
/// the lower bit positions. The preceding bits shall be 0." A payload that
/// violates that — bits set above `width_bits` — is not a legal value of
/// this DPT, so it is `InvalidData`, not silently masked.
fn require_short(payload: &GroupValue, dpt: DptRef, width_bits: u32) -> Result<u8, DptCodecError> {
    let raw = match payload {
        GroupValue::Short(v) => *v,
        GroupValue::Bytes(b) if b.len() == 1 => b[0],
        GroupValue::Bytes(b) => {
            return Err(DptCodecError::WrongLength {
                dpt,
                expected_bits: width_bits,
                got: (b.len() * 8) as u32,
            })
        }
    };
    let mask = 0xFFu8 << width_bits;
    if raw & mask != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    Ok(raw)
}

/// Extracts exactly `N` octets for a DPT wider than 6 bits. A
/// `GroupValue::Short` never legally carries one of these (AL-AS §3.1.3
/// puts anything over 6 bits in its own octet(s)), so it is `WrongLength`,
/// same as a `Bytes` payload of the wrong count.
fn require_bytes<const N: usize>(
    payload: &GroupValue,
    dpt: DptRef,
    expected_bits: u32,
) -> Result<[u8; N], DptCodecError> {
    match payload {
        GroupValue::Bytes(b) if b.len() == N => {
            let mut out = [0u8; N];
            out.copy_from_slice(b);
            Ok(out)
        }
        GroupValue::Short(_) => Err(DptCodecError::WrongLength {
            dpt,
            expected_bits,
            got: 6,
        }),
        GroupValue::Bytes(b) => Err(DptCodecError::WrongLength {
            dpt,
            expected_bits,
            got: (b.len() * 8) as u32,
        }),
    }
}

/// Parses `on`/`off`/`true`/`false`/`1`/`0`, case-insensitively, as the
/// brief requires for every boolean-carrying field in this slice (B1's own
/// bit, and B2/B1U3's control bit).
fn parse_bool_word(word: &str) -> Option<bool> {
    match word.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => Some(true),
        "off" | "false" | "0" => Some(false),
        _ => None,
    }
}

/// Renders a boolean flag field as `on`/`off`, for `Display` impls that
/// need the same word `parse_bool_word` accepts back (main types 15, 19).
fn bool_word(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}

/// Weekday name for main type 10's `Day` field (DPT-AS §3.11's table:
/// `1 = Monday ... 7 = Sunday`). Only ever called with `1..=7`, which
/// `decode_time_of_day`/`encode_time_of_day` both guarantee.
fn weekday_name(day: u8) -> &'static str {
    match day {
        1 => "monday",
        2 => "tuesday",
        3 => "wednesday",
        4 => "thursday",
        5 => "friday",
        6 => "saturday",
        7 => "sunday",
        _ => unreachable!("weekday_name is only called with a validated 1..=7 day code"),
    }
}

/// The inverse of `weekday_name`, case-insensitive.
fn weekday_from_name(word: &str) -> Option<u8> {
    match word.to_ascii_lowercase().as_str() {
        "monday" => Some(1),
        "tuesday" => Some(2),
        "wednesday" => Some(3),
        "thursday" => Some(4),
        "friday" => Some(5),
        "saturday" => Some(6),
        "sunday" => Some(7),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Main type 1 — B1 (DPT-AS §3.1)
// ---------------------------------------------------------------------
//
// Subtype effect: none. §3.1's own note: "these single bit DPTs are
// defined in a most generic way ... though keeping a common
// interpretation" — every 1.xxx subtype shares the identical 1-bit
// encoding; the subnumber only changes which pair of words (Off/On,
// False/True, Up/Down, ...) is attached to 0/1. `DptValue::Bool` always
// renders as on/off (see `Display`), which is why 1.008's Up/Down is not
// modelled here — see the brief.

fn decode_b1(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_short(payload, dpt, 1)?;
    Ok(DptValue::Bool(raw == 1))
}

fn encode_b1(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let value = parse_bool_word(input.trim()).ok_or_else(|| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    Ok(GroupValue::Short(u8::from(value)))
}

// ---------------------------------------------------------------------
// Main type 2 — B2 (DPT-AS §3.2)
// ---------------------------------------------------------------------
//
// Format: octet holds `c` (bit 1, the control flag) then `v` (bit 0), both
// single bits (DPT-AS §3.2 table: field names `c v`, encoding `B B`).
// `c=0` is "no control" (`v` don't-care on the wire; this codec still
// decodes whatever bit is there, since the Standard does not say the
// receiver must ignore it). `c=1`, `v` carries the value "according to
// Type 1.xxx" — which 1.xxx subtype is a functional-block concern this
// codec does not resolve, so `format`/`Display` always print on/off, like
// main type 1.
//
// Text grammar for `encode` (not specified by the Standard, which only
// defines the wire form — this module's own choice): `no control`,
// `control on`, `control off`, case-insensitively.

fn decode_b2(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_short(payload, dpt, 2)?;
    Ok(DptValue::ControlBool {
        control: raw & 0b10 != 0,
        value: raw & 0b01 != 0,
    })
}

fn encode_b2(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let normalized = input.trim().to_ascii_lowercase();
    let (control, value) = match normalized.as_str() {
        "no control" => (false, false),
        "control on" => (true, true),
        "control off" => (true, false),
        _ => return Err(unparsable()),
    };
    Ok(GroupValue::Short(
        (u8::from(control) << 1) | u8::from(value),
    ))
}

// ---------------------------------------------------------------------
// Main type 3 — B1U3 (DPT-AS §3.3, §3.3.1/§3.3.2)
// ---------------------------------------------------------------------
//
// Format: bit 3 (msb of the 4 significant bits) is `c`, bits 2-0 are the
// 3-bit `StepCode` (DPT-AS §3.3.2 table: field names `c StepCode`,
// encoding `B U U U`). `StepCode = 000b` is the dedicated "Break" code;
// `001b..111b` denote an interval count of `2^(StepCode-1)` — this codec
// stores the raw code (see `DptValue::Step`'s doc comment), not the
// derived count. 3.007 names `c` Increase/Decrease, 3.008 names it
// Up/Down (both "See 1.007"/"See 1.008" in their own clause); this slice
// always uses the generic Increase/Decrease wording, per the brief.
//
// Text grammar for `encode` (this module's own choice): `increase break`,
// `decrease break`, or `increase step <1..=7>` / `decrease step <1..=7>`,
// case-insensitive on the words.

fn decode_b1u3(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_short(payload, dpt, 4)?;
    Ok(DptValue::Step {
        increase: raw & 0b1000 != 0,
        step_code: raw & 0b0111,
    })
}

fn encode_b1u3(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let increase = match tokens.first() {
        Some(w) if w.eq_ignore_ascii_case("increase") => true,
        Some(w) if w.eq_ignore_ascii_case("decrease") => false,
        _ => return Err(unparsable()),
    };
    let step_code: u8 = match tokens.get(1..) {
        Some([w]) if w.eq_ignore_ascii_case("break") => 0,
        Some([w, n]) if w.eq_ignore_ascii_case("step") => {
            let n: u8 = n.parse().map_err(|_| unparsable())?;
            if !(1..=7).contains(&n) {
                return Err(DptCodecError::OutOfRange {
                    dpt,
                    value: n.to_string(),
                });
            }
            n
        }
        _ => return Err(unparsable()),
    };
    Ok(GroupValue::Short((u8::from(increase) << 3) | step_code))
}

// ---------------------------------------------------------------------
// Main type 4 — A8, single 8-bit character (DPT-AS §3.4)
// ---------------------------------------------------------------------
//
// One octet holding exactly one character, from either 4.001's 7-bit
// ASCII table (`[0...127]`, "the most significant bit shall always be
// 0") or 4.002's ISO 8859-1 table (`[0...255]`) — the same two character
// sets main type 16's A[14] string uses, reused verbatim through
// `char_set_is_ascii` (see that function's comment for the main-type-4
// arm and the deliberate non-extension of main type 16's bare-subtype
// default). Both subtypes occupy a full octet — 4.001's 7 significant
// bits already exceed AL-AS's ≤6-bit inline threshold — so, like main
// type 5's U8, this is always `GroupValue::Bytes([_])`, never `Short`.
//
// `DptValue::Text` (a `String`) is reused rather than adding a dedicated
// one-character variant — see that variant's doc comment. Control
// characters `00h`-`1Fh` decode unconditionally: DPT-AS §3.4's "Decoding
// of 00h to 1Fh" note ("The support of the control characters... is not
// mandatory. The receiver shall not react...") is a rule for a *receiving
// bus device* deciding whether to act on the character, not for this
// codec's decoding — it is not the "receiver" the Standard means there,
// so it reports every code point, control or not.
//
// Reserved-bit policy: a 4.001 byte with its top bit set violates that
// subtype's own "most significant bit shall always be 0" rule, so it is
// `InvalidData`, the same policy `require_short`/main type 16 apply to
// an equivalent violation elsewhere in this module.

fn decode_a8(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let ascii_only = char_set_is_ascii(dpt)?;
    let [raw] = require_bytes::<1>(payload, dpt, 8)?;
    if ascii_only && raw & 0x80 != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    Ok(DptValue::Text(char::from(raw).to_string()))
}

fn encode_a8(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let ascii_only = char_set_is_ascii(dpt)?;
    let mut chars = input.chars();
    let ch = chars.next().ok_or_else(unparsable)?;
    if chars.next().is_some() {
        // More than one character — not a value of this single-character
        // DPT (use main type 16's A[14] for strings).
        return Err(unparsable());
    }
    let code = ch as u32;
    let limit: u32 = if ascii_only { 0x7F } else { 0xFF };
    if code > limit {
        return Err(unparsable());
    }
    Ok(GroupValue::Bytes(vec![code as u8]))
}

// ---------------------------------------------------------------------
// Main type 5 — U8 (DPT-AS §3.5)
// ---------------------------------------------------------------------
//
// Subtype effect: unlike main type 1, the subtype *does* change the
// arithmetic here — DPT-AS splits main type 5 into §3.5.1 "Scaled values"
// (5.001 DPT_Scaling, 5.003 DPT_Angle) and §3.5.2 "Non-scaled values"
// (5.004, 5.006, 5.010, and by the shared preamble's identity default,
// every other subtype including `sub: None`). This codec branches on the
// subtype for exactly 5.001 and 5.003, and for no others — see below for
// why 5.005 is *not* a third scaled case despite sitting in §3.5.1's
// clause.

fn decode_u8(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [raw] = require_bytes(payload, dpt, 8)?;
    match dpt.sub {
        // 5.001 DPT_Scaling. Its home section is §3.5.1, which states the
        // range and gives the worked example this code is checked against
        // (NOTE 4): 50 % -> 80h, 100 % -> FFh. The formula itself is
        // spelled out in a sentence only once in the whole document, and
        // not there: DPT-AS §3.31.A, a cross-reference subsection under
        // DPT_Angle/DPT_Percent_U8, says "Encoding: 0 %...100 %. Full
        // Datapoint Type value: 0...255, i.e. 1 % = value 255/100". Both
        // are cited on purpose — do not go to §3.31 expecting 5.001's own
        // definition to live there.
        Some(1) => Ok(DptValue::Float(f64::from(raw) * 100.0 / 255.0)),
        // 5.003 DPT_Angle: DPT-AS §3.5.1 states the range ([0...360]) and
        // resolution ("≈ 1,4°") but, unlike 5.001, does not spell out the
        // raw<->engineering formula in a sentence. This codec uses the
        // same linear 0..255 -> 0..360 mapping by direct analogy with
        // 5.001 (both share §3.5.1's "Scaled values" format block); this
        // is an inference from the stated range, not a formula quoted
        // from the text — flagged for the reviewer in the task report.
        Some(3) => Ok(DptValue::Float(f64::from(raw) * 360.0 / 255.0)),
        // 5.006 DPT_Tariff (DPT-AS §3.5.2.2): 0..254 is the current/
        // desired tariff, identity. 255 is reserved: "shall not be used
        // ... On reception, the message with this value shall be
        // ignored" — this codec reports it as `InvalidData` rather than
        // decoding a tariff value that the Standard says must be ignored.
        Some(6) if raw == 255 => Err(DptCodecError::InvalidData { dpt }),
        // Every other subtype (5.004 DPT_Percent_U8, 5.006 in range,
        // 5.010 DPT_Value_1_Ucount, `sub: None`, and any subtype this
        // corpus does not name) is raw U8 identity. This also covers
        // 5.005 DPT_DecimalFactor: its Range/Resolution cells are
        // genuinely blank in the published document (confirmed against
        // the source PDF, not an extraction artifact) — there is no
        // formula to read or infer, so it takes main type 5's plain
        // identity form rather than a guessed scale factor.
        _ => Ok(DptValue::Unsigned(u32::from(raw))),
    }
}

fn encode_u8(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let out_of_range = |value: &str| DptCodecError::OutOfRange {
        dpt,
        value: value.to_string(),
    };
    let trimmed = input.trim();
    let raw: u8 = match dpt.sub {
        Some(1) => {
            let pct: f64 = trimmed.parse().map_err(|_| unparsable())?;
            if !(0.0..=100.0).contains(&pct) {
                return Err(out_of_range(trimmed));
            }
            (pct * 255.0 / 100.0).round() as u8
        }
        Some(3) => {
            let deg: f64 = trimmed.parse().map_err(|_| unparsable())?;
            if !(0.0..=360.0).contains(&deg) {
                return Err(out_of_range(trimmed));
            }
            (deg * 255.0 / 360.0).round() as u8
        }
        Some(6) => {
            let v: u32 = trimmed.parse().map_err(|_| unparsable())?;
            if v > 254 {
                // 254 is the top of the *usable* range; 255 is the
                // reserved "shall not be transmitted" code (DPT-AS
                // §3.5.2.2), and anything above 255 does not fit a U8.
                return Err(out_of_range(trimmed));
            }
            v as u8
        }
        _ => {
            let v: u32 = trimmed.parse().map_err(|_| unparsable())?;
            if v > 255 {
                return Err(out_of_range(trimmed));
            }
            v as u8
        }
    };
    Ok(GroupValue::Bytes(vec![raw]))
}

// ---------------------------------------------------------------------
// Main type 6 — V8 / "Status with Mode" (DPT-AS §3.6, §3.7)
// ---------------------------------------------------------------------
//
// 6.001/6.010 and every other subtype except 6.020 are plain 8-bit two's
// complement (DPT-AS §1.3.1's `V` notation; range confirmed at §3.6.1:
// "-128...127"). 6.020 DPT_Status_Mode3 is a structurally different
// format under the same main number — `B5N3`, 5 status bits (A..E,
// 0=set/1=clear) plus a 3-bit one-hot mode field (DPT-AS §3.7:
// `001b`/`010b`/`100b` = mode 0/1/2 active; no other code is assigned).
// `DptValue` has no variant for that shape, so 6.020 is `UnsupportedDpt`
// here rather than being squeezed into `Signed` and silently misread as a
// two's-complement number it is not.

fn decode_v8(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    if dpt.sub == Some(20) {
        return Err(DptCodecError::UnsupportedDpt(dpt));
    }
    let [raw] = require_bytes(payload, dpt, 8)?;
    Ok(DptValue::Signed(i32::from(raw as i8)))
}

fn encode_v8(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    if dpt.sub == Some(20) {
        return Err(DptCodecError::UnsupportedDpt(dpt));
    }
    let trimmed = input.trim();
    let v: i32 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    if !(-128..=127).contains(&v) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: trimmed.to_string(),
        });
    }
    Ok(GroupValue::Bytes(vec![(v as i8) as u8]))
}

// ---------------------------------------------------------------------
// Main type 7 — U16 (DPT-AS §3.8)
// ---------------------------------------------------------------------
//
// All 7.xxx subtypes share raw `U16` identity encoding (DPT-AS §3.8.3
// table: `octet nr 2 MSB 1 LSB`, "Binary encoded value", range
// `[0...65535]`); the subnumber only changes the unit and, for the time-
// period family (7.002-7.007), a fixed display multiplier this codec does
// not apply (it decodes the raw counter, not the multiplied duration —
// that is a unit concern, out of this slice's scope per the design doc).
// 7.012's raw `0` has a documented special *meaning* ("no bus power
// supply functionality available", DPT-AS §3.8.3) but is not flagged as
// invalid data by the Standard, so it decodes as an ordinary `Unsigned(0)`.

fn decode_u16(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_bytes(payload, dpt, 16)?;
    Ok(DptValue::Unsigned(u32::from(u16::from_be_bytes(raw))))
}

fn encode_u16(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let v: u32 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    if v > u32::from(u16::MAX) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: trimmed.to_string(),
        });
    }
    Ok(GroupValue::Bytes((v as u16).to_be_bytes().to_vec()))
}

// ---------------------------------------------------------------------
// Main type 8 — V16 (DPT-AS §3.9)
// ---------------------------------------------------------------------
//
// 8.001 DPT_Value_2_Count and every subtype except 8.010 are plain 16-bit
// two's complement (DPT-AS §3.9.1/§3.9.3, range `[-32768...32767]`).
//
// DPT-AS §3.9.1 footnote a), printed directly under the 8.001 row, reads:
// "Only for DPT_Value_2_Ucount, the value 7FFFh can be used to denote
// invalid data." "DPT_Value_2_Ucount" is main type **7**'s name (7.001),
// not 8.001's own name ("DPT_Value_2_Count") — confirmed verbatim against
// the source PDF, so this is a genuine error in the published standard,
// not extraction damage. Because the footnote names a different
// datapoint type than the row it annotates, this codec does not treat
// `7FFFh` as invalid data for 8.001: it decodes as a plain two's-
// complement integer across its whole range, including `32767`. Acting on
// an ambiguous sentinel would turn a legitimate reading of 32767 into a
// spurious error, which is the more expensive direction to be wrong in.
//
// 8.010 DPT_Percent_V16 has its own footnote b), unambiguously named:
// "For DPT_Percent_V16, the value 7FFFh shall be used to denote invalid
// data." Resolution is 0,01 % (DPT-AS §3.9.1 table), so `raw = round(pct
// * 100)`, `pct = raw / 100`. The Standard's own stated range for 8.010,
// "-327,68 % ... 327,67 %", collides with this: 327.67 % is exactly
// `raw = 32767 = 7FFFh`, the same code the Standard just said means
// "invalid data" — the document's stated maximum is its own sentinel.
// This codec resolves that by treating `7FFFh` as `InvalidData` (the
// unambiguous rule) and therefore never *producing* it from `encode`:
// 327.67 % is rejected as out of range, and the practical encodable/
// decodable maximum is 327.66 %. This is a real contradiction in the
// Standard's own table, not a rounding choice — flagged for the reviewer.

fn decode_v16(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw_bytes = require_bytes(payload, dpt, 16)?;
    let raw = i16::from_be_bytes(raw_bytes);
    if dpt.sub == Some(10) {
        if raw == i16::MAX {
            return Err(DptCodecError::InvalidData { dpt });
        }
        return Ok(DptValue::Float(f64::from(raw) / 100.0));
    }
    Ok(DptValue::Signed(i32::from(raw)))
}

fn encode_v16(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let trimmed = input.trim();
    let raw: i16 = if dpt.sub == Some(10) {
        let pct: f64 = trimmed.parse().map_err(|_| unparsable())?;
        // 327.67 is deliberately excluded — see the module-level comment
        // above: it is the Standard's own "invalid data" code.
        if !(-327.68..327.67).contains(&pct) {
            return Err(DptCodecError::OutOfRange {
                dpt,
                value: trimmed.to_string(),
            });
        }
        (pct * 100.0).round() as i16
    } else {
        let v: i32 = trimmed.parse().map_err(|_| unparsable())?;
        if !(i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&v) {
            return Err(DptCodecError::OutOfRange {
                dpt,
                value: trimmed.to_string(),
            });
        }
        v as i16
    };
    Ok(GroupValue::Bytes(raw.to_be_bytes().to_vec()))
}

// ---------------------------------------------------------------------
// Main type 9 — F16, KNX-proprietary 2-octet float (DPT-AS §3.10)
// ---------------------------------------------------------------------
//
// Not IEEE 754 (that is main type 14 — see below, and DPT-AS says so
// explicitly there). Format: `FloatValue = (0,01*M)*2^E`, encoding string
// `M E E E E M M M M M M M M M M M` (DPT-AS §3.10, confirmed against the
// source PDF): 16 bits total, but the mantissa `M` is *not* "1 sign bit +
// 11 magnitude bits" — it is a single 12-bit two's-complement field whose
// sign bit (bit 15) sits *before* the 4 exponent bits, with its other 11
// bits (bits 10..0) sitting *after* them. `M = [-2 048...2 047]`
// (DPT-AS §3.10) is exactly the range of a 12-bit two's-complement value.
// Reconstructing M as "sign bit, then negate an 11-bit magnitude" is
// wrong for almost every negative value (it reads -1 as -2047 instead of
// -1, for instance) — pinned by a test below.
//
// `E`'s upper bound: DPT-AS §3.10 states the formula's exponent range as
// `E = [0...15]` in so many words, then prints a *practical* range figure
// of `[-671 088,64 ... 670 433,28]`. That figure is not what "E capped at
// 14" would give (2047*0,01*2^14 = 335 380,48) — it is what `E=15,
// M=2046` gives (2046*0,01*2^15 = 670 433,28): one mantissa step below
// the arithmetic maximum at the top exponent, i.e. DPT-AS's own printed
// range already excludes the one raw value that collides with the
// invalid-data sentinel (see below). Application Note AN188 §4, restating
// the identical format/formula for the same F16 base type (in the course
// of adding 9.031 DPT_Coefficient), prints `[-671 088,64 ... 670 760,96]`
// instead — the true arithmetic maximum at `E=15, M=2047`
// (2047*0,01*2^15 = 670 760,96), sentinel collision included. Both
// figures were checked against the original PDFs, not just the Markdown
// extraction. AN188 is not even consistent with itself: its own §5 table
// (describing an HVAC compound datapoint's F16 fields, not a formal DPT
// range entry) reprints DPT-AS's smaller 670 433,28 figure a few pages
// after asserting 670 760,96 in §4. Ruling (brief, not relitigated here):
// use the formula as both documents state it — `E ∈ [0, 15]` — since nothing
// in either document actually narrows the formula itself; the sentinel
// collision at the top of that range is handled on its own terms below,
// not by pretending E stops at 14.
//
// Invalid data: DPT-AS §3.10, "For all Datapoint Types 9.xxx, the
// encoded value 7FFFh shall always be used to denote invalid data." This
// names the whole 9.xxx family unambiguously, unlike main type 8's
// misattributed footnote — decode honours it before applying the
// formula. It also happens to be the *only* bit pattern that would
// otherwise decode to the formula's arithmetic maximum (E=15, M=2047 ->
// 670 760,96, see above): encode therefore cannot produce 670 760,96
// either, and rejects it as out of range rather than silently emitting
// the sentinel for what looks like a legitimate reading — the same kind
// of range/sentinel collision main type 8's 8.010 has, flagged again
// here for the reviewer.
//
// Rounding tie-break for encode's mantissa: this codec rounds half away
// from zero (`f64::round`), the same choice already made for 5.001/
// 5.003/8.010. DPT-AS states no tie-breaking rule at all for main type 9
// — this is this module's own implementation choice, not a cited one.

fn decode_f16(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw_bytes = require_bytes::<2>(payload, dpt, 16)?;
    let raw = u16::from_be_bytes(raw_bytes);
    if raw == 0x7FFF {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let sign_bit = (raw >> 15) & 1;
    let exponent = (raw >> 11) & 0xF;
    let mantissa_low = raw & 0x7FF;
    // Reassemble the 12-bit two's-complement mantissa field (sign bit,
    // then the 11 bits that sat on the other side of the exponent) and
    // sign-extend it — see the module comment above for why this is not
    // "sign bit plus 11-bit magnitude".
    let m12 = (sign_bit << 11) | mantissa_low;
    let mantissa: i32 = if m12 & 0x800 != 0 {
        i32::from(m12) - 4096
    } else {
        i32::from(m12)
    };
    let value = 0.01 * f64::from(mantissa) * 2f64.powi(i32::from(exponent));
    Ok(DptValue::Float(value))
}

fn encode_f16(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let value: f64 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    let out_of_range = || DptCodecError::OutOfRange {
        dpt,
        value: trimmed.to_string(),
    };
    if !value.is_finite() {
        return Err(out_of_range());
    }
    // Smallest E gives the finest resolution; try E=0 upward and stop at
    // the first exponent whose rounded mantissa fits [-2048, 2047].
    for exponent in 0u16..=15 {
        let scale = 0.01 * 2f64.powi(i32::from(exponent));
        let mantissa = (value / scale).round();
        if !(-2048.0..=2047.0).contains(&mantissa) {
            continue;
        }
        let mantissa = mantissa as i32;
        // E=15, M=2047 is the reserved "invalid data" code (0x7FFF) — see
        // the module comment above. No larger E exists to retry with, so
        // this value cannot be represented.
        if exponent == 15 && mantissa == 2047 {
            return Err(out_of_range());
        }
        let m12 = (mantissa & 0xFFF) as u16;
        let sign_bit = (m12 >> 11) & 1;
        let mantissa_low = m12 & 0x7FF;
        let raw = (sign_bit << 15) | (exponent << 11) | mantissa_low;
        return Ok(GroupValue::Bytes(raw.to_be_bytes().to_vec()));
    }
    Err(out_of_range())
}

// ---------------------------------------------------------------------
// Main type 10 — N3U5r2U6r2U6, time of day (DPT-AS §3.11)
// ---------------------------------------------------------------------
//
// 3 octets. Octet 3 (MSB): `Day` (3 bits, `N`) then `Hour` (5 bits, `U`,
// `[0...23]`). Octet 2: 2 reserved bits then `Minutes` (6 bits, `U`,
// `[0...59]`). Octet 1: 2 reserved bits then `Seconds` (6 bits, `U`,
// `[0...59]`) — DPT-AS §3.11's own diagram and 10.001 DPT_TimeOfDay's
// field table.
//
// `Day`'s own table ([D], DPT-AS §3.11, page 41, 10.001's Day column):
// `1 = Monday ... 7 = Sunday`, `0 = no day`. Only the storage shape is
// this codec's own choice ([A]) — the Standard names the code, not a
// representation: this codec represents that as `Option<u8>` —
// `Some(1..=7)` for a named weekday, `None` for wire code `0` — rather
// than keeping the raw `0..=7` code, so a caller cannot mistake "no day"
// for an eighth weekday by forgetting to special-case `0`. Round trip is
// exact: `None` always encodes back to `0`, and `0` always decodes to
// `None`; no other code maps to `None`, so no information is lost either
// direction.
//
// Reserved-bit policy: the 2 top bits of octets 2 and 1 are `r` in the
// diagram; nonzero is `InvalidData`, the same policy this module applies
// everywhere else a diagram marks a bit `r`.
//
// Text grammar for `encode` (the Standard specifies wire encoding only,
// not human text — this module's own choice): `"<weekday|none>
// HH:MM:SS"`, e.g. `"monday 07:30:00"`, `"none 00:00:00"`; weekday names
// are lower-case, matching `Display`'s own rendering, so `Display`'s
// output always re-parses.

fn decode_time_of_day(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [b2, b1, b0] = require_bytes::<3>(payload, dpt, 24)?;
    let day_code = b2 >> 5;
    let hour = b2 & 0b0001_1111;
    if hour > 23 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    if b1 & 0b1100_0000 != 0 || b0 & 0b1100_0000 != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let minute = b1 & 0b0011_1111;
    if minute > 59 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let second = b0 & 0b0011_1111;
    if second > 59 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let day = if day_code == 0 { None } else { Some(day_code) };
    Ok(DptValue::TimeOfDay {
        day,
        hour,
        minute,
        second,
    })
}

fn encode_time_of_day(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let (day_tok, time_tok) = match tokens.as_slice() {
        [d, t] => (*d, *t),
        _ => return Err(unparsable()),
    };
    let day_code: u8 = if day_tok.eq_ignore_ascii_case("none") {
        0
    } else {
        weekday_from_name(day_tok).ok_or_else(unparsable)?
    };
    let parts: Vec<&str> = time_tok.split(':').collect();
    let (h, m, s) = match parts.as_slice() {
        [h, m, s] => (*h, *m, *s),
        _ => return Err(unparsable()),
    };
    let hour: u8 = h.parse().map_err(|_| unparsable())?;
    let minute: u8 = m.parse().map_err(|_| unparsable())?;
    let second: u8 = s.parse().map_err(|_| unparsable())?;
    if hour > 23 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: hour.to_string(),
        });
    }
    if minute > 59 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: minute.to_string(),
        });
    }
    if second > 59 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: second.to_string(),
        });
    }
    let b2 = (day_code << 5) | hour;
    Ok(GroupValue::Bytes(vec![b2, minute, second]))
}

// ---------------------------------------------------------------------
// Main type 11 — r3U5r4U4r1U7, date (DPT-AS §3.12)
// ---------------------------------------------------------------------
//
// 3 octets. Octet 3 (MSB): 3 reserved bits then `Day` (5 bits, `U`,
// `[1...31]`). Octet 2: 4 reserved bits then `Month` (4 bits, `U`,
// `[1...12]`). Octet 1: 1 reserved bit then `Year` (7 bits, `U`,
// `[0...99]`) — DPT-AS §3.12's diagram and 11.001 DPT_Date's field table.
//
// Century Encoding (DPT-AS §3.12, quoted in full because getting the
// boundary wrong by one is exactly the kind of bug this comment exists
// to prevent): "if Octet 3 [this codec's Year octet] contains value ≥ 90:
// interpret as 20th century[;] if Octet 3 contains value < 90: interpret
// as 21st century. This format covers the range 1990 to 2089." EXAMPLE 5
// gives the worked values this codec's test suite checks against: `99d
// equals 1999`, `0d equals 2000`, `4d equals 2004`. `DptValue::Date.year`
// stores the resolved 4-digit year (`1990..=2089`), not the raw 7-bit
// octet, since the resolution is a fact about the wire value, not a
// display choice — same reasoning as main type 5.001's percent, main
// type 11's own consumer should never have to redo century arithmetic.
//
// Reserved-bit policy: the `r` bits above `Day`, above `Month`, and above
// `Year` are all checked to be 0, `InvalidData` otherwise — same policy
// as the rest of this module. A `Year` octet's raw value `100..=127` is
// numerically representable in 7 bits but outside `[0...99]`'s
// documented range and outside the century rule's own domain, so it too
// is `InvalidData`, not silently resolved to some invented 22nd year.
//
// Text grammar for `encode` (this module's own choice, the Standard only
// defines the wire form): `"YYYY-MM-DD"`, matching `Display`'s own
// rendering.

fn decode_date(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [b2, b1, b0] = require_bytes::<3>(payload, dpt, 24)?;
    if b2 & 0b1110_0000 != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let day = b2 & 0b0001_1111;
    if !(1..=31).contains(&day) {
        return Err(DptCodecError::InvalidData { dpt });
    }
    if b1 & 0b1111_0000 != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let month = b1 & 0b0000_1111;
    if !(1..=12).contains(&month) {
        return Err(DptCodecError::InvalidData { dpt });
    }
    if b0 & 0b1000_0000 != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let year_raw = b0 & 0b0111_1111;
    if year_raw > 99 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    // DPT-AS §3.12 "Century Encoding" — see the section comment above.
    let year: u16 = if year_raw >= 90 {
        1900 + u16::from(year_raw)
    } else {
        2000 + u16::from(year_raw)
    };
    Ok(DptValue::Date { day, month, year })
}

fn encode_date(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let parts: Vec<&str> = input.trim().split('-').collect();
    let (y, m, d) = match parts.as_slice() {
        [y, m, d] => (*y, *m, *d),
        _ => return Err(unparsable()),
    };
    let year: u16 = y.parse().map_err(|_| unparsable())?;
    let month: u8 = m.parse().map_err(|_| unparsable())?;
    let day: u8 = d.parse().map_err(|_| unparsable())?;
    if !(1990..=2089).contains(&year) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: year.to_string(),
        });
    }
    if !(1..=12).contains(&month) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: month.to_string(),
        });
    }
    if !(1..=31).contains(&day) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: day.to_string(),
        });
    }
    // Inverse of decode_date's century rule: 1990-1999 -> 90-99,
    // 2000-2089 -> 0-89.
    let year_raw: u8 = if year >= 2000 {
        (year - 2000) as u8
    } else {
        (year - 1900) as u8
    };
    Ok(GroupValue::Bytes(vec![day, month, year_raw]))
}

// ---------------------------------------------------------------------
// Main type 12 — U32 (DPT-AS §3.13)
// ---------------------------------------------------------------------
//
// 4 octets, plain unsigned binary, octet 4 = MSB ... octet 1 = LSB
// (DPT-AS §3.13.1 table: `octet nr 4 MSB 3 2 1 LSB`, "Binary encoded").
// Every 12.xxx subtype (12.001 counter pulses; 12.100/101/102 operating
// hours in s/min/h — DPT-AS §3.13.2) shares this identity encoding; the
// subnumber only changes the unit and a usage constraint ("shall only be
// used if DPT_LongDeltaTimeSec (13.100) is also implemented", §3.13.2),
// not the bits. No range/reserved value beyond U32's own
// `[0...4294967295]` is documented for main type 12.

fn decode_u32(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_bytes::<4>(payload, dpt, 32)?;
    Ok(DptValue::Unsigned(u32::from_be_bytes(raw)))
}

fn encode_u32(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let v: u64 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    if v > u64::from(u32::MAX) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: trimmed.to_string(),
        });
    }
    Ok(GroupValue::Bytes((v as u32).to_be_bytes().to_vec()))
}

// ---------------------------------------------------------------------
// Main type 13 — V32 (DPT-AS §3.14)
// ---------------------------------------------------------------------
//
// 4 octets, two's complement, octet 4 = MSB ... octet 1 = LSB (DPT-AS
// §3.14.1 table). 13.001 (counter pulses, §3.14.1) and the electrical-
// energy family 13.010-13.016 (Wh/VAh/VARh/kWh/kVAh/kVARh/MWh, §3.14.2)
// are all identity, resolution = 1 of the named unit. 13.100
// DPT_LongDeltaTimeSec (§3.14.3) is likewise identity, resolution 1 s;
// its Markdown extraction is an image placeholder with no body — its
// content (format `V32`, range `[-2 147 483 648 s...2 147 483 647 s]`,
// "shall be used for operating hours") was confirmed complete in the
// source PDF, so it is cited here as `§3.14.3` even though the body used
// to write this comment came from the PDF, not the Markdown corpus.
//
// The one subtype that is *not* identity: 13.002 DPT_FlowRate_m3/h has
// resolution `0,0001 m3/h` (DPT-AS §3.14.1 table) — a fixed-point scale
// factor, the main-type-13 analogue of main type 5's scaled subtypes.
// This codec branches on `sub == Some(2)` for exactly that subtype.

fn decode_v32(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_bytes::<4>(payload, dpt, 32)?;
    let v = i32::from_be_bytes(raw);
    if dpt.sub == Some(2) {
        Ok(DptValue::Float(f64::from(v) * 0.0001))
    } else {
        Ok(DptValue::Signed(v))
    }
}

fn encode_v32(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let out_of_range = |value: &str| DptCodecError::OutOfRange {
        dpt,
        value: value.to_string(),
    };
    let trimmed = input.trim();
    let raw: i32 = if dpt.sub == Some(2) {
        let flow: f64 = trimmed.parse().map_err(|_| unparsable())?;
        let scaled = (flow / 0.0001).round();
        if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&scaled) {
            return Err(out_of_range(trimmed));
        }
        scaled as i32
    } else {
        let v: i64 = trimmed.parse().map_err(|_| unparsable())?;
        if !(i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&v) {
            return Err(out_of_range(trimmed));
        }
        v as i32
    };
    Ok(GroupValue::Bytes(raw.to_be_bytes().to_vec()))
}

// ---------------------------------------------------------------------
// Main type 14 — F32, IEEE 754 single precision (DPT-AS §3.15)
// ---------------------------------------------------------------------
//
// "The values are encoded in the IEEE floating point format according to
// IEEE 754 single precision format" (DPT-AS §3.15) — a *different*
// encoding from main type 9's proprietary F16, stated explicitly by the
// Standard rather than inferred. 4 octets, octet 4 = MSB ... octet 1 =
// LSB (DPT-AS §3.15 table), field layout `S Exponent Fraction` (1+8+23
// bits) — exactly IEEE 754 binary32. DPT-AS does not restate the
// exponent bias (127) or the subnormal/zero/inf/NaN rules itself (NOTE 8
// only notes that the exponent is biased "to allow negative exponent
// values"); those are IEEE 754's own rules, not this document's, so this
// codec defers to Rust's `f32` bit representation rather than
// re-deriving them.
//
// No KNX-specific "invalid data" sentinel is documented for main type 14
// anywhere in DPT-AS (unlike main type 9's 7FFFh — confirmed by no
// second "invalid data" clause appearing under §3.15). Decode therefore
// passes IEEE 754 special values (±infinity, NaN, subnormals) through
// exactly as `f32::from_bits` produces them, widened to `f64`, rather
// than inventing a sentinel the corpus does not state. Encode, in the
// other direction, rejects a non-finite *input* and rejects an input
// whose magnitude the 32-bit format cannot hold, rather than silently
// letting it become an infinity on the wire — both are `OutOfRange`, not
// a silent clamp.

fn decode_f32(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_bytes::<4>(payload, dpt, 32)?;
    let bits = u32::from_be_bytes(raw);
    Ok(DptValue::Float(f64::from(f32::from_bits(bits))))
}

fn encode_f32(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let v: f64 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    let out_of_range = || DptCodecError::OutOfRange {
        dpt,
        value: trimmed.to_string(),
    };
    if !v.is_finite() {
        return Err(out_of_range());
    }
    let narrowed = v as f32;
    if !narrowed.is_finite() {
        // `f64 as f32` saturates to infinity when the magnitude exceeds
        // f32::MAX rather than panicking — catch that here so an
        // over-large value is refused, not silently sent as ±inf.
        return Err(out_of_range());
    }
    Ok(GroupValue::Bytes(narrowed.to_bits().to_be_bytes().to_vec()))
}

// ---------------------------------------------------------------------
// Main type 15 — U4U4U4U4U4U4B4N4, access data (DPT-AS §3.16)
// ---------------------------------------------------------------------
//
// 4 octets, 6 BCD-style digits packed 2 per octet, then a flags-and-index
// octet: octet 4 (MSB) = `D6`(hi nibble)/`D5`(lo nibble), octet 3 =
// `D4`/`D3`, octet 2 = `D2`/`D1`, octet 1 = `E`(bit 7) `P`(bit 6)
// `D`(bit 5) `C`(bit 4) `Index`(bits 3-0) — DPT-AS §3.16's format line
// (`4 octets: U4U4U4U4U4U4B4N4`) plus its field table, cross-checked
// against EXAMPLE 6/7's byte-level worked encodings below.
//
// Field meanings, per §3.16's table (not inferred from the letters
// alone, which are easy to mis-guess): `D6..D1` are "digit x (1...6) of
// access identification code" — `D6` is the *first* transmitted digit,
// `D1` the *last*, each a BCD nibble `[0...9]` ("If 24 bits are not
// necessary, the most significant positions shall be set to zero", so a
// short code is left-padded with zero digits into `D6`/`D5`/...). `E` is
// "Detection error" (`1` = reading not successful). `P` is "Permission"
// (`1` = accepted). `D` is "Read direction" (`0` = left to right, `1` =
// right to left — *not* a reserved bit, despite the letter overlap with
// "Day" elsewhere in this module; there is no reserved bit in this
// type's B4N4 octet, all 8 bits are assigned). `C` is "Encryption"
// (`1` = yes). `Index` is a 4-bit `[0...15]` "future use" field.
//
// EXAMPLE 6 and EXAMPLE 7 ([D], §3.16, PDF page 47 — re-verified directly
// against the source PDF for this fix, both examples sit on that one
// page): both are printed bit by bit with a per-field decimal value row
// beneath, not as a single stated hex string, but every bit is there in
// the text — this is a worked derivation from printed evidence, not this
// codec's own arithmetic from an unstated field value.
// EXAMPLE 6: code "123456", no error, permission accepted, badge read
// left to right, no encryption, index 13 — `D6=1 D5=2 D4=3 D3=4 D2=5
// D1=6`, `E=0 P=1 D=0 C=0`, `Index=13(0xD)`, giving octets
// `[0x12, 0x34, 0x56, 0x4D]`. EXAMPLE 7: code "6789" zero-padded to
// `D6=0 D5=0 D4=6 D3=7 D2=8 D1=9`, no error, *not* accepted, left to
// right, no encryption, index 14, giving `[0x00, 0x67, 0x89, 0x0E]`.
// The test suite cites these two page-printed bit values directly.
//
// A BCD nibble decoding to `10..=15` has no digit meaning and is
// `InvalidData` — same "documented range violation" policy as this
// module applies to main type 11's year octet. There is no reserved bit
// in this format to police (all 32 bits are assigned per the field
// table above), so no such check applies here.
//
// Text grammar for `encode` (this module's own choice): six whitespace-
// separated tokens, `"<6 digits> <error:on|off> <accepted:on|off>
// <left|right> <encrypted:on|off> <index>"`, e.g.
// `"123456 off on left off 13"` — matching `Display`'s own rendering, so
// `Display`'s output always re-parses. Boolean tokens reuse
// `parse_bool_word`, same as every other boolean field in this module.

fn decode_access_data(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [o4, o3, o2, o1] = require_bytes::<4>(payload, dpt, 32)?;
    let d6 = o4 >> 4;
    let d5 = o4 & 0x0F;
    let d4 = o3 >> 4;
    let d3 = o3 & 0x0F;
    let d2 = o2 >> 4;
    let d1 = o2 & 0x0F;
    for d in [d6, d5, d4, d3, d2, d1] {
        if d > 9 {
            return Err(DptCodecError::InvalidData { dpt });
        }
    }
    let error = o1 & 0b1000_0000 != 0;
    let accepted = o1 & 0b0100_0000 != 0;
    let right_to_left = o1 & 0b0010_0000 != 0;
    let encrypted = o1 & 0b0001_0000 != 0;
    let index = o1 & 0b0000_1111;
    Ok(DptValue::AccessData {
        code: [d6, d5, d4, d3, d2, d1],
        error,
        accepted,
        right_to_left,
        encrypted,
        index,
    })
}

fn encode_access_data(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let (code_tok, error_tok, accepted_tok, direction_tok, encrypted_tok, index_tok) =
        match tokens.as_slice() {
            [a, b, c, d, e, f] => (*a, *b, *c, *d, *e, *f),
            _ => return Err(unparsable()),
        };
    if code_tok.len() != 6 || !code_tok.bytes().all(|b| b.is_ascii_digit()) {
        return Err(unparsable());
    }
    let mut code = [0u8; 6];
    for (i, b) in code_tok.bytes().enumerate() {
        code[i] = b - b'0';
    }
    let error = parse_bool_word(error_tok).ok_or_else(unparsable)?;
    let accepted = parse_bool_word(accepted_tok).ok_or_else(unparsable)?;
    let right_to_left = match direction_tok.to_ascii_lowercase().as_str() {
        "left" => false,
        "right" => true,
        _ => return Err(unparsable()),
    };
    let encrypted = parse_bool_word(encrypted_tok).ok_or_else(unparsable)?;
    let index: u8 = index_tok.parse().map_err(|_| unparsable())?;
    if index > 15 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: index.to_string(),
        });
    }
    let o4 = (code[0] << 4) | code[1];
    let o3 = (code[2] << 4) | code[3];
    let o2 = (code[4] << 4) | code[5];
    let o1 = (u8::from(error) << 7)
        | (u8::from(accepted) << 6)
        | (u8::from(right_to_left) << 5)
        | (u8::from(encrypted) << 4)
        | index;
    Ok(GroupValue::Bytes(vec![o4, o3, o2, o1]))
}

// ---------------------------------------------------------------------
// Main type 16 — A[14], fixed 14-octet character string (DPT-AS §3.17)
// ---------------------------------------------------------------------
//
// Fixed length of exactly 14 octets, filled from the most significant
// octet; "If the string to be transmitted is smaller then 14 octets,
// unused trailing octets in the character string shall be set to NULL
// (00h)" (DPT-AS §3.17). This codec strips trailing 0x00 bytes on decode
// (they are padding, not content) and never trims anything else — a
// leading/trailing space inside the 14 octets is content, unlike the
// numeric/boolean grammars elsewhere in this module, so `encode` does
// not call `.trim()` on its input.
//
// Character set is per subtype: 16.000 DPT_String_ASCII uses 4.001
// DPT_Char_ASCII (7-bit, `[0...127]`, "the most significant bit shall
// always be 0" — DPT-AS §3.4); 16.001 DPT_String_8859_1 uses 4.002
// DPT_Char_8859_1 (`[0...255]`). Ruling: a bare `DPT-16` (`sub: None`) is
// treated as the ISO 8859-1 form — every octet is then decodable and no
// input is rejected that the narrower ASCII form would accept, the same
// "bare main type takes the widest/unscaled reading" principle as main
// type 5's `sub: None`. Any other subnumber has no documented character
// set in this corpus, so it is `UnsupportedDpt` rather than a guess.
//
// `char_set_is_ascii` below also serves main type 4's single-character
// A8 (DPT-AS §3.4, 4.001/4.002 — the same two character sets, just one
// character wide instead of fourteen); see that section's own comment
// for why the "bare main type" default this function applies to a bare
// `DPT-16` is deliberately *not* extended to a bare `DPT-4` — §3.4 states
// no such default, and none is invented here.
//
// ISO 8859-1's code points 0-255 map 1:1 onto Unicode's first 256 code
// points by construction, which is exactly what `char::from(u8)` and
// `char as u32` do in Rust — no separate table is needed for that
// subtype's conversion in either direction. A 16.000/4.001 byte with its
// top bit set violates 4.001's own "most significant bit shall always be
// 0" rule, the same "preceding bits shall be 0" violation `require_short`
// checks elsewhere in this module, so it is `InvalidData` here too.

fn char_set_is_ascii(dpt: DptRef) -> Result<bool, DptCodecError> {
    match (dpt.main, dpt.sub) {
        // Main type 4 — DPT-AS §3.4: 4.001 DPT_Char_ASCII, 4.002
        // DPT_Char_8859_1. No bare-`DPT-4` default is documented; a
        // `sub: None` here is `UnsupportedDpt`, not a guessed default.
        (4, Some(1)) => Ok(true),
        (4, Some(2)) => Ok(false),
        // Main type 16 — DPT-AS §3.17: 16.000 DPT_String_ASCII, and
        // 16.001/bare `DPT-16` both DPT_String_8859_1 (see the "bare
        // main type" ruling above).
        (16, Some(0)) => Ok(true),
        (16, None) | (16, Some(1)) => Ok(false),
        _ => Err(DptCodecError::UnsupportedDpt(dpt)),
    }
}

fn decode_a14(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let ascii_only = char_set_is_ascii(dpt)?;
    let raw = require_bytes::<14>(payload, dpt, 112)?;
    let content_len = raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let mut s = String::with_capacity(content_len);
    for &b in &raw[..content_len] {
        if ascii_only && b & 0x80 != 0 {
            return Err(DptCodecError::InvalidData { dpt });
        }
        s.push(char::from(b));
    }
    Ok(DptValue::Text(s))
}

fn encode_a14(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let ascii_only = char_set_is_ascii(dpt)?;
    if input.chars().count() > 14 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: input.to_string(),
        });
    }
    let limit: u32 = if ascii_only { 0x7F } else { 0xFF };
    let mut bytes = [0u8; 14];
    for (i, ch) in input.chars().enumerate() {
        let code = ch as u32;
        if code > limit {
            return Err(DptCodecError::Unparsable {
                dpt,
                input: input.to_string(),
            });
        }
        bytes[i] = code as u8;
    }
    Ok(GroupValue::Bytes(bytes.to_vec()))
}

// ---------------------------------------------------------------------
// Main type 17 — Scene Number (DPT-AS §3.18)
// ---------------------------------------------------------------------
//
// 1 octet, `r2U6`: 2 reserved bits (must be 0) + 6-bit unsigned
// `SceneNumber`, "Value binary encoded" (DPT-AS §3.18 table, 17.001
// DPT_SceneNumber), range `[0...63]`. Six significant bits puts this at
// the AL-AS §3.1.2/§3.1.3 inline-payload threshold (`GroupValue::Short`)
// exactly, so `require_short` (already used for main types 1-3) applies
// unchanged; a reserved bit set is the same "preceding bits shall be 0"
// violation as elsewhere, so it is `InvalidData`.
//
// Ruling on the wire value vs. the human scene number (do not relitigate
// — see the brief): DPT-AS §3.25 NOTE 16 recommends displaying
// DPT_SceneInfo (26.001) scene numbers with a +1 offset ("KNX Association
// recommends displaying these scene numbers... numbered from 1 to 64,
// this is, with an offset of 1 compared to the actual transmitted
// value"). That NOTE is textually attached only to §3.25/26.001; §3.18
// (this type, 17.001 DPT_SceneNumber) carries no such note of its own —
// its section ends at the datapoint-type table — so applying the
// 26.001 recommendation here by analogy would invent a rule the
// Standard does not state for 17.001. (Main type 18 is different: its
// own section, §3.19, does carry a matching +1 recommendation in NOTE 9
// — see `decode_scene_control`'s comment. That is 18.001's citation,
// not 17.001's, and does not change this type's answer.) `DptValue::Scene`
// therefore holds and prints the wire value 0-63 exactly, undecorated.
// A future UI slice can choose to display `wire + 1`; that is a
// presentation decision, not this codec's.

fn decode_scene(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_short(payload, dpt, 6)?;
    Ok(DptValue::Scene { number: raw })
}

fn encode_scene(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let n: u8 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    if n > 63 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: trimmed.to_string(),
        });
    }
    Ok(GroupValue::Short(n))
}

// ---------------------------------------------------------------------
// Main type 18 — Scene Control (DPT-AS §3.19)
// ---------------------------------------------------------------------
//
// 1 octet, `B1r1U6`: field `C` (bit 7, control: 0=activate, 1=learn),
// `R` (bit 6, reserved, `{0}`), `SceneNumber` (bits 5-0, `[0...63]`, same
// field as main type 17's) — DPT-AS §3.19. Significant content is
// `C`(1) + `SceneNumber`(6) = 7 bits, one more than the AL-AS ≤6-bit
// inline threshold — this type is *not* eligible for the short/inline
// `GroupValue::Short` form and must always occupy its own octet, unlike
// main type 17 (whose 6 significant bits qualify exactly). `encode`
// always produces `GroupValue::Bytes([_])`, never `Short`, to keep that
// distinction — the E4-D3 fix in `knx-net` protects a `Short` whose value
// exceeds 0x3F from corrupting the APCI, but the cleaner discipline on
// this side of the boundary is to never *propose* a `Short` for a DPT the
// Standard does not put there in the first place. `decode` mirrors that:
// a `Short` payload is `WrongLength`, same as any other wrong-shaped
// payload, never silently accepted as if it were the inline form.
//
// Ruling on the wire value vs. the human scene number (do not relitigate
// — see the brief, corrected in Fix round 1): unlike main type 17, this
// type's own section does carry a display recommendation. DPT-AS §3.19
// NOTE 9, attached to 18.001 DPT_SceneControl, states that KNX
// Association recommends displaying scene numbers as 1-64 in ETS and
// other software controllers, an offset of 1 from the actually
// transmitted value. (Task 2's original comment here wrongly attributed
// this +1 convention to §3.25 NOTE 16/26.001 only, the same as main
// type 17's citation — that was incorrect specifically for main type
// 18: §3.25 NOTE 16 belongs to 26.001 DPT_SceneInfo, but §3.19 NOTE 9
// is 18.001's own, separate recommendation with the same +1 shape.)
// NOTE 9's text is missing from this corpus's Markdown extraction —
// the same shape of gap as 13.100's §3.14.3 — and was read from the
// source PDF instead; a reader who greps the Markdown and finds
// nothing here should not conclude this was invented. The
// recommendation is about *display*, a decision for a later UI layer,
// and that layer can only make it honestly if handed the untouched
// wire value — so this codec keeps 0-63 undecorated in both directions
// regardless, same as main type 17, and `format()` prints the wire
// value too.

fn decode_scene_control(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [raw] = require_bytes::<1>(payload, dpt, 8)?;
    if raw & 0b0100_0000 != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    Ok(DptValue::SceneControl {
        learn: raw & 0b1000_0000 != 0,
        number: raw & 0b0011_1111,
    })
}

fn encode_scene_control(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let learn = match tokens.first() {
        Some(w) if w.eq_ignore_ascii_case("activate") => false,
        Some(w) if w.eq_ignore_ascii_case("learn") => true,
        _ => return Err(unparsable()),
    };
    let number: u8 = match tokens.get(1..) {
        Some([w, n]) if w.eq_ignore_ascii_case("scene") => n.parse().map_err(|_| unparsable())?,
        _ => return Err(unparsable()),
    };
    if number > 63 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: number.to_string(),
        });
    }
    Ok(GroupValue::Bytes(vec![(u8::from(learn) << 7) | number]))
}

// ---------------------------------------------------------------------
// Main type 19 — U8[r4U4][r3U5][U3U5][r2U6][r2U6]B16, date and time
// (DPT-AS §3.20, notes in §3.20.1)
// ---------------------------------------------------------------------
//
// 8 octets. Octet 8 (MSB): `Year`, full byte, `U`, offset 1900
// (`0 = 1900`, `255 = 2155` — Note 10: "encoded on 8 bits instead as on 7
// bits as in DPT_Date"). Octet 7: 4 reserved bits then `Month` (4 bits,
// `[1...12]`). Octet 6: 3 reserved bits then `DayOfMonth` (5 bits,
// `[1...31]`). Octet 5: `DayOfWeek` (3 bits, `[0...7]`) then `HourOfDay`
// (5 bits, `[0...24]` — Note 11's widened range, see below). Octet 4: 2
// reserved bits then `Minutes` (6 bits, `[0...59]`). Octet 3: 2 reserved
// bits then `Seconds` (6 bits, `[0...59]`). Octet 2: 8 flag bits, MSB to
// LSB `F`(Fault) `WD`(Working Day) `NWD`(No WD) `NY`(No Year)
// `ND`(No Date) `NDoW`(No Day of Week) `NT`(No Time) `SUTI`(Standard/
// summer time). Octet 1: `CLQ`(Quality of Clock, bit 7) then 7 bits the
// diagram and Note 15 both mark `r` (reserved).
//
// Standard inconsistency — [D], confirmed by rendering DPT-AS page 50 as
// an image and inspecting it directly (the Markdown/plain-text table
// extraction for this octet is column-misaligned and cannot settle it on
// its own): octet 1's own diagram contradicts itself between its two
// rows. The field-*names* row gives eight cells — `CLQ | SRC | 0 | 0 |
// 0 | 0 | 0 | 0` — placing `SRC` ("Synchronisation source reliability")
// at bit 6. The bit-*encoding* row directly beneath it reads `B | r | r |
// r | r | r | r | r`: bit 6 is `r` (reserved), not `SRC`. Note 15 sides
// with the encoding row, stating plainly "Bit 7 of the octet 1 is used
// for 'Quality of Clock' bit (CLQ). The other bits of this octet are
// reserved for future extensions. Their values shall be 0. ... Receivers
// shall check these bits to be 0." — naming no `SRC` bit at all.
//
// [A]: this codec follows the encoding row and Note 15, not the
// field-names row — a ruling on which half of the Standard's own
// contradictory diagram to trust, not a case of "no bit exists" to read
// from. Accordingly bit 6 of octet 1 is policed exactly like the other
// six reserved bits in that octet (nonzero is `InvalidData`), `SRC` is
// never decoded, and `DptValue::DateTime` has no `src`/
// `synchronisation_source_reliability` field. A reader diffing this
// codec's field list against §3.20's field-*names* row and finding `SRC`
// "missing" should read this paragraph: the Standard disagrees with
// itself here, and this codec picked a side rather than silently
// dropping the bit.
//
// No-data-loss statement (the brief requires this explicitly for this
// type): every bit this section's diagram assigns a name to has a
// `DptValue::DateTime` field — `year`, `month`, `day_of_month`,
// `day_of_week`, `hour`, `minute`, `second`, `fault`, `working_day`,
// `working_day_unknown` (`NWD`), `year_invalid` (`NY`), `date_invalid`
// (`ND`), `day_of_week_invalid` (`NDoW`), `time_invalid` (`NT`),
// `summer_time` (`SUTI`), `externally_synchronized` (`CLQ`) — sixteen
// fields for sixteen assigned bits (`SRC` excepted, as above, because
// this codec follows the encoding row and Note 15 over the field-names
// row for octet 1 bit 6 — a ruling, not an absence). None of
// `month`/`day_of_month`/`hour`/`minute`/`second`'s
// raw values are discarded when their matching `_invalid` flag is set —
// see `DptValue::DateTime`'s doc comment for why holding the octet's
// bits and holding the Standard's "ignore this" instruction are two
// different things, both kept.
//
// NOTE 14's wildcard/invalid distinction (the brief also calls this out
// specifically): "`NDoW = 1`... the ddd information shall be ignored...
// `NDoW = 0` and `ddd = 0`... ddd is a wildcard." This codec keeps
// `day_of_week` (the raw `ddd`) and `day_of_week_invalid` (`NDoW`) as two
// separate fields rather than one `Option`, so "any day, valid"
// (`day_of_week == 0, day_of_week_invalid == false`) and "field not
// valid, ignore `ddd`" (`day_of_week_invalid == true`, `ddd` whatever it
// was) never collapse into the same representation.
//
// Range-enforcement policy (a judgment call, spelled out because the
// Standard does not state it for these specific fields): `Month`/
// `DayOfMonth`'s documented ranges (`[1...12]`/`[1...31]`) are enforced —
// `InvalidData` on violation — only when `ND` says the date fields ARE
// valid; when `ND` says they are not, this codec accepts any value the
// field's own bit width can hold (`Month` up to 15, `DayOfMonth` up to
// 31) without further range-checking, on the reasoning that a "no date"
// clock plausibly zero-fills or garbage-fills a field nobody is meant to
// read, and rejecting such a telegram outright would invent a stricter
// rule than §3.20 states for the not-valid case. The same reasoning
// applies to `Hour`/`Minute`/`Second` under `NT`. `DayOfWeek` needs no
// such conditional — its full 3-bit range `[0...7]` is already exactly
// its documented range, valid or not. `Year` needs none either — its
// full byte range `[0...255]` (`NY` or not) is already its documented
// range (Note 10).
//
// Note 11's Hour=24 cross-rule is the one exception this codec enforces
// unconditionally *whenever the time fields are otherwise being
// range-checked* (i.e. when `NT` says they are valid — see above): "the
// values of octet 3 (Minutes) and 2 (Seconds) have to be set to zero
// [when Hour=24]. Messages with invalid values (\"Hour = 24\", Minutes
// and Seconds not zero) have to be ignored by the receiver" — explicit
// "invalid...ignored" language, unlike the plain range statements this
// codec treats as valid-only.
//
// Reserved-bit policy: the `r` bits above `Month`, above `DayOfMonth`,
// above `Minutes`, above `Seconds`, and the low 7 bits of octet 1, are
// all checked to be 0 unconditionally (not gated on any validity flag) —
// `InvalidData` otherwise, the same policy as the rest of this module,
// and Note 15's own explicit "Receivers shall check these bits to be 0"
// for octet 1 specifically.
//
// Text grammar for `encode` (this module's own choice — the Standard
// specifies wire encoding only): space-separated `key=value` tokens, all
// required, in any order: `year month day dow hour minute second fault
// workday no-workday no-year no-date no-dow no-time summer-time synced`.
// Boolean values reuse `parse_bool_word`. This matches `Display`'s own
// rendering (which always emits every key in this order), so `Display`'s
// output always re-parses.

fn decode_datetime(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [o8, o7, o6, o5, o4, o3, o2, o1] = require_bytes::<8>(payload, dpt, 64)?;

    if o7 & 0b1111_0000 != 0
        || o6 & 0b1110_0000 != 0
        || o4 & 0b1100_0000 != 0
        || o3 & 0b1100_0000 != 0
        || o1 & 0b0111_1111 != 0
    {
        return Err(DptCodecError::InvalidData { dpt });
    }

    let year = 1900u16 + u16::from(o8);
    let month = o7 & 0b0000_1111;
    let day_of_month = o6 & 0b0001_1111;
    let day_of_week = o5 >> 5;
    let hour = o5 & 0b0001_1111;
    let minute = o4 & 0b0011_1111;
    let second = o3 & 0b0011_1111;

    let fault = o2 & 0b1000_0000 != 0;
    let working_day = o2 & 0b0100_0000 != 0;
    let working_day_unknown = o2 & 0b0010_0000 != 0;
    let year_invalid = o2 & 0b0001_0000 != 0;
    let date_invalid = o2 & 0b0000_1000 != 0;
    let day_of_week_invalid = o2 & 0b0000_0100 != 0;
    let time_invalid = o2 & 0b0000_0010 != 0;
    let summer_time = o2 & 0b0000_0001 != 0;
    let externally_synchronized = o1 & 0b1000_0000 != 0;

    if !date_invalid && (!(1..=12).contains(&month) || !(1..=31).contains(&day_of_month)) {
        return Err(DptCodecError::InvalidData { dpt });
    }
    if !time_invalid {
        if hour > 24 || minute > 59 || second > 59 {
            return Err(DptCodecError::InvalidData { dpt });
        }
        // Note 11: Hour=24 is only legal with Minutes=Seconds=0.
        if hour == 24 && (minute != 0 || second != 0) {
            return Err(DptCodecError::InvalidData { dpt });
        }
    }

    Ok(DptValue::DateTime {
        year,
        year_invalid,
        month,
        day_of_month,
        date_invalid,
        day_of_week,
        day_of_week_invalid,
        hour,
        minute,
        second,
        time_invalid,
        fault,
        working_day,
        working_day_unknown,
        summer_time,
        externally_synchronized,
    })
}

fn encode_datetime(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };

    let mut year: Option<u16> = None;
    let mut month: Option<u8> = None;
    let mut day: Option<u8> = None;
    let mut dow: Option<u8> = None;
    let mut hour: Option<u8> = None;
    let mut minute: Option<u8> = None;
    let mut second: Option<u8> = None;
    let mut fault = false;
    let mut workday = false;
    let mut no_workday = false;
    let mut no_year = false;
    let mut no_date = false;
    let mut no_dow = false;
    let mut no_time = false;
    let mut summer_time = false;
    let mut synced = false;

    for token in input.split_whitespace() {
        let (key, value) = token.split_once('=').ok_or_else(unparsable)?;
        match key {
            "year" => year = Some(value.parse().map_err(|_| unparsable())?),
            "month" => month = Some(value.parse().map_err(|_| unparsable())?),
            "day" => day = Some(value.parse().map_err(|_| unparsable())?),
            "dow" => dow = Some(value.parse().map_err(|_| unparsable())?),
            "hour" => hour = Some(value.parse().map_err(|_| unparsable())?),
            "minute" => minute = Some(value.parse().map_err(|_| unparsable())?),
            "second" => second = Some(value.parse().map_err(|_| unparsable())?),
            "fault" => fault = parse_bool_word(value).ok_or_else(unparsable)?,
            "workday" => workday = parse_bool_word(value).ok_or_else(unparsable)?,
            "no-workday" => no_workday = parse_bool_word(value).ok_or_else(unparsable)?,
            "no-year" => no_year = parse_bool_word(value).ok_or_else(unparsable)?,
            "no-date" => no_date = parse_bool_word(value).ok_or_else(unparsable)?,
            "no-dow" => no_dow = parse_bool_word(value).ok_or_else(unparsable)?,
            "no-time" => no_time = parse_bool_word(value).ok_or_else(unparsable)?,
            "summer-time" => summer_time = parse_bool_word(value).ok_or_else(unparsable)?,
            "synced" => synced = parse_bool_word(value).ok_or_else(unparsable)?,
            _ => return Err(unparsable()),
        }
    }

    let year = year.ok_or_else(unparsable)?;
    let month = month.ok_or_else(unparsable)?;
    let day = day.ok_or_else(unparsable)?;
    let dow = dow.ok_or_else(unparsable)?;
    let hour = hour.ok_or_else(unparsable)?;
    let minute = minute.ok_or_else(unparsable)?;
    let second = second.ok_or_else(unparsable)?;

    if !(1900..=2155).contains(&year) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: year.to_string(),
        });
    }
    if dow > 7 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: dow.to_string(),
        });
    }
    if !no_date {
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(DptCodecError::OutOfRange {
                dpt,
                value: format!("{month}-{day}"),
            });
        }
    } else if month > 15 || day > 31 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: format!("{month}-{day}"),
        });
    }
    if !no_time {
        if hour > 24 || minute > 59 || second > 59 || (hour == 24 && (minute != 0 || second != 0)) {
            return Err(DptCodecError::OutOfRange {
                dpt,
                value: format!("{hour}:{minute}:{second}"),
            });
        }
    } else if hour > 31 || minute > 63 || second > 63 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: format!("{hour}:{minute}:{second}"),
        });
    }

    let o8 = (year - 1900) as u8;
    let o7 = month & 0x0F;
    let o6 = day & 0x1F;
    let o5 = (dow << 5) | (hour & 0x1F);
    let o4 = minute & 0x3F;
    let o3 = second & 0x3F;
    let o2 = (u8::from(fault) << 7)
        | (u8::from(workday) << 6)
        | (u8::from(no_workday) << 5)
        | (u8::from(no_year) << 4)
        | (u8::from(no_date) << 3)
        | (u8::from(no_dow) << 2)
        | (u8::from(no_time) << 1)
        | u8::from(summer_time);
    let o1 = u8::from(synced) << 7;

    Ok(GroupValue::Bytes(vec![o8, o7, o6, o5, o4, o3, o2, o1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dpt(main: u16, sub: Option<u16>) -> DptRef {
        DptRef { main, sub }
    }

    // -- Main type 1 -----------------------------------------------------

    #[test]
    fn b1_round_trips_off_and_on() {
        for (text, raw) in [("off", 0u8), ("on", 1u8)] {
            let d = dpt(1, Some(1));
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Short(raw));
            assert_eq!(decode(d, &payload).unwrap(), DptValue::Bool(raw == 1));
        }
    }

    #[test]
    fn b1_accepts_case_insensitive_true_false_and_digits() {
        let d = dpt(1, Some(2));
        for text in ["TRUE", "True", "1"] {
            assert_eq!(encode(d, text).unwrap(), GroupValue::Short(1));
        }
        for text in ["FALSE", "False", "0"] {
            assert_eq!(encode(d, text).unwrap(), GroupValue::Short(0));
        }
    }

    #[test]
    fn b1_sub_none_behaves_like_any_other_subtype() {
        // Main type 1's subtype changes no bits (DPT-AS §3.1 NOTE 2).
        let d = dpt(1, None);
        assert_eq!(
            decode(d, &GroupValue::Short(1)).unwrap(),
            DptValue::Bool(true)
        );
    }

    #[test]
    fn b1_decode_accepts_single_byte_form_by_tolerance() {
        let d = dpt(1, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![1])).unwrap(),
            DptValue::Bool(true)
        );
    }

    #[test]
    fn b1_decode_rejects_bits_above_the_significant_bit() {
        let d = dpt(1, Some(1));
        // DPT-AS §1.3.1: "The preceding bits shall be 0."
        let err = decode(d, &GroupValue::Short(0b10)).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn b1_decode_rejects_multi_byte_payload() {
        let d = dpt(1, Some(1));
        let err = decode(d, &GroupValue::Bytes(vec![1, 2])).unwrap_err();
        assert_eq!(
            err,
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 1,
                got: 16
            }
        );
    }

    #[test]
    fn b1_encode_rejects_unparsable_text() {
        let d = dpt(1, Some(1));
        let err = encode(d, "maybe").unwrap_err();
        assert_eq!(
            err,
            DptCodecError::Unparsable {
                dpt: d,
                input: "maybe".to_string()
            }
        );
    }

    // -- Main type 2 -----------------------------------------------------

    #[test]
    fn b2_round_trips_all_four_encodings() {
        // All four raw 2-bit encodings (c,v). Three of them have a canonical
        // text form and round trip through `encode` too; the fourth, c=0
        // v=1, is still "no control" in text (§3.2: v is don't-care when
        // c=0), so `encode("no control")` can never reproduce its raw bits —
        // it is checked decode-only, proving the codec does not discard v
        // just because c said it does not matter.
        let d = dpt(2, Some(1));
        for (text, raw, control, value) in [
            (Some("no control"), 0b00u8, false, false),
            (None, 0b01u8, false, true),
            (Some("control off"), 0b10u8, true, false),
            (Some("control on"), 0b11u8, true, true),
        ] {
            if let Some(text) = text {
                let payload = encode(d, text).unwrap();
                assert_eq!(payload, GroupValue::Short(raw));
            }
            assert_eq!(
                decode(d, &GroupValue::Short(raw)).unwrap(),
                DptValue::ControlBool { control, value }
            );
        }
    }

    #[test]
    fn b2_decode_rejects_bits_above_the_two_significant_bits() {
        let d = dpt(2, Some(1));
        let err = decode(d, &GroupValue::Short(0b100)).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn b2_encode_rejects_unparsable_text() {
        let d = dpt(2, Some(1));
        assert!(matches!(
            encode(d, "control maybe"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    // -- Main type 3 -----------------------------------------------------

    #[test]
    fn b1u3_round_trips_break_and_every_step_code() {
        let d = dpt(3, Some(7));
        assert_eq!(
            encode(d, "increase break").unwrap(),
            GroupValue::Short(0b1000)
        );
        assert_eq!(
            decode(d, &GroupValue::Short(0b1000)).unwrap(),
            DptValue::Step {
                increase: true,
                step_code: 0
            }
        );
        for step in 1u8..=7 {
            let text = format!("decrease step {step}");
            let payload = encode(d, &text).unwrap();
            assert_eq!(payload, GroupValue::Short(step));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Step {
                    increase: false,
                    step_code: step
                }
            );
        }
    }

    #[test]
    fn b1u3_encode_rejects_step_code_out_of_range() {
        let d = dpt(3, Some(7));
        let err = encode(d, "increase step 8").unwrap_err();
        assert_eq!(
            err,
            DptCodecError::OutOfRange {
                dpt: d,
                value: "8".to_string()
            }
        );
        assert!(matches!(
            encode(d, "increase step 0"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn b1u3_encode_rejects_unparsable_direction() {
        let d = dpt(3, Some(7));
        assert!(matches!(
            encode(d, "sideways step 3"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn b1u3_decode_rejects_bits_above_the_four_significant_bits() {
        let d = dpt(3, Some(7));
        let err = decode(d, &GroupValue::Short(0b10000)).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    // -- Main type 4 ------------------------------------------------------

    #[test]
    fn a8_ascii_round_trips_a_printable_character() {
        // [V]: 'A' = 0x41, DPT-AS §3.4's own character table (page 31),
        // row LSN=1, column MSN=4.
        let d = dpt(4, Some(1));
        let payload = encode(d, "A").unwrap();
        assert_eq!(payload, GroupValue::Bytes(vec![0x41]));
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::Text("A".to_string())
        );
    }

    #[test]
    fn a8_8859_1_round_trips_a_character_outside_ascii() {
        // [D]+[V]: DPT-AS §3.4's character table (page 31), row LSN=0,
        // column MSN=B: 0xB0 = '°', outside 4.001's 7-bit ASCII set but
        // valid for 4.002.
        let d = dpt(4, Some(2));
        let payload = encode(d, "\u{b0}").unwrap();
        assert_eq!(payload, GroupValue::Bytes(vec![0xB0]));
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::Text("\u{b0}".to_string())
        );
    }

    #[test]
    fn a8_ascii_rejects_the_top_bit_in_both_directions() {
        let d = dpt(4, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0xB0])).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
        assert!(matches!(
            encode(d, "\u{b0}"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn a8_decode_rejects_wrong_length_payload() {
        let d = dpt(4, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x41, 0x42])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 8,
                got: 16
            }
        );
    }

    #[test]
    fn a8_encode_rejects_more_than_one_character() {
        let d = dpt(4, Some(1));
        assert!(matches!(
            encode(d, "AB"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn a8_encode_rejects_empty_input() {
        let d = dpt(4, Some(1));
        assert!(matches!(
            encode(d, ""),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn a8_bare_main_type_has_no_documented_default_and_is_unsupported() {
        // Unlike main type 16, §3.4 states no bare-`DPT-4` default — this
        // is a deliberate non-extrapolation, not an oversight.
        let d = dpt(4, None);
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x41])),
            Err(DptCodecError::UnsupportedDpt(d))
        );
        assert_eq!(encode(d, "A"), Err(DptCodecError::UnsupportedDpt(d)));
    }

    // -- Main type 5 -------------------------------------------------------

    #[test]
    fn u8_scaling_round_trips_min_max_zero_and_interior() {
        let d = dpt(5, Some(1));
        // DPT-AS §3.5.1 NOTE 4's own worked example: 50 % -> 80h, 100 % -> FFh.
        assert_eq!(encode(d, "0").unwrap(), GroupValue::Bytes(vec![0x00]));
        assert_eq!(encode(d, "50").unwrap(), GroupValue::Bytes(vec![0x80]));
        assert_eq!(encode(d, "100").unwrap(), GroupValue::Bytes(vec![0xFF]));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x80])).unwrap(),
            DptValue::Float(f64::from(0x80u8) * 100.0 / 255.0)
        );
    }

    #[test]
    fn u8_scaling_rejects_out_of_range_percent() {
        let d = dpt(5, Some(1));
        assert!(matches!(
            encode(d, "100.5"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-1"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn u8_angle_round_trips_min_max_zero_and_interior() {
        let d = dpt(5, Some(3));
        assert_eq!(encode(d, "0").unwrap(), GroupValue::Bytes(vec![0x00]));
        assert_eq!(encode(d, "360").unwrap(), GroupValue::Bytes(vec![0xFF]));
        // 180 does not divide evenly by 360/255, so the round trip lands on
        // the nearest representable angle, not exactly 180 — same rounding
        // behaviour as 5.001's scaling, just checked against the raw byte
        // instead of an exact value.
        let payload = encode(d, "180").unwrap();
        assert_eq!(payload, GroupValue::Bytes(vec![128]));
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::Float(f64::from(128u8) * 360.0 / 255.0)
        );
    }

    #[test]
    fn u8_sub_none_is_raw_unscaled_count_not_a_percentage() {
        // Ruling: `DptRef { sub: None }` uses main type 5's unscaled form.
        let d = dpt(5, None);
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x80])).unwrap(),
            DptValue::Unsigned(0x80)
        );
    }

    #[test]
    fn u8_percent_u8_is_raw_identity_not_scaled() {
        let d = dpt(5, Some(4));
        assert_eq!(encode(d, "255").unwrap(), GroupValue::Bytes(vec![255]));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![255])).unwrap(),
            DptValue::Unsigned(255)
        );
    }

    #[test]
    fn u8_decimal_factor_is_raw_identity_undocumented_meaning() {
        // 5.005: Range/Resolution are blank in the published document.
        let d = dpt(5, Some(5));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![42])).unwrap(),
            DptValue::Unsigned(42)
        );
    }

    #[test]
    fn u8_tariff_round_trips_and_reserves_255() {
        let d = dpt(5, Some(6));
        assert_eq!(encode(d, "0").unwrap(), GroupValue::Bytes(vec![0]));
        assert_eq!(encode(d, "254").unwrap(), GroupValue::Bytes(vec![254]));
        assert!(matches!(
            encode(d, "255"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![255])).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
    }

    #[test]
    fn u8_wrong_length_payload_is_reported() {
        let d = dpt(5, Some(1));
        let err = decode(d, &GroupValue::Short(1)).unwrap_err();
        assert_eq!(
            err,
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 8,
                got: 6
            }
        );
    }

    #[test]
    fn u8_encode_rejects_unparsable_text() {
        let d = dpt(5, Some(1));
        assert!(matches!(
            encode(d, "fifty"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    // -- Main type 6 ---------------------------------------------------

    #[test]
    fn v8_round_trips_min_max_zero_and_interior() {
        let d = dpt(6, Some(10));
        for (text, raw) in [("-128", 0x80u8), ("127", 0x7F), ("0", 0x00), ("-1", 0xFF)] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![raw]));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Signed(text.parse().unwrap())
            );
        }
    }

    #[test]
    fn v8_sub_none_is_plain_signed_identity() {
        let d = dpt(6, None);
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0xFF])).unwrap(),
            DptValue::Signed(-1)
        );
    }

    #[test]
    fn v8_rejects_out_of_range_values() {
        let d = dpt(6, Some(10));
        assert!(matches!(
            encode(d, "128"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-129"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn v8_status_mode3_is_unsupported_not_misread_as_a_plain_integer() {
        let d = dpt(6, Some(20));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0b0010_0001])),
            Err(DptCodecError::UnsupportedDpt(d))
        );
        assert_eq!(encode(d, "1"), Err(DptCodecError::UnsupportedDpt(d)));
    }

    // -- Main type 7 -----------------------------------------------------

    #[test]
    fn u16_round_trips_min_max_zero_and_interior() {
        let d = dpt(7, Some(1));
        for (text, bytes) in [
            ("0", vec![0x00, 0x00]),
            ("65535", vec![0xFF, 0xFF]),
            ("256", vec![0x01, 0x00]),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(bytes.clone()));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Unsigned(text.parse().unwrap())
            );
        }
    }

    #[test]
    fn u16_rejects_out_of_range_and_unparsable() {
        let d = dpt(7, Some(1));
        assert!(matches!(
            encode(d, "65536"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-1"),
            Err(DptCodecError::Unparsable { .. })
        ));
        assert!(matches!(
            encode(d, "1.5"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn u16_decode_rejects_short_payload() {
        let d = dpt(7, Some(1));
        let err = decode(d, &GroupValue::Short(1)).unwrap_err();
        assert_eq!(
            err,
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 16,
                got: 6
            }
        );
    }

    #[test]
    fn u16_no_bus_power_sentinel_decodes_as_an_ordinary_zero() {
        // 7.012: raw 0 has a documented *meaning* ("no bus power supply
        // functionality available") but is not flagged invalid data.
        let d = dpt(7, Some(12));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0])).unwrap(),
            DptValue::Unsigned(0)
        );
    }

    // -- Main type 8 -----------------------------------------------------

    #[test]
    fn v16_round_trips_min_max_zero_and_interior() {
        let d = dpt(8, Some(1));
        for (text, bytes) in [
            ("-32768", vec![0x80, 0x00]),
            ("32767", vec![0x7F, 0xFF]),
            ("0", vec![0x00, 0x00]),
            ("-1", vec![0xFF, 0xFF]),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(bytes.clone()));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Signed(text.parse().unwrap())
            );
        }
    }

    #[test]
    fn v16_8001_does_not_treat_7fffh_as_invalid() {
        // Ruling: the footnote naming 8.001's sentinel actually names
        // main type 7's DPT, so it is not acted on; 32767 round-trips.
        let d = dpt(8, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x7F, 0xFF])).unwrap(),
            DptValue::Signed(32767)
        );
    }

    #[test]
    fn v16_percent_round_trips_min_zero_and_interior() {
        let d = dpt(8, Some(10));
        assert_eq!(
            encode(d, "-327.68").unwrap(),
            GroupValue::Bytes(vec![0x80, 0x00])
        );
        assert_eq!(encode(d, "0").unwrap(), GroupValue::Bytes(vec![0x00, 0x00]));
        let payload = encode(d, "12.34").unwrap();
        assert_eq!(decode(d, &payload).unwrap(), DptValue::Float(12.34));
    }

    #[test]
    fn v16_percent_maximum_collides_with_the_invalid_sentinel() {
        let d = dpt(8, Some(10));
        // 327.67 % is exactly raw 7FFFh, the Standard's own invalid-data
        // code for this DPT — encode refuses to produce it.
        assert!(matches!(
            encode(d, "327.67"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        // The practical maximum is one resolution step below that.
        assert_eq!(
            encode(d, "327.66").unwrap(),
            GroupValue::Bytes(vec![0x7F, 0xFE])
        );
    }

    #[test]
    fn v16_percent_7fffh_decodes_as_invalid_data() {
        let d = dpt(8, Some(10));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x7F, 0xFF])).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
    }

    #[test]
    fn v16_decode_rejects_wrong_length_payload() {
        let d = dpt(8, Some(1));
        let err = decode(d, &GroupValue::Bytes(vec![1])).unwrap_err();
        assert_eq!(
            err,
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 16,
                got: 8
            }
        );
    }

    // -- Main type 9 -------------------------------------------------------

    #[test]
    fn f16_round_trips_min_max_zero_and_interior() {
        let d = dpt(9, Some(1));
        for (text, bytes) in [
            // E=15, M=-2 048: -2048*0,01*2^15 = -671 088,64.
            ("-671088.64", vec![0xF8, 0x00]),
            // E=15, M=2 046 (one below the sentinel-colliding M=2 047) —
            // DPT-AS's own printed practical maximum.
            ("670433.28", vec![0x7F, 0xFE]),
            ("0", vec![0x00, 0x00]),
            ("12.34", vec![0x04, 0xD2]),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(bytes.clone()));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Float(text.parse().unwrap())
            );
        }
    }

    #[test]
    fn f16_negative_value_a_sign_magnitude_misreading_would_get_wrong() {
        // Raw 87FFh: sign bit 1, E=0, low 11 mantissa bits all 1. A "sign
        // bit plus 11-bit magnitude" misreading takes the magnitude as
        // 0x7FF = 2047 and negates it: -20.47. The correct 12-bit two's
        // complement reassembly (sign bit + 11 bits = 0xFFF = -1) gives
        // -0.01 instead — see the module comment above `decode_f16`.
        let d = dpt(9, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x87, 0xFF])).unwrap(),
            DptValue::Float(-0.01)
        );
    }

    #[test]
    fn f16_invalid_data_sentinel_decodes_as_invalid_data() {
        let d = dpt(9, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x7F, 0xFF])).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
    }

    #[test]
    fn f16_encode_rejects_the_value_that_would_collide_with_the_sentinel() {
        // 670 760,96 (E=15, M=2 047) is the arithmetic maximum the formula
        // allows, but its raw encoding is exactly 7FFFh — reserved for
        // "invalid data". Encode must not silently produce it.
        let d = dpt(9, Some(1));
        assert!(matches!(
            encode(d, "670760.96"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn f16_encode_rejects_non_finite_and_unrepresentable_values() {
        let d = dpt(9, Some(1));
        assert!(matches!(
            encode(d, "NaN"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "inf"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        // No exponent makes this fit M's [-2048...2047] range.
        assert!(matches!(
            encode(d, "1e10"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn f16_encode_rejects_unparsable_text() {
        let d = dpt(9, Some(1));
        assert!(matches!(
            encode(d, "banana"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn f16_decode_rejects_wrong_length_payload() {
        let d = dpt(9, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x00])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 16,
                got: 8
            }
        );
    }

    // -- Main type 10 -------------------------------------------------------

    #[test]
    fn time_of_day_round_trips_a_named_weekday() {
        // [own reading]: DPT-AS §3.11's table (`1 = Monday`), no worked
        // byte example is printed for this type.
        let d = dpt(10, Some(1));
        let payload = encode(d, "monday 07:30:15").unwrap();
        assert_eq!(payload, GroupValue::Bytes(vec![(1 << 5) | 7, 30, 15]));
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::TimeOfDay {
                day: Some(1),
                hour: 7,
                minute: 30,
                second: 15
            }
        );
    }

    #[test]
    fn time_of_day_no_day_round_trips_exactly() {
        // The "day=0 means no day" judgment call: `None` must encode back
        // to wire code 0, and wire code 0 must decode back to `None`.
        let d = dpt(10, Some(1));
        let payload = encode(d, "none 00:00:00").unwrap();
        assert_eq!(payload, GroupValue::Bytes(vec![0, 0, 0]));
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::TimeOfDay {
                day: None,
                hour: 0,
                minute: 0,
                second: 0
            }
        );
    }

    #[test]
    fn time_of_day_round_trips_the_boundary_values() {
        let d = dpt(10, Some(1));
        for (text, day, hour, minute, second) in [
            ("sunday 23:59:59", 7u8, 23u8, 59u8, 59u8),
            ("wednesday 00:00:00", 3, 0, 0, 0),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::TimeOfDay {
                    day: Some(day),
                    hour,
                    minute,
                    second
                }
            );
        }
    }

    #[test]
    fn time_of_day_display_re_parses() {
        let d = dpt(10, Some(1));
        let v = DptValue::TimeOfDay {
            day: Some(5),
            hour: 9,
            minute: 5,
            second: 0,
        };
        let text = v.to_string();
        assert_eq!(encode(d, &text).and_then(|p| decode(d, &p)), Ok(v));
    }

    #[test]
    fn time_of_day_decode_rejects_reserved_bits() {
        let d = dpt(10, Some(1));
        // Reserved bits set above Minutes.
        let err = decode(d, &GroupValue::Bytes(vec![0, 0b1000_0000, 0])).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn time_of_day_decode_rejects_out_of_range_hour_minute_second() {
        let d = dpt(10, Some(1));
        for bytes in [
            vec![24u8, 0, 0], // hour 24 (only 0-23 legal)
            vec![0u8, 60, 0], // minute 60
            vec![0u8, 0, 60], // second 60
        ] {
            let err = decode(d, &GroupValue::Bytes(bytes)).unwrap_err();
            assert_eq!(err, DptCodecError::InvalidData { dpt: d });
        }
    }

    #[test]
    fn time_of_day_decode_rejects_wrong_length_payload() {
        let d = dpt(10, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 24,
                got: 16
            }
        );
    }

    #[test]
    fn time_of_day_encode_rejects_unparsable_text() {
        let d = dpt(10, Some(1));
        for text in ["", "monday", "someday 00:00:00", "monday 25:00:00"] {
            assert!(
                matches!(
                    encode(d, text),
                    Err(DptCodecError::Unparsable { .. }) | Err(DptCodecError::OutOfRange { .. })
                ),
                "expected an error for {text:?}"
            );
        }
    }

    // -- Main type 11 -------------------------------------------------------

    #[test]
    fn date_century_encoding_matches_example_5() {
        // [D]: DPT-AS §3.12 EXAMPLE 5 — `99d equals 1999`, `0d equals
        // 2000`, `4d equals 2004`.
        let d = dpt(11, Some(1));
        for (year_raw, year) in [(99u8, 1999u16), (0, 2000), (4, 2004)] {
            let payload = GroupValue::Bytes(vec![15, 6, year_raw]);
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Date {
                    day: 15,
                    month: 6,
                    year
                }
            );
            assert_eq!(encode(d, &format!("{year}-06-15")).unwrap(), payload);
        }
    }

    #[test]
    fn date_round_trips_the_boundary_values() {
        let d = dpt(11, Some(1));
        for text in ["1990-01-01", "2089-12-31"] {
            let payload = encode(d, text).unwrap();
            let decoded = decode(d, &payload).unwrap();
            assert_eq!(format!("{decoded}"), text);
        }
    }

    #[test]
    fn date_decode_rejects_reserved_bits() {
        let d = dpt(11, Some(1));
        // Reserved bits set above Day.
        let err = decode(d, &GroupValue::Bytes(vec![0b1000_0000 | 15, 6, 4])).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn date_decode_rejects_out_of_range_day_month_year() {
        let d = dpt(11, Some(1));
        for bytes in [vec![0u8, 6, 4], vec![15u8, 0, 4], vec![15u8, 13, 4]] {
            let err = decode(d, &GroupValue::Bytes(bytes)).unwrap_err();
            assert_eq!(err, DptCodecError::InvalidData { dpt: d });
        }
        // Year octet 100-127 is in range for 7 bits but out of §3.12's
        // documented [0...99].
        let err = decode(d, &GroupValue::Bytes(vec![15, 6, 100])).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn date_decode_rejects_wrong_length_payload() {
        let d = dpt(11, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![15, 6])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 24,
                got: 16
            }
        );
    }

    #[test]
    fn date_encode_rejects_year_outside_the_century_window() {
        let d = dpt(11, Some(1));
        for text in ["1989-01-01", "2090-01-01"] {
            assert!(matches!(
                encode(d, text),
                Err(DptCodecError::OutOfRange { .. })
            ));
        }
    }

    #[test]
    fn date_encode_rejects_unparsable_text() {
        let d = dpt(11, Some(1));
        for text in ["", "not-a-date", "2024/06/15"] {
            assert!(matches!(
                encode(d, text),
                Err(DptCodecError::Unparsable { .. })
            ));
        }
    }

    // -- Main type 12 ------------------------------------------------------

    #[test]
    fn u32_round_trips_min_max_zero_and_interior() {
        let d = dpt(12, Some(1));
        for (text, bytes) in [
            ("0", vec![0x00, 0x00, 0x00, 0x00]),
            ("4294967295", vec![0xFF, 0xFF, 0xFF, 0xFF]),
            ("1", vec![0x00, 0x00, 0x00, 0x01]),
            ("16909060", vec![0x01, 0x02, 0x03, 0x04]),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(bytes.clone()));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Unsigned(text.parse().unwrap())
            );
        }
    }

    #[test]
    fn u32_encode_rejects_out_of_range_and_negative_text() {
        let d = dpt(12, Some(1));
        assert!(matches!(
            encode(d, "4294967296"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-1"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn u32_encode_rejects_unparsable_text() {
        let d = dpt(12, Some(1));
        assert!(matches!(
            encode(d, "abc"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn u32_decode_rejects_wrong_length_payload() {
        let d = dpt(12, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0, 0])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 32,
                got: 24
            }
        );
    }

    // -- Main type 13 ------------------------------------------------------

    #[test]
    fn v32_round_trips_min_max_zero_and_interior() {
        let d = dpt(13, Some(1));
        for (text, bytes) in [
            ("-2147483648", vec![0x80, 0x00, 0x00, 0x00]),
            ("2147483647", vec![0x7F, 0xFF, 0xFF, 0xFF]),
            ("0", vec![0x00, 0x00, 0x00, 0x00]),
            ("-1", vec![0xFF, 0xFF, 0xFF, 0xFF]),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(bytes.clone()));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Signed(text.parse().unwrap())
            );
        }
    }

    #[test]
    fn v32_flow_rate_scaled_round_trips_min_max_zero_and_interior() {
        // 13.002 DPT_FlowRate_m3/h, resolution 0,0001 m3/h.
        let d = dpt(13, Some(2));
        for (text, bytes) in [
            ("-214748.3648", vec![0x80, 0x00, 0x00, 0x00]),
            ("214748.3647", vec![0x7F, 0xFF, 0xFF, 0xFF]),
            ("0", vec![0x00, 0x00, 0x00, 0x00]),
            ("5.5", vec![0x00, 0x00, 0xD6, 0xD8]),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(bytes.clone()));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Float(text.parse().unwrap())
            );
        }
    }

    #[test]
    fn v32_encode_rejects_out_of_range_values() {
        let d = dpt(13, Some(1));
        assert!(matches!(
            encode(d, "2147483648"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-2147483649"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        let flow = dpt(13, Some(2));
        assert!(matches!(
            encode(flow, "214748.3648"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn v32_encode_rejects_unparsable_text() {
        let d = dpt(13, Some(1));
        assert!(matches!(
            encode(d, "abc"),
            Err(DptCodecError::Unparsable { .. })
        ));
        let flow = dpt(13, Some(2));
        assert!(matches!(
            encode(flow, "xyz"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn v32_decode_rejects_wrong_length_payload() {
        let d = dpt(13, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 32,
                got: 16
            }
        );
    }

    // -- Main type 14 ------------------------------------------------------

    #[test]
    fn f32_round_trips_min_max_zero_and_interior() {
        let d = dpt(14, Some(1));
        for (text, expected) in [
            (f32::MIN.to_string(), f32::MIN),
            (f32::MAX.to_string(), f32::MAX),
            ("0".to_string(), 0.0f32),
            ("2.71".to_string(), 2.71f32),
        ] {
            let payload = encode(d, &text).unwrap();
            assert_eq!(
                payload,
                GroupValue::Bytes(expected.to_bits().to_be_bytes().to_vec())
            );
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Float(f64::from(expected))
            );
        }
    }

    #[test]
    fn f32_decode_passes_ieee754_specials_through_unmodified() {
        let d = dpt(14, Some(1));
        let infinity = f32::INFINITY.to_bits().to_be_bytes().to_vec();
        match decode(d, &GroupValue::Bytes(infinity)).unwrap() {
            DptValue::Float(v) => assert!(v.is_infinite() && v.is_sign_positive()),
            other => panic!("expected Float, got {other:?}"),
        }
        let neg_infinity = f32::NEG_INFINITY.to_bits().to_be_bytes().to_vec();
        match decode(d, &GroupValue::Bytes(neg_infinity)).unwrap() {
            DptValue::Float(v) => assert!(v.is_infinite() && v.is_sign_negative()),
            other => panic!("expected Float, got {other:?}"),
        }
        let nan = f32::NAN.to_bits().to_be_bytes().to_vec();
        match decode(d, &GroupValue::Bytes(nan)).unwrap() {
            DptValue::Float(v) => assert!(v.is_nan()),
            other => panic!("expected Float, got {other:?}"),
        }
    }

    #[test]
    fn f32_encode_rejects_non_finite_input() {
        let d = dpt(14, Some(1));
        assert!(matches!(
            encode(d, "NaN"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "inf"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn f32_encode_rejects_a_value_the_32_bit_format_cannot_hold() {
        // Finite as f64, but larger than f32::MAX — encode must refuse it
        // rather than let it become an infinity on the wire.
        let d = dpt(14, Some(1));
        assert!(matches!(
            encode(d, "1e40"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn f32_encode_rejects_unparsable_text() {
        let d = dpt(14, Some(1));
        assert!(matches!(
            encode(d, "banana"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn f32_decode_rejects_wrong_length_payload() {
        let d = dpt(14, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0, 0])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 32,
                got: 24
            }
        );
    }

    // -- Main type 15 -------------------------------------------------------

    #[test]
    fn access_data_matches_example_6() {
        // [D]+[V]: DPT-AS §3.16 EXAMPLE 6, PDF page 47 — code "123456", no
        // error, permission accepted, read left to right, no encryption,
        // index 13. §3.16 prints this example bit by bit with a decimal
        // value row beneath (not as a single hex string); the byte string
        // `[0x12, 0x34, 0x56, 0x4D]` is this codec's own grouping of those
        // printed bits into octets (see the section comment above
        // `decode_access_data`), re-verified against the source PDF.
        let d = dpt(15, Some(0));
        let payload = GroupValue::Bytes(vec![0x12, 0x34, 0x56, 0x4D]);
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::AccessData {
                code: [1, 2, 3, 4, 5, 6],
                error: false,
                accepted: true,
                right_to_left: false,
                encrypted: false,
                index: 13,
            }
        );
        assert_eq!(encode(d, "123456 off on left off 13").unwrap(), payload);
    }

    #[test]
    fn access_data_matches_example_7() {
        // [D]+[V]: DPT-AS §3.16 EXAMPLE 7, PDF page 47 — code "6789"
        // (zero-padded to six digits), no error, permission *not*
        // accepted, read left to right, no encryption, index 14. Byte
        // string `[0x00, 0x67, 0x89, 0x0E]`, grouped from the printed
        // bits the same way as EXAMPLE 6, re-verified against the PDF.
        let d = dpt(15, Some(0));
        let payload = GroupValue::Bytes(vec![0x00, 0x67, 0x89, 0x0E]);
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::AccessData {
                code: [0, 0, 6, 7, 8, 9],
                error: false,
                accepted: false,
                right_to_left: false,
                encrypted: false,
                index: 14,
            }
        );
        assert_eq!(encode(d, "006789 off off left off 14").unwrap(), payload);
    }

    #[test]
    fn access_data_round_trips_error_and_encryption_and_direction() {
        let d = dpt(15, Some(0));
        let payload = encode(d, "999999 on off right on 0").unwrap();
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::AccessData {
                code: [9, 9, 9, 9, 9, 9],
                error: true,
                accepted: false,
                right_to_left: true,
                encrypted: true,
                index: 0,
            }
        );
    }

    #[test]
    fn access_data_display_re_parses() {
        let d = dpt(15, Some(0));
        let v = DptValue::AccessData {
            code: [1, 2, 3, 4, 5, 6],
            error: false,
            accepted: true,
            right_to_left: false,
            encrypted: false,
            index: 13,
        };
        let text = v.to_string();
        assert_eq!(encode(d, &text).and_then(|p| decode(d, &p)), Ok(v));
    }

    #[test]
    fn access_data_decode_rejects_a_bcd_digit_above_nine() {
        let d = dpt(15, Some(0));
        // D6 nibble = 0xA (10), not a decimal digit.
        let err = decode(d, &GroupValue::Bytes(vec![0xA0, 0x00, 0x00, 0x00])).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn access_data_decode_rejects_wrong_length_payload() {
        let d = dpt(15, Some(0));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0, 0])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 32,
                got: 24
            }
        );
    }

    #[test]
    fn access_data_encode_rejects_index_above_fifteen() {
        let d = dpt(15, Some(0));
        assert!(matches!(
            encode(d, "123456 off on left off 16"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn access_data_encode_rejects_unparsable_text() {
        let d = dpt(15, Some(0));
        for text in ["", "12345 off on left off 13", "abcdef off on left off 13"] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
    }

    // -- Main type 16 ------------------------------------------------------

    #[test]
    fn a14_worked_example_round_trips_byte_for_byte() {
        // DPT-AS §3.17 EXAMPLE 8: 'KNX is OK' encoded as
        // 4B 4E 58 20 69 73 20 4F 4B 00 00 00 00 00.
        let expected = vec![
            0x4B, 0x4E, 0x58, 0x20, 0x69, 0x73, 0x20, 0x4F, 0x4B, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        for d in [dpt(16, Some(0)), dpt(16, Some(1))] {
            let payload = encode(d, "KNX is OK").unwrap();
            assert_eq!(payload, GroupValue::Bytes(expected.clone()));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Text("KNX is OK".to_string())
            );
        }
    }

    #[test]
    fn a14_round_trips_empty_and_full_length_strings() {
        let d = dpt(16, Some(1));
        assert_eq!(encode(d, "").unwrap(), GroupValue::Bytes(vec![0; 14]));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0; 14])).unwrap(),
            DptValue::Text(String::new())
        );
        let full = "12345678901234"; // exactly 14 characters, no padding
        let payload = encode(d, full).unwrap();
        assert_eq!(payload, GroupValue::Bytes(full.bytes().collect()));
        assert_eq!(
            decode(d, &payload).unwrap(),
            DptValue::Text(full.to_string())
        );
    }

    #[test]
    fn a14_bare_main_type_is_the_iso_8859_1_form() {
        // 0xE9 is 'é' in ISO 8859-1 — outside 16.000's 7-bit ASCII set,
        // but valid for the bare `DPT-16` / 16.001 form.
        let d = dpt(16, None);
        let mut bytes = vec![0xE9];
        bytes.resize(14, 0);
        assert_eq!(
            decode(d, &GroupValue::Bytes(bytes)).unwrap(),
            DptValue::Text("é".to_string())
        );
        let mut expected = vec![0xE9];
        expected.resize(14, 0);
        assert_eq!(encode(d, "é").unwrap(), GroupValue::Bytes(expected));
    }

    #[test]
    fn a14_16000_rejects_the_top_bit_in_both_directions() {
        let d = dpt(16, Some(0));
        let mut bytes = vec![0xE9];
        bytes.resize(14, 0);
        assert_eq!(
            decode(d, &GroupValue::Bytes(bytes)).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
        assert!(matches!(
            encode(d, "é"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn a14_encode_rejects_a_string_longer_than_14_characters() {
        let d = dpt(16, Some(1));
        assert!(matches!(
            encode(d, "123456789012345"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn a14_decode_rejects_wrong_length_payload() {
        let d = dpt(16, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0; 13])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 112,
                got: 104
            }
        );
    }

    #[test]
    fn a14_unsupported_subtype_is_reported() {
        let d = dpt(16, Some(2));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0; 14])),
            Err(DptCodecError::UnsupportedDpt(d))
        );
        assert_eq!(encode(d, "x"), Err(DptCodecError::UnsupportedDpt(d)));
    }

    // -- Main type 17 ------------------------------------------------------

    #[test]
    fn scene_round_trips_min_max_and_interior() {
        let d = dpt(17, Some(1));
        for (text, raw) in [("0", 0u8), ("63", 63u8), ("30", 30u8)] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Short(raw));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Scene { number: raw }
            );
        }
    }

    #[test]
    fn scene_accepted_in_both_the_inline_form_and_a_single_octet() {
        let d = dpt(17, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Short(42)).unwrap(),
            DptValue::Scene { number: 42 }
        );
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![42])).unwrap(),
            DptValue::Scene { number: 42 }
        );
    }

    #[test]
    fn scene_decode_rejects_bits_above_the_six_significant_bits() {
        let d = dpt(17, Some(1));
        let err = decode(d, &GroupValue::Short(0b0100_0000)).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn scene_encode_rejects_out_of_range_number() {
        let d = dpt(17, Some(1));
        assert!(matches!(
            encode(d, "64"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn scene_encode_rejects_unparsable_text() {
        let d = dpt(17, Some(1));
        assert!(matches!(
            encode(d, "abc"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn scene_decode_rejects_wrong_length_payload() {
        let d = dpt(17, Some(1));
        let err = decode(d, &GroupValue::Bytes(vec![0, 0])).unwrap_err();
        assert_eq!(
            err,
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 6,
                got: 16
            }
        );
    }

    // -- Main type 18 ------------------------------------------------------

    #[test]
    fn scene_control_round_trips_min_max_zero_and_interior() {
        let d = dpt(18, Some(1));
        for (text, raw, learn, number) in [
            ("activate scene 0", 0b0000_0000u8, false, 0u8),
            ("learn scene 63", 0b1011_1111u8, true, 63u8),
            ("activate scene 63", 0b0011_1111u8, false, 63u8),
            ("learn scene 30", 0b1001_1110u8, true, 30u8),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![raw]));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::SceneControl { learn, number }
            );
        }
    }

    #[test]
    fn scene_control_rejected_in_the_inline_form() {
        // Seven significant bits (C + SceneNumber) exceed the ≤6-bit
        // inline threshold — unlike main type 17, a `Short` payload is
        // simply the wrong shape, not an alternative encoding of it.
        let d = dpt(18, Some(1));
        let err = decode(d, &GroupValue::Short(5)).unwrap_err();
        assert_eq!(
            err,
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 8,
                got: 6
            }
        );
    }

    #[test]
    fn scene_control_encode_never_produces_the_inline_form() {
        let d = dpt(18, Some(1));
        let payload = encode(d, "activate scene 5").unwrap();
        assert!(matches!(payload, GroupValue::Bytes(_)));
    }

    #[test]
    fn scene_control_decode_rejects_the_reserved_bit() {
        let d = dpt(18, Some(1));
        let err = decode(d, &GroupValue::Bytes(vec![0b0100_0000])).unwrap_err();
        assert_eq!(err, DptCodecError::InvalidData { dpt: d });
    }

    #[test]
    fn scene_control_encode_rejects_out_of_range_scene_number() {
        let d = dpt(18, Some(1));
        assert!(matches!(
            encode(d, "activate scene 64"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn scene_control_encode_rejects_unparsable_text() {
        let d = dpt(18, Some(1));
        for text in ["banana", "activate 5", "perhaps scene 5", "activate scene"] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
    }

    #[test]
    fn scene_control_decode_rejects_wrong_length_payload() {
        let d = dpt(18, Some(1));
        let err = decode(d, &GroupValue::Bytes(vec![0, 0])).unwrap_err();
        assert_eq!(
            err,
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 8,
                got: 16
            }
        );
    }

    // -- Main type 19 -------------------------------------------------------

    // Plain (non-enum) mirror of `DptValue::DateTime`'s fields, so tests
    // can use ordinary struct-update syntax (`..Dt::valid()`) to vary one
    // or two fields at a time — Rust's functional record update does not
    // apply to enum struct variants directly.
    #[derive(Clone)]
    struct Dt {
        year: u16,
        year_invalid: bool,
        month: u8,
        day_of_month: u8,
        date_invalid: bool,
        day_of_week: u8,
        day_of_week_invalid: bool,
        hour: u8,
        minute: u8,
        second: u8,
        time_invalid: bool,
        fault: bool,
        working_day: bool,
        working_day_unknown: bool,
        summer_time: bool,
        externally_synchronized: bool,
    }

    impl Dt {
        fn valid() -> Self {
            Dt {
                year: 2024,
                year_invalid: false,
                month: 6,
                day_of_month: 15,
                date_invalid: false,
                day_of_week: 6,
                day_of_week_invalid: false,
                hour: 12,
                minute: 30,
                second: 45,
                time_invalid: false,
                fault: false,
                working_day: true,
                working_day_unknown: false,
                summer_time: true,
                externally_synchronized: true,
            }
        }
    }

    impl From<Dt> for DptValue {
        fn from(d: Dt) -> DptValue {
            DptValue::DateTime {
                year: d.year,
                year_invalid: d.year_invalid,
                month: d.month,
                day_of_month: d.day_of_month,
                date_invalid: d.date_invalid,
                day_of_week: d.day_of_week,
                day_of_week_invalid: d.day_of_week_invalid,
                hour: d.hour,
                minute: d.minute,
                second: d.second,
                time_invalid: d.time_invalid,
                fault: d.fault,
                working_day: d.working_day,
                working_day_unknown: d.working_day_unknown,
                summer_time: d.summer_time,
                externally_synchronized: d.externally_synchronized,
            }
        }
    }

    #[test]
    fn datetime_round_trips_a_fully_valid_value() {
        // [own reading]: no worked byte example is printed for §3.20;
        // this codec's own arithmetic from the field diagram.
        let d = dpt(19, Some(1));
        let value: DptValue = Dt::valid().into();
        let text = value.to_string();
        let payload = encode(d, &text).unwrap();
        assert_eq!(
            payload,
            GroupValue::Bytes(vec![
                124,
                6,
                15,
                (6 << 5) | 12,
                30,
                45,
                0b0100_0001,
                0b1000_0000
            ])
        );
        assert_eq!(decode(d, &payload).unwrap(), value);
    }

    #[test]
    fn datetime_day_of_week_wildcard_and_invalid_are_distinct() {
        // Note 14: `NDoW=0, ddd=0` is the "any day" wildcard (valid);
        // `NDoW=1` means the ddd field itself is not valid. Both must
        // decode to different `DptValue`s, not collapse into one.
        let d = dpt(19, Some(1));
        let wildcard: DptValue = Dt {
            day_of_week: 0,
            day_of_week_invalid: false,
            ..Dt::valid()
        }
        .into();
        let invalid: DptValue = Dt {
            day_of_week: 0,
            day_of_week_invalid: true,
            ..Dt::valid()
        }
        .into();
        assert_ne!(wildcard, invalid);

        let wildcard_payload = encode(d, &wildcard.to_string()).unwrap();
        let invalid_payload = encode(d, &invalid.to_string()).unwrap();
        assert_ne!(wildcard_payload, invalid_payload);
        assert_eq!(decode(d, &wildcard_payload).unwrap(), wildcard);
        assert_eq!(decode(d, &invalid_payload).unwrap(), invalid);
    }

    #[test]
    fn datetime_hour_24_is_legal_only_with_zero_minutes_and_seconds() {
        // Note 11.
        let d = dpt(19, Some(1));
        let ok: DptValue = Dt {
            hour: 24,
            minute: 0,
            second: 0,
            ..Dt::valid()
        }
        .into();
        let payload = encode(d, &ok.to_string()).unwrap();
        assert_eq!(decode(d, &payload).unwrap(), ok);

        let bad: DptValue = Dt {
            hour: 24,
            minute: 1,
            ..Dt::valid()
        }
        .into();
        assert!(matches!(
            encode(d, &bad.to_string()),
            Err(DptCodecError::OutOfRange { .. })
        ));
        // Build the illegal wire form directly, bypassing encode's own
        // validation, to check decode rejects it too.
        let bytes = vec![124, 6, 15, (6 << 5) | 24, 1, 0, 0b0100_0001, 0b1000_0000];
        assert_eq!(
            decode(d, &GroupValue::Bytes(bytes)).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
    }

    #[test]
    fn datetime_no_date_skips_month_and_day_range_enforcement() {
        // ND=1: this codec accepts an otherwise out-of-range Month/Day
        // rather than rejecting a "no date" clock's zero/garbage fill —
        // see the section comment's range-enforcement policy.
        let d = dpt(19, Some(1));
        let bytes = vec![124, 0, 0, (6 << 5) | 12, 30, 45, 0b0100_1001, 0b1000_0000];
        let decoded = decode(d, &GroupValue::Bytes(bytes)).unwrap();
        let expected: DptValue = Dt {
            date_invalid: true,
            month: 0,
            day_of_month: 0,
            ..Dt::valid()
        }
        .into();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn datetime_date_valid_still_enforces_month_and_day_range() {
        let d = dpt(19, Some(1));
        // Month 13 is out of [1...12] and ND=0 (date fields valid).
        let bytes = vec![124, 13, 15, (6 << 5) | 12, 30, 45, 0b0100_0001, 0b1000_0000];
        assert_eq!(
            decode(d, &GroupValue::Bytes(bytes)).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
    }

    #[test]
    fn datetime_no_time_skips_hour_minute_second_range_enforcement() {
        let d = dpt(19, Some(1));
        // Hour field raw 31 (max for 5 bits), Minute/Second raw 63 (max
        // for 6 bits) — all out of the documented ranges, but NT=1.
        let bytes = vec![124, 6, 15, (6 << 5) | 31, 63, 63, 0b0100_0011, 0b1000_0000];
        let decoded = decode(d, &GroupValue::Bytes(bytes)).unwrap();
        let expected: DptValue = Dt {
            time_invalid: true,
            hour: 31,
            minute: 63,
            second: 63,
            ..Dt::valid()
        }
        .into();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn datetime_decode_rejects_reserved_bits() {
        let d = dpt(19, Some(1));
        let base = |o1: u8, o2: u8, o3: u8, o4: u8, o6: u8, o7: u8| {
            GroupValue::Bytes(vec![124, o7, o6, (6 << 5) | 12, o4, o3, o2, o1])
        };
        // Reserved bits above Month (o7), above Day (o6), above Minutes
        // (o4), above Seconds (o3), and bit 0 of octet 1 (o1), each
        // checked independently. Octet 1's bit 6 — the disputed "SRC"
        // bit — has its own dedicated test below
        // (`datetime_decode_rejects_the_disputed_src_bit_position`)
        // rather than being folded into this generic sweep. Bits 1-5 of
        // octet 1 fall under the same mask and are not exercised here.
        let cases = [
            base(0b1000_0000, 0b0100_0001, 45, 30, 15, 0b1001_0110), // above Month
            base(0b1000_0000, 0b0100_0001, 45, 30, 0b0010_0000, 6),  // above Day
            base(0b1000_0000, 0b0100_0001, 45, 0b1100_0000, 15, 6),  // above Minutes
            base(0b1000_0000, 0b0100_0001, 0b1100_0000, 30, 15, 6),  // above Seconds
            base(0b0000_0001, 0b0100_0001, 45, 30, 15, 6),           // low bits of octet 1
        ];
        for payload in cases {
            assert_eq!(
                decode(d, &payload).unwrap_err(),
                DptCodecError::InvalidData { dpt: d },
                "expected InvalidData for {payload:?}"
            );
        }
    }

    #[test]
    fn datetime_decode_rejects_the_disputed_src_bit_position() {
        // Octet 1 bit 6: the field-*names* row of DPT-AS §3.20's octet 1
        // diagram calls this bit `SRC`, but the bit-*encoding* row
        // directly beneath it, and Note 15, both mark it `r` (reserved).
        // This codec follows the encoding row and Note 15 — a stated
        // ruling ([A]), not a missing-bit default — so bit 6 is policed
        // exactly like the octet's other six reserved bits. See the
        // section comment above `decode_datetime` for the full account
        // of the contradiction this pins.
        let d = dpt(19, Some(1));
        let bytes = vec![124, 6, 15, (6 << 5) | 12, 30, 45, 0b0100_0001, 0b0100_0000];
        assert_eq!(
            decode(d, &GroupValue::Bytes(bytes)).unwrap_err(),
            DptCodecError::InvalidData { dpt: d }
        );
    }

    #[test]
    fn datetime_not_valid_branches_encode_back_to_the_same_bytes() {
        // The "no date"/"no time" exemptions from range enforcement had
        // decode-only tests, so `encode`'s matching branch went
        // unexercised. Both round trips, byte for byte.
        let d = dpt(19, Some(1));

        let no_date_bytes = vec![124, 0, 0, (6 << 5) | 12, 30, 45, 0b0100_1001, 0b1000_0000];
        let no_date_payload = GroupValue::Bytes(no_date_bytes);
        let no_date_decoded = decode(d, &no_date_payload).unwrap();
        assert_eq!(
            encode(d, &no_date_decoded.to_string()).unwrap(),
            no_date_payload
        );

        let no_time_bytes = vec![124, 6, 15, (6 << 5) | 31, 63, 63, 0b0100_0011, 0b1000_0000];
        let no_time_payload = GroupValue::Bytes(no_time_bytes);
        let no_time_decoded = decode(d, &no_time_payload).unwrap();
        assert_eq!(
            encode(d, &no_time_decoded.to_string()).unwrap(),
            no_time_payload
        );
    }

    #[test]
    fn datetime_decode_rejects_wrong_length_payload() {
        let d = dpt(19, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0; 7])).unwrap_err(),
            DptCodecError::WrongLength {
                dpt: d,
                expected_bits: 64,
                got: 56
            }
        );
    }

    #[test]
    fn datetime_encode_rejects_unparsable_text() {
        let d = dpt(19, Some(1));
        for text in ["", "year=2024", "year=abcd month=6 day=15 dow=6 hour=12 minute=30 second=45 fault=off workday=on no-workday=off no-year=off no-date=off no-dow=off no-time=off summer-time=on synced=on"] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
    }

    // -- Unsupported main types -------------------------------------------

    #[test]
    fn unimplemented_main_type_is_unsupported_not_a_panic() {
        // Main type 9 graduated out of "unimplemented" this task — main
        // type 20 (1-octet enumeration) is spec §4.3's deliberately
        // excluded type: its wire value is a bare octet whose *meaning*
        // lives in enumeration tables this codec does not ingest, so
        // decoding it would be a guess dressed up as an answer.
        let d = dpt(20, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0])),
            Err(DptCodecError::UnsupportedDpt(d))
        );
        assert_eq!(encode(d, "21.0"), Err(DptCodecError::UnsupportedDpt(d)));
    }

    // -- Formatting --------------------------------------------------------

    #[test]
    fn format_matches_the_documented_examples() {
        assert_eq!(DptValue::Bool(true).format(dpt(1, Some(1))), "on");
        assert_eq!(DptValue::Bool(false).format(dpt(1, Some(1))), "off");
        assert_eq!(DptValue::Float(-3.4).format(dpt(9, Some(1))), "-3.4");
        assert_eq!(DptValue::Float(50.0).format(dpt(5, Some(1))), "50 %");
        assert_eq!(
            DptValue::Step {
                increase: true,
                step_code: 3
            }
            .format(dpt(3, Some(7))),
            "increase step 3"
        );
    }

    #[test]
    fn format_shows_no_unit_for_raw_counts() {
        assert_eq!(DptValue::Unsigned(128).format(dpt(5, None)), "128");
    }
}
