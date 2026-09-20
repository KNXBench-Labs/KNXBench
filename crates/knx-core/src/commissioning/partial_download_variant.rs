//! CP §3.5.3's five partial-download variants, transcribed whole as pure step-list data.
//!
//! `commissioning::procedure::partial_download()` models exactly one of the five — the
//! "application program 2" variant, CP §3.5.3, pp. 44-47 — as a citable [`Procedure`] value
//! for a report to show whole. CP §3.5.3 itself gives five, one per part that can be the
//! target of a partial download, and each numbers its own steps from 01 rather than sharing
//! the first one's numbering. This module transcribes all five, per design spec §11.2's "a procedure
//! model, not a script," so that a report can be read against whichever of the five clauses
//! actually applies. `[C12]` `knx-net`'s `Downloader::partial_download` is the sequencer that
//! consults this module: it looks up the target part's variant here and takes every outer
//! step number, every escalation target and the jump target from it rather than carrying a
//! second, hand-maintained copy of the same numbering.
//!
//! Every variant's Nr. 05 unloads the part being replaced, and its Nr. 06 loads it again via
//! the inner loop of CP §3.5.2 §7.2 with CP §3.5.3's own CRC comparison added. Four of the
//! five variants add a Nr. 07 escalation for when that allocation fails: unload every
//! following segment (in the download order `PartKind` fixes, `[C8]`, `knx-net`) and reload
//! them in that order, which is CP §3.5.3's own recovery and turns a partial download into
//! something close to a full one mid-flight. The fifth — Association Table, the last segment
//! in that order — has nothing following it to escalate to, so it has **no** Nr. 07
//! escalation at all: CP §3.5.3, p. 56 numbers its Nr. 07 "Modifying access keys" and its
//! Nr. 08 "Disconnect", the same two steps every other variant carries at the tail.
//!
//! | Variant                | Steps | Nr. 07 escalates to                              | Jump on success |
//! |-------------------------|-------|--------------------------------------------------|------------------|
//! | Application Program 2  | 01-14 | AP1, Group Object Table, Group Address Table, Association Table | ⇒ 13 |
//! | Application Program 1  | 01-13 | Group Object Table, Group Address Table, Association Table      | ⇒ 12 |
//! | Group Object Table     | 01-12 | Group Address Table, Association Table                          | ⇒ 11 |
//! | Group Address Table    | 01-11 | Association Table                                               | ⇒ 10 |
//! | Association Table      | 01-08 | *(none — Nr. 07 is "Modifying access keys")*                    | none |
//!
//! Transcribed from `03_05_03 Configuration Procedures v02.01.01 AS.pdf`, pp. 44-56, printed
//! page numbers (no offset). Every step's title and detail is this project's paraphrase, not
//! the clause's verbatim wording; the escalation lists, the jump targets and the step counts
//! are checked against the clause exactly, in this module's own tests.

use super::procedure::{Procedure, ProcedureKind, ProcedureStep, StepEffect};

/// Which of CP §3.5.3's five partial-download variants a step list models.
///
/// Declaration order is the download order `knx-net`'s `PartKind` enforces (`[C8]`):
/// Application Program 2, Application Program 1, Group Object Table, Group Address Table,
/// Association Table. CP §3.5.2 Nr. 06-10 and CP §3.5.3's own variant order (pp. 44-56) both
/// list the five in this order too, so this is the Standard's order, not one invented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartialDownloadVariant {
    /// CP §3.5.3, pp. 44-47.
    ApplicationProgram2,
    /// CP §3.5.3, pp. 47-50.
    ApplicationProgram1,
    /// CP §3.5.3, pp. 50-53.
    GroupObjectTable,
    /// CP §3.5.3, pp. 53-55.
    GroupAddressTable,
    /// CP §3.5.3, pp. 55-56. The one variant with no Nr. 07 escalation.
    AssociationTable,
}

impl PartialDownloadVariant {
    /// All five, in the download order.
    pub const ALL: [PartialDownloadVariant; 5] = [
        PartialDownloadVariant::ApplicationProgram2,
        PartialDownloadVariant::ApplicationProgram1,
        PartialDownloadVariant::GroupObjectTable,
        PartialDownloadVariant::GroupAddressTable,
        PartialDownloadVariant::AssociationTable,
    ];

    /// The clause and the page range this variant's steps were transcribed from.
    pub const fn source(self) -> &'static str {
        match self {
            PartialDownloadVariant::ApplicationProgram2 => {
                "CP §3.5.3 'application program 2', pp. 44-47"
            }
            PartialDownloadVariant::ApplicationProgram1 => {
                "CP §3.5.3 'application program 1', pp. 47-50"
            }
            PartialDownloadVariant::GroupObjectTable => "CP §3.5.3 'Group Object Table', pp. 50-53",
            PartialDownloadVariant::GroupAddressTable => {
                "CP §3.5.3 'Group Address Table', pp. 53-55"
            }
            PartialDownloadVariant::AssociationTable => "CP §3.5.3 'Association Table', pp. 55-56",
        }
    }

    /// The step list, in the clause's own numbering, as a full [`Procedure`] so it renders
    /// and reports exactly like the other cited procedures.
    ///
    /// `kind` is [`ProcedureKind::PartialDownload`] for all five: they share one authorisation
    /// scope and one Standard clause, and `source` is what distinguishes which of the five a
    /// given [`Procedure`] is.
    pub fn procedure(self) -> Procedure {
        Procedure {
            kind: ProcedureKind::PartialDownload,
            source: self.source(),
            steps: self.steps(),
        }
    }

    /// The segments Nr. 07 unloads and reloads, in the download order, when Nr. 06's
    /// allocation fails. Empty only for [`PartialDownloadVariant::AssociationTable`], which is
    /// the last segment in the download order and therefore has nothing left to escalate to
    /// (CP §3.5.3, p. 56 — its Nr. 07 is "Modifying access keys", not an escalation).
    pub const fn escalation_targets(self) -> &'static [&'static str] {
        match self {
            PartialDownloadVariant::ApplicationProgram2 => &[
                "Application Program 1",
                "Group Object Table",
                "Group Address Table",
                "Association Table",
            ],
            PartialDownloadVariant::ApplicationProgram1 => &[
                "Group Object Table",
                "Group Address Table",
                "Association Table",
            ],
            PartialDownloadVariant::GroupObjectTable => {
                &["Group Address Table", "Association Table"]
            }
            PartialDownloadVariant::GroupAddressTable => &["Association Table"],
            PartialDownloadVariant::AssociationTable => &[],
        }
    }

    /// The step number a successful Nr. 06 jumps straight to, skipping the escalation branch
    /// and the reload steps it would otherwise fall through to. `None` for
    /// [`PartialDownloadVariant::AssociationTable`], whose Nr. 06 carries no such jump at all
    /// (CP §3.5.3, p. 56): there is no escalation branch after it to skip.
    pub const fn jump_target(self) -> Option<u8> {
        match self {
            PartialDownloadVariant::ApplicationProgram2 => Some(13),
            PartialDownloadVariant::ApplicationProgram1 => Some(12),
            PartialDownloadVariant::GroupObjectTable => Some(11),
            PartialDownloadVariant::GroupAddressTable => Some(10),
            PartialDownloadVariant::AssociationTable => None,
        }
    }

    fn steps(self) -> Vec<ProcedureStep> {
        match self {
            PartialDownloadVariant::ApplicationProgram2 => application_program_2(),
            PartialDownloadVariant::ApplicationProgram1 => application_program_1(),
            PartialDownloadVariant::GroupObjectTable => group_object_table(),
            PartialDownloadVariant::GroupAddressTable => group_address_table(),
            PartialDownloadVariant::AssociationTable => association_table(),
        }
    }
}

fn step(
    number: u8,
    title: &'static str,
    detail: &'static str,
    effect: StepEffect,
) -> ProcedureStep {
    ProcedureStep {
        number,
        title,
        detail,
        effect,
    }
}

/// Steps 01-04, identical boilerplate across all five variants (CP §3.5.3, pp. 44, 47, 50,
/// 53, 55): connect, verify the device version, get access rights, check the manufacturer ID.
fn opening_steps() -> Vec<ProcedureStep> {
    vec![
        step(
            1,
            "connect",
            "connection-oriented connect via the bus",
            StepEffect::Connection,
        ),
        step(
            2,
            "verify the device version",
            "read Device Descriptor Type 0",
            StepEffect::Read,
        ),
        step(
            3,
            "get access rights",
            "authorise per MP §3.5.1",
            StepEffect::Read,
        ),
        step(
            4,
            "check the manufacturer ID",
            "compare against DeviceObject.PID_MANUFACTURER_ID",
            StepEffect::Guard,
        ),
    ]
}

/// CP §3.5.3, "Partial Download of the 'application program 2'", pp. 44-47.
fn application_program_2() -> Vec<ProcedureStep> {
    let mut steps = opening_steps();
    steps.extend([
        step(
            5,
            "unload Application Program 2",
            "LoadControl = Unload on Application Program 2 alone, not on the other four parts",
            StepEffect::Write,
        ),
        step(
            6,
            "allocate, load and compare the CRC",
            "the inner loop of §7.2 with CP §3.5.3's CRC comparison added; a zero base \
             address means allocation failed, which continues at Nr. 07 (CP §3.5.3, p. 45), \
             and a successful load continues at Nr. 13 (CP §3.5.3, p. 45), skipping 07-12",
            StepEffect::Write,
        ),
        step(
            7,
            "on failed allocation, escalate",
            "unload Application Program 1, the Group Object Table, the Group Address Table \
             and the Association Table (CP §3.5.3, p. 46), then reload them in that order",
            StepEffect::Write,
        ),
        step(
            8,
            "load Application Program 2 again",
            "the inner loop of §7.2, run after the escalation's unload; the happy path \
             already did this at Nr. 06 and jumped past here",
            StepEffect::Write,
        ),
        step(
            9,
            "load Application Program 1",
            "the first of the four segments the escalation reloads (CP §3.5.3, p. 47)",
            StepEffect::Write,
        ),
        step(
            10,
            "load the Group Object Table",
            "the second segment the escalation reloads",
            StepEffect::Write,
        ),
        step(
            11,
            "load the Group Address Table",
            "the third segment the escalation reloads; its PL110-only group responser table \
             write is out of scope for phase 2 (CP §3.5.3 footnote 8, p. 47)",
            StepEffect::Write,
        ),
        step(
            12,
            "load the Association Table",
            "the last segment the escalation reloads",
            StepEffect::Write,
        ),
        step(
            13,
            "modify access keys",
            "set access keys as required; A_Key_Write is out of scope for phase 2 (design spec §10.7)",
            StepEffect::Guard,
        ),
        step(
            14,
            "disconnect",
            "disconnect via the bus",
            StepEffect::Connection,
        ),
    ]);
    steps
}

/// CP §3.5.3, "Partial Download of the 'application program 1'", pp. 47-50.
fn application_program_1() -> Vec<ProcedureStep> {
    let mut steps = opening_steps();
    steps.extend([
        step(
            5,
            "unload Application Program 1",
            "LoadControl = Unload on Application Program 1 alone",
            StepEffect::Write,
        ),
        step(
            6,
            "allocate, load and compare the CRC",
            "the inner loop of §7.2 with the CRC comparison added; a zero base address means \
             allocation failed, which continues at Nr. 07 (CP §3.5.3, p. 48), and a \
             successful load continues at Nr. 12 (CP §3.5.3, p. 48), skipping 07-11",
            StepEffect::Write,
        ),
        step(
            7,
            "on failed allocation, escalate",
            "unload the Group Object Table, the Group Address Table and the Association \
             Table (CP §3.5.3, p. 49), then reload them in that order",
            StepEffect::Write,
        ),
        step(
            8,
            "load Application Program 1 again",
            "the inner loop of §7.2, run after the escalation's unload",
            StepEffect::Write,
        ),
        step(
            9,
            "load the Group Object Table",
            "the first segment the escalation reloads (CP §3.5.3, p. 50)",
            StepEffect::Write,
        ),
        step(
            10,
            "load the Group Address Table",
            "the second segment the escalation reloads; its PL110-only group responser table \
             write is out of scope for phase 2 (CP §3.5.3 footnote 9, p. 50)",
            StepEffect::Write,
        ),
        step(
            11,
            "load the Association Table",
            "the last segment the escalation reloads",
            StepEffect::Write,
        ),
        step(
            12,
            "modify access keys",
            "set access keys as required; A_Key_Write is out of scope for phase 2 (design spec §10.7)",
            StepEffect::Guard,
        ),
        step(
            13,
            "disconnect",
            "disconnect via the bus",
            StepEffect::Connection,
        ),
    ]);
    steps
}

/// CP §3.5.3, "Partial Download of the 'Group Object Table'", pp. 50-53.
fn group_object_table() -> Vec<ProcedureStep> {
    let mut steps = opening_steps();
    steps.extend([
        step(
            5,
            "unload the Group Object Table",
            "LoadControl = Unload on the Group Object Table alone",
            StepEffect::Write,
        ),
        step(
            6,
            "allocate, load and compare the CRC",
            "the inner loop of §7.2 with the CRC comparison added; a zero base address means \
             allocation failed, which continues at Nr. 07 (CP §3.5.3, p. 51), and a \
             successful load continues at Nr. 11 (CP §3.5.3, p. 51), skipping 07-10",
            StepEffect::Write,
        ),
        step(
            7,
            "on failed allocation, escalate",
            "unload the Group Address Table and the Association Table (CP §3.5.3, p. 51), \
             then reload them in that order",
            StepEffect::Write,
        ),
        step(
            8,
            "load the Group Object Table again",
            "the inner loop of §7.2, run after the escalation's unload",
            StepEffect::Write,
        ),
        step(
            9,
            "load the Group Address Table",
            "the first segment the escalation reloads (CP §3.5.3, p. 52); its PL110-only \
             group responser table write is out of scope for phase 2 (CP §3.5.3 \
             footnote 10, p. 52)",
            StepEffect::Write,
        ),
        step(
            10,
            "load the Association Table",
            "the last segment the escalation reloads",
            StepEffect::Write,
        ),
        step(
            11,
            "modify access keys",
            "set access keys as required; A_Key_Write is out of scope for phase 2 (design spec §10.7)",
            StepEffect::Guard,
        ),
        step(
            12,
            "disconnect",
            "disconnect via the bus",
            StepEffect::Connection,
        ),
    ]);
    steps
}

/// CP §3.5.3, "Partial Download of the 'Group Address Table'", pp. 53-55.
fn group_address_table() -> Vec<ProcedureStep> {
    let mut steps = opening_steps();
    steps.extend([
        step(
            5,
            "unload the Group Address Table",
            "LoadControl = Unload on the Group Address Table alone",
            StepEffect::Write,
        ),
        step(
            6,
            "allocate, load and compare the CRC",
            "the inner loop of §7.2 with the CRC comparison added; a zero base address means \
             allocation failed, which continues at Nr. 07 (CP §3.5.3, p. 54), and a \
             successful load continues at Nr. 10 (CP §3.5.3, p. 54), skipping 07-09",
            StepEffect::Write,
        ),
        step(
            7,
            "on failed allocation, escalate",
            "unload the Association Table (CP §3.5.3, p. 54), then reload it",
            StepEffect::Write,
        ),
        step(
            8,
            "load the Group Address Table again",
            "the inner loop of §7.2, run after the escalation's unload",
            StepEffect::Write,
        ),
        step(
            9,
            "load the Association Table",
            "the one segment the escalation reloads (CP §3.5.3, p. 55)",
            StepEffect::Write,
        ),
        step(
            10,
            "modify access keys",
            "set access keys as required; A_Key_Write is out of scope for phase 2 (design spec §10.7)",
            StepEffect::Guard,
        ),
        step(
            11,
            "disconnect",
            "disconnect via the bus",
            StepEffect::Connection,
        ),
    ]);
    steps
}

/// CP §3.5.3, "Partial Download of the 'Association Table'", pp. 55-56.
///
/// The Association Table is the last segment in the download order, so its Nr. 06 has
/// nothing following it to escalate to: CP §3.5.3, p. 56 gives it no "⇒ Continue" jump and no
/// escalation branch at all. Its Nr. 07 is "Modifying access keys" and its Nr. 08 is
/// "Disconnect" — the same tail every other variant carries, just two steps earlier because
/// there is no escalation block in between.
fn association_table() -> Vec<ProcedureStep> {
    let mut steps = opening_steps();
    steps.extend([
        step(
            5,
            "unload the Association Table",
            "LoadControl = Unload on the Association Table alone",
            StepEffect::Write,
        ),
        step(
            6,
            "load the Association Table",
            "the inner loop of §7.2 with the CRC comparison added; a zero base address means \
             allocation failed, which is reported to the operator directly (CP §3.5.3, \
             p. 56) — there is no following segment to escalate to",
            StepEffect::Write,
        ),
        step(
            7,
            "modify access keys",
            "set access keys as required; A_Key_Write is out of scope for phase 2 (design spec §10.7)",
            StepEffect::Guard,
        ),
        step(
            8,
            "disconnect",
            "disconnect via the bus",
            StepEffect::Connection,
        ),
    ]);
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant numbers its steps from one without gaps, same as every other cited
    /// procedure in this crate (design spec §11.2).
    #[test]
    fn every_variant_numbers_its_steps_from_one_without_gaps() {
        for variant in PartialDownloadVariant::ALL {
            let procedure = variant.procedure();
            for (index, step) in procedure.steps.iter().enumerate() {
                assert_eq!(
                    step.number as usize,
                    index + 1,
                    "{:?} step {} is out of order",
                    variant,
                    step.number
                );
            }
        }
    }

    /// The acceptance table: step counts, escalation targets and jump targets, all five,
    /// checked against the brief's table and against the source pages transcribed above.
    #[test]
    fn the_five_variants_match_cp_3_5_3s_table() {
        let cases: [(PartialDownloadVariant, usize, &[&str], Option<u8>); 5] = [
            (
                PartialDownloadVariant::ApplicationProgram2,
                14,
                &[
                    "Application Program 1",
                    "Group Object Table",
                    "Group Address Table",
                    "Association Table",
                ],
                Some(13),
            ),
            (
                PartialDownloadVariant::ApplicationProgram1,
                13,
                &[
                    "Group Object Table",
                    "Group Address Table",
                    "Association Table",
                ],
                Some(12),
            ),
            (
                PartialDownloadVariant::GroupObjectTable,
                12,
                &["Group Address Table", "Association Table"],
                Some(11),
            ),
            (
                PartialDownloadVariant::GroupAddressTable,
                11,
                &["Association Table"],
                Some(10),
            ),
            (PartialDownloadVariant::AssociationTable, 8, &[], None),
        ];

        for (variant, step_count, escalation_targets, jump_target) in cases {
            let procedure = variant.procedure();
            assert_eq!(
                procedure.steps.len(),
                step_count,
                "{variant:?} should have {step_count} steps, source {}",
                variant.source()
            );
            assert_eq!(
                variant.escalation_targets(),
                escalation_targets,
                "{variant:?} Nr. 07 escalation targets"
            );
            assert_eq!(
                variant.jump_target(),
                jump_target,
                "{variant:?} Nr. 06 jump target"
            );
            assert!(
                procedure.source.contains("CP §3.5.3"),
                "{variant:?} source must name the clause: {}",
                procedure.source
            );
        }
    }

    /// The defect this task exists to remove: giving the Association Table variant an
    /// escalation step it does not have. Its Nr. 07 is "Modifying access keys" and its
    /// Nr. 08 is "Disconnect" (CP §3.5.3, p. 56) — neither one an escalation, and there is no
    /// ninth step for a reload to land on.
    #[test]
    fn the_association_table_variant_has_no_escalation_at_nr_07() {
        let procedure = PartialDownloadVariant::AssociationTable.procedure();
        assert!(
            PartialDownloadVariant::AssociationTable
                .escalation_targets()
                .is_empty(),
            "the Association Table variant has nothing left to escalate to"
        );
        assert_eq!(
            PartialDownloadVariant::AssociationTable.jump_target(),
            None,
            "the Association Table variant's Nr. 06 carries no jump"
        );
        assert_eq!(procedure.steps.len(), 8);
        let nr_07 = &procedure.steps[6];
        assert_eq!(nr_07.number, 7);
        assert_eq!(nr_07.title, "modify access keys");
        assert!(
            !nr_07.title.to_lowercase().contains("escalat"),
            "Nr. 07 must not be an escalation step: {}",
            nr_07.title
        );
        let nr_08 = &procedure.steps[7];
        assert_eq!(nr_08.number, 8);
        assert_eq!(nr_08.title, "disconnect");
    }

    /// The other four variants all do escalate at Nr. 07, which is the contrast the previous
    /// test needs: a suite that only ever checks "no escalation" everywhere could not
    /// distinguish the real defect from a model that never escalates at all.
    #[test]
    fn the_other_four_variants_escalate_at_nr_07() {
        for variant in [
            PartialDownloadVariant::ApplicationProgram2,
            PartialDownloadVariant::ApplicationProgram1,
            PartialDownloadVariant::GroupObjectTable,
            PartialDownloadVariant::GroupAddressTable,
        ] {
            let procedure = variant.procedure();
            let nr_07 = &procedure.steps[6];
            assert_eq!(nr_07.number, 7);
            assert!(
                nr_07.title.to_lowercase().contains("escalate"),
                "{variant:?} Nr. 07 must escalate: {}",
                nr_07.title
            );
            assert!(!variant.escalation_targets().is_empty());
            assert!(variant.jump_target().is_some());
        }
    }

    /// Every variant's last step is the disconnect, and its number equals the step count —
    /// the fifth variant ends at 08, not 14, and a plan that assumed a fixed tail would miss
    /// that.
    #[test]
    fn every_variant_ends_with_disconnect_as_its_last_step() {
        for variant in PartialDownloadVariant::ALL {
            let procedure = variant.procedure();
            let last = procedure.steps.last().unwrap();
            assert_eq!(last.title, "disconnect");
            assert_eq!(last.number as usize, procedure.steps.len());
        }
    }

    /// Every variant's escalation target list, where one exists, is in the same download
    /// order `PartKind` fixes (`[C8]`) — reusing that order rather than restating an
    /// unordered set that a table-driven test could satisfy by accident.
    #[test]
    fn escalation_targets_are_in_the_download_order() {
        const DOWNLOAD_ORDER: [&str; 5] = [
            "Application Program 2",
            "Application Program 1",
            "Group Object Table",
            "Group Address Table",
            "Association Table",
        ];
        for variant in PartialDownloadVariant::ALL {
            let targets = variant.escalation_targets();
            let positions: Vec<usize> = targets
                .iter()
                .map(|target| {
                    DOWNLOAD_ORDER
                        .iter()
                        .position(|kind| kind == target)
                        .unwrap_or_else(|| panic!("{target} is not a known part kind"))
                })
                .collect();
            let mut sorted = positions.clone();
            sorted.sort_unstable();
            assert_eq!(
                positions, sorted,
                "{variant:?} escalation targets are out of the download order: {targets:?}"
            );
        }
    }

    /// `escalation_targets()` and the reload steps' own titles are two independently
    /// hand-maintained copies of the same fact (the order Nr. 07's escalation reloads
    /// segments in), and only `escalation_targets_are_in_the_download_order` checked the
    /// first copy. This walks the second: for every variant with an escalation, the reload
    /// step immediately after "load `<part>` again" (step 8, 0-indexed 7) must name
    /// `escalation_targets()[0]` in its title, the next one `escalation_targets()[1]`, and so
    /// on — so a title swap between two reload steps (moving the right words to the wrong
    /// step number) fails here even though the target list itself stays untouched and sorted.
    #[test]
    fn reload_step_titles_name_their_escalation_target_in_order() {
        for variant in PartialDownloadVariant::ALL {
            let targets = variant.escalation_targets();
            if targets.is_empty() {
                continue;
            }
            let steps = variant.procedure().steps;
            // Step 8 (index 7) is "load `<part>` again," not a target; the reload steps for
            // the escalation targets start right after it, one step number per target.
            let reload_titles = &steps[8..8 + targets.len()];
            for (target, reload_step) in targets.iter().zip(reload_titles) {
                assert!(
                    reload_step.title.contains(target),
                    "{variant:?} step {} (\"{}\") should name escalation target \"{target}\"",
                    reload_step.number,
                    reload_step.title
                );
            }
        }
    }

    /// Nothing here claims the unspecified differential-download algorithm CP §3.5.3's own
    /// text names (design spec §7.4, §12) — the same guard `procedure.rs` keeps for the one variant
    /// it already models.
    #[test]
    fn nothing_claims_the_unspecified_differential_download_algorithm() {
        for variant in PartialDownloadVariant::ALL {
            for step in variant.procedure().steps {
                let text = step.detail.to_lowercase();
                assert!(
                    !text.contains("differential download algorithm is"),
                    "{variant:?} step {} claims an unspecified algorithm",
                    step.number
                );
            }
        }
    }
}
