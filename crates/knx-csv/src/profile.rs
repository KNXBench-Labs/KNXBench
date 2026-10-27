//! Fixed header profiles for address-only group-address CSV import (see the profiles contract).

use knx_core::{DptRef, GroupAddress, GroupAddressStyle};

use crate::read::{
    read_bool_column, CsvAction, CsvProblem, CsvRow, IgnoredColumn, IgnoredColumnReason, Severity,
};

const SECURITY_AUTO_WARNING: &str = "Security Auto is not imported; project security is unchanged";
const RANGE_WARNING: &str =
    "group range name and address are not imported; this import only creates and updates addresses";

/// How a profile spells the row's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Names {
    Single(usize),
    Triplet(usize),
}

/// How a profile spells the row's address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Addresses {
    Composite(usize),
    Split(usize),
}

/// One of the four complete, ordered header shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Profile {
    names: Names,
    addresses: Addresses,
    central: usize,
}

impl Profile {
    const fn description(self) -> usize {
        self.central + 2
    }
    const fn datapoint_type(self) -> usize {
        self.central + 3
    }
    const fn security(self) -> usize {
        self.central + 4
    }
}

const SHAPES: [(&[&str], Profile); 4] = [
    (
        &[
            "group name",
            "address",
            "central",
            "unfiltered",
            "description",
            "datapointtype",
            "security",
        ],
        Profile {
            names: Names::Single(0),
            addresses: Addresses::Composite(1),
            central: 2,
        },
    ),
    (
        &[
            "group name",
            "main",
            "middle",
            "sub",
            "central",
            "unfiltered",
            "description",
            "datapointtype",
            "security",
        ],
        Profile {
            names: Names::Single(0),
            addresses: Addresses::Split(1),
            central: 4,
        },
    ),
    (
        &[
            "main",
            "middle",
            "sub",
            "address",
            "central",
            "unfiltered",
            "description",
            "datapointtype",
            "security",
        ],
        Profile {
            names: Names::Triplet(0),
            addresses: Addresses::Composite(3),
            central: 4,
        },
    ),
    (
        &[
            "main",
            "middle",
            "sub",
            "main",
            "middle",
            "sub",
            "central",
            "unfiltered",
            "description",
            "datapointtype",
            "security",
        ],
        Profile {
            names: Names::Triplet(0),
            addresses: Addresses::Split(3),
            central: 6,
        },
    ),
];

/// What the header row says about the file's format.
pub(crate) enum Detected {
    Native,
    Profile(Profile),
    Refused(String),
}

/// Classifies `header`: a profile, native, or a refusal for external-looking headers that are
/// not exactly one supported shape or do not fit the project's address style.
pub(crate) fn detect(header: &csv::StringRecord, style: GroupAddressStyle) -> Detected {
    let cells: Vec<String> = header
        .iter()
        .map(|c| c.trim().to_ascii_lowercase())
        .collect();
    // Native required columns keep their established precedence. Extra columns,
    // including profile-like names, are reported by the native reader as unknown.
    if cells.iter().any(|c| c == "address") && cells.iter().any(|c| c == "name") {
        return Detected::Native;
    }
    if let Some((_, profile)) = SHAPES.iter().find(|(shape, _)| cells == *shape) {
        return if style == GroupAddressStyle::ThreeLevel {
            Detected::Profile(*profile)
        } else {
            Detected::Refused(
                "group-address CSV profiles require a ThreeLevel project address style".to_string(),
            )
        };
    }
    let external = cells
        .iter()
        .any(|c| matches!(c.as_str(), "group name" | "main" | "middle" | "sub"));
    if external {
        Detected::Refused(
            "header looks like a group-address CSV profile but is not exactly one of the four \
             supported column orders"
                .to_string(),
        )
    } else {
        Detected::Native
    }
}

/// The columns a profile reads, validates, and never applies.
pub(crate) fn ignored_columns(header: &csv::StringRecord, profile: Profile) -> Vec<IgnoredColumn> {
    [
        profile.description(),
        profile.datapoint_type(),
        profile.security(),
    ]
    .into_iter()
    .map(|index| IgnoredColumn {
        name: header.get(index).unwrap_or("").trim().to_string(),
        reason: IgnoredColumnReason::ReadOnly,
    })
    .collect()
}

/// What an address cell group denotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Leaf(GroupAddress),
    MainRange,
    MiddleRange,
}

/// Parses one data record. Returns a row for a valid leaf; a valid range row yields only a
/// warning. Every supplied field is validated before a range row is skipped.
pub(crate) fn parse_record(
    profile: Profile,
    record: &csv::StringRecord,
    line: usize,
) -> (Option<CsvRow>, Vec<CsvProblem>) {
    let mut problems = Vec::new();
    let mut ok = true;
    let fail = |problems: &mut Vec<CsvProblem>, detail: String| {
        problems.push(CsvProblem {
            row: Some(line),
            severity: Severity::Error,
            detail,
        });
        false
    };
    let cell = |index: usize| record.get(index).unwrap_or("");

    let target = match parse_target(profile.addresses, record) {
        Ok(target) => Some(target),
        Err(detail) => {
            ok = fail(&mut problems, detail);
            None
        }
    };

    let name = match (profile.names, target) {
        (Names::Single(index), _) => {
            if cell(index).trim().is_empty() {
                ok = fail(&mut problems, "name is blank".to_string());
            }
            cell(index).to_string()
        }
        (Names::Triplet(first), Some(target)) => {
            let depth = match target {
                Target::MainRange => 0,
                Target::MiddleRange => 1,
                Target::Leaf(_) => 2,
            };
            let named: Vec<usize> = (0..3)
                .filter(|d| !cell(first + d).trim().is_empty())
                .collect();
            if named == [depth] {
                cell(first + depth).to_string()
            } else {
                let level = ["Main", "Middle", "Sub"][depth];
                ok = fail(
                    &mut problems,
                    format!("name must be given exactly once, in the {level} name column"),
                );
                String::new()
            }
        }
        (Names::Triplet(_), None) => String::new(),
    };

    let is_range = matches!(target, Some(Target::MainRange | Target::MiddleRange));
    let (central, unfiltered) = if is_range {
        for (index, label) in [
            (profile.central, "Central"),
            (profile.central + 1, "Unfiltered"),
        ] {
            if !cell(index).trim().is_empty() {
                ok = fail(
                    &mut problems,
                    format!("column \"{label}\" is not supported on range rows"),
                );
            }
        }
        (None, None)
    } else {
        (
            read_bool_column(
                record,
                Some(profile.central),
                "Central",
                line,
                &mut problems,
                &mut ok,
            ),
            read_bool_column(
                record,
                Some(profile.central + 1),
                "Unfiltered",
                line,
                &mut problems,
                &mut ok,
            ),
        )
    };

    if !cell(profile.description()).is_empty() {
        ok = fail(
            &mut problems,
            "column \"Description\" is not supported; leave it empty".to_string(),
        );
    }

    let dpt_raw = cell(profile.datapoint_type()).trim();
    let datapoint_type = if dpt_raw.is_empty() {
        None
    } else if is_range {
        ok = fail(
            &mut problems,
            "column \"DatapointType\" is not supported on range rows".to_string(),
        );
        None
    } else if !is_dpt_token(dpt_raw) {
        ok = fail(
            &mut problems,
            format!(
                "column \"DatapointType\": invalid datapoint type {dpt_raw:?}: \
                 expected DPT-<n> or DPST-<n>-<n>"
            ),
        );
        None
    } else {
        match DptRef::parse(dpt_raw) {
            Ok(dpt) => Some(dpt.to_string()),
            Err(error) => {
                ok = fail(&mut problems, format!("column \"DatapointType\": {error}"));
                None
            }
        }
    };

    match cell(profile.security()) {
        "" => {}
        "Auto" => problems.push(CsvProblem {
            row: Some(line),
            severity: Severity::Warning,
            detail: SECURITY_AUTO_WARNING.to_string(),
        }),
        other => {
            ok = fail(
                &mut problems,
                format!("column \"Security\": unsupported value {other:?}; only empty or Auto is accepted"),
            );
        }
    }

    match (ok, target) {
        (true, Some(Target::Leaf(address))) => {
            let row = CsvRow {
                line,
                address,
                action: CsvAction::Upsert,
                new_address: None,
                name,
                central,
                unfiltered,
                datapoint_type,
                main_group: None,
                middle_group: None,
            };
            (Some(row), problems)
        }
        (true, _) => {
            problems.push(CsvProblem {
                row: Some(line),
                severity: Severity::Warning,
                detail: RANGE_WARNING.to_string(),
            });
            (None, problems)
        }
        (false, _) => (None, problems),
    }
}

fn parse_target(addresses: Addresses, record: &csv::StringRecord) -> Result<Target, String> {
    match addresses {
        Addresses::Composite(index) => parse_composite(record.get(index).unwrap_or("").trim()),
        Addresses::Split(first) => {
            let part = |d: usize| record.get(first + d).unwrap_or("").trim();
            parse_split([part(0), part(1), part(2)])
        }
    }
}

fn parse_composite(text: &str) -> Result<Target, String> {
    let parts: Vec<&str> = text.split('/').collect();
    let [main, middle, sub] = parts[..] else {
        return Err(format!(
            "invalid address {text:?}: expected n/n/n, n/n/- or n/-/-"
        ));
    };
    match (middle, sub) {
        ("-", "-") => range_target(main, "0", Target::MainRange),
        (_, "-") => range_target(main, middle, Target::MiddleRange),
        _ => leaf_target(main, middle, sub),
    }
}

fn parse_split([main, middle, sub]: [&str; 3]) -> Result<Target, String> {
    match (main.is_empty(), middle.is_empty(), sub.is_empty()) {
        (false, true, true) => range_target(main, "0", Target::MainRange),
        (false, false, true) => range_target(main, middle, Target::MiddleRange),
        (false, false, false) => leaf_target(main, middle, sub),
        _ => Err(format!(
            "invalid split address {main:?}/{middle:?}/{sub:?}: fill Main, then Middle, then Sub without gaps"
        )),
    }
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// The contract spelling `DPT-<digits>` or `DPST-<digits>-<digits>`; checked before
/// `DptRef::parse`, which also accepts signed numbers.
fn is_dpt_token(text: &str) -> bool {
    if let Some(rest) = text.strip_prefix("DPST-") {
        matches!(rest.split_once('-'), Some((main, sub)) if is_digits(main) && is_digits(sub))
    } else if let Some(rest) = text.strip_prefix("DPT-") {
        is_digits(rest)
    } else {
        false
    }
}

fn leaf_target(main: &str, middle: &str, sub: &str) -> Result<Target, String> {
    if ![main, middle, sub].into_iter().all(is_digits) {
        return Err(format!(
            "invalid address {main}/{middle}/{sub}: components must be numbers"
        ));
    }
    match GroupAddress::parse(
        &format!("{main}/{middle}/{sub}"),
        GroupAddressStyle::ThreeLevel,
    ) {
        Ok(a) if a.raw() == 0 => {
            Err("group address 0 is reserved for broadcast and cannot be used".to_string())
        }
        Ok(a) => Ok(Target::Leaf(a)),
        Err(error) => Err(error.to_string()),
    }
}

/// Validates the numeric parts of a range through the same parser as leaves; `middle` is
/// `"0"` for a main range.
fn range_target(main: &str, middle: &str, target: Target) -> Result<Target, String> {
    if !is_digits(main) || !is_digits(middle) {
        return Err(format!(
            "invalid range address {main}/{middle}: components must be numbers"
        ));
    }
    GroupAddress::parse(&format!("{main}/{middle}/0"), GroupAddressStyle::ThreeLevel)
        .map(|_| target)
        .map_err(|error| error.to_string())
}
