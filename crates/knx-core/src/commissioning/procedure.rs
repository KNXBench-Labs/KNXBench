//! The cited procedures as declarative step lists, renderable and dry-runnable without a bus.
//!
//! Spec §11.2: *"A procedure model, not a script."* Each list is the
//! Standard's own numbering for one procedure — §4.2 (MP §2.3), §7.1/§7.2
//! (CP §3.5.2), §7.4 (CP §3.5.3), §7.5 (CP §3.5.4) and §9.1 — with the
//! order, the guards and the destructive steps marked. Nothing here sends
//! anything.

use std::fmt;

use super::mutation::WriteScope;

/// What a step does to the device, which decides whether it needs the
/// mutation API and whether the procedure can still be abandoned safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StepEffect {
    /// Opens or closes the connection-oriented Transport Layer link.
    /// `[D]` CP §3.5.4: *"The load procedure shall be connection
    /// oriented."*
    Connection,
    /// Reads something. Leaves the device exactly as it was.
    Read,
    /// Compares something already read and may stop the procedure. Writes
    /// nothing.
    Guard,
    /// Waits for the device to reach a state (§5.5).
    Wait,
    /// Changes the device. Everything from here on is not undoable: §9.3
    /// — there is no rollback in any procedure this document cites.
    Write,
}

impl StepEffect {
    /// Whether the step changes the device.
    pub fn is_destructive(self) -> bool {
        self == StepEffect::Write
    }
}

impl fmt::Display for StepEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            StepEffect::Connection => "connection",
            StepEffect::Read => "read",
            StepEffect::Guard => "guard",
            StepEffect::Wait => "wait",
            StepEffect::Write => "write",
        })
    }
}

/// One step of a cited procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureStep {
    /// The Standard's own step number within its procedure, so that a
    /// report and the clause can be read side by side.
    pub number: u8,
    /// What the step is called.
    pub title: &'static str,
    /// What it does, in enough detail to be checked against the clause.
    pub detail: &'static str,
    /// What it does to the device.
    pub effect: StepEffect,
}

impl fmt::Display for ProcedureStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02} {} [{}] — {}",
            self.number, self.title, self.effect, self.detail
        )
    }
}

/// A cited procedure, with its source clause and its steps in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Procedure {
    /// Which procedure this is.
    pub kind: ProcedureKind,
    /// The clause the steps were transcribed from.
    pub source: &'static str,
    /// The steps, in the Standard's order.
    pub steps: Vec<ProcedureStep>,
}

impl Procedure {
    /// The scope of authorisation a caller needs before running it, if it
    /// writes at all.
    pub fn required_scope(&self) -> Option<WriteScope> {
        self.steps
            .iter()
            .any(|step| step.effect.is_destructive())
            .then_some(self.kind.scope())
    }

    /// The first step that changes the device. Everything before it can be
    /// abandoned with the device untouched, which is what makes §7.1 step
    /// 03's authorisation failure safe.
    pub fn first_destructive_step(&self) -> Option<&ProcedureStep> {
        self.steps.iter().find(|step| step.effect.is_destructive())
    }

    /// Renders the procedure as a numbered list, for a dry run.
    pub fn render(&self) -> String {
        let mut out = format!("{} ({})", self.kind, self.source);
        for step in &self.steps {
            out.push_str(&format!("\n  {step}"));
        }
        out
    }
}

/// Which cited procedure a step list belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcedureKind {
    /// §4.2, MP §2.3 `NM_IndividualAddress_Write`.
    IndividualAddressWrite,
    /// §7.1, CP §3.5.2, complete download.
    CompleteDownload,
    /// §7.2, CP §3.5.2 step 06, the inner loop for one loadable part.
    LoadOnePart,
    /// §7.4, CP §3.5.3, partial download.
    PartialDownload,
    /// §7.5, CP §3.5.4, unload — steps 01–06 only.
    Unload,
    /// §9.1, recovery after an interrupted download.
    Recovery,
}

impl ProcedureKind {
    /// The authorisation scope this procedure's writes fall under.
    pub fn scope(self) -> WriteScope {
        match self {
            ProcedureKind::IndividualAddressWrite => WriteScope::IndividualAddressProgramming,
            ProcedureKind::CompleteDownload
            | ProcedureKind::LoadOnePart
            | ProcedureKind::PartialDownload
            | ProcedureKind::Recovery => WriteScope::Download,
            ProcedureKind::Unload => WriteScope::Unload,
        }
    }

    /// The step list.
    pub fn procedure(self) -> Procedure {
        match self {
            ProcedureKind::IndividualAddressWrite => individual_address_write(),
            ProcedureKind::CompleteDownload => complete_download(),
            ProcedureKind::LoadOnePart => load_one_part(),
            ProcedureKind::PartialDownload => partial_download(),
            ProcedureKind::Unload => unload(),
            ProcedureKind::Recovery => recovery(),
        }
    }

    /// Every procedure phase 2 models.
    pub const ALL: [ProcedureKind; 6] = [
        ProcedureKind::IndividualAddressWrite,
        ProcedureKind::CompleteDownload,
        ProcedureKind::LoadOnePart,
        ProcedureKind::PartialDownload,
        ProcedureKind::Unload,
        ProcedureKind::Recovery,
    ];
}

impl fmt::Display for ProcedureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProcedureKind::IndividualAddressWrite => "programme the individual address",
            ProcedureKind::CompleteDownload => "complete download",
            ProcedureKind::LoadOnePart => "load one loadable part",
            ProcedureKind::PartialDownload => "partial download",
            ProcedureKind::Unload => "unload",
            ProcedureKind::Recovery => "recovery after an interrupted download",
        })
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

/// §4.2, MP §2.3. The write is step 3 of 4, not step 1 of 1.
pub fn individual_address_write() -> Procedure {
    Procedure {
        kind: ProcedureKind::IndividualAddressWrite,
        source: "MP §2.3 NM_IndividualAddress_Write",
        steps: vec![
            step(
                1,
                "check the new address is free",
                "connect to IA_new and read Device Descriptor Type 0; any answer means \
                 the address is occupied and the procedure stops",
                StepEffect::Read,
            ),
            step(
                2,
                "count devices in programming mode",
                "broadcast A_IndividualAddress_Read, wait out the full 3 s time-out, \
                 count distinct source addresses and not frames; continue only at \
                 exactly one",
                StepEffect::Read,
            ),
            step(
                3,
                "write the new address",
                "broadcast A_IndividualAddress_Write, and only if IA_new differs from \
                 the responder's current address; re-verify step 2 immediately before, \
                 because programming mode may have switched itself off",
                StepEffect::Write,
            ),
            step(
                4,
                "verify and restart",
                "connect to the new address, read Device Descriptor Type 0, A_Restart, \
                 then abort the client-side connection; a failure here means the write \
                 failed or the router is misconfigured, and both are reported",
                StepEffect::Write,
            ),
        ],
    }
}

/// §7.1, CP §3.5.2. Step 05 unloads everything before anything is loaded,
/// so a complete download passes through a state with no valid
/// configuration at all.
pub fn complete_download() -> Procedure {
    Procedure {
        kind: ProcedureKind::CompleteDownload,
        source: "CP §3.5.2",
        steps: vec![
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
                "authorise per MP §3.5.1; failure ends the procedure here, before \
                 anything has been written",
                StepEffect::Read,
            ),
            step(
                4,
                "check the manufacturer ID",
                "compare the expected ID against DeviceObject.PID_MANUFACTURER_ID; \
                 this is the Standard's own guard against loading one manufacturer's \
                 application into another's device",
                StepEffect::Guard,
            ),
            step(
                5,
                "unload the device",
                "LoadControl = Unload on Application Program 2 and 1, Group Object \
                 Table, Association Table and Address Table, then wait until each \
                 reports Unloaded",
                StepEffect::Write,
            ),
            step(
                6,
                "load Application Program 2",
                "the inner loop of §7.2",
                StepEffect::Write,
            ),
            step(
                7,
                "load Application Program 1",
                "the inner loop of §7.2",
                StepEffect::Write,
            ),
            step(
                8,
                "load the Group Object Table",
                "the inner loop of §7.2",
                StepEffect::Write,
            ),
            step(
                9,
                "load the Address Table",
                "the inner loop of §7.2; the group responser table write applies to \
                 PL110 devices only and therefore not on TP1",
                StepEffect::Write,
            ),
            step(
                10,
                "load the Association Table",
                "the inner loop of §7.2",
                StepEffect::Write,
            ),
            step(
                11,
                "modify access keys",
                "not implemented in phase 2: A_Key_Write is out of scope (§10.7)",
                StepEffect::Guard,
            ),
            step(
                12,
                "disconnect",
                "disconnect via the bus",
                StepEffect::Connection,
            ),
        ],
    }
}

/// §7.2, CP §3.5.2 step 06, for one loadable part.
pub fn load_one_part() -> Procedure {
    Procedure {
        kind: ProcedureKind::LoadOnePart,
        source: "CP §3.5.2 step 06",
        steps: vec![
            step(
                1,
                "start",
                "LoadControl = Start Loading, and wait for LoadState = Loading, \
                 because allocation outside Loading is silently ignored (§7.6)",
                StepEffect::Write,
            ),
            step(
                2,
                "allocate",
                "LoadControl = Additional Load Control with the subtype the device's \
                 mask profiles (§7.3); there is no fallback between allocation styles",
                StepEffect::Write,
            ),
            step(
                3,
                "read back the base address",
                "PropertyRead PID_REFERENCE; zero means allocation failed and is never \
                 used as an address",
                StepEffect::Read,
            ),
            step(
                4,
                "write the data",
                "direct memory access in chunks per §6.4, with the service chosen from \
                 base + length per §6.5, and a client-side read-back of every write",
                StepEffect::Write,
            ),
            step(
                5,
                "set the version",
                "PropertyWrite PID_PROGRAM_VERSION",
                StepEffect::Write,
            ),
            step(
                6,
                "complete",
                "LoadControl = Load Completed, then wait per §5.5, accepting \
                 LoadCompleting as still working",
                StepEffect::Write,
            ),
            step(
                7,
                "store the checksum",
                "read PID_MCB and keep the CRC; skipping this makes every later \
                 partial download impossible",
                StepEffect::Read,
            ),
        ],
    }
}

/// §7.4, CP §3.5.3. Same shape as §7.1 with one insertion and one branch,
/// and the branch escalates to a larger download rather than aborting.
pub fn partial_download() -> Procedure {
    Procedure {
        kind: ProcedureKind::PartialDownload,
        source: "CP §3.5.3",
        steps: vec![
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
            step(
                5,
                "unload only the part being replaced",
                "LoadControl = Unload on that part alone, not on everything",
                StepEffect::Write,
            ),
            step(
                6,
                "allocate and compare the CRC",
                "after the PID_REFERENCE read-back, read PID_MCB and compare against \
                 the stored CRC; a match means the part is unchanged and may be \
                 skipped — it does not mean a differential download is performed, \
                 because that algorithm is not specified anywhere (GAP-T30-04)",
                StepEffect::Write,
            ),
            step(
                7,
                "on failed allocation, escalate",
                "CP §3.5.3's own recovery is to continue at Nr. 07: unload the \
                 following segments and reload them in ascending order, which turns a \
                 partial download into a full one mid-flight",
                StepEffect::Write,
            ),
            step(
                8,
                "load the part",
                "the inner loop of §7.2",
                StepEffect::Write,
            ),
            step(
                9,
                "disconnect",
                "disconnect via the bus",
                StepEffect::Connection,
            ),
        ],
    }
}

/// §7.5, CP §3.5.4, steps 01–06 only.
///
/// Step 07 — `SerialNumber_IndividualAddress_Write(FFFFh)` by broadcast —
/// is deliberately absent. It makes a device unaddressable by design and
/// phase 2 does not build it.
pub fn unload() -> Procedure {
    Procedure {
        kind: ProcedureKind::Unload,
        source: "CP §3.5.4 steps 01-06",
        steps: vec![
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
            step(
                5,
                "unload every part",
                "LoadControl = Unload on the Address Table, Association Table, Object \
                 Table, Application Program 2 and Application Program 1, then wait \
                 until the load states read Unloaded",
                StepEffect::Write,
            ),
            step(
                6,
                "disconnect",
                "disconnect via the bus",
                StepEffect::Connection,
            ),
        ],
    }
}

/// §9.1, recovery after an interrupted download. No retry loop: a failed
/// recovery is reported as one.
pub fn recovery() -> Procedure {
    Procedure {
        kind: ProcedureKind::Recovery,
        source: "spec §9.1",
        steps: vec![
            step(
                1,
                "connect, verify, authorise, check the manufacturer ID",
                "the same four opening steps as §7.1",
                StepEffect::Connection,
            ),
            step(
                2,
                "read every load state",
                "PID_LOAD_STATE_CONTROL per loadable part",
                StepEffect::Read,
            ),
            step(
                3,
                "read PID_ERROR_CODE before unloading anything",
                "RES §4.2.28 clears the error code when the state leaves Error, so \
                 unloading first destroys the only evidence of what went wrong",
                StepEffect::Read,
            ),
            step(
                4,
                "unload every part that is not Loaded",
                "write Unload, wait per §5.5, and accept Unloaded directly without \
                 ever observing Unloading",
                StepEffect::Write,
            ),
            step(
                5,
                "re-run the complete download from step 06",
                "the loading half of §7.1, the unload of step 05 having just happened",
                StepEffect::Write,
            ),
            step(
                6,
                "confirm every part reports Loaded",
                "anything else is a failed recovery and is reported as one, not \
                 retried in a loop",
                StepEffect::Read,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §14 item 6: the step lists match the clauses' own numbering and
    /// order.
    #[test]
    fn every_procedure_numbers_its_steps_from_one_without_gaps() {
        for kind in ProcedureKind::ALL {
            let procedure = kind.procedure();
            for (index, step) in procedure.steps.iter().enumerate() {
                assert_eq!(
                    step.number as usize,
                    index + 1,
                    "{kind} step {} is out of order",
                    step.number
                );
            }
        }
    }

    #[test]
    fn the_complete_download_has_the_twelve_steps_cp_3_5_2_lists() {
        let procedure = complete_download();
        assert_eq!(procedure.steps.len(), 12);
        assert_eq!(procedure.steps[3].title, "check the manufacturer ID");
        assert_eq!(procedure.steps[4].title, "unload the device");
    }

    /// The ordering property that makes an authorisation failure safe:
    /// nothing is written before the manufacturer-ID guard has passed.
    #[test]
    fn no_download_writes_anything_before_the_manufacturer_id_guard() {
        for kind in [
            ProcedureKind::CompleteDownload,
            ProcedureKind::PartialDownload,
            ProcedureKind::Unload,
        ] {
            let procedure = kind.procedure();
            let guard = procedure
                .steps
                .iter()
                .position(|step| step.effect == StepEffect::Guard)
                .expect("each of these procedures has a manufacturer-ID guard");
            let first_write = procedure
                .steps
                .iter()
                .position(|step| step.effect.is_destructive())
                .expect("each of these procedures writes something");
            assert!(
                guard < first_write,
                "{kind}: the guard at {guard} must precede the first write at {first_write}"
            );
        }
    }

    #[test]
    fn the_first_destructive_step_of_a_complete_download_is_the_unload() {
        let procedure = complete_download();
        let first = procedure.first_destructive_step().unwrap();
        assert_eq!(first.number, 5);
        assert_eq!(first.title, "unload the device");
    }

    #[test]
    fn the_unload_procedure_stops_at_step_six_and_never_writes_a_serial_number() {
        let procedure = unload();
        assert_eq!(procedure.steps.len(), 6);
        assert!(
            procedure
                .steps
                .iter()
                .all(|step| !step.detail.contains("SerialNumber")),
            "step 07 is out of scope for phase 2 and must not appear"
        );
        assert!(procedure.source.contains("01-06"));
    }

    #[test]
    fn recovery_reads_the_error_code_before_it_unloads_anything() {
        let procedure = recovery();
        let error_code = procedure
            .steps
            .iter()
            .position(|step| step.title.contains("PID_ERROR_CODE"))
            .unwrap();
        let unload = procedure
            .steps
            .iter()
            .position(|step| step.title.starts_with("unload every part"))
            .unwrap();
        assert!(
            error_code < unload,
            "unloading first would clear the only evidence (RES §4.2.28)"
        );
    }

    #[test]
    fn the_inner_loop_reads_the_base_address_before_it_writes_any_data() {
        let procedure = load_one_part();
        let reference = procedure
            .steps
            .iter()
            .position(|step| step.detail.contains("PID_REFERENCE"))
            .unwrap();
        let data = procedure
            .steps
            .iter()
            .position(|step| step.title == "write the data")
            .unwrap();
        assert!(reference < data);
    }

    #[test]
    fn the_inner_loop_ends_by_storing_the_checksum() {
        let procedure = load_one_part();
        let last = procedure.steps.last().unwrap();
        assert_eq!(last.number, 7);
        assert!(last.detail.contains("PID_MCB"));
    }

    #[test]
    fn the_partial_download_records_the_escalation_branch_rather_than_an_abort() {
        let procedure = partial_download();
        let escalation = procedure
            .steps
            .iter()
            .find(|step| step.title.contains("escalate"))
            .expect("CP §3.5.3's failed-allocation branch must be modelled");
        assert!(escalation.detail.contains("continue at Nr. 07"));
        // And it must not claim the unspecified algorithm.
        assert!(procedure
            .steps
            .iter()
            .any(|step| step.detail.contains("GAP-T30-04")));
    }

    #[test]
    fn nothing_claims_to_implement_differential_download() {
        for kind in ProcedureKind::ALL {
            for step in kind.procedure().steps {
                let text = step.detail.to_lowercase();
                assert!(
                    !text.contains("differential download algorithm is"),
                    "{kind} step {} claims an unspecified algorithm",
                    step.number
                );
            }
        }
    }

    #[test]
    fn the_individual_address_write_is_step_three_of_four() {
        let procedure = individual_address_write();
        assert_eq!(procedure.steps.len(), 4);
        assert_eq!(procedure.steps[2].number, 3);
        assert_eq!(procedure.steps[2].title, "write the new address");
        assert!(procedure.steps[1]
            .detail
            .contains("distinct source addresses"));
    }

    #[test]
    fn each_procedure_that_writes_names_the_scope_it_needs() {
        assert_eq!(
            complete_download().required_scope(),
            Some(WriteScope::Download)
        );
        assert_eq!(unload().required_scope(), Some(WriteScope::Unload));
        assert_eq!(
            individual_address_write().required_scope(),
            Some(WriteScope::IndividualAddressProgramming)
        );
    }

    #[test]
    fn a_rendered_procedure_names_its_source_clause() {
        let text = complete_download().render();
        assert!(text.contains("CP §3.5.2"), "{text}");
        assert!(text.contains("01 connect"), "{text}");
        assert!(text.contains("[write]"), "{text}");
    }
}
