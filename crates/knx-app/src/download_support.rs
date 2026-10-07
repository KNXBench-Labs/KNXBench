//! Download support levels for product application programs.
//!
//! No tool owns every KNX device, so a download to an application nobody has
//! tried on a bus must be possible, and must say so. This module answers,
//! per application program, one of three things:
//!
//! - [`SupportLevel::Verified`]: the plan builds, and a download of this
//!   program has run on real hardware and been read back. The evidence is
//!   data ([`VerifiedEvidence`], `data/verified_downloads.json`), not code:
//!   adding a program there is a documentation change with a reference to
//!   the run that justifies it.
//! - [`SupportLevel::Untested`]: the product's own load procedure translates
//!   into a complete plan offline (product defaults, no group links), but no
//!   download of it has been seen on a bus. A write needs an extra
//!   acknowledgement ([`SupportLevel::needs_acknowledgement`]).
//! - [`SupportLevel::Unsupported`]: the plan does not build; the reason is
//!   the refusal the download itself would give, by category and in full.
//!
//! What the offline check does *not* show: that a project's own values and
//! links build (the download prepares its own plan and refuses on its own
//! terms), or that the device behaves as its product file says. That is
//! what `Verified` adds.

use std::collections::BTreeMap;
use std::fmt;

use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::IndividualAddress;
use knx_productdb::code::load_program_code;
use knx_productdb::download_plan::{
    check_program_kind, plan_memory_download_with_inferences, DownloadPlanError,
};
use knx_productdb::image::{build_download_image, ImageError, ImageRequest};
use knx_productdb::inference::Inference;
use knx_productdb::query::programs;
use knx_productdb::{Connection, ProductDbError};
use serde::Deserialize;

/// The shipped evidence file. Data, not code: see the module note.
const VERIFIED_DOWNLOADS: &str = include_str!("../data/verified_downloads.json");

/// The individual address the offline check builds its address table for.
/// Never sent anywhere: the check has no socket. `15.15.255` is the
/// highest address, chosen so that nothing mistakes it for a real device.
const OFFLINE_ADDRESS: &str = "15.15.255";

/// One program's verified download, as the evidence file records it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedEvidence {
    /// The application program id.
    pub program_id: String,
    /// What was downloaded: `"complete"` or `"partial-parameters"`, ….
    pub scopes: Vec<String>,
    /// The device it ran on, as the operator knows it (address and mask).
    pub device: String,
    /// When (ISO date).
    pub date: String,
    /// Where the run and its read-back are documented.
    pub reference: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceFile {
    format: u32,
    verified: Vec<VerifiedEvidence>,
}

/// The only evidence-file format this build reads.
const EVIDENCE_FORMAT: u32 = 1;

/// Why the evidence file was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    /// Not JSON of the expected shape.
    Malformed(String),
    /// A format this build does not read.
    Format(u32),
    /// A program listed twice.
    Duplicate(String),
}

impl std::error::Error for EvidenceError {}

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(e) => write!(f, "verified-download evidence is malformed: {e}"),
            Self::Format(v) => write!(
                f,
                "verified-download evidence has format {v}; this build reads {EVIDENCE_FORMAT}"
            ),
            Self::Duplicate(id) => write!(f, "verified-download evidence lists {id} twice"),
        }
    }
}

/// Parses an evidence file, by program id.
pub fn parse_evidence(text: &str) -> Result<BTreeMap<String, VerifiedEvidence>, EvidenceError> {
    let file: EvidenceFile =
        serde_json::from_str(text).map_err(|e| EvidenceError::Malformed(e.to_string()))?;
    if file.format != EVIDENCE_FORMAT {
        return Err(EvidenceError::Format(file.format));
    }
    let mut by_program = BTreeMap::new();
    for evidence in file.verified {
        let id = evidence.program_id.clone();
        if by_program.insert(id.clone(), evidence).is_some() {
            return Err(EvidenceError::Duplicate(id));
        }
    }
    Ok(by_program)
}

/// The evidence this build ships with.
pub fn shipped_evidence() -> Result<BTreeMap<String, VerifiedEvidence>, EvidenceError> {
    parse_evidence(VERIFIED_DOWNLOADS)
}

/// A refusal's category: stable, short, and the same for every program
/// refused for the same reason, so that a corpus can be counted by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnsupportedCategory {
    /// The mask does not load through memory (not `070nh`).
    NotMemoryMapped,
    /// A `LoadProcedureStyle` other than `ProductProcedure`.
    ProcedureStyle,
    /// A load-procedure step the translation does not know.
    UnmodelledStep,
    /// Merge procedures, or not exactly one procedure.
    ProcedureShape,
    /// The load procedure's contents (segments, flags, addresses) do not
    /// fit a BIM M112 download.
    ProcedureContents,
    /// The parameter tree could not be decided with product defaults.
    ParameterEvaluation,
    /// Module instances (`ModuleDef`), not written by a download.
    Modules,
    /// A default value that does not fit its parameter's type, or two
    /// active references that disagree: the product data contradicts
    /// itself, or uses a value encoding the image builder does not read.
    ParameterValue,
    /// A product structure the image builder does not write.
    ImageStructure,
    /// The product database itself failed or lacks the program's code.
    ProductData,
    /// The project's configuration of the device does not yield a request:
    /// no program named or installed, conflicting values or links. Only a
    /// project device has one; the product-default coverage never reports
    /// it.
    Configuration,
}

impl UnsupportedCategory {
    /// The category's stable name, for reports and the API.
    pub fn code(self) -> &'static str {
        match self {
            Self::NotMemoryMapped => "not-memory-mapped",
            Self::ProcedureStyle => "procedure-style",
            Self::UnmodelledStep => "unmodelled-step",
            Self::ProcedureShape => "procedure-shape",
            Self::ProcedureContents => "procedure-contents",
            Self::ParameterEvaluation => "parameter-evaluation",
            Self::Modules => "modules",
            Self::ParameterValue => "parameter-value",
            Self::ImageStructure => "image-structure",
            Self::ProductData => "product-data",
            Self::Configuration => "configuration",
        }
    }
}

impl fmt::Display for UnsupportedCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// How far a download to one program is supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportLevel {
    /// The plan builds and a download ran on hardware.
    Verified {
        /// The run.
        evidence: VerifiedEvidence,
        /// Steps and octets of the offline plan.
        steps: usize,
        /// Segment octets the offline plan writes.
        octets: usize,
    },
    /// The plan builds offline; no download of it was seen on a bus, or it
    /// rests on an inference the hardware run did not need.
    Untested {
        /// Steps of the offline plan.
        steps: usize,
        /// Segment octets the offline plan writes.
        octets: usize,
        /// The inferences the plan rests on (ADR-0086), for the user to
        /// see before acknowledging a write.
        inferences: Vec<Inference>,
    },
    /// The plan does not build.
    Unsupported {
        /// The category.
        category: UnsupportedCategory,
        /// The refusal, in full.
        detail: String,
    },
}

impl SupportLevel {
    /// The level's stable name: `verified`, `untested` or `unsupported`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Verified { .. } => "verified",
            Self::Untested { .. } => "untested",
            Self::Unsupported { .. } => "unsupported",
        }
    }

    /// Whether a write to hardware needs the operator to acknowledge that
    /// this program has never been downloaded on a bus.
    pub fn needs_acknowledgement(&self) -> bool {
        matches!(self, Self::Untested { .. })
    }

    /// The level of a program whose plan built (`Some`) or was refused.
    /// A plan resting on `inferences` is `Untested` even with evidence: the
    /// hardware run did not exercise them (ADR-0086).
    pub fn from_outcome(
        outcome: Result<&MemoryDownloadPlan, (UnsupportedCategory, String)>,
        evidence: Option<&VerifiedEvidence>,
        inferences: &[Inference],
    ) -> Self {
        match (outcome, evidence) {
            (Ok(plan), Some(evidence)) if inferences.is_empty() => Self::Verified {
                evidence: evidence.clone(),
                steps: plan.steps.len(),
                octets: plan.data_octets(),
            },
            (Ok(plan), _) => Self::Untested {
                steps: plan.steps.len(),
                octets: plan.data_octets(),
                inferences: inferences.to_vec(),
            },
            // Evidence never overrides a refusal: a program that no longer
            // plans is unsupported, whatever ran once.
            (Err((category, detail)), _) => Self::Unsupported { category, detail },
        }
    }
}

impl fmt::Display for SupportLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verified { evidence, .. } => write!(
                f,
                "verified: downloaded on {} ({}), {}; see {}",
                evidence.device,
                evidence.scopes.join(", "),
                evidence.date,
                evidence.reference
            ),
            Self::Untested { inferences, .. } => {
                f.write_str(
                    "untested: the product's load procedure plans completely offline, \
                     but no download of this application has been seen on a bus",
                )?;
                for inference in inferences {
                    write!(f, "; inference {inference}")?;
                }
                Ok(())
            }
            Self::Unsupported { category, detail } => {
                write!(f, "unsupported ({category}): {detail}")
            }
        }
    }
}

/// The category of an image refusal.
pub fn image_category(error: &ImageError) -> UnsupportedCategory {
    match error {
        ImageError::Database(_) | ImageError::Code(_) | ImageError::UnknownProgram(_) => {
            UnsupportedCategory::ProductData
        }
        ImageError::Evaluation(_) => UnsupportedCategory::ParameterEvaluation,
        ImageError::Unsupported { what } if what.ends_with("a module instance") => {
            UnsupportedCategory::Modules
        }
        ImageError::Value { .. }
        | ImageError::ConflictingValues { .. }
        | ImageError::Overlap(_)
        | ImageError::Parameter { .. } => UnsupportedCategory::ParameterValue,
        _ => UnsupportedCategory::ImageStructure,
    }
}

/// The category of a plan refusal.
pub fn plan_category(error: &DownloadPlanError) -> UnsupportedCategory {
    match error {
        DownloadPlanError::NotMemoryMapped(_) => UnsupportedCategory::NotMemoryMapped,
        DownloadPlanError::NotAProductProcedure(_) => UnsupportedCategory::ProcedureStyle,
        DownloadPlanError::Unmodelled(_) => UnsupportedCategory::UnmodelledStep,
        DownloadPlanError::ProcedureCount(_) | DownloadPlanError::DoesNotConnectFirst => {
            UnsupportedCategory::ProcedureShape
        }
        _ => UnsupportedCategory::ProcedureContents,
    }
}

/// The offline plan of `program_id` with product defaults and no links.
pub fn offline_plan(
    conn: &Connection,
    program_id: &str,
) -> Result<MemoryDownloadPlan, (UnsupportedCategory, String)> {
    offline_download(conn, program_id).map(|(plan, _)| plan)
}

/// [`offline_plan`] and the inferences it rests on.
pub fn offline_download(
    conn: &Connection,
    program_id: &str,
) -> Result<(MemoryDownloadPlan, Vec<Inference>), (UnsupportedCategory, String)> {
    check_kind(conn, program_id)?;
    let request = ImageRequest {
        program_id: program_id.to_owned(),
        individual_address: OFFLINE_ADDRESS
            .parse::<IndividualAddress>()
            .expect("a valid constant address"),
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
    };
    let image = build_download_image(conn, &request)
        .map_err(|e| (image_category(&e), format!("memory image: {e}")))?;
    plan_memory_download_with_inferences(&image)
        .map_err(|e| (plan_category(&e), format!("load procedure: {e}")))
}

/// Refuses a program of a kind no download translates, before its image is
/// built ([`check_program_kind`]). A program the database lacks passes: its
/// image build names that.
pub fn check_kind(
    conn: &Connection,
    program_id: &str,
) -> Result<(), (UnsupportedCategory, String)> {
    match load_program_code(conn, program_id) {
        Ok(Some(code)) => check_program_kind(&code)
            .map(|_| ())
            .map_err(|e| (plan_category(&e), format!("load procedure: {e}"))),
        Ok(None) => Ok(()),
        Err(e) => Err((
            UnsupportedCategory::ProductData,
            format!("program code: {e}"),
        )),
    }
}

/// The support level of one program.
pub fn support_level(
    conn: &Connection,
    program_id: &str,
    evidence: &BTreeMap<String, VerifiedEvidence>,
) -> SupportLevel {
    let outcome = offline_download(conn, program_id);
    let (plan, inferences) = match &outcome {
        Ok((plan, inferences)) => (Ok(plan), inferences.as_slice()),
        Err(refusal) => (Err(refusal.clone()), &[][..]),
    };
    SupportLevel::from_outcome(plan, evidence.get(program_id), inferences)
}

/// The evidence-file name of a download's scope: `complete`,
/// `partial-parameters`, `partial-group-addresses` or `partial-both`.
pub fn scope_code(partial: Option<PartialDownloadParts>) -> &'static str {
    match partial.map(|parts| (parts.parameters, parts.group_addresses)) {
        None => "complete",
        Some((true, true)) => "partial-both",
        Some((true, false)) => "partial-parameters",
        Some(_) => "partial-group-addresses",
    }
}

/// The level of one prepared download: its plan built, so it is
/// `Verified` when the evidence names this program *and this scope* and the
/// plan rests on no inference, and `Untested` otherwise. A partial download
/// of a program verified only complete is untested: it runs other steps.
pub fn download_level(
    program_id: &str,
    partial: Option<PartialDownloadParts>,
    plan: &MemoryDownloadPlan,
    inferences: &[Inference],
    evidence: &BTreeMap<String, VerifiedEvidence>,
) -> SupportLevel {
    let scope = scope_code(partial);
    let matching = evidence
        .get(program_id)
        .filter(|evidence| evidence.scopes.iter().any(|s| s == scope));
    SupportLevel::from_outcome(Ok(plan), matching, inferences)
}

/// The phrase that acknowledges an untested download to `target`. Spelled
/// out on purpose, like the write confirmation: it names what is accepted.
pub fn untested_acknowledgement(target: IndividualAddress) -> String {
    format!("I accept an untested download to {target}")
}

/// One program's row in a coverage report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramSupport {
    /// The program id.
    pub program_id: String,
    /// Its manufacturer id.
    pub manufacturer_id: String,
    /// Its name, if the product gives one.
    pub name: Option<String>,
    /// Its `MaskVersion`, verbatim.
    pub mask_version: Option<String>,
    /// The level.
    pub level: SupportLevel,
}

/// The support level of every program in the database, optionally of one
/// manufacturer, ordered by id.
pub fn coverage(
    conn: &Connection,
    manufacturer: Option<&str>,
    evidence: &BTreeMap<String, VerifiedEvidence>,
) -> Result<Vec<ProgramSupport>, ProductDbError> {
    Ok(programs(conn, manufacturer)?
        .into_iter()
        .map(|row| ProgramSupport {
            level: support_level(conn, &row.id, evidence),
            program_id: row.id,
            manufacturer_id: row.manufacturer_id,
            name: row.name,
            mask_version: row.mask_version,
        })
        .collect())
}

/// Counts by level and, for refusals, by category.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageSummary {
    /// Programs examined.
    pub programs: usize,
    /// Verified on hardware.
    pub verified: usize,
    /// Planned offline, never seen on a bus.
    pub untested: usize,
    /// Refused, by category.
    pub unsupported: BTreeMap<UnsupportedCategory, usize>,
    /// Per `MaskVersion`: (programs, of which plan).
    pub by_mask: BTreeMap<String, (usize, usize)>,
}

impl CoverageSummary {
    /// Summarises `rows`.
    pub fn of(rows: &[ProgramSupport]) -> Self {
        let mut summary = Self {
            programs: rows.len(),
            ..Self::default()
        };
        for row in rows {
            let plans = match &row.level {
                SupportLevel::Verified { .. } => {
                    summary.verified += 1;
                    true
                }
                SupportLevel::Untested { .. } => {
                    summary.untested += 1;
                    true
                }
                SupportLevel::Unsupported { category, .. } => {
                    *summary.unsupported.entry(*category).or_default() += 1;
                    false
                }
            };
            let mask = row.mask_version.clone().unwrap_or_else(|| "(none)".into());
            let entry = summary.by_mask.entry(mask).or_default();
            entry.0 += 1;
            entry.1 += usize::from(plans);
        }
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::commissioning::load_state::MaskVersion;
    use knx_core::commissioning::memory_download::MemoryDownloadStep;

    fn evidence(id: &str) -> VerifiedEvidence {
        VerifiedEvidence {
            program_id: id.into(),
            scopes: vec!["complete".into()],
            device: "1.1.67 (0701h)".into(),
            date: "2026-09-28".into(),
            reference: "docs/RESEARCH.md §19.4".into(),
        }
    }

    fn plan() -> MemoryDownloadPlan {
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0x0083,
            steps: vec![
                MemoryDownloadStep::Connect,
                MemoryDownloadStep::WriteMemory {
                    address: 0x4400,
                    octets: vec![1, 2, 3],
                },
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    #[test]
    fn the_shipped_evidence_reads_and_names_the_push_button() {
        let shipped = shipped_evidence().expect("the shipped file parses");
        let mdt = shipped
            .get("M-0083_A-0027-15-0BAC")
            .expect("the push button is verified");
        assert!(mdt.scopes.iter().any(|s| s == "complete"), "{mdt:?}");
        // Every scope ran live on 1.1.67 (RESEARCH §19.4, §19.8, §19.15,
        // §19.17), and nothing else is listed.
        for ran in [
            "partial-parameters",
            "partial-group-addresses",
            "partial-both",
        ] {
            assert!(mdt.scopes.iter().any(|s| s == ran), "{ran}: {mdt:?}");
        }
        assert_eq!(mdt.scopes.len(), 4, "{mdt:?}");
        assert!(mdt.reference.starts_with("docs/"), "{mdt:?}");
    }

    #[test]
    fn a_plan_with_evidence_is_verified_and_without_is_untested() {
        let plan = plan();
        let verified = SupportLevel::from_outcome(Ok(&plan), Some(&evidence("P")), &[]);
        assert_eq!(verified.code(), "verified");
        assert!(!verified.needs_acknowledgement());
        let untested = SupportLevel::from_outcome(Ok(&plan), None, &[]);
        assert_eq!(
            untested,
            SupportLevel::Untested {
                steps: 3,
                octets: 3,
                inferences: vec![],
            }
        );
        assert!(untested.needs_acknowledgement());
    }

    /// ADR-0086: a hardware run of a program does not cover a plan that
    /// rests on an inference the run did not need.
    #[test]
    fn a_plan_resting_on_an_inference_is_untested_and_names_it() {
        let plan = plan();
        let inference = Inference {
            rule: "union-later-member",
            detail: "UP-1_R-1 is not written".into(),
            reference: "RESEARCH §19.12",
        };
        let level = SupportLevel::from_outcome(
            Ok(&plan),
            Some(&evidence("P")),
            std::slice::from_ref(&inference),
        );
        assert_eq!(
            level,
            SupportLevel::Untested {
                steps: 3,
                octets: 3,
                inferences: vec![inference],
            }
        );
        assert!(level.needs_acknowledgement());
        assert!(
            level
                .to_string()
                .contains("union-later-member: UP-1_R-1 is not written (RESEARCH §19.12)"),
            "{level}"
        );
    }

    #[test]
    fn evidence_never_turns_a_refusal_into_support() {
        let level = SupportLevel::from_outcome(
            Err((UnsupportedCategory::UnmodelledStep, "LdCtrlMerge".into())),
            Some(&evidence("P")),
            &[],
        );
        assert_eq!(level.code(), "unsupported");
        assert!(!level.needs_acknowledgement());
        assert!(level.to_string().contains("unmodelled-step"), "{level}");
    }

    #[test]
    fn plan_refusals_fall_into_their_categories() {
        assert_eq!(
            plan_category(&DownloadPlanError::NotMemoryMapped(Some("MV-07B0".into()))),
            UnsupportedCategory::NotMemoryMapped
        );
        assert_eq!(
            plan_category(&DownloadPlanError::NotAProductProcedure(Some(
                "MergedProcedure".into()
            ))),
            UnsupportedCategory::ProcedureStyle
        );
        assert_eq!(
            plan_category(&DownloadPlanError::Unmodelled("LdCtrlWriteProp".into())),
            UnsupportedCategory::UnmodelledStep
        );
        assert_eq!(
            plan_category(&DownloadPlanError::ProcedureCount(2)),
            UnsupportedCategory::ProcedureShape
        );
        assert_eq!(
            plan_category(&DownloadPlanError::AllocatedTwice(0x4000)),
            UnsupportedCategory::ProcedureContents
        );
    }

    #[test]
    fn image_refusals_fall_into_their_categories() {
        assert_eq!(
            image_category(&ImageError::UnknownProgram("X".into())),
            UnsupportedCategory::ProductData
        );
        assert_eq!(
            image_category(&ImageError::Evaluation(vec![])),
            UnsupportedCategory::ParameterEvaluation
        );
        assert_eq!(
            image_category(&ImageError::Unsupported {
                what: "P_MD-1_R-1: a module instance".into()
            }),
            UnsupportedCategory::Modules
        );
        assert_eq!(
            image_category(&ImageError::Unsupported {
                what: "a parameter in a relative segment".into()
            }),
            UnsupportedCategory::ImageStructure
        );
        assert_eq!(
            image_category(&ImageError::Value {
                parameter_ref: "P".into(),
                cause: "\"1.0E+1\" is not an unsigned number".into()
            }),
            UnsupportedCategory::ParameterValue
        );
        assert_eq!(
            image_category(&ImageError::ConflictingValues {
                parameter: "P".into()
            }),
            UnsupportedCategory::ParameterValue
        );
    }

    #[test]
    fn malformed_duplicate_and_future_evidence_is_refused() {
        assert!(matches!(
            parse_evidence("{"),
            Err(EvidenceError::Malformed(_))
        ));
        assert_eq!(
            parse_evidence(r#"{"format":2,"verified":[]}"#),
            Err(EvidenceError::Format(2))
        );
        let one =
            r#"{"program_id":"P","scopes":["complete"],"device":"d","date":"x","reference":"r"}"#;
        assert_eq!(
            parse_evidence(&format!(r#"{{"format":1,"verified":[{one},{one}]}}"#)),
            Err(EvidenceError::Duplicate("P".into()))
        );
        assert!(matches!(
            parse_evidence(r#"{"format":1,"verified":[{"program_id":"P"}]}"#),
            Err(EvidenceError::Malformed(_))
        ));
    }

    #[test]
    fn a_download_is_verified_only_for_the_scope_that_ran() {
        let plan = plan();
        let evidence = BTreeMap::from([(
            "P".to_owned(),
            VerifiedEvidence {
                scopes: vec!["complete".into(), "partial-parameters".into()],
                ..evidence("P")
            },
        )]);
        let parts = |parameters, group_addresses| {
            Some(PartialDownloadParts {
                parameters,
                group_addresses,
            })
        };
        assert_eq!(
            download_level("P", None, &plan, &[], &evidence).code(),
            "verified"
        );
        assert_eq!(
            download_level("P", parts(true, false), &plan, &[], &evidence).code(),
            "verified"
        );
        for untested in [parts(false, true), parts(true, true)] {
            assert_eq!(
                download_level("P", untested, &plan, &[], &evidence).code(),
                "untested",
                "{untested:?}"
            );
        }
        assert_eq!(
            download_level("Q", None, &plan, &[], &evidence).code(),
            "untested"
        );
        assert_eq!(
            untested_acknowledgement("1.1.67".parse().unwrap()),
            "I accept an untested download to 1.1.67"
        );
    }

    #[test]
    fn an_unknown_program_is_unsupported_as_product_data() {
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let level = support_level(&conn, "M-0000_A-0000-00-0000", &BTreeMap::new());
        assert!(
            matches!(
                level,
                SupportLevel::Unsupported {
                    category: UnsupportedCategory::ProductData,
                    ..
                }
            ),
            "{level:?}"
        );
    }

    #[test]
    fn a_summary_counts_levels_categories_and_masks() {
        let row = |id: &str, mask: &str, level| ProgramSupport {
            program_id: id.into(),
            manufacturer_id: "M-0083".into(),
            name: None,
            mask_version: Some(mask.into()),
            level,
        };
        let rows = vec![
            row(
                "A",
                "MV-0701",
                SupportLevel::Verified {
                    evidence: evidence("A"),
                    steps: 25,
                    octets: 1416,
                },
            ),
            row(
                "B",
                "MV-0705",
                SupportLevel::Untested {
                    steps: 9,
                    octets: 10,
                    inferences: vec![],
                },
            ),
            row(
                "C",
                "MV-07B0",
                SupportLevel::Unsupported {
                    category: UnsupportedCategory::ProcedureStyle,
                    detail: String::new(),
                },
            ),
            row(
                "D",
                "MV-0705",
                SupportLevel::Unsupported {
                    category: UnsupportedCategory::ProcedureStyle,
                    detail: String::new(),
                },
            ),
        ];
        let summary = CoverageSummary::of(&rows);
        assert_eq!(summary.programs, 4);
        assert_eq!((summary.verified, summary.untested), (1, 1));
        assert_eq!(
            summary
                .unsupported
                .get(&UnsupportedCategory::ProcedureStyle),
            Some(&2)
        );
        assert_eq!(summary.by_mask.get("MV-0705"), Some(&(2, 1)));
        assert_eq!(summary.by_mask.get("MV-0701"), Some(&(1, 1)));
    }
}
