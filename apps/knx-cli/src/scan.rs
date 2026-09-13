//! Pure argument parsing, output formatting and project comparison for `knx bus scan`.
//!
//! Everything in this module is deterministic and socket-free: turning
//! `--line`/`--range`/`--exclude` strings into a `knx_core::scan::ScanPlan`,
//! turning a `ProbeOutcome` into a printable line, turning a batch of
//! results into a summary, and comparing a scan against a stored project.
//! `main.rs` owns the only two things that need a runtime: opening the
//! tunnel and loading the project file from disk. Splitting it this way is
//! what makes "no test in this task opens a socket" (T17 Task 4 brief)
//! possible without faking a `TunnelClient`.

use std::collections::HashSet;
use std::time::Duration;

use knx_core::scan::ScanPlan;
use knx_core::IndividualAddress;
use knx_net::{ProbeOutcome, ProbePolicy};

/// Raw `knx bus scan` arguments, one field per flag, unvalidated. Kept
/// separate from the validated forms below (`ScannedRange`, `ScanPlan`,
/// `ProbePolicy`) the same way `BusWriteArgs` in `main.rs` separates
/// "what the user typed" from "what it means".
#[derive(Debug)]
pub struct ScanArgs {
    pub gateway: String,
    pub line: String,
    pub range: Option<String>,
    /// One entry per `--exclude` occurrence, each itself possibly a
    /// comma-separated list. Deliberately not "last one wins" like every
    /// other flag in this file — see the comment on the `--exclude` match
    /// arm below.
    pub exclude: Vec<String>,
    pub timeout_ms: Option<String>,
    pub pause_ms: Option<String>,
    pub project: Option<String>,
    pub dry_run: bool,
}

/// Parses `knx bus scan`'s flags. Unknown flags and a missing
/// `--gateway`/`--line` are errors; everything else is deferred to
/// [`build_scan_plan`]/[`build_probe_policy`], which do the actual
/// validation.
pub fn parse_scan_args(args: &[String]) -> Result<ScanArgs, String> {
    let mut gateway = None;
    let mut line = None;
    let mut range = None;
    let mut exclude = Vec::new();
    let mut timeout_ms = None;
    let mut pause_ms = None;
    let mut project = None;
    let mut dry_run = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gateway" => {
                gateway = Some(crate::take_value(args, i + 1, "--gateway")?);
                i += 2;
            }
            "--line" => {
                line = Some(crate::take_value(args, i + 1, "--line")?);
                i += 2;
            }
            "--range" => {
                range = Some(crate::take_value(args, i + 1, "--range")?);
                i += 2;
            }
            "--exclude" => {
                // Every other flag here is last-wins: repeating it is a
                // harmless way to overwrite a mistake. `--exclude` cannot
                // work that way, because "overwrite" and "silently narrow
                // the set of addresses this scan refuses to probe" are the
                // same operation for this one flag. A second `--exclude`
                // must add to the set, never replace it — the whole point
                // of the flag (see the 1.1.220 rule) is that a name once
                // given never quietly falls out of it.
                exclude.push(crate::take_value(args, i + 1, "--exclude")?);
                i += 2;
            }
            "--timeout-ms" => {
                timeout_ms = Some(crate::take_value(args, i + 1, "--timeout-ms")?);
                i += 2;
            }
            "--pause-ms" => {
                pause_ms = Some(crate::take_value(args, i + 1, "--pause-ms")?);
                i += 2;
            }
            "--project" => {
                project = Some(crate::take_value(args, i + 1, "--project")?);
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }
    Ok(ScanArgs {
        gateway: gateway.ok_or_else(|| "--gateway is required".to_string())?,
        line: line.ok_or_else(|| "--line is required".to_string())?,
        range,
        exclude,
        timeout_ms,
        pause_ms,
        project,
        dry_run,
    })
}

/// The device-address span a scan was asked to cover, before exclusions:
/// `--line`'s area/line, narrowed by `--range` if one was given. Used to
/// decide which excluded and which project addresses are actually in
/// scope for this scan's summary and comparison — a `ScanPlan` itself
/// exposes neither its excluded set nor its original (pre-exclusion)
/// bounds, by design (Task 1: nothing outside `verify()` gets to see the
/// exclusion set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScannedRange {
    pub area: u8,
    pub line: u8,
    pub first_device: u8,
    pub last_device: u8,
}

impl ScannedRange {
    /// Whether `addr` falls on this scan's line, within its device bounds
    /// — regardless of whether it was actually probed (an excluded address
    /// is "in range" for this purpose; it is simply not in `ScanPlan::addresses()`).
    pub fn contains(&self, addr: IndividualAddress) -> bool {
        addr.area() == self.area
            && addr.line() == self.line
            && addr.device() >= self.first_device
            && addr.device() <= self.last_device
    }
}

/// Parses `--line <area.line>`, e.g. `"1.1"`.
fn parse_line(spec: &str) -> Result<(u8, u8), String> {
    let parts: Vec<&str> = spec.split('.').collect();
    let [area, line] = parts[..] else {
        return Err(format!("--line must be area.line, e.g. 1.1 (got {spec:?})"));
    };
    let area: u8 = area
        .trim()
        .parse()
        .map_err(|_| format!("--line area is not a number: {spec:?}"))?;
    let line: u8 = line
        .trim()
        .parse()
        .map_err(|_| format!("--line line is not a number: {spec:?}"))?;
    Ok((area, line))
}

/// Parses every `--exclude <addr>[,<addr>...]` occurrence into one union
/// exclusion set for Task 1. Refuses the whole argument on the first
/// malformed token rather than dropping it — see the module doc on
/// `--exclude` in the task brief: a typo an exclusion list silently drops
/// is exactly the failure the 1.1.220 rule exists to prevent. Every
/// occurrence contributes to the same set; none of them replace an
/// earlier one (see `parse_scan_args`'s `--exclude` arm).
fn parse_exclusions(specs: &[String]) -> Result<HashSet<IndividualAddress>, String> {
    let mut excluded = HashSet::new();
    for spec in specs {
        for token in spec.split(',') {
            let address = token
                .trim()
                .parse::<IndividualAddress>()
                .map_err(|e| format!("invalid address in --exclude: {token:?}: {e}"))?;
            excluded.insert(address);
        }
    }
    Ok(excluded)
}

/// Parses `--range <first>-<last>`, where `first`/`last` are full
/// `area.line.device` addresses (not bare device numbers) — only a full
/// address can "imply a different line" than `--line`, which is the
/// exact case this function must reject rather than clamp.
fn parse_range(
    spec: &str,
    area: u8,
    line: u8,
) -> Result<(IndividualAddress, IndividualAddress), String> {
    let (first_s, last_s) = spec.split_once('-').ok_or_else(|| {
        format!("--range must be <first>-<last>, e.g. 1.1.10-1.1.50 (got {spec:?})")
    })?;
    let first: IndividualAddress = first_s
        .trim()
        .parse()
        .map_err(|e| format!("invalid --range start address {first_s:?}: {e}"))?;
    let last: IndividualAddress = last_s
        .trim()
        .parse()
        .map_err(|e| format!("invalid --range end address {last_s:?}: {e}"))?;
    if first.area() != area || first.line() != line || last.area() != area || last.line() != line {
        return Err(format!(
            "--range {spec} is not within line {area}.{line} given by --line"
        ));
    }
    if first.device() > last.device() {
        return Err(format!(
            "--range {spec} has a start address after its end address"
        ));
    }
    if first.device() == 0 {
        return Err(format!(
            "--range {spec} starts at device 0, which is the line coupler's own address, \
             not a device to probe"
        ));
    }
    Ok((first, last))
}

/// Turns `--line`/`--range`/`--exclude` into a verified [`ScanPlan`], the
/// [`ScannedRange`] it was built from, and the exclusion set that went
/// into it. All string-to-value conversion; touches no socket. A
/// malformed `--exclude` or a `--range` spanning two lines aborts here,
/// before a `ScanPlan` — let alone a connection — ever exists.
///
/// With no `--range`, this defers to [`ScanPlan::line`] rather than
/// hand-rolling a device 1..=255 span through [`ScanPlan::range`]: `line`
/// is the one that carries the `[D]`-cited device-0-is-the-coupler
/// exclusion (`ScanPlanBuilder::line`'s doc comment), and duplicating its
/// bounds here would strand that citation on a second, unreviewed copy.
pub fn build_scan_plan(
    line_spec: &str,
    range_spec: Option<&str>,
    exclude_specs: &[String],
) -> Result<(ScanPlan, ScannedRange, HashSet<IndividualAddress>), String> {
    let (area, line) = parse_line(line_spec)?;
    let excluded = parse_exclusions(exclude_specs)?;
    let (plan, first_device, last_device) = match range_spec {
        Some(spec) => {
            let (first, last) = parse_range(spec, area, line)?;
            let plan = ScanPlan::range(first, last, excluded.iter().copied())
                .map_err(|e| format!("could not build scan plan: {e}"))?;
            (plan, first.device(), last.device())
        }
        None => {
            let plan = ScanPlan::line(area, line, excluded.iter().copied())
                .map_err(|e| format!("could not build scan plan: {e}"))?;
            (plan, 1, 255)
        }
    };
    let range = ScannedRange {
        area,
        line,
        first_device,
        last_device,
    };
    Ok((plan, range, excluded))
}

/// Turns `--timeout-ms`/`--pause-ms` into a [`ProbePolicy`], defaulting to
/// [`ProbePolicy::default`] for whichever one is not given.
/// `vacant_confirmations` has no CLI flag (not named in the T17 Task 4
/// brief) and always keeps the default's value.
pub fn build_probe_policy(
    timeout_ms: Option<&str>,
    pause_ms: Option<&str>,
) -> Result<ProbePolicy, String> {
    let default = ProbePolicy::default();
    let timeout = match timeout_ms {
        Some(s) => Duration::from_millis(
            s.trim()
                .parse::<u64>()
                .map_err(|_| format!("--timeout-ms is not a number: {s:?}"))?,
        ),
        None => default.response_timeout(),
    };
    let pause = match pause_ms {
        Some(s) => Duration::from_millis(
            s.trim()
                .parse::<u64>()
                .map_err(|_| format!("--pause-ms is not a number: {s:?}"))?,
        ),
        None => default.inter_probe_pause(),
    };
    // `ProbePolicy::new` only rejects `vacant_confirmations == 0` — a zero
    // `response_timeout` passes it happily, and then reports every
    // present device as `Vacant`, because no reply can ever arrive within
    // a window of zero. That failure mode is silent and looks exactly
    // like an empty line, so it is caught here instead.
    if timeout.is_zero() {
        return Err(
            "--timeout-ms must be greater than zero (0 would time out before any \
             reply could ever arrive, misreporting every present device as vacant)"
                .to_string(),
        );
    }
    ProbePolicy::new(timeout, default.vacant_confirmations(), pause).map_err(|e| e.to_string())
}

/// Renders `knx bus scan --dry-run`'s preview: how many candidates the
/// plan holds, its first and last candidate address, and the full
/// excluded list — everything needed to sanity-check a scan's *shape*
/// without a socket ever being opened, modelled on `knx bus write
/// --dry-run`. The excluded list is sorted by raw address so the output
/// is deterministic; `HashSet` iteration order is not.
pub fn format_dry_run(plan: &ScanPlan, excluded: &HashSet<IndividualAddress>) -> String {
    let addresses = plan.addresses();
    let mut out = format!("dry run: {} candidate address(es)", addresses.len());
    if let (Some(first), Some(last)) = (addresses.first(), addresses.last()) {
        out.push_str(&format!(", first {first}, last {last}"));
    }
    let mut excluded_sorted: Vec<IndividualAddress> = excluded.iter().copied().collect();
    excluded_sorted.sort_by_key(|a| a.raw());
    out.push_str(&format!("\n  excluded ({}):", excluded_sorted.len()));
    for addr in &excluded_sorted {
        out.push_str(&format!("\n    {addr}"));
    }
    out
}

/// Renders one probed address's line, printed as its outcome becomes
/// known. Every `ProbeOutcome` variant gets its own label — `Occupied`
/// and `OccupiedSilent` are never merged (a merged label would misreport
/// whether a descriptor was actually read), and `Indeterminate` is
/// reported as itself, never folded into `Vacant` (Global Constraint 3).
pub fn format_probe_line(
    addr: IndividualAddress,
    outcome: ProbeOutcome,
    round_trip: Duration,
) -> String {
    let label = match outcome {
        ProbeOutcome::Occupied {
            mask_version: Some(mask_version),
        } => format!("occupied (mask {mask_version:#06x})"),
        ProbeOutcome::Occupied { mask_version: None } => "occupied (mask unknown)".to_string(),
        ProbeOutcome::OccupiedSilent => "occupied-silent".to_string(),
        ProbeOutcome::OccupiedBusy => "busy".to_string(),
        ProbeOutcome::Vacant => "vacant".to_string(),
        ProbeOutcome::Indeterminate => "indeterminate".to_string(),
        ProbeOutcome::SelfAddress => "self".to_string(),
    };
    format!("{addr}  {label}  {}ms", round_trip.as_millis())
}

/// Derives one probed address's round trip from the wall-clock gap
/// between two successive `scan_line` progress callbacks.
///
/// `scan_line` sleeps `policy.inter_probe_pause()` after every probed
/// address except when that address's outcome was `SelfAddress`
/// (`crates/knx-net/src/scan.rs`, the loop driving each `probe_address`
/// call) — so subtracting `pause` from `elapsed` undoes exactly that
/// sleep, unless the *previous* outcome was `SelfAddress`, in which case
/// `scan_line` never slept and `elapsed` already is the round trip.
/// `prev == None` (the very first address) is treated the same as
/// `SelfAddress`: nothing has slept yet.
///
/// This is only exact when `policy.vacant_confirmations() == 1`. With a
/// higher confirmation count, `probe_address` sleeps `inter_probe_pause`
/// *between its own confirmation passes* as well
/// (`crates/knx-net/src/scan.rs`), and this function has no way to know
/// how many of those internal sleeps the previous address needed — with
/// confirmations > 1 the result overstates the true round trip by
/// `(vacant_confirmations - 1) * inter_probe_pause` whenever the previous
/// address needed more than one pass. `build_probe_policy` never lets
/// `vacant_confirmations` differ from the default of 1 today (T17 Task 4
/// names no flag for it), so the precondition currently always holds —
/// but this function cannot enforce it itself, only assume it.
pub fn round_trip(elapsed: Duration, pause: Duration, prev: Option<ProbeOutcome>) -> Duration {
    match prev {
        Some(outcome) if outcome != ProbeOutcome::SelfAddress => elapsed.saturating_sub(pause),
        _ => elapsed,
    }
}

/// Per-outcome counts for one scan, plus how many candidates in scope
/// never got probed at all because `--exclude` named them. Every
/// `ProbeOutcome` variant has its own field — see [`summarize`] — so a
/// count can never silently swallow one kind into another.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanSummary {
    pub occupied: usize,
    pub occupied_silent: usize,
    pub busy: usize,
    pub vacant: usize,
    pub indeterminate: usize,
    pub self_address: usize,
    pub excluded: usize,
}

impl ScanSummary {
    /// Every address a probe was actually attempted against — excludes
    /// `excluded`, which by definition was never sent a frame.
    pub fn total_probed(&self) -> usize {
        self.occupied
            + self.occupied_silent
            + self.busy
            + self.vacant
            + self.indeterminate
            + self.self_address
    }
}

/// Tallies a batch of `scan_line` results into a [`ScanSummary`].
/// `excluded` is the count of addresses that were in scope for this scan
/// but never probed — computed by the caller from the pre-exclusion
/// [`ScannedRange`] and the exclusion set, since a `ScanPlan` does not
/// expose either back out.
pub fn summarize(results: &[(IndividualAddress, ProbeOutcome)], excluded: usize) -> ScanSummary {
    let mut summary = ScanSummary {
        excluded,
        ..Default::default()
    };
    for (_, outcome) in results {
        match outcome {
            ProbeOutcome::Occupied { .. } => summary.occupied += 1,
            ProbeOutcome::OccupiedSilent => summary.occupied_silent += 1,
            ProbeOutcome::OccupiedBusy => summary.busy += 1,
            ProbeOutcome::Vacant => summary.vacant += 1,
            ProbeOutcome::Indeterminate => summary.indeterminate += 1,
            ProbeOutcome::SelfAddress => summary.self_address += 1,
        }
    }
    summary
}

/// Renders a [`ScanSummary`] together with the elapsed wall time and the
/// [`ProbePolicy`] that was actually used — per the task brief, "a scan
/// result without its timeout policy is not interpretable".
pub fn format_summary(summary: &ScanSummary, elapsed: Duration, policy: &ProbePolicy) -> String {
    format!(
        "{} probed: {} occupied, {} occupied-silent, {} busy, {} vacant, {} indeterminate, \
         {} self, {} excluded; elapsed {}ms; policy: timeout={}ms confirmations={} pause={}ms",
        summary.total_probed(),
        summary.occupied,
        summary.occupied_silent,
        summary.busy,
        summary.vacant,
        summary.indeterminate,
        summary.self_address,
        summary.excluded,
        elapsed.as_millis(),
        policy.response_timeout().as_millis(),
        policy.vacant_confirmations(),
        policy.inter_probe_pause().as_millis(),
    )
}

/// What `--project` is actually for (E2): addresses the bus answered that
/// the project does not list, and addresses the project lists that did
/// not answer — plus a third bucket this task's brief does not name but
/// Global Constraint 3 requires anyway: a project device sitting on an
/// address `--exclude` removed from the scan never got a chance to
/// answer, so counting it as "did not answer" would misreport a skip as a
/// negative result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectComparison {
    /// The bus answered (occupied, occupied-silent or busy); the project
    /// does not list a device at this address.
    pub unexpected: Vec<IndividualAddress>,
    /// The project lists a device at this address, in scan range, not
    /// excluded; the bus did not confirm it present (vacant or
    /// indeterminate).
    pub missing: Vec<IndividualAddress>,
    /// The project lists a device at this address, but `--exclude`
    /// removed it from the scan before a single frame was sent.
    pub excluded_in_project: Vec<IndividualAddress>,
}

/// Compares a scan's results against a stored project's device addresses,
/// restricted to `range` (a project may hold devices on lines this scan
/// never touched; those are out of scope for this comparison, not silently
/// "missing"). Never writes anything back — this task discovers, it does
/// not reconcile.
pub fn compare_with_project(
    range: &ScannedRange,
    results: &[(IndividualAddress, ProbeOutcome)],
    excluded: &HashSet<IndividualAddress>,
    project_addresses: &[IndividualAddress],
) -> ProjectComparison {
    let present: HashSet<IndividualAddress> = results
        .iter()
        .filter(|(_, outcome)| {
            matches!(
                outcome,
                ProbeOutcome::Occupied { .. }
                    | ProbeOutcome::OccupiedSilent
                    | ProbeOutcome::OccupiedBusy
            )
        })
        .map(|(addr, _)| *addr)
        .collect();

    let in_range_project: HashSet<IndividualAddress> = project_addresses
        .iter()
        .copied()
        .filter(|addr| range.contains(*addr))
        .collect();

    let mut unexpected: Vec<IndividualAddress> = present
        .iter()
        .copied()
        .filter(|addr| !in_range_project.contains(addr))
        .collect();
    unexpected.sort_by_key(|a| a.raw());

    let mut excluded_in_project: Vec<IndividualAddress> = in_range_project
        .iter()
        .copied()
        .filter(|addr| excluded.contains(addr))
        .collect();
    excluded_in_project.sort_by_key(|a| a.raw());

    let mut missing: Vec<IndividualAddress> = in_range_project
        .iter()
        .copied()
        .filter(|addr| !present.contains(addr) && !excluded.contains(addr))
        .collect();
    missing.sort_by_key(|a| a.raw());

    ProjectComparison {
        unexpected,
        missing,
        excluded_in_project,
    }
}

/// Renders a [`ProjectComparison`] as counts plus a per-bucket address
/// listing, printed only — never written back into the project.
pub fn format_project_comparison(cmp: &ProjectComparison) -> String {
    let mut out = format!(
        "project comparison: {} answered but not in project, {} in project but did not answer, \
         {} in project but excluded from this scan",
        cmp.unexpected.len(),
        cmp.missing.len(),
        cmp.excluded_in_project.len(),
    );
    let mut section = |title: &str, addrs: &[IndividualAddress]| {
        if !addrs.is_empty() {
            out.push_str(&format!("\n  {title}:"));
            for addr in addrs {
                out.push_str(&format!("\n    {addr}"));
            }
        }
    };
    section("answered, not in project", &cmp.unexpected);
    section("in project, did not answer", &cmp.missing);
    section("in project, excluded from scan", &cmp.excluded_in_project);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).unwrap()
    }

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // --- parse_scan_args ---

    #[test]
    fn parses_every_flag() {
        let parsed = parse_scan_args(&args(&[
            "--gateway",
            "192.0.2.1:3671",
            "--line",
            "1.1",
            "--range",
            "1.1.2-1.1.9",
            "--exclude",
            "1.1.220",
            "--timeout-ms",
            "500",
            "--pause-ms",
            "50",
            "--project",
            "proj.knxdb",
            "--dry-run",
        ]))
        .unwrap();
        assert_eq!(parsed.gateway, "192.0.2.1:3671");
        assert_eq!(parsed.line, "1.1");
        assert_eq!(parsed.range.as_deref(), Some("1.1.2-1.1.9"));
        assert_eq!(parsed.exclude, vec!["1.1.220".to_string()]);
        assert_eq!(parsed.timeout_ms.as_deref(), Some("500"));
        assert_eq!(parsed.pause_ms.as_deref(), Some("50"));
        assert_eq!(parsed.project.as_deref(), Some("proj.knxdb"));
        assert!(parsed.dry_run);
    }

    #[test]
    fn missing_gateway_is_an_error() {
        let err = parse_scan_args(&args(&["--line", "1.1"])).unwrap_err();
        assert!(err.contains("--gateway"));
    }

    #[test]
    fn missing_line_is_an_error() {
        let err = parse_scan_args(&args(&["--gateway", "192.0.2.1:3671"])).unwrap_err();
        assert!(err.contains("--line"));
    }

    #[test]
    fn unrecognized_flag_is_rejected() {
        let err = parse_scan_args(&args(&[
            "--gateway",
            "192.0.2.1:3671",
            "--line",
            "1.1",
            "--bogus",
        ]))
        .unwrap_err();
        assert!(err.contains("--bogus"));
    }

    #[test]
    fn repeated_exclude_flags_accumulate_instead_of_last_one_winning() {
        let parsed = parse_scan_args(&args(&[
            "--gateway",
            "192.0.2.1:3671",
            "--line",
            "1.1",
            "--exclude",
            "1.1.220",
            "--exclude",
            "1.1.221",
        ]))
        .unwrap();
        assert_eq!(
            parsed.exclude,
            vec!["1.1.220".to_string(), "1.1.221".to_string()]
        );
        let (plan, _, excluded) = build_scan_plan("1.1", None, &parsed.exclude).unwrap();
        assert!(!plan.addresses().contains(&addr(1, 1, 220)));
        assert!(!plan.addresses().contains(&addr(1, 1, 221)));
        assert!(excluded.contains(&addr(1, 1, 220)));
        assert!(excluded.contains(&addr(1, 1, 221)));
    }

    // --- build_scan_plan ---

    #[test]
    fn full_line_with_no_range_or_exclude_covers_every_device() {
        let (plan, range, excluded) = build_scan_plan("1.1", None, &[]).unwrap();
        assert_eq!(plan.addresses().len(), 255);
        assert_eq!(
            range,
            ScannedRange {
                area: 1,
                line: 1,
                first_device: 1,
                last_device: 255
            }
        );
        assert!(excluded.is_empty());
    }

    #[test]
    fn range_narrows_the_device_span_within_the_line() {
        let (plan, range, _) = build_scan_plan("1.1", Some("1.1.2-1.1.9"), &[]).unwrap();
        assert_eq!(plan.addresses().len(), 8);
        assert_eq!(range.first_device, 2);
        assert_eq!(range.last_device, 9);
    }

    #[test]
    fn range_spanning_two_lines_is_rejected() {
        let err = build_scan_plan("1.1", Some("1.1.2-1.2.9"), &[]).unwrap_err();
        assert!(
            err.contains("1.1"),
            "error should name the requested line: {err}"
        );
    }

    #[test]
    fn range_implying_a_different_line_than_line_flag_is_rejected() {
        // Both endpoints agree with each other, but not with --line.
        let err = build_scan_plan("1.1", Some("1.2.2-1.2.9"), &[]).unwrap_err();
        assert!(err.contains("1.1"));
    }

    #[test]
    fn range_with_start_after_end_is_rejected() {
        let err = build_scan_plan("1.1", Some("1.1.9-1.1.2"), &[]).unwrap_err();
        assert!(err.contains("start"));
    }

    #[test]
    fn range_starting_at_device_zero_is_rejected_as_the_coupler_address() {
        let err = build_scan_plan("1.1", Some("1.1.0-1.1.9"), &[]).unwrap_err();
        assert!(
            err.contains("coupler"),
            "error should name the reason: {err}"
        );
    }

    #[test]
    fn malformed_exclude_refuses_to_start_rather_than_dropping_the_entry() {
        let err = build_scan_plan("1.1", None, &args(&["1.1.5,not-an-address"])).unwrap_err();
        assert!(err.contains("not-an-address"));
    }

    #[test]
    fn the_alarm_panel_example_address_is_never_a_scan_candidate_once_excluded() {
        let (plan, range, excluded) = build_scan_plan("1.1", None, &args(&["1.1.220"])).unwrap();
        assert!(!plan.addresses().contains(&addr(1, 1, 220)));
        assert_eq!(plan.addresses().len(), 254);
        assert!(excluded.contains(&addr(1, 1, 220)));
        assert_eq!(range.first_device, 1);
        assert_eq!(range.last_device, 255);
    }

    #[test]
    fn invalid_line_is_rejected() {
        assert!(build_scan_plan("16.1", None, &[]).is_err());
    }

    // --- format_dry_run ---

    #[test]
    fn dry_run_reports_candidate_count_bounds_and_excluded_list() {
        let (plan, _, excluded) =
            build_scan_plan("1.1", Some("1.1.2-1.1.9"), &args(&["1.1.4,1.1.5"])).unwrap();
        let text = format_dry_run(&plan, &excluded);
        assert!(text.contains("6 candidate address(es)"));
        assert!(text.contains("first 1.1.2"));
        assert!(text.contains("last 1.1.9"));
        assert!(text.contains("excluded (2):"));
        assert!(text.contains("1.1.4"));
        assert!(text.contains("1.1.5"));
    }

    #[test]
    fn dry_run_with_no_exclusions_still_names_the_zero_count() {
        let (plan, _, excluded) = build_scan_plan("1.1", Some("1.1.2-1.1.3"), &[]).unwrap();
        let text = format_dry_run(&plan, &excluded);
        assert!(text.contains("excluded (0):"));
    }

    // --- build_probe_policy ---

    #[test]
    fn default_policy_when_nothing_given() {
        let policy = build_probe_policy(None, None).unwrap();
        let default = ProbePolicy::default();
        assert_eq!(policy.response_timeout(), default.response_timeout());
        assert_eq!(policy.inter_probe_pause(), default.inter_probe_pause());
    }

    #[test]
    fn overrides_timeout_and_pause() {
        let policy = build_probe_policy(Some("500"), Some("25")).unwrap();
        assert_eq!(policy.response_timeout(), Duration::from_millis(500));
        assert_eq!(policy.inter_probe_pause(), Duration::from_millis(25));
    }

    #[test]
    fn non_numeric_timeout_is_rejected() {
        assert!(build_probe_policy(Some("soon"), None).is_err());
    }

    #[test]
    fn zero_timeout_is_rejected_rather_than_silently_misreporting_every_device_as_vacant() {
        let err = build_probe_policy(Some("0"), None).unwrap_err();
        assert!(err.contains("--timeout-ms"));
    }

    // --- round_trip ---

    #[test]
    fn round_trip_of_the_first_address_is_the_raw_elapsed_time() {
        let elapsed = Duration::from_millis(30);
        let pause = Duration::from_millis(100);
        assert_eq!(round_trip(elapsed, pause, None), elapsed);
    }

    #[test]
    fn round_trip_after_a_normal_outcome_subtracts_the_inter_probe_pause() {
        let elapsed = Duration::from_millis(130);
        let pause = Duration::from_millis(100);
        assert_eq!(
            round_trip(elapsed, pause, Some(ProbeOutcome::Vacant)),
            Duration::from_millis(30)
        );
    }

    #[test]
    fn round_trip_after_self_address_is_not_reduced_because_scan_line_never_slept() {
        let elapsed = Duration::from_millis(30);
        let pause = Duration::from_millis(100);
        assert_eq!(
            round_trip(elapsed, pause, Some(ProbeOutcome::SelfAddress)),
            elapsed
        );
    }

    // --- format_probe_line ---

    #[test]
    fn formats_every_outcome_distinctly() {
        let a = addr(1, 1, 5);
        let d = Duration::from_millis(12);
        let occupied = format_probe_line(
            a,
            ProbeOutcome::Occupied {
                mask_version: Some(0x0705),
            },
            d,
        );
        let occupied_unknown_mask =
            format_probe_line(a, ProbeOutcome::Occupied { mask_version: None }, d);
        let silent = format_probe_line(a, ProbeOutcome::OccupiedSilent, d);
        let busy = format_probe_line(a, ProbeOutcome::OccupiedBusy, d);
        let vacant = format_probe_line(a, ProbeOutcome::Vacant, d);
        let indeterminate = format_probe_line(a, ProbeOutcome::Indeterminate, d);
        let own = format_probe_line(a, ProbeOutcome::SelfAddress, d);
        let labels = [
            &occupied,
            &occupied_unknown_mask,
            &silent,
            &busy,
            &vacant,
            &indeterminate,
            &own,
        ];
        let unique: HashSet<&&String> = labels.iter().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "every outcome must render distinctly: {labels:?}"
        );
        assert!(occupied.contains("0x0705"));
        assert!(occupied.contains("12ms"));
    }

    // --- summarize / format_summary ---

    fn fixed_results() -> Vec<(IndividualAddress, ProbeOutcome)> {
        vec![
            (
                addr(1, 1, 2),
                ProbeOutcome::Occupied {
                    mask_version: Some(0x0705),
                },
            ),
            (addr(1, 1, 3), ProbeOutcome::OccupiedSilent),
            (addr(1, 1, 4), ProbeOutcome::OccupiedBusy),
            (addr(1, 1, 5), ProbeOutcome::Vacant),
            (addr(1, 1, 6), ProbeOutcome::Vacant),
            (addr(1, 1, 7), ProbeOutcome::Indeterminate),
            (addr(1, 1, 8), ProbeOutcome::SelfAddress),
        ]
    }

    #[test]
    fn summarize_counts_every_outcome_kind_separately() {
        let summary = summarize(&fixed_results(), 3);
        assert_eq!(
            summary,
            ScanSummary {
                occupied: 1,
                occupied_silent: 1,
                busy: 1,
                vacant: 2,
                indeterminate: 1,
                self_address: 1,
                excluded: 3,
            }
        );
        assert_eq!(summary.total_probed(), 7);
    }

    #[test]
    fn format_summary_names_the_policy_used() {
        let summary = summarize(&fixed_results(), 3);
        let policy =
            ProbePolicy::new(Duration::from_millis(500), 1, Duration::from_millis(25)).unwrap();
        let text = format_summary(&summary, Duration::from_millis(4200), &policy);
        assert!(text.contains("timeout=500ms"));
        assert!(text.contains("pause=25ms"));
        assert!(text.contains("confirmations=1"));
        assert!(text.contains("4200ms"));
        assert!(text.contains("1 occupied"));
        assert!(text.contains("1 indeterminate"));
        assert!(text.contains("3 excluded"));
    }

    // --- compare_with_project ---

    #[test]
    fn compare_finds_unexpected_missing_and_excluded_devices() {
        let range = ScannedRange {
            area: 1,
            line: 1,
            first_device: 1,
            last_device: 9,
        };
        let results = fixed_results();
        // 1.1.8 is excluded from this fixture's project comparison scenario
        // (it was probed as SelfAddress in `fixed_results`, not relevant
        // here — this test exercises the *project* exclusion bucket via a
        // separate excluded address, 1.1.9, which never appears in results
        // at all because the plan never probed it).
        let excluded: HashSet<IndividualAddress> = [addr(1, 1, 9)].into_iter().collect();
        let project_addresses = vec![
            addr(1, 1, 3), // occupied-silent: present in both -> neither bucket
            addr(1, 1, 5), // vacant, in project, not excluded -> missing
            addr(1, 1, 9), // excluded from the scan -> excluded_in_project
            addr(1, 2, 1), // different line entirely -> out of scope, ignored
        ];
        let cmp = compare_with_project(&range, &results, &excluded, &project_addresses);
        assert_eq!(cmp.unexpected, vec![addr(1, 1, 2), addr(1, 1, 4)]);
        assert_eq!(cmp.missing, vec![addr(1, 1, 5)]);
        assert_eq!(cmp.excluded_in_project, vec![addr(1, 1, 9)]);
    }

    #[test]
    fn format_project_comparison_lists_every_bucket() {
        let cmp = ProjectComparison {
            unexpected: vec![addr(1, 1, 2)],
            missing: vec![addr(1, 1, 5)],
            excluded_in_project: vec![addr(1, 1, 9)],
        };
        let text = format_project_comparison(&cmp);
        assert!(text.contains("1 answered but not in project"));
        assert!(text.contains("1 in project but did not answer"));
        assert!(text.contains("1 in project but excluded from this scan"));
        assert!(text.contains("1.1.2"));
        assert!(text.contains("1.1.5"));
        assert!(text.contains("1.1.9"));
    }

    #[test]
    fn format_project_comparison_with_nothing_to_report_still_prints_the_zero_counts() {
        let text = format_project_comparison(&ProjectComparison::default());
        assert!(text.contains("0 answered but not in project"));
        assert!(text.contains("0 in project but did not answer"));
        assert!(text.contains("0 in project but excluded from this scan"));
    }
}
