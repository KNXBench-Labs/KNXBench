//! Encode/decode between engineering values (text) and `GroupValue` wire
//! payloads, for main types 1-30: the fourteen the original design doc's
//! §4.1 lists (`docs/superpowers/specs/2026-09-11-dpt-codec-design.md` —
//! eleven marked in its table, and 6, 12 and 16 named in the paragraph
//! below it), plus 4, 10, 11, 15, and 19, added by task E4 (2026-09-13),
//! plus 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30 and the one subtype the
//! earlier slices carved out of main type 6 (`6.020 DPT_Status_Mode3`),
//! added by E4's second slice the same day — see
//! `docs/KNOWN_LIMITATIONS.md` §61 for those extensions' per-type
//! citations and judgment calls (03_07_02 Datapoint Types v02.02.01 AS,
//! hereafter "DPT-AS").
//!
//! What this codec deliberately does *not* do, uniformly, for every main
//! type whose subtypes carry their own semantic table: it does not resolve
//! an enumeration code to its name, does not enforce a subtype's narrowed
//! range, and does not reject a bit a subtype table calls "reserved". The
//! formats it implements (`N8`, `N2`, `Z8`, `B8`, `B16`, `B24`, `B32`,
//! `U4U4`)
//! are defined at the *format* level in DPT-AS and are implemented at that
//! level; every raw field reaches the caller intact. See
//! `docs/KNOWN_LIMITATIONS.md` §61 for the reasoning and the Standard's
//! own self-contradiction (22.100) that makes the alternative worse.
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
    /// well as fourteen. Also the two variable-length `A[n]` strings:
    /// main type 24 (ISO 8859-1, DPT-AS §3.24) and main type 28 (UTF-8,
    /// DPT-AS §3.27). The NULL octet those two formats use as their
    /// terminator is a wire-framing detail and is not part of the
    /// `String` — see `require_var_string_body`.
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
    /// A raw enumeration code: `N8` — main type 20 (DPT-AS §3.21,
    /// "1 octet: N8", "Encoding absolute value N = [0 … 255]") — and
    /// `N2` — main type 23 (DPT-AS §3.23/§4.6, "2 bit: N2", encoding
    /// row `NN`). `code` is the code exactly as transmitted; this codec
    /// neither resolves it to the name its subtype's table gives it nor
    /// refuses a code that table leaves unassigned (see the module
    /// header).
    Enum { code: u8 },
    /// A bit set transmitted as whole octets, most significant octet
    /// first: `Z8`/`B8` — main type 21 (DPT-AS §3.22.1 "1 octet: Z8",
    /// §3.22.2 "1 octet: B8"), `B16` — main type 22 (DPT-AS §4.5,
    /// §8.3, "2 octets: B16"), `B24` — main type 30 (DPT-AS §8.5,
    /// "3 octets: B24") and `B32` — main type 27 (DPT-AS §3.26,
    /// "4 octets: B32"). Bit `b_n` of the Standard's own numbering sits
    /// at bit `n` of `bits`, so `b0` is the least significant bit of the
    /// last octet transmitted. `width` is 8, 16, 24 or 32 and says how
    /// many bits the format defines; bits above it are always zero.
    BitSet { bits: u32, width: u8 },
    /// `U4U4` — main type 25 (DPT-AS §8.4, "1 octet: U4U4", "All field
    /// values binary encoded"). `busy` and `nak` are §8.4's own two
    /// field names, left to right in its field-names row, so `busy`
    /// occupies bits 7-4 and `nak` bits 3-0. Both are `0..=15`, the
    /// format's own `U4` range; 25.1000 DPT_DoubleNibble's narrower
    /// `[0 … 3]` is a subtype range and is not enforced here (see the
    /// module header).
    DoubleNibble { busy: u8, nak: u8 },
    /// `r1B1U6` — main type 26 (DPT-AS §3.25, encoding row
    /// `0 b UUUUUU`). `inactive` is field `B` in the Standard's own
    /// polarity — §3.25 reads "0 = scene is active, 1 = scene is
    /// inactive", so `true` means the scene is *inactive*. `number` is
    /// the undecorated 6-bit wire value 0-63, like `Scene`'s and
    /// `SceneControl`'s, even though §3.25 NOTE 16 (the note main types
    /// 17 and 18 both cite) recommends a +1 display offset.
    SceneInfo { inactive: bool, number: u8 },
    /// `B5N3` — the one subtype main type 6's `V8` arm excludes, 6.020
    /// DPT_Status_Mode3 (DPT-AS §3.7, "8 bit: B5N3", encoding row
    /// `B B B B B NNN`). `status` holds §3.7's fields `a`..`e` in that
    /// order, so `status[0]` is `a` at bit 7 and `status[4]` is `e` at
    /// bit 3; 6.020's own row spells the same five fields in upper case
    /// and gives their polarity as "0 = set, 1 = clear", which this codec
    /// transmits rather than normalises. `mode_code` is the 3-bit field
    /// `f`, which §3.7's Range row restricts to `{001b,010b,100b}`.
    StatusMode3 { status: [bool; 5], mode_code: u8 },
    /// `V64` — main type 29 (DPT-AS §3.28.1, "8 octets: V64", "Two's
    /// complement notation", resolution 1 of the subtype's unit). Kept
    /// separate from `Signed` so the `V8`/`V16`/`V32` families keep their
    /// documented 32-bit domain rather than being widened by a type they
    /// do not share a range with.
    Signed64(i64),
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
            DptValue::Signed64(v) => write!(f, "{v}"),
            DptValue::Enum { code } => write!(f, "{code}"),
            DptValue::BitSet { bits, width } => {
                let w = usize::from(*width);
                write!(f, "{bits:0w$b}")
            }
            DptValue::DoubleNibble { busy, nak } => write!(f, "busy {busy} nak {nak}"),
            DptValue::SceneInfo { inactive, number } => {
                write!(
                    f,
                    "{} scene {number}",
                    if *inactive { "inactive" } else { "active" }
                )
            }
            DptValue::StatusMode3 { status, mode_code } => {
                let digits: String = status.iter().map(|s| if *s { '1' } else { '0' }).collect();
                write!(f, "status {digits} mode {mode_code:03b}")
            }
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
        20 => decode_n8(dpt, payload),
        21 => decode_bit_set::<1>(dpt, payload),
        22 => decode_bit_set::<2>(dpt, payload),
        23 => decode_n2(dpt, payload),
        24 => decode_var_string_8859_1(dpt, payload),
        25 => decode_double_nibble(dpt, payload),
        26 => decode_scene_info(dpt, payload),
        27 => decode_bit_set::<4>(dpt, payload),
        28 => decode_var_string_utf8(dpt, payload),
        29 => decode_v64(dpt, payload),
        30 => decode_bit_set::<3>(dpt, payload),
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
        20 => encode_n8(dpt, input),
        21 => encode_bit_set::<1>(dpt, input),
        22 => encode_bit_set::<2>(dpt, input),
        23 => encode_n2(dpt, input),
        24 => encode_var_string_8859_1(dpt, input),
        25 => encode_double_nibble(dpt, input),
        26 => encode_scene_info(dpt, input),
        27 => encode_bit_set::<4>(dpt, input),
        28 => encode_var_string_utf8(dpt, input),
        29 => encode_v64(dpt, input),
        30 => encode_bit_set::<3>(dpt, input),
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

/// Extracts the octets of a variable-length, NULL-terminated character
/// string — main type 24 (DPT-AS §3.24) and main type 28 (DPT-AS
/// §3.27), which share this framing exactly. §3.24: "the string shall be
/// terminated by a single character NULL (00h). No length information
/// shall be transmitted in the APDU"; footnote 18 to the same section:
/// "The NULL character is actually part of the DPT_VarString_8859_1
/// format." §3.27: "The string shall be terminated by the NULL- character
/// (00h). No length information shall be transmitted in the APDU". So the
/// shortest legal payload is the terminator alone, which rules out an
/// empty `Bytes`; and an octet string is wider than AL-AS's 6-bit inline
/// threshold, so a `GroupValue::Short` is never the right shape either.
/// `expected_bits` is reported as 8, the minimum — the format has no
/// fixed length to report.
///
/// Ruling (recorded in `docs/KNOWN_LIMITATIONS.md` §61): a payload whose
/// last octet is not 00h, or which carries an interior 00h, is
/// `InvalidData`. Neither section says what a receiver should do with
/// one, and every tolerant reading is a guess about a non-conforming
/// sender's intent — cutting at the first 00h assumes the rest is
/// padding, appending a terminator assumes one was lost. Refusing states
/// the fact without inventing a value. (§3.24 and §3.27 do prescribe
/// "cut to the maximum supported length" for a string that is *too long*
/// for the receiver; that is a length limit belonging to whatever layer
/// owns the APDU budget, not a rule about a missing terminator, and this
/// codec imposes no maximum of its own.)
fn require_var_string_body(payload: &GroupValue, dpt: DptRef) -> Result<&[u8], DptCodecError> {
    let wrong_length = |got: u32| DptCodecError::WrongLength {
        dpt,
        expected_bits: 8,
        got,
    };
    let raw = match payload {
        GroupValue::Bytes(b) => b.as_slice(),
        GroupValue::Short(_) => return Err(wrong_length(6)),
    };
    let Some((last, body)) = raw.split_last() else {
        return Err(wrong_length(0));
    };
    if *last != 0x00 || body.contains(&0x00) {
        return Err(DptCodecError::InvalidData { dpt });
    }
    Ok(body)
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
// format under the same main number: DPT-AS §3.7 gives it as "8 bit:
// B5N3" with the encoding row `B B B B B NNN` over the field-names row
// `a b c d e` / `f` — five status bits at bits 7-3, then a 3-bit mode
// field at bits 2-0. §3.7's Range row reads `a, b, c, d, e = {0,1}` and
// `f = {001b,010b,100b}`; 6.020's datapoint-type row names the same
// fields in upper case and gives their meaning: "A,B,C,D,E: 0 = set,
// 1 = clear" and "FFF: 001b = mode 0 is active, 010b = mode 1 is active,
// 100b = mode 2 is active".
//
// It therefore decodes to its own `DptValue::StatusMode3`, not into
// `Signed`, where it would be silently misread as a two's-complement
// number it is not. A mode field outside the three assigned one-hot codes
// is `InvalidData`: that restriction is stated for the *format* in §3.7's
// own Range row, so enforcing it is not the per-subtype table checking
// the module header rules out. Eight significant bits puts this above the
// AL-AS §3.1.3 inline threshold, like the rest of main type 6, so the
// payload is always `Bytes([_])`.
//
// Text grammar (this module's choice — the Standard specifies wire
// encoding, not human text): `status abcde mode fff`, e.g.
// `status 01001 mode 010`. Both fields are printed as the binary digits
// the Standard itself prints, `a` first and most significant. The status
// bits are deliberately not rendered as set/clear words: 6.020's polarity
// inverts the usual reading (`0` is *set*), so a word would invite
// exactly the misreading the digits prevent.

fn decode_v8(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    if dpt.sub == Some(20) {
        return decode_status_mode3(dpt, payload);
    }
    let [raw] = require_bytes(payload, dpt, 8)?;
    Ok(DptValue::Signed(i32::from(raw as i8)))
}

fn encode_v8(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    if dpt.sub == Some(20) {
        return encode_status_mode3(dpt, input);
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

/// The `f` field's three assigned codes (DPT-AS §3.7 Range row:
/// `f = {001b,010b,100b}`). Any other 3-bit value is unassigned.
fn mode3_code_is_assigned(code: u8) -> bool {
    matches!(code, 0b001 | 0b010 | 0b100)
}

fn decode_status_mode3(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [raw] = require_bytes::<1>(payload, dpt, 8)?;
    let mode_code = raw & 0b0000_0111;
    if !mode3_code_is_assigned(mode_code) {
        return Err(DptCodecError::InvalidData { dpt });
    }
    let mut status = [false; 5];
    for (i, bit) in status.iter_mut().enumerate() {
        *bit = raw & (0b1000_0000 >> i) != 0;
    }
    Ok(DptValue::StatusMode3 { status, mode_code })
}

fn encode_status_mode3(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let (status_digits, mode_digits) = match tokens.as_slice() {
        [s, status, m, mode]
            if s.eq_ignore_ascii_case("status") && m.eq_ignore_ascii_case("mode") =>
        {
            (*status, *mode)
        }
        _ => return Err(unparsable()),
    };
    if status_digits.len() != 5 || mode_digits.len() != 3 {
        return Err(unparsable());
    }
    let status_bits = u8::from_str_radix(status_digits, 2).map_err(|_| unparsable())?;
    let mode_code = u8::from_str_radix(mode_digits, 2).map_err(|_| unparsable())?;
    if !mode3_code_is_assigned(mode_code) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: mode_digits.to_string(),
        });
    }
    Ok(GroupValue::Bytes(vec![(status_bits << 3) | mode_code]))
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

// ---------------------------------------------------------------------
// Main type 20 — N8, 1-octet enumeration (DPT-AS §3.21)
// ---------------------------------------------------------------------
//
// "Format: 1 octet: N8", field-names row `field1`, encoding row
// `NNNNNNNN`, "Encoding: Encoding absolute value N = [0 … 255]", unit
// none, PDT_ENUM8 (alt: PDT_UNSIGNED_CHAR) — DPT-AS §3.21. Eight
// significant bits is above the AL-AS §3.1.3 inline threshold, so the
// payload is always one octet of its own, `Bytes([_])`, never `Short`.
//
// Ruling, no per-subtype enumeration check — this is the ruling the
// module header states, recorded here once in full because main type 20
// is where it bites hardest, and cited from the other affected types
// (21, 22, 23, 25, 27, 30):
//
// Main type 20 has sixty-eight subtypes in DPT-AS, each with its own
// value table, and those tables are not gathered in §3.21. They are
// spread over §3.21 (20.001-20.022, sixteen), §4.3 (20.100-20.122,
// nineteen), §6.3 (20.600-20.613, fourteen), §7.1 (20.801-20.804, four),
// §8.1 (20.1000-20.1005, six) and §9.5 (20.1200-20.1209, nine), keyed
// by application domain, and their wording for an unassigned code varies
// between "not used; reserved" (20.001, 20.003), "reserved, shall not be
// used" (20.002) and a bare "reserved" (20.005's "3 to 16 : reserved").
// Enforcing them here would mean transcribing sixty-eight tables into
// this codec, where every transcription slip silently *rejects* a legal
// bus value — the failure mode this
// project's correctness ordering likes least — or enforcing some and not
// others, which is worse than enforcing none because it makes the codec's
// behaviour unpredictable per subtype. So this codec validates what §3.21
// states for the format: one octet, all 256 codes legal. Nothing is
// discarded — the raw code reaches the caller in `DptValue::Enum` — and a
// caller that does know its subtype's table can apply it. The
// commissioning layer's `DPT_ErrorClass_System` (20.011) mapping in
// `docs/superpowers/specs/2026-09-13-commissioning-download-design.md`
// §5.6 is exactly such a caller, and its `[0 to 18]` range remains
// correct at that layer. Recorded in `docs/KNOWN_LIMITATIONS.md` §61.
//
// Text grammar (this module's choice): the bare decimal code, `0`-`255`.
// Not a name — a name could only come from the per-subtype table this
// codec deliberately does not carry.

fn decode_n8(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [raw] = require_bytes::<1>(payload, dpt, 8)?;
    Ok(DptValue::Enum { code: raw })
}

fn encode_n8(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let code: u32 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    if code > u32::from(u8::MAX) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: trimmed.to_string(),
        });
    }
    Ok(GroupValue::Bytes(vec![code as u8]))
}

// ---------------------------------------------------------------------
// Main types 21, 22, 27 and 30 — bit sets
// (DPT-AS §3.22, §4.5, §8.3, §3.26, §8.5)
// ---------------------------------------------------------------------
//
// Four main types, one format family: a fixed number of whole octets of
// individually-meaningful bits, most significant octet transmitted first
// (DPT-AS §1.4: "Data encoded according a DPT that is transmitted on the
// KNX system shall be transmitted with the most significant octet first
// in the frame and the least significant octet last"), with the
// Standard's bit numbering `b0` at the least significant bit of the last
// octet — §8.3.1's and §8.5.1's own diagrams spell that out, octet 1
// (LSB) carrying `b7 … b0`. All four are above the AL-AS §3.1.3 inline
// threshold, so each is always `Bytes` of its own length.
//
// * Main type 21 — 1 octet. §3.22.1 "General Status" gives the format as
//   "1 octet: Z8" (the LTE status/command form of §1.3.1's `Z8`) with the
//   encoding row `bbbbbbbb` and PDT_BITSET8 (alt: PDT_GENERIC_01); 21.001
//   DPT_StatusGen defines b0 OutOfService, b1 Fault, b2 Overridden, b3
//   InAlarm, b4 AlarmUnAck and calls "b5, b6, b7" "reserved, set 0".
//   §3.22.2 "Device Control" gives "1 octet: B8", same encoding row and
//   PDT; 21.002 DPT_Device_Control defines b0 UserStopped, b1 OwnIA, b2
//   VerifyMode and calls "b3…b7" "reserved, set 0". Either way it is
//   eight bits in one octet. Further subtypes live in §4.4
//   (21.100-21.106), §6.4 (21.601), §8.2 (21.1000-21.1002, 21.1010) and
//   §9.6 (21.1200), with their own field splits.
// * Main type 22 — 2 octets. §4.5's subclauses "Data Type "16-Bit Set"":
//   "2 octets: B16", PDT_BITSET16 (alt: PDT_GENERIC_02) in both §4.5.1
//   (22.100) and §4.5.2 (22.101); §8.3 "Datatype B16" repeats the format
//   for the system subtypes (22.1000, 22.1010), and 22.1000 DPT_Media
//   reads b1 TP1, b2 PL110, b4 RF, b5 KNX IP with b0, b3 and "b6 … b15"
//   reserved.
// * Main type 27 — 4 octets. §3.26 "Datatype B32", §3.26.1
//   DPT_CombinedInfoOnOff: "4 octets: B32", encoding row all `B`, "Range:
//   All fields: {0, 1}", PDT_GENERIC_04. Its data-field tables put
//   `s0`…`s15` at bits 0-15 ("0 = output state is Off / 1 = output state
//   is On") and `m0`…`m15` at bits 16-31 ("0 = output state is not valid
//   / 1 = output state is valid").
// * Main type 30 — 3 octets. §8.5 "Datapoint Types B24", §8.5.1: "3
//   octets: B24", with octet 3 (MSB) carrying b23…b16, octet 2 b15…b8 and
//   octet 1 (LSB) b7…b0, PDT_GENERIC_03. 30.1010
//   DPT_Channel_Activation_24 reads `bn` as the "Activation state of
//   channel n+1" for `n = 0 to 23`.
//
// Per the ruling at main type 20, none of those per-subtype splits is
// applied or enforced here: `DptValue::BitSet` carries every bit and its
// width, and a caller that knows its subtype can mask what it needs.
//
// Main type 22 is the strongest case for that ruling, because the
// Standard contradicts itself there. 22.100 DPT_StatusDHWC's encoding row
// (§4.5.1) reads `0 0 0 0 0 0 0 B BBBBBBBB` — bit 8 is a defined `B` —
// while the data-field table under the same diagram lists bits "8 to 15"
// as "reserved", "default 0". A codec enforcing "reserved" would refuse a
// bit the encoding row defines; one enforcing the encoding row would
// accept a bit the table reserves. §4.5.2's own Encoding paragraph points
// at the tolerant reading without settling the bit: "depending on the
// usage of this DPT in a given Datapoint, some bit-fields may be unused
// and set to '0' by the sender and will be ignored by the receiver."
// Sixteen bits in, sixteen bits out, none refused.
//
// Text grammar (this module's choice, and `Display`'s output, so every
// value round-trips): exactly `width` binary digits, most significant bit
// first, with an optional `0b` prefix — or a `0x` hexadecimal form for
// the same value, which is far easier to type for the 24- and 32-bit
// sets.

/// Decodes a `B8`/`B16`/`B24`/`B32` payload of exactly `N` octets, most
/// significant octet first. `N` is 1, 2, 3 or 4, so the result always fits
/// a `u32`.
fn decode_bit_set<const N: usize>(
    dpt: DptRef,
    payload: &GroupValue,
) -> Result<DptValue, DptCodecError> {
    let raw = require_bytes::<N>(payload, dpt, (N * 8) as u32)?;
    let mut bits = 0u32;
    for octet in raw {
        bits = (bits << 8) | u32::from(octet);
    }
    Ok(DptValue::BitSet {
        bits,
        width: (N * 8) as u8,
    })
}

/// Encodes the bit-set text grammar described above into `N` octets.
fn encode_bit_set<const N: usize>(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let trimmed = input.trim();
    let width = N * 8;
    let bits: u32 = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        let value = u32::from_str_radix(hex, 16).map_err(|_| unparsable())?;
        // `width == 32` would make the shift below overflow, and every
        // `u32` fits a 32-bit set anyway.
        if width < 32 && value >> width != 0 {
            return Err(DptCodecError::OutOfRange {
                dpt,
                value: trimmed.to_string(),
            });
        }
        value
    } else {
        let digits = trimmed
            .strip_prefix("0b")
            .or_else(|| trimmed.strip_prefix("0B"))
            .unwrap_or(trimmed);
        if digits.len() != width {
            return Err(unparsable());
        }
        u32::from_str_radix(digits, 2).map_err(|_| unparsable())?
    };
    let mut octets = Vec::with_capacity(N);
    for shift in (0..N).rev() {
        octets.push((bits >> (shift * 8)) as u8);
    }
    Ok(GroupValue::Bytes(octets))
}

// ---------------------------------------------------------------------
// Main type 23 — N2, 2-bit enumeration (DPT-AS §3.23, §4.6)
// ---------------------------------------------------------------------
//
// "Format: 2 bit: N2", field-names row `s`, encoding row `NN`, unit none,
// PDT_ENUM8 (alt: PDT_UNSIGNED_CHAR) — DPT-AS §3.23, and §4.6 for 23.102
// with an identical format block. Two significant bits is well under the
// AL-AS §3.1.2/§3.1.3 six-bit inline threshold, so this type travels in
// the inline data field: `encode` produces `GroupValue::Short` and
// `require_short` enforces §1.3.1's "The preceding bits shall be 0",
// which makes a payload of 4-255 `InvalidData` — exactly as for main
// types 1-3 and 17.
//
// The subtype ranges differ — 23.001 DPT_OnOffAction `[00b…11b]`, 23.002
// DPT_Alarm_Reaction `[00b…10b]` with "(11b = reserved; shall not be
// used)", 23.003 DPT_UpDown_Action `[00b…11b]`, 23.102 DPT_HVAC_PB_Action
// `[00b…11b]` — and per the ruling at main type 20 this codec enforces the
// 2-bit format rather than the subtype's narrowing: a `11b` on a 23.002
// decodes to `Enum { code: 3 }` instead of being refused.

fn decode_n2(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_short(payload, dpt, 2)?;
    Ok(DptValue::Enum { code: raw })
}

fn encode_n2(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let code: u32 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    if code > 0b11 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: trimmed.to_string(),
        });
    }
    Ok(GroupValue::Short(code as u8))
}

// ---------------------------------------------------------------------
// Main type 24 — A[n], variable-length string (DPT-AS §3.24)
// ---------------------------------------------------------------------
//
// "Format: variable length: A[n]", with the diagram's last position
// holding `00` — DPT-AS §3.24. The terminator framing is
// `require_var_string_body`'s, shared with main type 28; what belongs to
// this type is the character encoding, which §3.24's own Encoding block
// states for the format: "Each character shall be encoded according to
// ISO 8859-1." 24.001 DPT_VarString_8859_1's range row says the same from
// the other direction, "Acc. DPT 4.002 (DPT_Char_8859_1)" — the same
// 0-255 character set main type 16's 8859-1 subtype uses, so the same 1:1
// `char::from(u8)` mapping applies, for the reason main type 16's comment
// gives.
//
// Because the character set is stated for the format and not per subtype,
// every subnumber decodes the same way and none is refused — unlike main
// type 16, where §3.17 offers two character sets and the subnumber is what
// picks between them.
//
// EXAMPLE 15 in §3.24: 'KNX is OK' is `4Bh 4Eh 58h 20h 69h 73h 20h 4Fh
// 4Bh 00h` — nine characters plus the terminator, and no length octet.
//
// Text grammar: the string itself, untrimmed (a leading or trailing space
// inside a text DPT is content, same as main type 16). A character above
// U+00FF is not representable in ISO 8859-1 and is `Unparsable`; so is an
// embedded U+0000, which the format cannot transmit because 00h is its
// terminator.

fn decode_var_string_8859_1(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let body = require_var_string_body(payload, dpt)?;
    Ok(DptValue::Text(
        body.iter().copied().map(char::from).collect(),
    ))
}

fn encode_var_string_8859_1(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let mut octets = Vec::with_capacity(input.len() + 1);
    for ch in input.chars() {
        let code = ch as u32;
        if code == 0 || code > 0xFF {
            return Err(DptCodecError::Unparsable {
                dpt,
                input: input.to_string(),
            });
        }
        octets.push(code as u8);
    }
    octets.push(0x00);
    Ok(GroupValue::Bytes(octets))
}

// ---------------------------------------------------------------------
// Main type 25 — U4U4, two 4-bit unsigned fields (DPT-AS §8.4)
// ---------------------------------------------------------------------
//
// "Format: 1 octet: U4U4" with the field-names row `Busy   Nak` left to
// right — so `Busy` occupies bits 7-4 and `Nak` bits 3-0 — "Encoding: All
// field values binary encoded.", unit none, PDT_GENERIC_01 (DPT-AS §8.4).
// One octet, above the AL-AS inline threshold, `Bytes([_])` both ways.
//
// 25.1000 DPT_DoubleNibble, the only subtype §8.4 lists, narrows both
// fields to `[0 … 3]` ("Number of busy repetitions." / "Number of inack
// repetitions."). Per the ruling at main type 20 the codec does not
// enforce that narrowing: the format is `U4U4`, each field is 0-15 on the
// wire, and `DptValue::DoubleNibble` carries what was sent.
//
// Text grammar (this module's choice): `busy 3 nak 2` — §8.4's own two
// field names in §8.4's own order, in main type 18's `<word> <value>`
// shape.

fn decode_double_nibble(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [raw] = require_bytes::<1>(payload, dpt, 8)?;
    Ok(DptValue::DoubleNibble {
        busy: raw >> 4,
        nak: raw & 0x0F,
    })
}

fn encode_double_nibble(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let (busy, nak): (u32, u32) = match tokens.as_slice() {
        [b, busy, n, nak] if b.eq_ignore_ascii_case("busy") && n.eq_ignore_ascii_case("nak") => (
            busy.parse().map_err(|_| unparsable())?,
            nak.parse().map_err(|_| unparsable())?,
        ),
        _ => return Err(unparsable()),
    };
    if busy > 0x0F || nak > 0x0F {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: format!("busy {busy} nak {nak}"),
        });
    }
    Ok(GroupValue::Bytes(vec![((busy as u8) << 4) | nak as u8]))
}

// ---------------------------------------------------------------------
// Main type 26 — r1B1U6, scene information (DPT-AS §3.25)
// ---------------------------------------------------------------------
//
// "Format: 1 octet: r1B1U6", encoding row `0 b UUUUUU`, "Encoding: All
// values binary encoded.", PDT_GENERIC_01 (DPT-AS §3.25). 26.001
// DPT_SceneInfo's own rows name the fields: `r` "Reserved (0)", `B`
// "info: 0 = scene is active, 1 = scene is inactive" with range `[0, 1]`,
// and `SceneNumber` "Scene number" with range `[0 … 63]`. So bit 7 is
// reserved, bit 6 is the info flag and bits 5-0 the scene number.
//
// Significant content is 1 + 6 = 7 bits, one over the AL-AS §3.1.2/
// §3.1.3 six-bit inline threshold — so, exactly like main type 18, this
// type always occupies its own octet: `encode` produces `Bytes([_])` and
// `decode` refuses a `Short` as `WrongLength` rather than quietly
// accepting it. The reserved bit 7 is checked; a set bit is the same
// §1.3.1 "shall be 0" violation `require_short` catches elsewhere, so it
// is `InvalidData`.
//
// This is the section whose NOTE 16 main types 17 and 18 both cite for the
// +1 display convention: "DPT_SceneInfo allows numbering the scene from 0
// to 63. KNX Association recommends displaying these scene numbers in
// ETS™, other software and controllers numbered from 1 to 64, this is,
// with an offset of 1 compared to the actual transmitted value." Here the
// note is this type's own rather than a borrowed one, and the answer is
// still the same as for main types 17 and 18: the offset is a display
// decision, a later layer can only make it honestly if handed the
// untouched wire value, so this codec keeps 0-63 undecorated in both
// directions and `format()` prints the wire value.
//
// Text grammar (this module's choice): `active scene 5` /
// `inactive scene 5` — §3.25's own two words for field `B`, in main type
// 18's `<word> scene <n>` shape.

fn decode_scene_info(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let [raw] = require_bytes::<1>(payload, dpt, 8)?;
    if raw & 0b1000_0000 != 0 {
        return Err(DptCodecError::InvalidData { dpt });
    }
    Ok(DptValue::SceneInfo {
        inactive: raw & 0b0100_0000 != 0,
        number: raw & 0b0011_1111,
    })
}

fn encode_scene_info(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let unparsable = || DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    };
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let (inactive, number): (bool, u32) = match tokens.as_slice() {
        [state, scene, number] if scene.eq_ignore_ascii_case("scene") => {
            let inactive = if state.eq_ignore_ascii_case("active") {
                false
            } else if state.eq_ignore_ascii_case("inactive") {
                true
            } else {
                return Err(unparsable());
            };
            (inactive, number.parse().map_err(|_| unparsable())?)
        }
        _ => return Err(unparsable()),
    };
    if number > 63 {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: number.to_string(),
        });
    }
    Ok(GroupValue::Bytes(vec![
        (u8::from(inactive) << 6) | number as u8,
    ]))
}

// ---------------------------------------------------------------------
// Main type 28 — A[n], variable-length UTF-8 string (DPT-AS §3.27)
// ---------------------------------------------------------------------
//
// "Format: A[n]" with the diagram's last position holding `00` — DPT-AS
// §3.27, §3.27.1 DPT_UTF-8. The terminator framing is
// `require_var_string_body`'s, shared with main type 24; what belongs to
// this type is the character encoding: "This Datapoint Type shall be used
// to transmit Unicode strings, whereas the UTF-8 encoding scheme shall be
// used for Unicode Transformation to data contents for transmission. The
// data length for one character is variable from 1 octet to 4 octets."
// §3.27's Range row is `U+000000 … U+10FFFF`.
//
// A body that is not valid UTF-8 is `InvalidData`: the Standard names the
// encoding, and an octet sequence that violates it is not a value of this
// type. Rust's `String` is UTF-8 by construction and `char` cannot hold a
// value above U+10FFFF, so the range bound and the re-encoding both come
// for free rather than needing a check this codec writes. As with main
// type 24, an embedded U+0000 is `Unparsable` — it is inside §3.27's range
// but is the format's own terminator, so it cannot be transmitted
// unambiguously.

fn decode_var_string_utf8(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let body = require_var_string_body(payload, dpt)?;
    let text = std::str::from_utf8(body).map_err(|_| DptCodecError::InvalidData { dpt })?;
    Ok(DptValue::Text(text.to_string()))
}

fn encode_var_string_utf8(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    if input.contains('\0') {
        return Err(DptCodecError::Unparsable {
            dpt,
            input: input.to_string(),
        });
    }
    let mut octets = Vec::with_capacity(input.len() + 1);
    octets.extend_from_slice(input.as_bytes());
    octets.push(0x00);
    Ok(GroupValue::Bytes(octets))
}

// ---------------------------------------------------------------------
// Main type 29 — V64, 8-octet signed counter (DPT-AS §3.28)
// ---------------------------------------------------------------------
//
// "Format: 8 octets: V64" over a single `SignedValue` field, octet 8 the
// MSB down to octet 1 the LSB, "Encoding: Two's complement notation",
// PDT_GENERIC_08 — DPT-AS §3.28, §3.28.1 "DPTs for electrical energy".
// The three subtypes differ only in unit and all have resolution 1:
// 29.010 DPT_ActiveEnergy_V64 (Wh), 29.011 DPT_ApparantEnergy_V64 (VAh),
// 29.012 DPT_ReactiveEnergy_V64 (VARh). Since no scaling applies,
// `format()` prints the bare number, the same treatment main types 12 and
// 13's unscaled subtypes get.
//
// Ruling on §3.28.1's Range row (recorded in
// `docs/KNOWN_LIMITATIONS.md` §61): that row reads "SignedValue = [9 223
// 372 036 854 775 808 to 9 223 372 036 854 775 807]", whose lower bound
// is missing its minus sign — as printed it is an empty, decreasing
// range, and an asymmetric positive range of that size does not fit 64
// bits in the first place. The datapoint-type rows in the same clause
// give the bound with its sign, "-9 223 372 036 854 775 808 Wh to
// 9 223 372 036 854 775 807 Wh", which is exactly `i64`'s domain and is
// what a two's-complement 64-bit field can hold. This codec follows those
// rows. A well-formed number outside that range is `OutOfRange`, not
// `Unparsable`, which is why `encode` parses into an `i128` first.

fn decode_v64(dpt: DptRef, payload: &GroupValue) -> Result<DptValue, DptCodecError> {
    let raw = require_bytes::<8>(payload, dpt, 64)?;
    Ok(DptValue::Signed64(i64::from_be_bytes(raw)))
}

fn encode_v64(dpt: DptRef, input: &str) -> Result<GroupValue, DptCodecError> {
    let trimmed = input.trim();
    let value: i128 = trimmed.parse().map_err(|_| DptCodecError::Unparsable {
        dpt,
        input: input.to_string(),
    })?;
    if !(i128::from(i64::MIN)..=i128::from(i64::MAX)).contains(&value) {
        return Err(DptCodecError::OutOfRange {
            dpt,
            value: trimmed.to_string(),
        });
    }
    Ok(GroupValue::Bytes((value as i64).to_be_bytes().to_vec()))
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
    fn status_mode3_round_trips_every_assigned_mode_and_both_status_edges() {
        let d = dpt(6, Some(20));
        for (text, raw, status, mode_code) in [
            ("status 00000 mode 001", 0b0000_0001u8, [false; 5], 0b001u8),
            ("status 11111 mode 100", 0b1111_1100, [true; 5], 0b100),
            (
                "status 01001 mode 010",
                0b0100_1010,
                [false, true, false, false, true],
                0b010,
            ),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![raw]), "encoding {text:?}");
            let value = decode(d, &payload).unwrap();
            assert_eq!(value, DptValue::StatusMode3 { status, mode_code });
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn status_mode3_refuses_a_mode_code_the_format_does_not_assign() {
        // DPT-AS §3.7's Range row: `f = {001b,010b,100b}`. That is a
        // format-level restriction, not a subtype table's, so it is
        // enforced.
        let d = dpt(6, Some(20));
        for raw in [
            0b0000_0000u8,
            0b0000_0011,
            0b0000_0101,
            0b0000_0110,
            0b0000_0111,
        ] {
            assert_eq!(
                decode(d, &GroupValue::Bytes(vec![raw])),
                Err(DptCodecError::InvalidData { dpt: d }),
                "decoding {raw:#010b}"
            );
        }
        assert!(matches!(
            encode(d, "status 00000 mode 000"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "status 00000 mode 111"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    #[test]
    fn status_mode3_refuses_malformed_text_and_malformed_payloads() {
        let d = dpt(6, Some(20));
        for text in [
            "status 0000 mode 001",
            "status 000000 mode 001",
            "status 00000 mode 01",
            "status 00002 mode 001",
            "00000 001",
            "",
        ] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
        // Eight significant bits — never the inline form, never two octets.
        assert!(matches!(
            decode(d, &GroupValue::Short(1)),
            Err(DptCodecError::WrongLength { .. })
        ));
        assert!(matches!(
            decode(d, &GroupValue::Bytes(vec![1, 2])),
            Err(DptCodecError::WrongLength { .. })
        ));
    }

    #[test]
    fn status_mode3_is_not_read_as_a_plain_v8_integer() {
        // The point the older `..._is_unsupported_...` test made, kept now
        // that 6.020 is implemented: the same octet means something
        // entirely different under 6.020 (B5N3) than under any other main
        // type 6 subtype (V8, two's complement).
        let payload = GroupValue::Bytes(vec![0b1111_1100]);
        assert_eq!(
            decode(dpt(6, Some(20)), &payload).unwrap(),
            DptValue::StatusMode3 {
                status: [true; 5],
                mode_code: 0b100,
            }
        );
        assert_eq!(
            decode(dpt(6, Some(1)), &payload).unwrap(),
            DptValue::Signed(-4)
        );
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

    // -- Main type 20 ------------------------------------------------------

    #[test]
    fn n8_round_trips_both_ends_of_its_range() {
        let d = dpt(20, Some(1));
        for code in [0u8, 1, 128, 254, 255] {
            let payload = encode(d, &code.to_string()).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![code]));
            let value = decode(d, &payload).unwrap();
            assert_eq!(value, DptValue::Enum { code });
            assert_eq!(value.to_string(), code.to_string());
        }
    }

    #[test]
    fn n8_rejects_a_code_outside_the_octet() {
        let d = dpt(20, Some(1));
        assert!(matches!(
            encode(d, "256"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-1"),
            Err(DptCodecError::Unparsable { .. })
        ));
        assert!(matches!(
            encode(d, "autonomous"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn n8_rejects_payloads_that_are_not_one_octet() {
        let d = dpt(20, Some(11));
        assert!(matches!(
            decode(d, &GroupValue::Short(3)),
            Err(DptCodecError::WrongLength { .. })
        ));
        assert!(matches!(
            decode(d, &GroupValue::Bytes(vec![1, 2])),
            Err(DptCodecError::WrongLength { .. })
        ));
        assert!(matches!(
            decode(d, &GroupValue::Bytes(vec![])),
            Err(DptCodecError::WrongLength { .. })
        ));
    }

    #[test]
    fn n8_keeps_a_code_a_subtype_table_reserves() {
        // 20.011 DPT_ErrorClass_System's range is `[0 to 18]` and its own
        // table calls "19 to 255" reserved (DPT-AS §3.21). The codec
        // validates the N8 format, not the subtype table — see the module
        // header — so the raw code survives instead of being refused.
        let d = dpt(20, Some(11));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![19])).unwrap(),
            DptValue::Enum { code: 19 }
        );
        // 20.002 DPT_BuildingMode's range is `[0 to 2]`; same rule.
        assert_eq!(
            decode(dpt(20, Some(2)), &GroupValue::Bytes(vec![200])).unwrap(),
            DptValue::Enum { code: 200 }
        );
    }

    // -- Main type 21 ------------------------------------------------------

    #[test]
    fn b8_round_trips_its_boundary_patterns() {
        let d = dpt(21, Some(1));
        for (text, raw) in [
            ("00000000", 0x00u8),
            ("00000001", 0x01),
            ("10000000", 0x80),
            ("11111111", 0xFF),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![raw]), "encoding {text:?}");
            let value = decode(d, &payload).unwrap();
            assert_eq!(
                value,
                DptValue::BitSet {
                    bits: u32::from(raw),
                    width: 8,
                }
            );
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn b8_accepts_the_hex_form_and_refuses_a_value_that_does_not_fit() {
        let d = dpt(21, Some(2));
        assert_eq!(
            encode(d, "0x07").unwrap(),
            GroupValue::Bytes(vec![0b0000_0111])
        );
        assert_eq!(encode(d, "0b00000111").unwrap(), encode(d, "0x7").unwrap());
        assert!(matches!(
            encode(d, "0x100"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        for text in ["0000000", "000000000", "00000002", "", "seven"] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
    }

    #[test]
    fn b8_keeps_the_bits_its_subtype_tables_reserve() {
        // 21.001 DPT_StatusGen calls b5, b6, b7 "reserved, set 0"
        // (DPT-AS §3.22.1); 21.002 DPT_Device_Control calls b3…b7 the same
        // (§3.22.2). Format-level validation only — the bits survive.
        assert_eq!(
            decode(dpt(21, Some(1)), &GroupValue::Bytes(vec![0xE0])).unwrap(),
            DptValue::BitSet {
                bits: 0xE0,
                width: 8,
            }
        );
        assert_eq!(
            decode(dpt(21, Some(2)), &GroupValue::Bytes(vec![0xF8])).unwrap(),
            DptValue::BitSet {
                bits: 0xF8,
                width: 8,
            }
        );
    }

    #[test]
    fn b8_rejects_payloads_that_are_not_one_octet() {
        let d = dpt(21, Some(1));
        assert!(matches!(
            decode(d, &GroupValue::Short(1)),
            Err(DptCodecError::WrongLength { .. })
        ));
        assert!(matches!(
            decode(d, &GroupValue::Bytes(vec![0, 0])),
            Err(DptCodecError::WrongLength { .. })
        ));
    }

    // -- Main type 22 ------------------------------------------------------

    #[test]
    fn b16_round_trips_its_boundary_patterns_most_significant_octet_first() {
        let d = dpt(22, Some(101));
        for (text, octets) in [
            ("0000000000000000", vec![0x00u8, 0x00]),
            ("0000000000000001", vec![0x00, 0x01]),
            ("1000000000000000", vec![0x80, 0x00]),
            ("1111111111111111", vec![0xFF, 0xFF]),
            ("0000000111111111", vec![0x01, 0xFF]),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(
                payload,
                GroupValue::Bytes(octets.clone()),
                "encoding {text:?}"
            );
            let value = decode(d, &payload).unwrap();
            let bits = (u32::from(octets[0]) << 8) | u32::from(octets[1]);
            assert_eq!(value, DptValue::BitSet { bits, width: 16 });
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn b16_keeps_bit_8_of_22_100_that_the_standard_contradicts_itself_about() {
        // 22.100 DPT_StatusDHWC's encoding row (DPT-AS §4.5.1) defines bit
        // 8 as a `B`; the data-field table under the same diagram calls
        // bits "8 to 15" reserved. The codec refuses neither reading and
        // keeps all sixteen bits — see the bit-set section's comment.
        let d = dpt(22, Some(100));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0xFF, 0x00])).unwrap(),
            DptValue::BitSet {
                bits: 0xFF00,
                width: 16,
            }
        );
    }

    #[test]
    fn b16_rejects_payloads_that_are_not_two_octets() {
        let d = dpt(22, Some(1000));
        for payload in [
            GroupValue::Short(1),
            GroupValue::Bytes(vec![0]),
            GroupValue::Bytes(vec![0, 0, 0]),
        ] {
            assert!(
                matches!(decode(d, &payload), Err(DptCodecError::WrongLength { .. })),
                "expected WrongLength for {payload:?}"
            );
        }
        assert!(matches!(
            encode(d, "0x10000"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    // -- Main type 23 ------------------------------------------------------

    #[test]
    fn n2_round_trips_every_code_in_the_inline_form() {
        let d = dpt(23, Some(1));
        for code in 0u8..=0b11 {
            let payload = encode(d, &code.to_string()).unwrap();
            assert_eq!(payload, GroupValue::Short(code));
            let value = decode(d, &payload).unwrap();
            assert_eq!(value, DptValue::Enum { code });
            assert_eq!(value.to_string(), code.to_string());
        }
    }

    #[test]
    fn n2_refuses_a_value_outside_its_two_bits() {
        let d = dpt(23, Some(1));
        assert!(matches!(
            encode(d, "4"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-1"),
            Err(DptCodecError::Unparsable { .. })
        ));
        // "The preceding bits shall be 0" — DPT-AS §1.3.1.
        assert_eq!(
            decode(d, &GroupValue::Short(0b100)),
            Err(DptCodecError::InvalidData { dpt: d })
        );
        assert!(matches!(
            decode(d, &GroupValue::Bytes(vec![0, 0])),
            Err(DptCodecError::WrongLength { .. })
        ));
    }

    #[test]
    fn n2_keeps_the_code_23_002_reserves() {
        // 23.002 DPT_Alarm_Reaction: "(11b = reserved; shall not be used)"
        // (DPT-AS §3.23). Format-level validation only.
        let d = dpt(23, Some(2));
        assert_eq!(
            decode(d, &GroupValue::Short(0b11)).unwrap(),
            DptValue::Enum { code: 0b11 }
        );
    }

    // -- Main type 24 ------------------------------------------------------

    #[test]
    fn var_string_8859_1_round_trips_the_standards_own_example() {
        // DPT-AS §3.24 EXAMPLE 15: 'KNX is OK' is encoded as
        // 4Bh 4Eh 58h 20h 69h 73h 20h 4Fh 4Bh 00h.
        let d = dpt(24, Some(1));
        let payload = encode(d, "KNX is OK").unwrap();
        assert_eq!(
            payload,
            GroupValue::Bytes(vec![
                0x4B, 0x4E, 0x58, 0x20, 0x69, 0x73, 0x20, 0x4F, 0x4B, 0x00
            ])
        );
        let value = decode(d, &payload).unwrap();
        assert_eq!(value, DptValue::Text("KNX is OK".to_string()));
        assert_eq!(value.to_string(), "KNX is OK");
    }

    #[test]
    fn var_string_8859_1_round_trips_both_ends_of_its_character_range() {
        let d = dpt(24, Some(1));
        // The shortest legal payload is the terminator on its own.
        assert_eq!(encode(d, "").unwrap(), GroupValue::Bytes(vec![0x00]));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x00])).unwrap(),
            DptValue::Text(String::new())
        );
        // 01h and FFh are the lowest transmittable and highest characters
        // of 4.002 DPT_Char_8859_1's [0...255] range (00h being the
        // terminator).
        for (text, octet) in [("\u{1}", 0x01u8), ("\u{ff}", 0xFF)] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![octet, 0x00]));
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Text(text.to_string())
            );
        }
        // A trailing space is content, not padding — no trimming.
        assert_eq!(
            encode(d, " a ").unwrap(),
            GroupValue::Bytes(vec![0x20, 0x61, 0x20, 0x00])
        );
    }

    #[test]
    fn var_string_8859_1_refuses_malformed_payloads_and_untransmittable_text() {
        let d = dpt(24, Some(1));
        // No terminator.
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x41])),
            Err(DptCodecError::InvalidData { dpt: d })
        );
        // Interior NULL.
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x41, 0x00, 0x42, 0x00])),
            Err(DptCodecError::InvalidData { dpt: d })
        );
        // Not even the terminator, and never the inline form.
        assert!(matches!(
            decode(d, &GroupValue::Bytes(vec![])),
            Err(DptCodecError::WrongLength { .. })
        ));
        assert!(matches!(
            decode(d, &GroupValue::Short(0)),
            Err(DptCodecError::WrongLength { .. })
        ));
        // Outside ISO 8859-1, and the terminator as content.
        for text in ["\u{100}", "\u{20ac}", "a\u{0}b"] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
    }

    // -- Main type 25 ------------------------------------------------------

    #[test]
    fn double_nibble_round_trips_both_field_boundaries() {
        let d = dpt(25, Some(1000));
        for (text, raw, busy, nak) in [
            ("busy 0 nak 0", 0x00u8, 0u8, 0u8),
            ("busy 0 nak 15", 0x0F, 0, 15),
            ("busy 15 nak 0", 0xF0, 15, 0),
            ("busy 15 nak 15", 0xFF, 15, 15),
            ("busy 3 nak 2", 0x32, 3, 2),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![raw]), "encoding {text:?}");
            let value = decode(d, &payload).unwrap();
            assert_eq!(value, DptValue::DoubleNibble { busy, nak });
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn double_nibble_keeps_a_field_value_25_1000_narrows_away() {
        // 25.1000 DPT_DoubleNibble's own range is `[0 … 3]` per field
        // (DPT-AS §8.4); the format's is `[0 … 15]`, and that is what is
        // enforced.
        assert_eq!(
            decode(dpt(25, Some(1000)), &GroupValue::Bytes(vec![0x45])).unwrap(),
            DptValue::DoubleNibble { busy: 4, nak: 5 }
        );
    }

    #[test]
    fn double_nibble_refuses_an_out_of_range_field_and_a_foreign_grammar() {
        let d = dpt(25, Some(1000));
        assert!(matches!(
            encode(d, "busy 16 nak 0"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "busy 0 nak 16"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        for text in ["3 2", "busy 3", "nak 2 busy 3", "busy x nak 2", ""] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
        assert!(matches!(
            decode(d, &GroupValue::Bytes(vec![0, 0])),
            Err(DptCodecError::WrongLength { .. })
        ));
        assert!(matches!(
            decode(d, &GroupValue::Short(1)),
            Err(DptCodecError::WrongLength { .. })
        ));
    }

    // -- Main type 26 ------------------------------------------------------

    #[test]
    fn scene_info_round_trips_both_states_at_both_scene_boundaries() {
        let d = dpt(26, Some(1));
        for (text, raw, inactive, number) in [
            ("active scene 0", 0x00u8, false, 0u8),
            ("active scene 63", 0x3F, false, 63),
            ("inactive scene 0", 0x40, true, 0),
            ("inactive scene 63", 0x7F, true, 63),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(vec![raw]), "encoding {text:?}");
            let value = decode(d, &payload).unwrap();
            assert_eq!(value, DptValue::SceneInfo { inactive, number });
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn scene_info_keeps_the_wire_scene_number_without_the_note_16_offset() {
        // DPT-AS §3.25 NOTE 16 recommends displaying 1-64; that is a
        // display decision, and this codec hands back the wire value.
        assert_eq!(
            decode(dpt(26, Some(1)), &GroupValue::Bytes(vec![0x00]))
                .unwrap()
                .to_string(),
            "active scene 0"
        );
    }

    #[test]
    fn scene_info_refuses_the_reserved_bit_the_inline_form_and_bad_text() {
        let d = dpt(26, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x80])),
            Err(DptCodecError::InvalidData { dpt: d })
        );
        assert!(matches!(
            decode(d, &GroupValue::Short(0x01)),
            Err(DptCodecError::WrongLength { .. })
        ));
        assert!(matches!(
            encode(d, "active scene 64"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        for text in ["maybe scene 1", "active 1", "scene 1", "active scene x", ""] {
            assert!(
                matches!(encode(d, text), Err(DptCodecError::Unparsable { .. })),
                "expected Unparsable for {text:?}"
            );
        }
    }

    // -- Main type 27 ------------------------------------------------------

    #[test]
    fn b32_round_trips_its_boundary_patterns() {
        let d = dpt(27, Some(1));
        for (text, octets, bits) in [
            (
                "00000000000000000000000000000000",
                vec![0x00u8, 0x00, 0x00, 0x00],
                0x0000_0000u32,
            ),
            (
                "00000000000000000000000000000001",
                vec![0x00, 0x00, 0x00, 0x01],
                0x0000_0001,
            ),
            (
                "10000000000000000000000000000000",
                vec![0x80, 0x00, 0x00, 0x00],
                0x8000_0000,
            ),
            (
                "11111111111111111111111111111111",
                vec![0xFF, 0xFF, 0xFF, 0xFF],
                0xFFFF_FFFF,
            ),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(octets), "encoding {text:?}");
            let value = decode(d, &payload).unwrap();
            assert_eq!(value, DptValue::BitSet { bits, width: 32 });
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn b32_hex_form_accepts_the_whole_32_bit_width() {
        // The width guard in `encode_bit_set` must not shift by 32.
        let d = dpt(27, Some(1));
        assert_eq!(
            encode(d, "0xFFFFFFFF").unwrap(),
            GroupValue::Bytes(vec![0xFF, 0xFF, 0xFF, 0xFF])
        );
        assert_eq!(
            encode(d, "0x00010002").unwrap(),
            GroupValue::Bytes(vec![0x00, 0x01, 0x00, 0x02])
        );
        assert!(matches!(
            encode(d, "0x1FFFFFFFF"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn b32_splits_nothing_into_the_27_001_mask_and_state_halves() {
        // 27.001 DPT_CombinedInfoOnOff puts s0…s15 at bits 0-15 and
        // m0…m15 at bits 16-31 (DPT-AS §3.26.1). The codec keeps the flat
        // bit set; a caller that knows it has a 27.001 masks it itself.
        let d = dpt(27, Some(1));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x00, 0x03, 0x00, 0x01])).unwrap(),
            DptValue::BitSet {
                bits: 0x0003_0001,
                width: 32,
            }
        );
    }

    #[test]
    fn b32_rejects_payloads_that_are_not_four_octets() {
        let d = dpt(27, Some(1));
        for payload in [
            GroupValue::Short(1),
            GroupValue::Bytes(vec![0, 0, 0]),
            GroupValue::Bytes(vec![0, 0, 0, 0, 0]),
        ] {
            assert!(
                matches!(decode(d, &payload), Err(DptCodecError::WrongLength { .. })),
                "expected WrongLength for {payload:?}"
            );
        }
    }

    // -- Main type 28 ------------------------------------------------------

    #[test]
    fn utf8_string_round_trips_multi_octet_characters() {
        let d = dpt(28, Some(1));
        let text = "Gr\u{fc}\u{df}e \u{1f600}";
        let payload = encode(d, text).unwrap();
        let mut expected = text.as_bytes().to_vec();
        expected.push(0x00);
        assert_eq!(payload, GroupValue::Bytes(expected));
        let value = decode(d, &payload).unwrap();
        assert_eq!(value, DptValue::Text(text.to_string()));
        assert_eq!(value.to_string(), text);
    }

    #[test]
    fn utf8_string_round_trips_each_octet_width_boundary() {
        // The four rows of DPT-AS §3.27's UTF-8 table: 1, 2, 3 and 4
        // octets per character, at both ends of each row's range.
        let d = dpt(28, Some(1));
        for (text, octet_count) in [
            ("\u{1}", 1usize),
            ("\u{7f}", 1),
            ("\u{80}", 2),
            ("\u{7ff}", 2),
            ("\u{800}", 3),
            ("\u{ffff}", 3),
            ("\u{10000}", 4),
            ("\u{10ffff}", 4),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(
                payload,
                GroupValue::Bytes({
                    let mut v = text.as_bytes().to_vec();
                    v.push(0x00);
                    v
                }),
                "encoding {text:?}"
            );
            assert_eq!(text.len(), octet_count, "octet count for {text:?}");
            assert_eq!(
                decode(d, &payload).unwrap(),
                DptValue::Text(text.to_string())
            );
        }
        // The empty string is the terminator alone.
        assert_eq!(encode(d, "").unwrap(), GroupValue::Bytes(vec![0x00]));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x00])).unwrap(),
            DptValue::Text(String::new())
        );
        // U+0000 is inside §3.27's range but is the format's terminator.
        assert!(matches!(
            encode(d, "\u{0}"),
            Err(DptCodecError::Unparsable { .. })
        ));
    }

    #[test]
    fn utf8_string_refuses_a_body_that_is_not_valid_utf8() {
        let d = dpt(28, Some(1));
        for body in [vec![0xC3u8], vec![0xFF], vec![0xE2, 0x82], vec![0x80]] {
            let mut payload = body.clone();
            payload.push(0x00);
            assert_eq!(
                decode(d, &GroupValue::Bytes(payload)),
                Err(DptCodecError::InvalidData { dpt: d }),
                "decoding body {body:02X?}"
            );
        }
        // Terminator rules, shared with main type 24.
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x41])),
            Err(DptCodecError::InvalidData { dpt: d })
        );
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x41, 0x00, 0x42, 0x00])),
            Err(DptCodecError::InvalidData { dpt: d })
        );
        assert!(matches!(
            decode(d, &GroupValue::Short(0)),
            Err(DptCodecError::WrongLength { .. })
        ));
    }

    // -- Main type 29 ------------------------------------------------------

    #[test]
    fn v64_round_trips_both_ends_of_its_range() {
        let d = dpt(29, Some(10));
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            let payload = encode(d, &value.to_string()).unwrap();
            assert_eq!(payload, GroupValue::Bytes(value.to_be_bytes().to_vec()));
            let decoded = decode(d, &payload).unwrap();
            assert_eq!(decoded, DptValue::Signed64(value));
            assert_eq!(decoded.to_string(), value.to_string());
        }
    }

    #[test]
    fn v64_is_most_significant_octet_first() {
        // DPT-AS §3.28.1: octet 8 is the MSB, octet 1 the LSB.
        let d = dpt(29, Some(11));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0, 0, 0, 0, 0, 0, 0, 1])).unwrap(),
            DptValue::Signed64(1)
        );
        assert_eq!(
            decode(
                d,
                &GroupValue::Bytes(vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
            )
            .unwrap(),
            DptValue::Signed64(-1)
        );
    }

    #[test]
    fn v64_refuses_a_value_one_past_each_boundary_and_a_wrong_length() {
        let d = dpt(29, Some(12));
        assert!(matches!(
            encode(d, "9223372036854775808"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "-9223372036854775809"),
            Err(DptCodecError::OutOfRange { .. })
        ));
        assert!(matches!(
            encode(d, "lots of watt-hours"),
            Err(DptCodecError::Unparsable { .. })
        ));
        for payload in [
            GroupValue::Short(1),
            GroupValue::Bytes(vec![0; 4]),
            GroupValue::Bytes(vec![0; 9]),
        ] {
            assert!(
                matches!(decode(d, &payload), Err(DptCodecError::WrongLength { .. })),
                "expected WrongLength for {payload:?}"
            );
        }
    }

    #[test]
    fn v64_prints_no_unit_because_its_resolution_is_one() {
        assert_eq!(DptValue::Signed64(-5).format(dpt(29, Some(10))), "-5");
    }

    // -- Main type 30 ------------------------------------------------------

    #[test]
    fn b24_round_trips_its_boundary_patterns() {
        let d = dpt(30, Some(1010));
        for (text, octets, bits) in [
            ("000000000000000000000000", vec![0x00u8, 0x00, 0x00], 0u32),
            (
                "000000000000000000000001",
                vec![0x00, 0x00, 0x01],
                0x00_0001,
            ),
            (
                "100000000000000000000000",
                vec![0x80, 0x00, 0x00],
                0x80_0000,
            ),
            (
                "111111111111111111111111",
                vec![0xFF, 0xFF, 0xFF],
                0xFF_FFFF,
            ),
        ] {
            let payload = encode(d, text).unwrap();
            assert_eq!(payload, GroupValue::Bytes(octets), "encoding {text:?}");
            let value = decode(d, &payload).unwrap();
            assert_eq!(value, DptValue::BitSet { bits, width: 24 });
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn b24_puts_octet_3_first_and_octet_1_last() {
        // DPT-AS §8.5.1: octet 3 (MSB) carries b23…b16, octet 1 (LSB)
        // carries b7…b0. So b23 — channel 24 — is the top bit of the
        // first octet transmitted.
        let d = dpt(30, Some(1010));
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x80, 0x00, 0x00])).unwrap(),
            DptValue::BitSet {
                bits: 1 << 23,
                width: 24,
            }
        );
        assert_eq!(
            decode(d, &GroupValue::Bytes(vec![0x00, 0x00, 0x01])).unwrap(),
            DptValue::BitSet { bits: 1, width: 24 }
        );
    }

    #[test]
    fn b24_rejects_payloads_that_are_not_three_octets() {
        let d = dpt(30, Some(1010));
        for payload in [
            GroupValue::Short(1),
            GroupValue::Bytes(vec![0, 0]),
            GroupValue::Bytes(vec![0, 0, 0, 0]),
        ] {
            assert!(
                matches!(decode(d, &payload), Err(DptCodecError::WrongLength { .. })),
                "expected WrongLength for {payload:?}"
            );
        }
        assert!(matches!(
            encode(d, "0x1000000"),
            Err(DptCodecError::OutOfRange { .. })
        ));
    }

    // -- Unsupported main types -------------------------------------------

    #[test]
    fn unimplemented_main_type_is_unsupported_not_a_panic() {
        // Main type 20 graduated out of "unimplemented" this task. Main
        // type 31 takes its place, and not arbitrarily: DPT-AS §4.7/§4.7.1
        // gives it as "3 bit: N3" with exactly one subtype, 31.101
        // DPT_PB_Action_HVAC_Extended, whose own entry states "This DPT
        // shall not be used for runtime communication. This DPT shall only
        // be used for encoding Parameter values in CH_PB_HVAC_Mode_1." A
        // group-value codec has nothing legitimate to do with it, so it
        // stays unsupported rather than being implemented for symmetry.
        let d = dpt(31, Some(101));
        assert_eq!(
            decode(d, &GroupValue::Short(1)),
            Err(DptCodecError::UnsupportedDpt(d))
        );
        assert_eq!(encode(d, "1"), Err(DptCodecError::UnsupportedDpt(d)));
    }

    #[test]
    fn a_main_type_above_the_implemented_range_is_unsupported_not_a_panic() {
        // DPT-AS §2's overview table jumps from main type 31 to the
        // 200-series LTE types; 46 is not a main type at all (it is the
        // *count* of main types in the ETS master data, see
        // `docs/KNOWN_LIMITATIONS.md` §90). Whatever the number, an
        // unimplemented main type refuses rather than panics.
        for main in [32u16, 46, 200, 251, u16::MAX] {
            let d = dpt(main, Some(1));
            assert_eq!(
                decode(d, &GroupValue::Bytes(vec![0])),
                Err(DptCodecError::UnsupportedDpt(d)),
                "decoding main type {main}"
            );
            assert_eq!(
                encode(d, "0"),
                Err(DptCodecError::UnsupportedDpt(d)),
                "encoding main type {main}"
            );
        }
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
