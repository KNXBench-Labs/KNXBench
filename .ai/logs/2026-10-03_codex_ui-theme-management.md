# U17 — accessible management and reversible visual preview

## Startup and ownership

Fresh own checkout/branch `ui-theme-management` from published U16 receipt
3839e3a3. Reservation 0889c102 is handover-only, doc-gated and full-blob remote
readback verified. U16 source/receipt/actual cleanup verified, private input and
root/foreign worktrees unchanged. No productive backend or KNX session.

## Inspected seams and choice

Read actual App.tsx, SettingsPanel.tsx/tests, theme.ts/runtime tests, themePack.ts,
themePackDom.ts and U16 plans. App still supplies the static builtin list;
installed choices are already derivable through getThemeDefinitions. The root
hook is the existing single DOM writer; the pack lease restores its prior inline
values unconditionally, so a competing component-owned preview lease would be
unsafe. Extend the existing root hook with a transient visual candidate, without
changing the stored selection or adding a preference store. Appearance manager
owns user intent and calls U16 conditional plans for durable changes. Do not route
pack management through the hook's ordinary optimistic preference setter.

## Primary-source check

React useEffect reference fetched during this task:
https://react.dev/reference/react/useEffect (Reference/Parameters/Caveats).
It documents cleanup of the old dependency instance before the next setup,
unmount cleanup and the extra development Strict Mode setup/cleanup cycle.
This is an unversioned live reference; this repo pins React19.2.8. A draft's
unverified v19.3 label was removed after fetching/inspecting actual primary text.
No new React API/dependency upgrade is intended: installed-version tests must
prove preview release/reapply and Strict Mode, not infer their success from docs.

## Visual direction

Keep Settings → Appearance and its existing engineering-app typography, theme
roles, spacing, Overlay/focus primitives and semantic form controls. Add a quiet
name/origin/version/status list, labelled Import/Export/Remove and explicit Apply/
Cancel. No new palette, editor, decorative cards, screen or framework. Invalid
entries remain visible as diagnostics/recovery candidates, never silently erased.

## Vertical tracer sequence

1. Existing root runtime renders a transient builtin choice without writes.
2. Cancellation/unmount/System/validated pack release restore current authority.
3. Actual Appearance manager + file selection -> inert draft -> explicit guarded
   installation/selection; ID collision requires context-bound replacement.
4. Failure/conflict/late response and cross-client changes restore latest ack.
5. Recovery/active removal/System reset, localization and real accent capabilities.
6. Keyboard/focus/stacked overlays and local intercepted browser flows; no native
   Orca/WebKitGTK/general WCAG/ETS/Alpha approval inferred.

## Evidence

### Actual merged acceptance and documentation-only integration

Actual chain proc_cdcd42b97d47/PID2816334 completed exit0 on f16f1e40.
All23 commands accepted:17 repository and six selected offline inventory/
execution commands. Web1702/95 files, Chromium72 without failed/skipped/flaky,
ordinary Rust2984/0/165 across149 result blocks, compiled ignored inventory165,
17 equal shadow bindings and697 unchanged code/configuration inputs. Six offline
suites execute27 private cases and one release-matrix case; matrix115 instances/
113 unique products with recursively verified status-only item shape. All420
private originals unchanged and transient corpus link removed. No raw private
diagnostics retained, live bus or native/Orca/general-alpha/ETS approval.

Attempt1 was cancelled to close U17-R1. Attempt2 was killed by agent_close during
workspace compilation; stale RUNNING was reconciled against process/OS evidence,
not called a product failure or acceptance. Run3 had separate fresh directories,
shared leases, frozen HEAD/source and explicit notified lifecycle persistence.
Eleven actual manager browser cases and three added compiled guard controls close
U17-R1. Independent review is not claimed; bounded in-session self-review.

New upstream5ca570a0 adds only Markdown. Conventional integration40642ea2 keeps
all complete upstream/owned histories; independent full-blob expectations and
truncated-suffix controls prove both conflicting implementation prefixes and
handover histories survive. All697 gated code/configuration inputs remain equal.
Updated acceptance/operator/ADR/roadmap/limitations documents; their final gates
and publication/readback are tracked in the handover, not assumed here.

Preliminary U18 audit found IMPORTANT U18-R1: generic root-switch/table/input
fixtures do not establish the explicitly required representative editor/inspector/
dialog/diagnostic/focus/selection/disabled/accent states across palettes. This is
U18 closing work, not a claim that U17 implementation is missing or native AT
has been accepted. U18 remains open. All preceding candidate checkpoints are
historical evidence for their own trees and do not override this actual receipt.

### Final frontend candidate (2026-10-03)

Final Web1,702 in95 files, intercepted Chromium69 (zero failed/skipped/flaky),
TypeScript and production build pass. All262 frontend inputs remained unchanged
during execution and match again on resume.31 actual parent/five root cases and
eight actual manager browser flows cover async/authority/consent, canonical
download/cold reload, conflict/removal/reset, live diagnostics and narrow layout.
Full Web has historical fixture stderr; no warning-free whole-suite claim.
Six behavioral controls plus two cache-propagation controls were caught by named
assertion failures; sources restored byte-exactly. New diagnostic module TS2322
inclusion canary caught/restored, final TypeScript green.

Separate in-session self-review traced the complete runtime delta and new manager
through acknowledged settings, U16 plans and the existing conditional queue.
No open critical/important finding in this bounded U17 candidate; not independent
agent/model approval. Earlier Debug report second-DOM-owner gap is closed by
read-only useSavedThemeId and actual report/root regression. Both action paths
now preserve server success while reporting local cache failure separately.
Managed builtin selection is acknowledged/conditional; standalone legacy callers
retain their ordinary callback. Closing Settings cancels presentation only, never
an already dispatched server write.409 is not replayed; uncertainty uses existing
read reconciliation. Recovery is observed JSON theme scope, not original lexical
bytes or a freshly fetched server-file backup.

Full repository candidate proc_aa97c2d70d72 accepted13/13 commands, explicitly
reusing the already executed/hash-verified final frontend evidence. Rust2940/0/164
over148 result blocks; compiled ignored inventory164,17 equal bindings and693
frozen non-Markdown inputs unchanged. Audit magnitudes:448 graph packages,
414 valid/157 absent headers,376 links/239 Markdown files,334 corpus-lint sources.
Actual-merged acceptance/publication remain pending.
U18 owns extension-wide review and representative component visual acceptance.
Root/foreign trees/private inputs unchanged; no native/Orca/ETS/global-alpha claim.

### Historical intermediate checkpoints

### Acceptance audit finding U17-R1 (closed locally, renewed full gate pending)

IMPORTANT: the eight initial manager browser flows lacked explicit hostile-file
rejection, direct System reset and uncertain HTTP500 rollback. Component cases
and an older runtime fixture did not establish these actual manager gestures.
First actual run proc_7630de2f820c was deliberately cancelled before acceptance,
not relabelled as a product failure. Its five completed prerequisite stages are
retained separately and not substituted for a full actual-merged verdict.

Added three fully intercepted actual-UI browser cases; all11 manager cases pass.
The tests assert no DOM/network/settings effect for hostile data, exact
selection-only System reset with dark/light OS follow-through, and rollback/
one reconciliation read/no write replay for500. All runtime source remains equal
reviewed d07555f0. Three additional controls compile and fail their named browser
assertions; sources restore byte-exactly. Initial file-admission mutant's unused
import caused TS6133; it was restored and rejected as instrumentation failure,
then retried with realistic parser-import removal. No invented production RED.

Candidate d07555f0 conventionally merged with cbc6b0b2 at82fc4e44. Complete
upstream/owned history retained, including historical upstream insertions that
made the simple suffix-to-old-base check inapplicable. Independent expectation
and truncated-suffix negative controls verify preservation. Corrected browser
coverage is a follow-up acceptance delta; renewed full actual gates/publication
are pending. U18 remains open, not folded into these U17 checks.

As of 2026-10-03 05:43 CEST, three root/fourteen parent tracers pass. New:
HTTP409 restores the last acknowledgment and never replays PUT; peer state is
adopted by the existing focus refresh, not invented from a bare409 response.
Active removal requires a safe-focus explicit context-bound question and atomically
sets System while retaining opaque foreign entries. Future and opaque data remain
visible and unchanged; recovery Blob exactly matches the published selectedTheme/
uiThemePacks envelope and is intentionally rejected as invalidContract on import.
Draft helper signature/schema and diagnostic precedence probes were corrected
from the actual helpers; no published parser/store contract changed for a test.

Focused84, TypeScript and full current Web1683 pass. Focused stderr/act warnings
are zero after a failing harness diagnostic: declare act environment, wrap exactly
35 standalone cleanup calls, reimport the language template inside act. The
intentional fallback warning is individually asserted against its actual one-string
call and its spy restored; unexpected console output is never suppressed.
Live translated outcomes/detailed diagnostic vocabulary, builtin/status metadata,
per-accent capability and async/focus/browser/mutation/integration remain open.
U17 is local uncommitted source, not delivered; U18 not started.

As of 2026-10-03 04:55 CEST, local uncommitted source has three root and eleven
actual parent RED→GREEN tracers. New flows after the first checkpoint: late-file
owner generation, authoritative peer-theme cancellation, exact atomic install/
selection Apply, same-ID/version replacement with safe initial focus and old-map
conditional consent, installed metadata/preview/selection-only Apply, exact Blob
download/canonical reimport, selection-only System reset preserving pack map/
accent/density/motion/language, and original selector's imported-ID conditional
route. Latest focused80 and TypeScript pass. Last full Web1677 predates reset/
selector, so it is not final-current acceptance. Actual App now derives installed
choices. Removed inferred origin/support properties from a failed draft after
reading actual ThemeDef: id/name/hasAccentVariations, pack accents.

Remaining: removal, opaque diagnostics/recovery, builtin/status metadata, live
translation and preview accent capability, async/race/failure/Strict Mode/focus/
browser acceptance. Normal builtin preferences remain optimistic/legacy-capable;
explicitly test cancellation against an unacknowledged failed ordinary intent
before asserting current-ACK rendering. U17/U18 open, not delivered.

Local uncommitted evidence as of 2026-10-03 04:15 CEST: three RED→GREEN root
tracers (builtin candidate without persistence, root-owner unmount, admitted
uninstalled palette) and three actual SettingsPanel/parent/root tracers (file
preview without writes, explicit Cancel, Escape unmount releasing overrides).
Focused72, TypeScript and full current Web1671/95 files pass. All original runtime
and Settings tests remain green. ANSI was stripped for receipt count parsing;
original successful commands were not replayed. A close-RED receipt's invented
fixture-ID marker was corrected against its actual `user-blueprint` failure.

App owns ephemeral candidate state; existing theme hook remains the sole writer.
Appearance manager uses U16 file admission and refuses missing acknowledgement;
Apply/list/diagnostics/removal/export/consent/cross-client and late-file guards
still PENDING. The full Web run is a partial-source regression, not acceptance.
Next tracer is a late file result after Settings close; current owner cleanup
does not yet guard that async continuation. U17 not delivered; U18 still open.
