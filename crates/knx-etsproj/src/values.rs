//! Attribute value conversions shared by Stage 4 (validation) and Stage 5
//! (mapping): turning a raw XML attribute string into a typed value, or a
//! [`ValueError`] describing exactly what was wrong with it. Nothing here
//! fails an import outright — Task 10 turns a `ValueError` into an
//! `ImportError` report entry and continues with the rest of the project.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use knx_core::{BuildingPartType, CompletionStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueError {
    NotABoolean(String),
    NotATimestamp(String),
    NotAnInteger { field: &'static str, value: String },
    UnknownEnumValue { kind: &'static str, value: String },
    MalformedRefId(String),
}

impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueError::NotABoolean(v) => write!(f, "not a boolean: {v:?}"),
            ValueError::NotATimestamp(v) => write!(f, "not a timestamp: {v:?}"),
            ValueError::NotAnInteger { field, value } => {
                write!(f, "{field}: not an integer: {value:?}")
            }
            ValueError::UnknownEnumValue { kind, value } => {
                write!(f, "{kind}: unknown value {value:?}")
            }
            ValueError::MalformedRefId(v) => write!(f, "malformed RefId: {v:?}"),
        }
    }
}

impl std::error::Error for ValueError {}

/// Schema 11 writes `"1"`/`"0"`; schema 23 writes `"true"`/`"false"`
/// (RESEARCH §3.3). Both are accepted regardless of source schema, since
/// the same converter serves both.
pub fn parse_bool(s: &str) -> Result<bool, ValueError> {
    match s {
        "1" => Ok(true),
        "0" => Ok(false),
        _ if s.eq_ignore_ascii_case("true") => Ok(true),
        _ if s.eq_ignore_ascii_case("false") => Ok(false),
        _ => Err(ValueError::NotABoolean(s.to_string())),
    }
}

/// ETS4 writes a naive local timestamp with no offset (`2023-07-14T11:55:33`);
/// ETS6 writes an RFC 3339 instant (`2025-12-17T10:12:14.3525475Z`). The
/// naive form is treated as UTC — a recorded assumption, not a measured
/// fact: ETS4 does not persist the machine's timezone anywhere in the
/// project, so the true local offset is unrecoverable from the file alone.
pub fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, ValueError> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&naive));
    }
    Err(ValueError::NotATimestamp(s.to_string()))
}

pub fn parse_u8(s: &str, field: &'static str) -> Result<u8, ValueError> {
    s.parse().map_err(|_| ValueError::NotAnInteger {
        field,
        value: s.to_string(),
    })
}

pub fn parse_u16(s: &str, field: &'static str) -> Result<u16, ValueError> {
    s.parse().map_err(|_| ValueError::NotAnInteger {
        field,
        value: s.to_string(),
    })
}

pub fn parse_completion_status(s: &str) -> Result<CompletionStatus, ValueError> {
    match s {
        "Undefined" => Ok(CompletionStatus::Undefined),
        "Editing" => Ok(CompletionStatus::Editing),
        "FinishedDesign" => Ok(CompletionStatus::FinishedDesign),
        "Accepted" => Ok(CompletionStatus::Accepted),
        _ => Err(ValueError::UnknownEnumValue {
            kind: "CompletionStatus",
            value: s.to_string(),
        }),
    }
}

pub fn parse_building_part_type(s: &str) -> Result<BuildingPartType, ValueError> {
    match s {
        "Building" => Ok(BuildingPartType::Building),
        "Floor" => Ok(BuildingPartType::Floor),
        "Room" => Ok(BuildingPartType::Room),
        "Corridor" => Ok(BuildingPartType::Corridor),
        "DistributionBoard" => Ok(BuildingPartType::DistributionBoard),
        "BuildingPart" => Ok(BuildingPartType::BuildingPart),
        _ => Err(ValueError::UnknownEnumValue {
            kind: "BuildingPart/@Type",
            value: s.to_string(),
        }),
    }
}

/// Splits a compound `RefId` into its application-program prefix and its
/// trailing `_O-<n>_R-<m>` communication-object tail, requiring the last
/// segment to start with `R-` and the one before it to match `O-<digits>`
/// exactly. The program-ref prefix can itself contain an `O`-looking token
/// (e.g. `M-006A_A-0001-22-26C0-O0079`), so only the final two
/// underscore-delimited segments are ever inspected.
fn split_object_tail(ref_id: &str) -> Result<(&str, u16), ValueError> {
    let mut parts = ref_id.rsplitn(3, '_');
    let last = parts
        .next()
        .ok_or_else(|| ValueError::MalformedRefId(ref_id.to_string()))?;
    let second_last = parts
        .next()
        .ok_or_else(|| ValueError::MalformedRefId(ref_id.to_string()))?;
    let program_ref = parts
        .next()
        .ok_or_else(|| ValueError::MalformedRefId(ref_id.to_string()))?;

    if !last.starts_with("R-") {
        return Err(ValueError::MalformedRefId(ref_id.to_string()));
    }
    let digits = second_last
        .strip_prefix("O-")
        .ok_or_else(|| ValueError::MalformedRefId(ref_id.to_string()))?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ValueError::MalformedRefId(ref_id.to_string()));
    }
    let number: u16 = digits
        .parse()
        .map_err(|_| ValueError::MalformedRefId(ref_id.to_string()))?;
    Ok((program_ref, number))
}

/// The `_O-<n>` object number inside a schema-11 compound `RefId`.
pub fn com_object_number(ref_id: &str) -> Result<u16, ValueError> {
    split_object_tail(ref_id).map(|(_, number)| number)
}

/// The application program part of a compound `RefId`, e.g.
/// `M-006A_A-0001-22-26C0-O0079`.
pub fn application_program_ref(ref_id: &str) -> Result<&str, ValueError> {
    split_object_tail(ref_id).map(|(program_ref, _)| program_ref)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_boolean_spellings_parse() {
        // Schema 11 writes "1"/"0"; schema 23 writes "true"/"false" (RESEARCH §3.3).
        assert!(parse_bool("1").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(parse_bool("true").unwrap());
        assert!(!parse_bool("false").unwrap());
        assert!(parse_bool("True").unwrap());
        assert!(matches!(parse_bool("yes"), Err(ValueError::NotABoolean(_))));
        assert!(matches!(parse_bool(""), Err(ValueError::NotABoolean(_))));
    }

    #[test]
    fn both_timestamp_spellings_parse() {
        // ETS4 writes a naive local timestamp, ETS6 an RFC 3339 instant.
        let naive = parse_timestamp("2023-07-14T11:55:33").unwrap();
        let rfc = parse_timestamp("2025-12-17T10:12:14.3525475Z").unwrap();
        assert_eq!(naive.to_rfc3339(), "2023-07-14T11:55:33+00:00");
        assert_eq!(rfc.date_naive().to_string(), "2025-12-17");
        assert!(matches!(
            parse_timestamp("14.07.2023"),
            Err(ValueError::NotATimestamp(_))
        ));
    }

    #[test]
    fn a_compound_ref_id_yields_its_object_number_and_program() {
        let r = "M-006A_A-0001-22-26C0-O0079_O-0_R-10001";
        assert_eq!(com_object_number(r).unwrap(), 0);
        assert_eq!(
            application_program_ref(r).unwrap(),
            "M-006A_A-0001-22-26C0-O0079"
        );
        // The program id itself contains "-O0079"; only the "_O-<n>_R-" tail counts.
        assert_eq!(
            com_object_number("M-0083_A-0019-16-ECA7_O-59_R-149").unwrap(),
            59
        );
    }

    #[test]
    fn a_ref_id_without_the_object_tail_is_an_error() {
        assert!(matches!(
            com_object_number("M-0083_A-0019-16-ECA7"),
            Err(ValueError::MalformedRefId(_))
        ));
        // A parameter RefId is not a communication object RefId.
        assert!(com_object_number("M-0083_A-0026-15-7565_UP-411_R-411").is_err());
    }

    #[test]
    fn completion_status_and_building_part_type_cover_the_observed_values() {
        assert_eq!(
            parse_completion_status("FinishedDesign").unwrap(),
            CompletionStatus::FinishedDesign
        );
        assert_eq!(
            parse_building_part_type("DistributionBoard").unwrap(),
            BuildingPartType::DistributionBoard
        );
        assert!(matches!(
            parse_building_part_type("Cupboard"),
            Err(ValueError::UnknownEnumValue {
                kind: "BuildingPart/@Type",
                ..
            })
        ));
    }
}
