//! `knx device readiness`: which of a project's devices KNXBench can download.
//!
//! Offline, per project device: the download `knx device download` would
//! prepare, graded as that command grades it
//! ([`knx_app::project_readiness`]). No socket, no gateway flag.

use std::fmt::Write as _;

use knx_app::download_support::SupportLevel;
use knx_app::project_readiness::{DeviceReadiness, DeviceRow, ReadinessSummary};

/// `knx device readiness`'s arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadinessArgs {
    /// The project.
    pub project: String,
    /// The product database, if not the default one.
    pub product_db: Option<String>,
}

/// Parses them. Every flag besides these two is refused, `--gateway`
/// included: this command never opens a connection.
pub fn parse_readiness_args(args: &[String]) -> Result<ReadinessArgs, String> {
    let mut project = None;
    let mut product_db = None;
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
            other => {
                return Err(format!(
                    "unexpected argument {other} (readiness is offline and takes only \
                     --project and --product-db)"
                ))
            }
        }
        i += 1;
    }
    Ok(ReadinessArgs {
        project: project.ok_or("--project <path.knxdb> is required")?,
        product_db,
    })
}

/// One line per device, then the counts.
pub fn format_readiness(rows: &[DeviceRow]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "== download readiness per project device: offline, nothing sent =="
    );
    for row in rows {
        let address = row
            .address
            .map_or_else(|| "-".to_string(), |address| address.to_string());
        let detail = match &row.readiness {
            DeviceReadiness::Excluded => {
                "on the project exclusion list: never contacted".to_string()
            }
            DeviceReadiness::NoAddress => "no individual address".to_string(),
            DeviceReadiness::Graded(SupportLevel::Verified { evidence, .. }) => {
                format!("verified on {}, {}", evidence.device, evidence.date)
            }
            DeviceReadiness::Graded(SupportLevel::Untested {
                steps,
                octets,
                inferences,
            }) => {
                let mut detail =
                    format!("{steps} steps, {octets} octets; never downloaded on hardware");
                for inference in inferences {
                    let _ = write!(detail, "\n           inference {inference}");
                }
                detail
            }
            DeviceReadiness::Graded(SupportLevel::Unsupported { category, detail }) => {
                format!("{category}: {detail}")
            }
        };
        let _ = writeln!(
            out,
            "{address:<10} {:<11} {}  {detail}",
            row.readiness.code(),
            row.name
        );
    }
    let summary = ReadinessSummary::of(rows);
    let counts: Vec<String> = summary
        .by_code
        .iter()
        .map(|(code, count)| format!("{count} {code}"))
        .collect();
    let _ = writeln!(out, "\n{} devices: {}", summary.devices, counts.join(", "));
    for (category, count) in &summary.unsupported {
        let _ = writeln!(out, "  unsupported {category}: {count}");
    }
    let _ = writeln!(
        out,
        "`untested` plans completely but needs --accept-untested to write; \
         `knx device compare` shows what it would change, read only"
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_app::download_support::UnsupportedCategory;
    use knx_productdb::inference::Inference;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn it_takes_a_project_and_refuses_a_gateway() {
        assert_eq!(
            parse_readiness_args(&args("--project p.knxdb --product-db d.sqlite")),
            Ok(ReadinessArgs {
                project: "p.knxdb".into(),
                product_db: Some("d.sqlite".into()),
            })
        );
        assert!(parse_readiness_args(&args("--product-db d")).is_err());
        let error =
            parse_readiness_args(&args("--project p --gateway 192.0.2.1:3671")).unwrap_err();
        assert!(error.contains("offline"), "{error}");
        assert!(parse_readiness_args(&args("--project p 1.1.5")).is_err());
    }

    #[test]
    fn each_device_gets_a_line_and_the_counts_follow() {
        let rows = vec![
            DeviceRow {
                address: Some("1.1.10".parse().unwrap()),
                name: "Schaltaktor A.1".into(),
                program_ref: "P".into(),
                readiness: DeviceReadiness::Graded(SupportLevel::Untested {
                    steps: 12,
                    octets: 1778,
                    inferences: vec![],
                }),
            },
            DeviceRow {
                address: Some("1.1.11".parse().unwrap()),
                name: "Schaltaktor A.2".into(),
                program_ref: "P13".into(),
                readiness: DeviceReadiness::Graded(SupportLevel::Untested {
                    steps: 12,
                    octets: 1778,
                    inferences: vec![Inference {
                        rule: "union-later-member",
                        detail: "UP-33_R-33 is not written".into(),
                        reference: "RESEARCH §19.12",
                    }],
                }),
            },
            DeviceRow {
                address: Some("1.1.22".parse().unwrap()),
                name: "SmartSensor".into(),
                program_ref: "Q".into(),
                readiness: DeviceReadiness::Graded(SupportLevel::Unsupported {
                    category: UnsupportedCategory::NotMemoryMapped,
                    detail: "load procedure: mask 07B0h".into(),
                }),
            },
            DeviceRow {
                address: Some("1.1.220".parse().unwrap()),
                name: "Alarm".into(),
                program_ref: "R".into(),
                readiness: DeviceReadiness::Excluded,
            },
        ];
        let out = format_readiness(&rows);
        assert!(
            out.contains("1.1.10     untested    Schaltaktor A.1  12 steps, 1778 octets"),
            "{out}"
        );
        assert!(
            out.contains(
                "1.1.22     unsupported SmartSensor  not-memory-mapped: load procedure: mask 07B0h"
            ),
            "{out}"
        );
        assert!(
            out.contains("1.1.220    excluded    Alarm  on the project exclusion list"),
            "{out}"
        );
        assert!(
            out.contains(
                "1.1.11     untested    Schaltaktor A.2  12 steps, 1778 octets; never downloaded \
                 on hardware\n           inference union-later-member: UP-33_R-33 is not written \
                 (RESEARCH §19.12)\n"
            ),
            "{out}"
        );
        assert!(
            out.contains("4 devices: 1 excluded, 1 unsupported, 2 untested"),
            "{out}"
        );
        assert!(out.contains("  unsupported not-memory-mapped: 1"), "{out}");
    }
}
