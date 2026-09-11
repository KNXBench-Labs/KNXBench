//! Encode/decode between engineering values (text) and `GroupValue` wire
//! payloads, for the datapoint types this slice implements: main types 1,
//! 2, 3, 5, 6, 7, 8 (03_07_02 Datapoint Types v02.02.01 AS, hereafter
//! "DPT-AS"). Main types 9, 12, 13, 14, 16, 17, 18 are a later task —
//! `DptValue` already declares the variants they will use (see each
//! variant's doc comment) so the wire-format vocabulary does not change
//! shape when they land.
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
    /// U16 (main type 7), and U32 (main type 12 — a later task).
    Unsigned(u32),
    /// Two's-complement signed counts: V8 (main type 6, except 6.020),
    /// V16 (main type 8, except 8.010), and V32 (main type 13 — a later
    /// task).
    Signed(i32),
    /// Engineering values that need a fractional/scaled representation:
    /// main type 5's scaled subtypes (5.001 DPT_Scaling, 5.003 DPT_Angle),
    /// main type 8's 8.010 DPT_Percent_V16, and — a later task — the F16
    /// and F32 float families (main types 9 and 14). The codec's job is
    /// the number, not its storage width, so one variant serves all of
    /// them.
    Float(f64),
    /// A[14] fixed-length character strings — main type 16, a later task.
    Text(String),
    /// Scene number — main type 17, a later task.
    Scene { number: u8 },
    /// Scene control (learn/activate + scene number) — main type 18, a
    /// later task.
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
        5 => decode_u8(dpt, payload),
        6 => decode_v8(dpt, payload),
        7 => decode_u16(dpt, payload),
        8 => decode_v16(dpt, payload),
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
        5 => encode_u8(dpt, input),
        6 => encode_v8(dpt, input),
        7 => encode_u16(dpt, input),
        8 => encode_v16(dpt, input),
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
        // 5.001 DPT_Scaling (DPT-AS §3.31.A, the one place the exact
        // formula is spelled out: "Encoding: 0 %...100 %. Full Datapoint
        // Type value: 0...255, i.e. 1 % = value 255/100"). Confirmed
        // against §3.5.1's own worked example (NOTE 4): 50 % -> 80h,
        // 100 % -> FFh.
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
        let d = dpt(2, Some(1));
        for (text, raw, control, value) in [
            ("no control", 0b00u8, false, false),
            ("control off", 0b10u8, true, false),
            ("control on", 0b11u8, true, true),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Short(raw));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::ControlBool { control, value }
            );
        }
    }

    #[test]
    fn b2_decode_reads_the_no_control_value_bit_too() {
        // c=0, v=1: "no control", but the codec does not discard v — the
        // Standard only says v is don't-care, not that it must be 0.
        let d = dpt(2, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Short(0b01)).unwrap(),
            DptValue::ControlBool {
                control: false,
                value: true
            }
        );
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

    #[test]
    fn b2_unsupported_main_type_is_reported() {
        assert_eq!(
            decode(dpt(9, Some(1)), &GroupValue::Bytes(vec![0, 0])),
            Err(DptCodecError::UnsupportedDpt(dpt(9, Some(1))))
        );
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

    // -- Unsupported main types -------------------------------------------

    #[test]
    fn unimplemented_main_type_is_unsupported_not_a_panic() {
        let d = dpt(9, Some(1));
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
