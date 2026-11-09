//! `TypeFloat` parameter values as ETS 6.4 writes them: `DPT 9` and `IEEE-754 Single`.
//!
//! `[D]` *Datapoint Types* `03_07_02` v02.02.01 §3.10 (printed page 39)
//! defines the two-octet float `F16`: `MEEEEMMM MMMMMMMM`, the value
//! `0,01 * M * 2^E` with `E` in `0..=15` and `M` a 12-bit two's-complement
//! number in `-2048..=2047`; `7FFFh` means invalid data. The clause does not
//! say which of several representations an encoder picks, nor how it rounds.
//! `[V]` ETS 6.4, downloading a memory (`0705h`) program to KNXBench's
//! simulator, wrote the smallest exponent whose mantissa fits, the mantissa
//! rounded to nearest with ties to even on the exact decimal value (`1000` →
//! `1562.5` → `361Ah`; `32.09` → `1604.5` → `0E44h`), high octet first
//! (docs/research/2026-11-09-float-parameters.md).
//! `IEEE-754 Single` is the 32-bit binary interchange format, the value
//! rounded to nearest, high octet first in a `BigEndian` program like any
//! number. Any other `Encoding` is refused by name.

/// A `TypeFloat/@Encoding` the image writer admits (ADR-0169).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatEncoding {
    /// `DPT 9`: the two-octet KNX float `F16`.
    Dpt9,
    /// `IEEE-754 Single`: the 32-bit binary float.
    Ieee754Single,
}

impl FloatEncoding {
    /// The encoding a declared `@Encoding` names, verbatim; `None` for any
    /// other (e.g. `IEEE-754 Double`, unobserved).
    pub fn named(name: &str) -> Option<FloatEncoding> {
        match name {
            "DPT 9" => Some(FloatEncoding::Dpt9),
            "IEEE-754 Single" => Some(FloatEncoding::Ieee754Single),
            _ => None,
        }
    }

    /// The field width the encoding fills.
    pub fn size_in_bit(self) -> u32 {
        match self {
            FloatEncoding::Dpt9 => 16,
            FloatEncoding::Ieee754Single => 32,
        }
    }

    /// The bits `raw` is written as, or why it cannot be.
    pub fn encode(self, raw: &str) -> Result<u64, String> {
        match self {
            FloatEncoding::Dpt9 => encode_dpt9(raw).map(u64::from),
            FloatEncoding::Ieee754Single => {
                let value: f32 = parse_text(raw)?
                    .parse()
                    .map_err(|_| format!("{raw:?} is not a decimal number"))?;
                if !value.is_finite() {
                    return Err(format!("{raw:?} does not fit an IEEE-754 single"));
                }
                // A non-zero value that underflows to zero or a subnormal was
                // not observed; refuse it rather than write a different number.
                if !value.is_normal() && parse(raw)? != 0.0 {
                    return Err(format!("{raw:?} underflows an IEEE-754 single"));
                }
                Ok(u64::from(value.to_bits()))
            }
        }
    }

    /// The value `bits` stand for, as text; `None` for the invalid marker or
    /// a non-finite single.
    pub fn decode(self, bits: u64) -> Option<String> {
        match self {
            FloatEncoding::Dpt9 => decode_dpt9(u16::try_from(bits).ok()?).map(hundredths_text),
            FloatEncoding::Ieee754Single => {
                let value = f32::from_bits(u32::try_from(bits).ok()?);
                value.is_finite().then(|| value.to_string())
            }
        }
    }
}

/// `raw` checked for the plain decimal grammar product data and projects
/// use (`1.3`, `-0.26`, `1.000000000000000E+000`): no decimal comma, no
/// `inf`/`NaN`, no surrounding spaces.
fn parse_text(raw: &str) -> Result<&str, String> {
    let plain = !raw.is_empty()
        && raw
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'E' | 'e'))
        && raw.chars().any(|c| c.is_ascii_digit());
    if plain {
        Ok(raw)
    } else {
        Err(format!("{raw:?} is not a decimal number"))
    }
}

/// `raw` as a finite number.
pub fn parse(raw: &str) -> Result<f64, String> {
    let value: f64 = parse_text(raw)?
        .parse()
        .map_err(|_| format!("{raw:?} is not a decimal number"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{raw:?} is not a finite number"))
    }
}

/// `raw` in hundredths as the exact fraction `±numerator / denominator`,
/// from its decimal digits, not a binary approximation.
fn exact_hundredths(raw: &str) -> Result<(bool, u128, u128), String> {
    let invalid = || format!("{raw:?} is not a decimal number");
    let text = parse_text(raw)?;
    let (negative, unsigned) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (significand, exponent) = match unsigned.find(['e', 'E']) {
        Some(at) => {
            let exponent: i32 = unsigned[at + 1..].parse().map_err(|_| invalid())?;
            (&unsigned[..at], exponent)
        }
        None => (unsigned, 0),
    };
    let (whole, fraction) = significand.split_once('.').unwrap_or((significand, ""));
    if (whole.is_empty() && fraction.is_empty())
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }
    let digits = format!("{whole}{fraction}");
    let digits = digits.trim_start_matches('0');
    let mut scale = i64::from(exponent) + 2 - fraction.len() as i64;
    let trimmed = digits.trim_end_matches('0');
    scale += (digits.len() - trimmed.len()) as i64;
    if trimmed.is_empty() {
        return Ok((negative, 0, 1));
    }
    let too_precise = || format!("{raw:?} has more digits than KNXBench reads for DPT 9");
    let mut numerator: u128 = if trimmed.len() <= 38 {
        trimmed.parse().map_err(|_| invalid())?
    } else {
        return Err(too_precise());
    };
    let mut denominator: u128 = 1;
    if scale >= 0 {
        // Any scale past 10 is far beyond F16's 67 108 864 hundredths.
        if scale > 10 {
            return Err(format!("{raw:?} is outside the DPT 9 range"));
        }
        numerator = numerator
            .checked_mul(10u128.pow(scale as u32))
            .ok_or_else(|| format!("{raw:?} is outside the DPT 9 range"))?;
    } else {
        if -scale > 30 {
            return Err(too_precise());
        }
        denominator = 10u128.pow((-scale) as u32);
    }
    Ok((negative, numerator, denominator))
}

/// The `F16` bits for `raw`: the smallest exponent whose mantissa fits
/// `-2048..=2047`, the mantissa rounded to nearest with ties to even, on the
/// exact decimal value (`[V]` ETS: `32.09` → `0E44h`, where the binary double
/// `3209.0000000000005` would round to `0E45h`).
fn encode_dpt9(raw: &str) -> Result<u16, String> {
    let (negative, numerator, denominator) = exact_hundredths(raw)?;
    for exponent in 0..=15u16 {
        let divisor = denominator << exponent;
        let (quotient, remainder) = (numerator / divisor, numerator % divisor);
        let twice = remainder * 2;
        let magnitude = if twice > divisor || (twice == divisor && quotient % 2 == 1) {
            quotient + 1
        } else {
            quotient
        };
        let limit = if negative { 2048 } else { 2047 };
        if magnitude <= limit {
            // At most 2048, so the casts are exact.
            let mantissa = if negative {
                -(magnitude as i16)
            } else {
                magnitude as i16
            };
            let twelve = (mantissa as u16) & 0x0FFF;
            let bits = ((twelve & 0x0800) << 4) | (exponent << 11) | (twelve & 0x07FF);
            if bits == 0x7FFF {
                return Err(format!("{raw} encodes as 7FFFh, the invalid-data marker"));
            }
            return Ok(bits);
        }
    }
    Err(format!("{raw} is outside the DPT 9 range"))
}

/// The value of `F16` bits in hundredths, `M * 2^E` exactly; `None` for
/// `7FFFh`.
fn decode_dpt9(bits: u16) -> Option<i64> {
    if bits == 0x7FFF {
        return None;
    }
    let exponent = (bits >> 11) & 0x0F;
    let twelve = ((bits & 0x8000) >> 4) | (bits & 0x07FF);
    // Sign-extend the 12-bit two's-complement mantissa.
    let mantissa = i64::from(((twelve << 4) as i16) >> 4);
    Some(mantissa << exponent)
}

/// `hundredths / 100` as an exact decimal, without trailing zeros.
fn hundredths_text(hundredths: i64) -> String {
    let sign = if hundredths < 0 { "-" } else { "" };
    let (whole, cents) = (
        hundredths.unsigned_abs() / 100,
        hundredths.unsigned_abs() % 100,
    );
    match cents {
        0 => format!("{sign}{whole}"),
        c if c % 10 == 0 => format!("{sign}{whole}.{}", c / 10),
        c => format!("{sign}{whole}.{c:02}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpt9_encodes_the_smallest_exponent_with_a_fitting_mantissa() {
        let dpt9 = |raw| FloatEncoding::Dpt9.encode(raw);
        // Exact in E = 0.
        assert_eq!(dpt9("0"), Ok(0x0000));
        assert_eq!(dpt9("2.5"), Ok(0x00FA));
        assert_eq!(dpt9("1.2"), Ok(0x0078));
        // 21.5 → 2150 does not fit 11 bits: E = 1, M = 1075.
        assert_eq!(dpt9("21.5"), Ok(0x0C33));
        // Negative: 12-bit two's complement, sign in the top bit.
        assert_eq!(dpt9("-12.5"), Ok(0x831E));
        assert_eq!(dpt9("-5.5"), Ok(0x85DA));
        assert_eq!(dpt9("-20.48"), Ok(0x8000));
        // `[V]` ETS: 1000 → 100000 / 64 = 1562.5, a tie: to even, 1562.
        assert_eq!(dpt9("1000"), Ok(0x361A));
        // 82.03125 → 8203.125: E = 3, 1025.39 → 1025.
        assert_eq!(dpt9("82.03125"), Ok(0x1C01));
        // `[V]` ETS: 25000 → 2500000 / 2048 = 1220.7: nearest, 1221.
        assert_eq!(dpt9("25000"), Ok(0x5CC5));
        // Exact decimal ties, to even: 0.025 → 2.5 → 2; 0.035 → 3.5 → 4.
        assert_eq!(dpt9("0.025"), Ok(0x0002));
        assert_eq!(dpt9("0.035"), Ok(0x0004));
        // `[V]` ETS: decimal ties at E = 1, where the binary double is not a
        // tie (3209.0000000000005 and 3222.9999999999995).
        assert_eq!(dpt9("32.09"), Ok(0x0E44));
        assert_eq!(dpt9("32.45"), Ok(0x0E56));
        assert_eq!(dpt9("32.23"), Ok(0x0E4C));
        // Spellings of one value encode alike.
        for raw in [
            "1",
            "1.0",
            "+1",
            "1e0",
            "100E-2",
            "1.000000000000000E+000",
            ".01e2",
        ] {
            assert_eq!(dpt9(raw), Ok(0x0064), "{raw}");
        }
        assert_eq!(dpt9("-0"), Ok(0x0000));
        // The mantissa limits are asymmetric: +2048 needs E + 1, -2048 fits.
        assert_eq!(dpt9("20.48"), Ok(0x0C00));
        assert_eq!(dpt9("-20.48"), Ok(0x8000));
        assert_eq!(dpt9("-20.49"), Ok(0x8C00));
        assert_eq!(dpt9("1.000000000000000E+000"), Ok(0x0064));
    }

    #[test]
    fn dpt9_refuses_what_it_cannot_carry() {
        let dpt9 = |raw| FloatEncoding::Dpt9.encode(raw);
        // The range ends at M = 2046, E = 15 (§3.10's 670 433,28); M = 2047
        // there is 7FFFh, the invalid-data marker.
        assert_eq!(dpt9("670433.28"), Ok(0x7FFE));
        assert!(dpt9("670760").is_err());
        assert!(dpt9("680000").is_err(), "no exponent fits");
        assert_eq!(dpt9("-671088.64"), Ok(0xF800));
        assert!(dpt9("-672000").is_err());
        assert!(dpt9("5,0").is_err(), "decimal comma");
        assert!(dpt9("NaN").is_err());
        assert!(dpt9("inf").is_err());
        assert!(dpt9(" 1").is_err());
        assert!(dpt9("").is_err());
        assert!(dpt9("1e400").is_err());
        assert!(dpt9("1e-400").is_err(), "more digits than KNXBench reads");
        assert!(dpt9("1.2.3").is_err());
        assert!(dpt9("e5").is_err());
        assert!(dpt9("--1").is_err());
    }

    #[test]
    fn dpt9_decodes_what_it_encodes() {
        for raw in [
            "0",
            "2.5",
            "-12.5",
            "21.5",
            "-5.5",
            "1000",
            "-0.01",
            "-671088.64",
        ] {
            let bits = FloatEncoding::Dpt9.encode(raw).expect(raw);
            let back = FloatEncoding::Dpt9.decode(bits).expect(raw);
            let value: f64 = back.parse().unwrap();
            assert!(
                (value - raw.parse::<f64>().unwrap()).abs() <= 0.01 * 64.0,
                "{raw} -> {back}"
            );
        }
        assert_eq!(FloatEncoding::Dpt9.decode(0x7FFF), None);
        assert_eq!(FloatEncoding::Dpt9.decode(0x0C33).as_deref(), Some("21.5"));
        assert_eq!(FloatEncoding::Dpt9.decode(0x831E).as_deref(), Some("-12.5"));
        // Exact in hundredths: 1562 * 2^6 = 99968, not 999.6800000000001.
        assert_eq!(
            FloatEncoding::Dpt9.decode(0x361A).as_deref(),
            Some("999.68")
        );
        assert_eq!(
            FloatEncoding::Dpt9.decode(0x8001).as_deref(),
            Some("-20.47")
        );
        assert_eq!(FloatEncoding::Dpt9.decode(0x87FF).as_deref(), Some("-0.01"));
        assert_eq!(FloatEncoding::Dpt9.decode(0x0000).as_deref(), Some("0"));
    }

    #[test]
    fn ieee754_single_is_the_nearest_binary32() {
        let single = |raw| FloatEncoding::Ieee754Single.encode(raw);
        assert_eq!(single("1.3"), Ok(0x3FA6_6666));
        assert_eq!(single("-0.26"), Ok(0xBE85_1EB8));
        assert_eq!(single("5"), Ok(0x40A0_0000));
        assert_eq!(single("0"), Ok(0));
        assert_eq!(single("3.4E+38"), Ok(u64::from(3.4e38f32.to_bits())));
        assert!(single("3.5E+38").is_err(), "overflows to infinity");
        assert!(single("1e-50").is_err(), "underflows to zero");
        assert!(single("1e-40").is_err(), "a subnormal");
        assert_eq!(single("1.2E-38"), Ok(u64::from(1.2e-38f32.to_bits())));
        assert!(single("5,0").is_err());
        assert_eq!(
            FloatEncoding::Ieee754Single.decode(0x3FA6_6666).as_deref(),
            Some("1.3")
        );
        assert_eq!(FloatEncoding::Ieee754Single.decode(0x7F80_0000), None);
    }

    #[test]
    fn only_the_two_observed_encodings_are_named() {
        assert_eq!(FloatEncoding::named("DPT 9"), Some(FloatEncoding::Dpt9));
        assert_eq!(
            FloatEncoding::named("IEEE-754 Single"),
            Some(FloatEncoding::Ieee754Single)
        );
        assert_eq!(FloatEncoding::named("IEEE-754 Double"), None);
        assert_eq!(FloatEncoding::named("dpt 9"), None);
    }
}
