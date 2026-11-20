# ADR 0178: The REL-01 catalogue defines the beta exit

Date: 2026-11-20
Status: Accepted
Session: REL-01 / OWN-01 owner decisions (WebUI main session)

## Context

Until now no document defined "beta". [ADR-0040](0040-programming-requires-release-stage-consent.md)
only makes the programming consent stage-aware (a remembered alpha consent does
not cover beta), and [ADR-0084](0084-first-run-guide-once-per-release-stage.md)
shows the first-run guide once per stage. Without criteria, the step from
`0.1.0-alpha.N` to a beta version would be a label change, not a promise.

The catalogue was proposed on 12 November 2026 in
[REL-01](../open-work/REL-01-beta-exit-criteria.md) as 19 criteria (Must/Should)
with the then-current state of each. The version scheme was already decided on
12 November 2026 (standard SemVer, beta is `0.1.0-beta.N`; the web update check
implements SemVer 2.0.0 §11 precedence with tests over
`alpha.1 → beta → beta.11 → rc.1`). On 20 November 2026 the owner accepted the
catalogue itself.

## Decision

1. **The REL-01 catalogue is the definition of beta.** A KNXBench beta promises
   exactly four things and nothing more (see
   [REL-01](../open-work/REL-01-beta-exit-criteria.md#proposed-definition)):
   data survival with forward migration, a declared scope, no known blocker in
   the declared scope, and written-down limits. A beta is not a stable release,
   not ETS-equivalent and not KNX-certified; KNXBench stays "KNX-compatible".
2. **The 19 criteria B-01 to B-19 stand as accepted.** **Must** blocks the beta;
   **Should** is expected but can be waived only by a recorded owner decision in
   [OWN-01](../open-work/OWN-01-owner-decisions.md).
3. **REL-01 remains the living checklist.** Its per-criterion current-state
   cells are updated as work lands; this ADR records the definition, not a
   snapshot of the states.
4. **The items in "Not required for beta" stay open and visible** but do not
   block the stage change: KNX Secure, `.knxproj` export, multi-installation
   projects, lifting the recovery-gated address writes, full layout fidelity /
   module authority / codecs (provided B-13 holds), Docker image signing (before
   the first stable release), and LS-01 to LS-07.

Owner decisions recorded with this acceptance (20 November 2026):

- **B-01:** the catalogue is accepted; this ADR is the record.
- **B-11:** ETS 5.7 (schema 20) import is **declared supported for beta**, on
  the evidence that schema-20 projects import since 12 October 2026
  (`f23be82cc`, KL-177) from three public ETS 5.7 samples, and unknown schemas
  are refused with a message naming the schema and the supported list
  (11, 20, 21, 23; schema 22 stays refused). Not verified on the #7 reporter's
  project; the supported ETS versions must be listed in the release notes.
- **B-06, B-05/B-08, B-18:** decided in a second owner round on 20 November 2026
  (System-2 ships experimental with its own consent text; the declared list may
  include simulator-evidence-only families; Docker prerelease channel `next`) —
  recorded in [OWN-01](../open-work/OWN-01-owner-decisions.md#decision-recorded--20-november-2026).
  All five REL-01 owner decisions are now recorded.

## Alternatives considered

- **Label-only stage change** (no criteria): rejected — it makes "beta" a
  marketing label without a promise.
- **Fewer promises** (e.g. only data survival): rejected — the four promises
  are the minimum an owner can audit before leaving alpha.
- **More promises** (e.g. field evidence for every declared family as Must):
  rejected — B-08 stays a Should because WP-12 consents are not the project's
  to grant unilaterally.

## Consequences

- The step from alpha to beta is now gated by an auditable checklist; each Must
  cell needs evidence or a recorded waiver before `0.1.0-beta.1` is tagged.
- REL-01's current-state cells must be kept current; stale cells are a defect
  of the checklist, not of the definition.
- The pending decisions B-06, B-05/B-08 and B-18 are listed in OWN-01 and block
  only their own cells, not the catalogue.
- No bus, hardware, release or deployment authority follows from this ADR.
