# Prepared Goal — device programming support and actionable refusals

Prepared: 2026-10-13. Local, unpublished prompt; preparation starts no session,
takes no lock and changes no hardware authorization or support classification.

## Session status

Owner invoked S1–S4 in session `d03c0319b8c1` on 2026-10-13 and explicitly
released the affected support-guidance surface after the initial ownership
blocker. Current reconciled worktree `device-support-resume-20261014`; branch
`feature/device-support-resume-20261014`, from published B15 main. The original
candidate is retained as history. Goal B retains all unrelated work.
No nested goal is launched.

| Package | State |
| --- | --- |
| S1 | Complete: structured preparation/start refusals and shared mapping accepted in the full merged-result gate |
| S2 | Complete: EN/DE pre-plan reason/actions, local details/copy and late-target guards; 92 focused tests, 522 merged fixture-browser and 33 production download tests pass |
| S3 | Complete: fixed unsent docs/form/mail links, zero project-derived prefill, privacy tests and EN/DE scope documentation pass the merged gate; no external submission |
| S4 | Two real offline corpus runs passed with identical privacy-checked aggregates (105 packages, 467 programs); bounded F1–F5 follow-ups documented; all executable dependencies remain unassessed |

## Final acceptance

Merged runtime `6bf86976` passes all 35 required stages with frozen inputs:
Rust 4442/0 (200 ignored), Vitest 3504/0 in 249 files, Chromium 522/0,
production download/report/scope/B15 specs 33/0, focused browser 8/0,
private corpus 145/145; all typechecks, Clippy, build, bindings, documentation
and repository checks pass. [Aggregate receipt](../evidence/device-support-guidance-acceptance-2026-10-14.json).
Final closure is documentation-only; source equivalence and doc/disclosure gates
are required before the ordinary main push. No release/deployment is included.
[Bounded F1–F5 follow-ups](../research/2026-10-14-device-support-guidance-offline-audit.md)
remain separate scopes; all audited executable dependencies remain unassessed.
No further goal is launched.

## Invocation

Read this file and the root-local research note from the selected workspace
**before** creating a fresh worktree: neither may exist on origin/main yet.
This file is the full goal contract; the short /goal starter adopts it directly,
without launching another nested goal.

## Objective

Make programming devices without KNXBench hardware verification understandable
and accessible through the existing supported-but-untested path. Initially
scope this workflow to applications that need no unresolved manufacturer
DLL/executable extension behaviour. For every actual refusal, show why and
what the operator can do to obtain support, including GitHub documentation,
a support-gap issue and a documented mail route. Do not promise universal
support for DLL-free devices or confuse lack of live verification with a
technical inability to prepare a download.

## Startup and ownership

1. Workspace: <old-sources>/KNXBench. Read .ai/CURRENT_STATE.md first,
   then AGENTS.md, this file and
   docs/research/2026-10-13-unverified-plugin-free-programming.md.
2. Fetch origin/main and inspect its current implementation, handover, complete
   lock/release queue, worktrees and relevant docs. The preparation inspected
   origin/main 0781c672 on 2026-10-13; this is not a future freshness guarantee.
3. Goal B session ba673d45ff9d owns the web surface. Use a sequential handoff:
   do not modify its web files, shared locales, contracts or queued package
   documents until Goal B explicitly hands off the affected surface or ends
   and releases it. Independent read-only inspection may proceed. If entry is
   blocked, checkpoint and report the exact ownership dependency; do not
   occupy its slot or invent a polling loop. Starting this goal does not
   authorize taking over or extending B's package table.
4. After prerequisites hold, create your own worktree from fresh origin/main.
   Preserve the shared root's uncommitted research/prompt and other owners'
   worktrees, branches, files, stashes and journals. Carry only these two
   named local planning files into the new worktree when needed, after reading
   and reconciling their current bytes; never copy the whole stale docs tree.
5. Recheck ownership immediately before edits and integration. Record a finite
   package plan and your claim through the existing repository procedure.

## Source and reuse boundaries

Read docs/ARCHITECTURE.md, IMPLEMENTATION_STATUS.md, OPEN_WORK.md, status/LEDGER.md,
COMPATIBILITY.md, relevant limitations and current contracts. In particular:

- ADR-0049, ADR-0086, ADR-0121 and contracts/download-verification-reports.md.
- knx-app download_support, device_download and download_report.
- knx-productdb image and download_plan admission.
- CLI and server device-download preparation/start/refusal entry points.
- DeviceDownloadPanel, downloadReadiness and DownloadVerification.
- docs/contribution-intake/{README,DEUTSCH}.md and analysis.yml.

Reuse the existing verified / untested / unsupported classification and plan
executor. Verified is application-and-operation evidence, not a blanket
manufacturer/device certificate. Untested requires its existing extra exact
acknowledgement; unsupported is not made programmable by an acknowledgement.
Do not create a second support registry, executor or generic force override.
Do not infer absence of required executable behaviour merely from not finding
a .dll filename; distinguish known absent, required and unresolved dependencies
using actual available source data, preserving unknowns.

## Finite packages, in order

### S1 — support/refusal contract and actionable guidance

Trace the real public path and implement the smallest necessary contract to
retain a concrete refusal code/detail and a category-appropriate next action.
Reuse current readiness data when adequate; extend application/API projections
only if required, with version/binding handling consistent with the repository.
Do not parse arbitrary error prose in the UI to manufacture domain semantics.

Distinguish verified, untested, unsupported, missing product data, invalid
configuration, exclusion/policy and transient transport/server failure.
Missing data should direct the user to install the right application package;
correctable configuration should identify the correction; missing encoders or
procedures should offer a support-gap request. Unknown/new codes remain visible
with the retained technical detail and generic reporting guidance. Transport
failures must not falsely downgrade device compatibility.

### S2 — refusal help at the point of failure, EN/DE

Show the plain reason, next step and expandable/copyable technical details next
to the selected device and at a failed preparation, including when no plan,
download session or export exists. The reason must not depend on a tooltip.

Offer Instructions, Report support gap and Email actions there. The existing
untested flow remains available only for fully prepared supported plans with
all original confirmations; make its meaning clear without inventing approval.
Preserve the existing post-run DownloadVerification/report workflow rather than
replacing it with the support-gap form. A successful or failed actual run and a
preparation refusal are different evidence types.

Retain stable device/project selection across asynchronous readiness requests;
late results must not paint another selected device or stale project. Busy,
error, missing-detail and unknown-category states remain understandable and
accessible. All strings in both EN and DE, using existing components/themes.

### S3 — documentation and deliberate reporting

Reuse these trusted, application-owned targets (verify them at implementation):

- German instructions: https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/contribution-intake/DEUTSCH.md
- English instructions: https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/contribution-intake/README.md
- Support-gap form: https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml

Document support-gap inquiries through the existing issue@knxbench.com mailbox
as part of this requested issue/mail workflow; do not provision a mailbox or
send a test message. Current docs restrict it to download reports, so update
both guides and affected scope statements consistently. Do not use unconfirmed
contribute@knxbench.com or claim delivery/private-original handling was tested.
Keep public issue versus mail privacy wording precise; email is not a guarantee
of encryption, confidentiality, retention or successful delivery.

A mail action opens an unsent draft; an issue action opens an unsent form.
Reporting must work without an attachment. Reuse existing reduced-report
analysis/export only where it actually applies; never present a fabricated
post-run download report for a refusal that opened no connection.

Allowlist any prefilled metadata: build, public product/application identifiers
and stable refusal code, shown for review. Do not auto-copy free-text errors,
parameter values, project/device/customer names, addresses, gateway endpoints,
paths, originals, backups, keys or credentials into external URLs/drafts.
Technical detail may contain private text: explicit local copy is not automatic
permission to publish it. Bound/encode URLs; never trust product-provided links.
Nothing uploads, submits, sends mail or connects to the bus automatically.

### S4 — offline coverage audit and bounded follow-up

Against available permitted product/project fixtures, verify the actual
untested path and inventory the concrete refusal categories of relevant
extension-independent candidates. Preserve unknown dependency coverage; do not
label uninspected packages DLL-free. Use private-corpus safeguards where needed,
report sample scope/counts mechanically and distinguish defaults-only coverage
from a project's concrete parameters/links.

Identify the next evidence-backed encoder/planner work packages that would
increase support, with missing sources and target-specific fixture needs.
If no relevant fixture is available, state that precise evidence gap rather
than manufacturing a device result. This goal does not implement new masks,
byte-order rules, property-load procedures or other device-family executors;
those need separate finite scope based on this audit, not a blanket promise.
An external-evidence gap does not block delivery of the verified UI/report work.

## Safety and explicit exclusions

- No live bus use, device programming, address/reset operation or read probe;
  no ETS execution, manufacturer binaries, vendor contact or approval route.
- No bypass of target identity, valid key provenance, plan freshness, backup,
  readback, tunnel exclusivity, stop/drain admission or operator consent.
- No automatic promotion to verified, guessed encoding or unsupported writes.
- Backup remains plan-scoped; never promise a full-device backup or universal rollback.
- No new unrelated Goal B features, global refactors, release/tag, deployment,
  container operation, external issue posting or mail sending.
- No subagents or quota checks. Work in this session, sequential packages.

## Acceptance and real verification

1. A device refused before planning shows reason, concrete next action and
   docs/issue/mail links without any download session, in EN and DE.
2. Missing application data, unsupported byte order/procedure/module shape,
   invalid configuration, unknown/null category and transient errors have
   accurate tested handling; use real existing categories, not invented ones.
3. Existing supported untested plans retain their extra acknowledgement and
   server checks; unsupported plans still cannot start, including via direct API.
4. Reporting causes no hardware I/O or automatic external submission; drafts
   and URLs use the correct template/recipient and only approved metadata.
5. Stale selection/revision and late-response tests show the correct target.
6. Tests cover pure mapping, API projections where changed, component behaviour
   and browser clicks for docs/form/mail at pre-plan refusal. Use isolated
   fictional projects/simulator and network restrictions, never live gateways.
7. Exercise real code/builds, review regressions and run repository-required
   scoped checks plus bindings/typechecks if applicable. Test negative safety
   paths and privacy with representative sensitive sentinel data. State exactly
   which checks ran and which environment/evidence gaps remain; mocks are not
   device verification or actual mail delivery.
8. Update the affected contract, guides, implementation status and limitations;
   ADR only where a real architectural/policy decision is introduced. Maintain
   .ai/CURRENT_STATE.md and appropriate .ai/logs entries throughout.

## Delivery and stop condition

The user's invocation authorizes implementation and normal reviewed repository
commits/integration/push at green package boundaries under existing ownership
rules, not hardware or release/deployment/external-publication actions.
Commit as KNXBench <github@knxbench.com>, no Co-Authored-By. Independently review,
verify the merged artifact and exact remote revision before reporting published.
Do not commit other owners' root-local material or private corpus evidence.

Continue S1 through S4 without asking after every package. Stop when the named
acceptance is verified and delivered, or a concrete ownership/environment/source
blocker prevents the remaining package; report partial scope truthfully. Do not
launch another goal automatically. Finish with changes, executed verification,
published commits, remaining device-family candidates and evidence gaps.
