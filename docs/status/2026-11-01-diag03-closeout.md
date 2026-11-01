# DIAG-03 supplementary sections and quit-layer closeout

Date: 1 November 2026. **Local closeout candidate; package integration and
publication are pending.** Alpha 18 remains immutable. This is source work for a
later release, not deployment, native/hardware certification or raw System-2
export authority.

## What this closes

The unified recorder, bundle and wizard were already published through
`bc77abdb2`, including the real simulator/server/production-Web/ZIP join,
installed-CLI flag boundaries and the original fourteen-case matrix. The
[work dossier](../open-work/DIAG-03-unified-diagnostic-bundle.md) retains those
source-bound results and earlier failures; they are not new executions here.

A fresh completion audit found a missed O1 requirement: session log, bus monitor
and browser state were always unavailable. This change supplies actual reduced,
typed sections, with visible omissions, and fixes the quit confirmation painting
under the report wizard.

| Part | Implemented boundary | Named regression |
| --- | --- | --- |
| Session log | Ordered time/severity, known sources, fixed messages, undo/redo and import counts; paths and arbitrary error/detail text withheld per field, counted; live log untouched | `session_log_leaves_as_typed_entries_with_disclosed_withholding`, `undo_redo_import_counts_and_source_observations_keep_their_known_text` |
| Monitor | Existing buffer only, inclusive ten-minute preview window; exact session/sequence binding carried in review; later traffic cannot substitute contents | `new_traffic_after_review_keeps_text_and_zip_identical`, `a_pinned_window_projects_exactly_its_rows_and_ignores_later_traffic` |
| Monitor privacy | Text/access DPTs, undecoded/conflicting/failed and management payloads withheld at any length; only successful admitted typed group values can carry raw octets | `dpt15_access_codes_from_the_real_decoder_never_leave`, `raw_octets_without_a_typed_non_text_decode_never_leave` |
| Source loss | Replaced/stopped session, eviction, missing/duplicate pinned rows and changed/dropped/unasked binding refuse; empty retained window is distinct from unavailable | `a_replaced_session_or_an_evicted_pinned_row_refuses`, `a_tampered_dropped_or_unasked_monitor_binding_refuses` |
| Browser facts | Closed bounded fields, known identities, reduced errors; unknown names counted without echo; selected-body bytes bind preview/text/ZIP | `client_state_leaves_only_the_closed_schema`, `carries the reviewed bus-monitor window to text and ZIP unchanged` |
| Desktop-close UI | Inert wizard falls below the active modal; safe cancellation preserves wizard/consent; failed save never approves close | `native close under the report wizard asks first and hands back the same wizard` (EN/DE, 1440/400 px) |

The [contract](../contracts/diagnostic-bundle.md#reviewed-monitor-window) defines
`review.monitor`, exact body formats, loss semantics and authentication limits.
The [manual](../manual/user-guide/08-reports-and-diff.md#step-2-contents) describes
what the reduced views do and do not contain.

## Executed focused evidence, before package acceptance

- New section HTTP tests first failed 6/6 against the original unconditional
  withholding. The first whole-section review blocked on one critical and three
  important findings; the corrected candidate passed a second independent review
  with no remaining critical/important finding.
- The correction's server regressions first produced 12 intended failures, then
  the bundle library filter passed 119 tests. Full server library: 796 passed,
  15 ignored. Key-redaction/diagnostic-bundle integration selection: 9 passed.
  These populations overlap; they are not added as a unique total.
- Owning all-target Clippy with `-D warnings` and formatting passed. Four Web
  suites passed 89 tests after the calendar-admission correction; fixture
  TypeScript and the Web build passed. The calendar regression had three genuine
  original-source failures and a passing trailing-newline control before the fix.
- Independent isolated controls passed three baseline cases, then killed a
  DPT-15-export mutant and a fresh-window-instead-of-reviewed-window mutant;
  restored sources matched the reviewed candidate.
- The hardened quit test reproduced the original paint defect in four cases,
  with four clean-project controls still passing. With the fix, all 21
  report-wizard browser cases and its fixture TypeScript project passed. The
  parent inspected the fixed narrow German prompt and restored wide English
  wizard screenshots. This is Chromium plus a test-only Tauri IPC bridge, not a
  real native WebKitGTK/shell or save-as-picker run.
- Documentation and the 14 limitation-catalogue tests passed without changing
  the retained catalogue block. Static security scan matched one synthetic
  credential sentinel in a test fixture, no production assignment.

Full branch/merged package gates, final built-server smoke and remote publication
remain pending. Focused green checks are not relabelled as those results.

## Deliberate limits

- The log is a reduced chronology, not full raw free-text error-chain delivery.
  Arbitrary descriptions/errors remain `null` with omission counts. Other
  operation/import/report sections carry independently admitted detail.
- A monitor without a project or resolved DPT exports transport metadata only,
  not values/octet payloads. Known group values and capped project labels remain
  installation content; there is no comprehensive secret detection.
- Monitor bytes/status/loss counters describe the reviewed interval. Later
  broadcast losses are not included. A selected live session-log change still
  invalidates review, as do other selected-content/project-context changes.
- Review fingerprints bind content; they are not authentication, authorization
  or proof that a human read the preview. There is no upload/mail send or new
  snapshot cache. Collection can wait on the monitor mutex; it never starts a
  bus session.
- System-2 exports its closed projection only. P1/P2 gate a future raw export,
  not this goal. The physical allocation mismatch in issues #30/#22 is not fixed
  or certified by diagnostic export work.
- The achievements dialog's separate elevated-layer pattern, native save-as
  and wider native acceptance were not changed or newly certified here.
