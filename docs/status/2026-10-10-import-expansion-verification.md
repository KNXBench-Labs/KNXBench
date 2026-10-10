# Import expansion: local verification, 10 October 2026

## Historical delivery record

The owner subsequently authorized commit/merge/push on 10 October 2026.
[Separate integration acceptance](2026-10-10-import-expansion-integration.md) records
the combined source; this original local evidence is not relabelled as a merge run.
The boundary below describes the original delivery, not current source status.

## Delivery and source boundary

This is an **uncommitted, unpublished local implementation**, not an integrated
main revision, release or deployment. Owner authorization excluded publication
and real-bus actions. Worktree: `import-expansion-20261009`; branch:
`feature/import-expansion-20261009`; base:
`b042048b318801acc67927fd55b3dbf61cfa62fd`.

The retained application-input manifest contains 1,169 repository-relative
public source/configuration files. Its SHA-256 commitment is
`37035b087199e360b4cff4726c2d236681a4093feb5c6a2e217a744d3e48193a`
(sorted JSON mapping, compact separators). The closure candidate includes the
read-only recovered-journal serial guard and localized unnamed-installation
display fallback; original names and IDs remain unchanged. Earlier source
receipts are historical, not substituted for this candidate.
The compact [machine-readable receipt](../evidence/import-expansion-local-2026-10-10.json)
records admitted results and artifact commitments. Earlier failed attempts are
not counted as successful gates. Review was **self-review**, not independent.

## Delivered behavior

[ADR-0106](../adr/0106-selective-project-import.md) and
[SELECTIVE_IMPORT](../SELECTIVE_IMPORT.md) define the shared application service,
HTTP routes, CLI and File-menu dialog. Devices/lines are merged into one explicit
target installation in the open project. Required communication objects, links,
parameters, modules and structural dependencies are remapped; existing target
identity/settings are preserved. Conflicts and stale confirmations refuse the
operation rather than silently overwriting data. Compatible existing area/line
labels remain unchanged, with a preview note when source metadata differ.

Complete source archives and the importer's opaque evidence are retained with
source-scoped paths. The GUI requires consent because this can include
unselected private data. Undo removes imported model entities, **not** the retained
source evidence. Selected source attributes use `SelectiveImportRetainedAttribute`;
they are not unscoped target access-key defaults. Serial-number lookup requires
unambiguous device and source identity. The v11 reference closure refuses future
normalized model versions pending a new mapping audit; this is not an ETS-schema
restriction or permission to merge another owner's model changes unexamined.

## Executed acceptance

- Fresh closure pipeline: formatter, warnings-denied Clippy for `knx-app`,
  `knx-store`, `knx-cli`, `knx-server`, their ordinary Rust suites, analyzer tests,
  full frontend suite and production build all exited zero.
- Rust: **1,331 passed, 0 failed, 97 ignored**, across 136 result blocks including
  doctests. Ignored tests are not passed private/hardware coverage.
- Frontend: **2,577 passed, 0 failed, 163 files** from the Vitest JSON report.
- Analyzer: **7 passed**. Separate server/CLI binary builds and focused
  `SelectiveImportButton`/`App` tests plus frontend build exited zero.
- Actual built-server Chromium run: loopback-only network namespace, disposable
  HOME/XDG/project data, public synthetic `.knxproj`, no intercepted import API.
  Source inspection and preview are read-only; consent gates device apply;
  one-step undo/redo works. A whole-line import preserves its dependency closure,
  replaces the redo branch, and survives native Save/Open with exact final
  installation projection and expected history state.
- Independent CLI run against a native backup copy: inspect/preview/apply exit
  zero; preview leaves the native file byte-identical; repeated import refuses
  the existing individual address with exit one and leaves the target unchanged;
  source archive remains byte-identical.
- The real dashboard screenshot was inspected: one device, one group address
  and one communication object are visible, with no modal obscuring the result.
  This historical frame is not a current all-theme/native-accessibility claim.
- Fresh production Chromium proof: four independent startup contexts, English/
  German at 1440/400px; actual source picker/inspection, read-only preview,
  localized unnamed source, consent, HTTP apply and exact one-step Undo pass.
  Eight top/bottom frames inspected: legible wrapping and accessible scrollable
  actions; zero dialog horizontal overflow. Backend notes retain raw English
  diagnostic text even under German chrome. No API interception or physical bus.
- The localized fallback regression is RED2/GREEN7: empty/whitespace installation
  names render an ID-bearing fallback in both languages without changing raw
  names or numeric selection IDs. Journal serial regression separately has a
  valid corrected RED and GREEN1; the first fixture setup refusal is excluded.

### Rejected verifier attempts and browser diagnostics

The first fresh visual observer raced asynchronous onboarding; the second used
an unsuitable accessible-label/option locator. Both timed out before apply and
remain rejected harness attempts. The corrected observer waits for the guide and
reads the actual select state; no application workaround was added.

An earlier line-versus-device comparison wrongly demanded identical allocated
IDs after Undo and a second import. Its failure is retained, not relabelled.
The corrected check explicitly verifies larger newly allocated IDs, remapped
links, equality of non-identity projection, and exact native save/reopen after
that import. The domain's monotonic allocator was not changed to satisfy a test.

The first CLI probe used a nonexistent plural SQLite table name; the second
expected the word `conflict` rather than the actual address-refusal message.
The final fresh-copy probe passes with the real schema and exact refusal.
Neither earlier probe is accepted evidence.

Browser console contains three HTTP diagnostics: startup discovery 502 inside
the isolated namespace, missing favicon 404, and the intentional conflicting
selective-preview 422. This is **not** a zero-console-error claim. No physical
bus evidence follows from the run.

## Separate evidence tracks and remaining prerequisites

[AUTHORIZED_RESTORE_EXPORTS](../AUTHORIZED_RESTORE_EXPORTS.md) describes the
opt-in private regression manifest and descriptor-confined reads. Public tests
exercise manifest authorization, exact baseline requirements, missing-input and
unsafe/symlink-path refusal. **No authorized historical ETS export set was run.**
The ignored real regression must be invoked only with independently reviewed
baselines and authorized exports of one project's distinct revision states.
Direct ETS internal-store/restorepoint interpretation remains excluded.

[cvexc research](../research/cvexc.md) records bounded read-only analysis of three
local inputs/two distinct contents: cvexc11/14, 33 programs each, 202/203 exclusion
declarations. The newer content adds one declaration for one matched program;
observed options do not change. Unknown fields/text are reported by the analyzer.
No primary specification, signature verification, option-default/precedence or
executable compare/download semantics is claimed.

Fresh closure private corpus: **143/143 in31 targets**, no failed target.
All five explicit-root repository gates, documentation and whitespace pass;
fresh xtask checks478 resolved packages,191 ledger rows,495 corpus-guard Rust
files,496 anchors and2039 documentation targets across222 Markdown files.
The source manifest remains byte-identical after final self-review. Earlier
private-oracle/format/frontend/namespace failures remain separate attempts.
These completed gates and retained artifact commitments are recorded in the
receipt. They are not
substitutes for the missing historical exports. Publication/integration,
future-model closure, native WebKitGTK/Orca and real ETS/hardware acceptance are
separate decisions or evidence prerequisites, not performed by this package.
