# WP-48 — Recent-change correctness and regression-test audit

Owner request: 21 November 2026. Search changes since approximately 14 November,
including unlabelled contributions, and record actionable defects in one work package.

**Status: closed 22 November 2026 — delivered on main** (slices 48.1–48.8 by PRs #127,
#129–#135, slice 48.9 documentation closeout; see the slice table and the closeout summary
below). The sections from "Requested outcome" to "Verification leads" are the intake text
of 21 November 2026, kept as written with later implementation notes; they describe the
state at audit target `163e38b85`, not today's code. Remaining items live in their owning
items (closeout summary).

## Requested outcome and review boundary

Correct wrong module ownership, caption provenance and editor/last-download behaviour,
then restore regression tests that can actually detect the failures. Reuse the existing
WP-05, WP-07, WP-37, WP-42 and WP-43 implementations; do not introduce parallel engines.
This package owns the corrective backlog, not those packages' entire remaining scope.

Audit target: `163e38b85a23eab3f8c9101638ead7a596db3456` on main, 21 November 2026.
All source coordinates below refer to that revision, not a moving branch.
The chronological inventory starts at `2026-11-14T00:00:00+01:00` (197 non-merge commits).
Model-session subject matches were a selection aid, **not proof of exclusive authorship**:
a commit mentioned by a model can have been implemented or reviewed by somebody else.
The source review concentrated on parameter layout/module ownership, scoped storage/image
building, catalogue/reconciliation and the changed Web UI/tests. This is a bounded audit,
not an exhaustive inspection of every line in the 197 commits or a model leaderboard.

Evidence vocabulary:
- **Executed helper probe:** real production function bodies, or the actual Rust source
  module, exercised with synthetic inputs and minimal surrounding types. This is not a
  full-crate, HTTP, mounted React, browser, device or integration-test result.
- **Static trace:** checked producer and consumer source; the acceptance case below has
  not yet been added to the maintained regression suite or run end to end.
- **Verification lead:** a real uncertainty that must not be promoted to a confirmed bug
  or used to expand write authority without a discriminating test/source decision.

No live installation, provider login/download, ETS VM or hardware operation was used.
At intake (21 November) no runtime finding was fixed by publishing this document; the
slices below fixed them afterwards. The documentation package's own gate is separate from
the runtime acceptance cases below.

## Confirmed findings

### F01 — Module ownership is a substring/first-match guess (P1)

Introduced in the WP-07 authority series (`84884afd6`, `d78983790`, `09a0b06ad`).
Anchors: `crates/knx-productdb/src/device_evaluation.rs:133–181`,
`apps/knx-server/src/com_object_activation.rs:197–229`,
`apps/knx-server/src/parameter_objects.rs:192–215`.

The matcher uses `contains("_<RefId>")` without a right boundary. `MD-2_M-4`
therefore also matches `…_MD-2_M-41_MI-1`. A single candidate is admitted without
checking the program-side counter; multiple candidates with the same counter use
`find`, so order decides. This can associate activation or edit reconciliation with
another imported module instance instead of refusing an unresolved owner.

Executed helper probes observed:
- only an M-4 instance, requested M-41: `Found("1")`, rather than no match;
- only MI-1, requested MI-99: `Found("1")`, rather than refusal;
- M-4 followed by M-41, requested M-41: the M-4 owner is returned;
- duplicate matching instances: the first owner is returned instead of ambiguity.

Acceptance: one shared structural resolver; exact RefId boundaries and counter agreement;
zero, malformed, duplicate and conflicting candidates refuse. Tests must distinguish
M-4/M-41, MI-1/MI-99, duplicate owners and input-order permutations in evaluation,
communication-object activation and parameter-edit admission. A refused edit must leave
IDs, project state and native history untouched. Do not merely replace one substring
check with another unverified ID grammar.

### F02 — Module Buttons read program-scope caption entries (P1)

Commits `8801793e3`, `b89dc07b2`; static trace.
Anchors: `apps/knx-server/src/domain.rs:7491–7501`,
`apps/knx-server/src/parameter_captions.rs:386–395`,
`crates/knx-productdb/src/parameter_layout/button.rs:74–104`.

Button literals bind per definition and node numbers restart per tree. Caption fills
are stored by `(scope.node_chain(), node_id)`, but a literal Button is retrieved with
`(None, node_id)`. A module Button misses its own translation and can borrow an unrelated
program node's translation/template and fill when the node numbers collide.

Acceptance: use the Button's exact scope key throughout. An HTTP panel fixture must
contain a program caption and two module Buttons sharing a node number, different
translations and a Repeat expansion; each must retain its own text/provenance.

### F03 — Literal translations are reported as stored parameter values (P2)

Commit `8801793e3`; static trace.
Anchors: `apps/knx-server/src/parameter_captions.rs:358–368,199–214`,
`apps/knx-web/src/parameterCaption.ts:14–22`,
`apps/knx-web/src/ParameterWorkspace.tsx`,
`apps/knx-web/src/ComObjectTable.tsx:51–64`.

Every admitted placeholder-free label translation creates `TextFillSource::StoredValue`.
That is parameter-substitution provenance, not translation provenance. Blocks, channels,
separators and Buttons consequently carry a stored-value attribution without any stored
parameter having supplied their text. `translated_text()` also abuses the fill's template
as a separate translation cache.

Acceptance: keep resolved translations separate from positional fills. Plain translations
must not claim a stored-value fill; real stored/default/empty/unresolved substitutions
must preserve their actual origin, including Button consumers. Cover wire DTO and the
user-visible source description, not just the label string.

### F04 — The configuration editor cannot load an existing document (P2)

Commit `df896afe1`; static trace.
Anchors: `apps/knx-server/src/devconf_routes.rs:85–102`,
`apps/knx-web/src/features/devconf/client.ts:3–7`,
`apps/knx-web/src/DeviceConfigPanel.tsx:42–77,178–180`.

The read API exposes module identity and document length, but no document bytes.
`reload()` fills only identity inputs; the raw editor starts empty and is reset to an
empty string. Reopening the editor cannot show or edit the stored JSON, despite comments
claiming synchronization. A replacement must be retyped without seeing the prior content.

Acceptance: a bounded, identity-bound read of the stored document through the existing
profile API; save, switch away/back and reopen the native project, then edit the original
content without re-entering it. Preserve unknown JSON members and undo/redo behaviour.
Do not expose every device's full document through an unbounded installation-wide list.

### F05 — Configuration editor state does not refresh with the project revision (P2)

Commit `df896afe1`; static trace.
Anchors: `apps/knx-web/src/DeviceConfigPanel.tsx:25–40,42–77,81–100`,
`apps/knx-web/src/Inspector.tsx:1150`, `apps/knx-web/src/App.tsx:1792`.

The load effect depends on device id, not server/project/snapshot identity; `loadedKey`
contains only module SHA and byte length. While the same selected device remains mounted,
undo or another revision update cannot refresh its stored profile. Equal-length content
changes are indistinguishable even after a reload. `preview()` meanwhile uses the newer
tree context. A parent remount protects some switches, but does not fix same-device
revision refresh; cross-project draft leakage is a case to verify, not a reproduced claim.

Acceptance: refresh/undo for the same device must load the appropriate stored state under
an explicit dirty-draft policy. Delayed GET/preview responses must never cross context
boundaries; test equal-length documents and equal numeric device IDs in different projects,
including the actual parent remount path.

### F06 — The server accepts non-JSON configuration documents (P2)

Commits `df896afe1`, `af51c2983`; executed constructor probe plus static route trace.
Anchors: `crates/knx-core/src/devconf.rs:40,114–148`,
`crates/knx-app/src/devconf_edit.rs:60–83`,
`apps/knx-server/src/devconf_routes.rs:44–55`,
`docs/contracts/device-configuration-modules.md:26–30`.

`DeviceConfigProfile::new(..., b"not json".to_vec())` returns success. The route decodes
base64 and delegates to that constructor; only the browser calls `JSON.parse`.
The retained field is documented as strict JSON, but direct clients can bypass that
contract. Module identity syntax is not proof that a module package or its rules were
validated; do not conflate those different validation levels.

Acceptance: reject malformed/non-UTF-8/duplicate-key or otherwise disallowed JSON at the
appropriate application admission boundary, under explicit strictness/size/depth rules;
prove unchanged state/history on refusal. Preserve opaque historical bytes rather than
silently discarding them during load. Define/report module validation separately; this
finding does not authorize hard-coded modules or private device packages.

**Open (owner/contract decision):** the strict document parser refuses the JSON-grammar
integer `-0` ("numbers must be integers without fraction or exponent"; serde_json reads it
as a float). This is pre-existing engine behaviour, left unchanged in WP-48.4; pinned by
knx-app `negative_zero_is_refused_as_a_non_integer_number`. Other negative integers work.

**WP-48.4 review fixes:** the editor adopts a new revision silently when its stored
content equals the draft (own apply/remove no longer raises the stale alert); an edit
during an in-flight preview discards the answer (draft-version check) instead of showing
a confirmation for text no longer displayed; HTTP save-as/open tests prove byte-identical
documents, the non-UTF-8 historical path and `projectIncarnation` binding; a Playwright
fixture spec (`device-config-panel.e2e.ts`, default lane) covers the panel in a real DOM.
Review MINORs left as is: unstyled `device-config-*` classes, redundant device-id check.

### F07 — Last-download address marks have no user-accessible enable control (P2)

Commit `b40ab254a`; static trace and production/test reference inventory.
Anchors: `apps/knx-web/src/deviceLastDownloads.ts:13–18,77–100`.

The feature requires setting `gaLastDownloadMarks` to the string `"1"`; absence means off.
There is no production setter/settings control. Tests inject the key themselves. The
feature described as delivered in WP-37 therefore stays disabled in ordinary use.

Acceptance: a visible translated preference, or an explicitly agreed default, reachable
without console/settings-file edits. Test a fresh profile and toggling on/off through the
actual UI; off must still avoid the optional detail requests. Do not hide integration
behind a test-only setting merely to keep older test fixtures unchanged.

### F08 — Partial last-download coverage can announce an all-clear (P2)

Commit `b40ab254a`; executed extracted summary logic.
Anchors: `apps/knx-web/src/GroupAddressLastDownload.tsx:21–43`,
`apps/knx-web/src/deviceLastDownloads.ts:103–109`,
`apps/knx-web/src/messages/en.ts:745`.

One usable device is enough for `compared=true`. With one usable/no-difference device
and one `absent` device the only summary key is `gaWorkspace.lastDownload.noneDiffer`
("No address's links differ from the last download."); mixed-state coverage is omitted.
Loading/failed/unread devices are absent from `current` altogether. Unknown is not clean.

Acceptance: compare against the complete relevant device population; distinguish loading,
failed, absent, other-program and current-unreadable states. Say which subset was compared;
never issue an installation-wide all-clear for partial coverage. Exercise both delayed
responses and mixed states, without counting repeated presentations as more devices.

### F09 — Bus-monitor timestamps are sorted as natural-language strings (P2)

Commit `72eaa7eca`; executed extracted production comparator.
Anchors: `apps/knx-web/src/BusMonitorPanel.tsx:57–60,850–856`.

The time column returns the RFC3339 string and uses numeric `localeCompare`. For the
same second, `.100+00:00` sorts before `.099999+00:00`, although it is later. The probe
returned comparator sign -1 while chronological order requires +1.

Acceptance: order timestamps chronologically with a deliberate sub-millisecond precision
policy and stable sequence tie-break. Cover variable fractional widths, no fraction,
equivalent timezone offsets, ascending/descending and stable equality. Do not truncate
higher precision accidentally when replacing the comparator.

Implementation note (WP-48.6): `timestampOrder.ts` compares RFC 3339 instants as whole
UTC seconds plus exact fraction digits (no precision ceiling; width and trailing zeros
insignificant; unparsable text is greatest); `monitorRowOrder.ts` types the monitor cells
and breaks every tie by ascending `seq` in both directions. The same comparator now
orders session-log rows (`logRows.ts`, which parsed only to milliseconds; an unparsable
time stays above dated entries as before). Project versions (`projectHistory.ts`) keep
`Date.parse` plus id: the server stores project-history timestamps at millisecond
precision (`SecondsFormat::Millis` in `crates/knx-store/src/project_history.rs`), which
`Date.parse` represents exactly, and the file stays out of the companion import graph.
Unparsable timestamps are the greatest value: last ascending, first descending (pinned
in `monitorRowOrder.test.ts`).

### F10 — Native select marquee moves the control rather than revealing its text (P2)

Commit `ebe7b35ee`; static component/CSS trace, no browser rendering claim.
Anchors: `apps/knx-web/src/CompactSelect.tsx:17–22,54–69`,
`apps/knx-web/src/styles.css:5638–5665`.

The animation translates the entire native select (border and arrow included); the
wrapper does not clip it, and moving a native select cannot reveal text already clipped
inside that select. The component also removes `onMouseLeave` from incoming props without
calling it. Comments promise mount measurement, but no such effect exists.

Acceptance: keep the control's hit box/arrow stationary; make the complete selected text
available without shifting neighbouring controls. Verify narrow rendered geometry,
keyboard focus, reduced-motion/off and forwarded handlers. Measure changed values/layout
at an appropriate lifecycle boundary; do not settle this with CSS-source regexes alone.

### F11 — UDP fallback traps can listen on the wrong endpoint (P2)

Commit `bdba7d0bd`; static trace of the test harness, not a live-bus test.
Anchors: `apps/knx-cli/tests/it/cli_bus_tcp/diagnostics.rs:208–223`,
`apps/knx-cli/tests/it/cli_bus_tcp/find_serial.rs:117–130`;
the same retry/fallback pattern also occurs in `scan.rs`.

After 64 failed binds to the selected TCP gateway address, the trap binds a random UDP
port, while the CLI still receives the original address. `no_udp(&trap)` then observes
a different endpoint and cannot detect fallback traffic to the gateway. Retry repetition
and a TIME_WAIT comment do not repair that loss of test coverage.

Acceptance: reserve a matching TCP/UDP address pair together, retrying selection of the
pair rather than moving only the observer. Assert endpoint equality and prove a deliberate
UDP-fallback mutant fails the tests. Retain the strict no-fallback contract.

Implementation note (WP-48.7): every CLI TCP-gateway or refused-TCP site with a UDP trap
(`scan.rs`, `find_serial.rs`, `diagnostics.rs`, `cli_bus_tcp.rs::refused_tcp_with_udp_trap`)
now goes through `knx_testsupport::{tcp_listener_with_udp_trap, with_udp_trap}`; the refused
variants hold a bound, non-listening `TcpSocket` for the whole run. The random-UDP-port
fallback is gone, and the helpers assert `trap.local_addr() == gateway address`. Proof: a
mutant in `KnxNetIpClient::connect_tunnel` that retried over UDP after a failed TCP connect
failed 11 `cli_bus_tcp` tests covering diagnostics, compare, find-serial, download, restore,
scan, service-control, probe and System 2 (log retained in the slice scratch); restored
byte-identical.

### F12 — Narrow-settings tests accept contradictory states (P2)

Commit `f97b75914`; static test trace.
Anchors: `apps/knx-web/e2e/settings-workspace.e2e.ts:140–147,200–207`,
`apps/knx-web/e2e/devices-navigation.e2e.ts:979–989`.

One assertion reads `aria-expanded` and immediately compares the element to that same
value. Another checks modal blocking only if the settings button happens to be visible,
after swallowing a failed attempt to open the pane. Both can pass without establishing
the layout/modal contract the test names.

Acceptance: explicit setup and one expected result for each viewport/context; required
controls must be present before the modal-blocking assertion. A mutation that breaks
collapse or removes blocking must fail a maintained browser test. Do not blanket-remove
all conditional setup: distinguish legitimate viewport preparation from vacuous checks.

Implementation note (WP-48.7): `settings-workspace.e2e.ts` now states the single contract
`openSettings()` implements (collapse the navigation pane at <= 650 px, leave it open
above) as explicit `aria-expanded` expectations after each step; the self-comparing
assertion and the "accept either steady state" branches are gone. `devices-navigation.e2e.ts`
requires the pane toggle before the modal assertion: at 1440 px the Settings entry must be
visible and its pointer action blocked; at 400 px the pane must be collapsed, the entry
hidden and the toggle isolated (`[inert]` ancestor, pointer blocked). Mutants: never
collapsing in `openSettings()` failed the four 400 px settings tests; removing modal
blocking (overlay `pointer-events: none` plus no `inert` on siblings) failed all four
retained-comparison tests (the 1440 px ones on the blocking assertion). Removing only one
of the two mechanisms is deliberately not detected: either one blocks the pointer.

### F14 — Scoped-image tests do not distinguish per-instance values (P2)

Commit `e0738472a`; static test/producer trace.
Anchors: `crates/knx-productdb/tests/it/module_scoped_image.rs:67–95`,
`crates/knx-productdb/src/image.rs:760–766`.

Both instances are fed the same unscoped `MP_R` value through the production fallback.
The tests check different base offsets, not distinct scoped values or MI ownership.
They cannot establish the advertised per-instance value isolation.

Acceptance: two real scoped keys, different values, shared local reference, MI counter
and RefId collision negative cases; assert exact written octets and unchanged neighbours.
Separately decide/test when an unscoped fallback is allowed. This finding is a test gap,
not proof that every module image is wrong or that hardware behaviour was verified.

Implementation note (WP-48.7), policy and finding: `ImageRequest.values` reaches the image
as an unscoped map (`activate_values` -> `resolve_values`); the scoped half of the
`ValueMap` is filled only by `device_evaluation::evaluate_device`, which the image path never
calls. A scoped value read in `build_image_selected` therefore falls back (via
`ValueMap::get`, then `get_unscoped`) to the one unscoped value, which every instantiation
shares at its own `BaseOffset`; module-qualified request keys are refused by name
(`no such ParameterRef`). The `ImageRequest.values` doc comment claimed such keys were
decomposed and consumed per instance (ADR-0189, then numbered 0177); it was wrong and is corrected. Per-instance
image values are not implemented: that is a producer gap, recorded here, not changed in
this slice, and no hardware claim. Tests in `module_scoped_image.rs` pin the policy (exact
octets, unchanged neighbours, default and shared unscoped value), the refusal of scoped
keys, and the evaluator-side per-instance ownership (two scoped keys with different values
and one shared local reference, MI counter mismatch, duplicate-owner collision, sibling
never read). Mutants: removing the unscoped fallback (both `ValueMap::get` and the image
`get_unscoped`) failed the policy tests; dropping the owner/counter guard in
`evaluate_device` failed the counter/collision test. Removing only the image-side
`get_unscoped` survives because `ValueMap::get` already falls back.

### F15 — Decision/status references conflict (P3)

Static documentation/source inventory.
Anchors: `docs/adr/0177-accepted-wp07-mapping-is-the-module-instance-authority.md:4,40–45`,
`docs/adr/0177-grid-axis-text-parameter-captions.md:1`,
`docs/OPEN_WORK.md:10,19,30` at the audit target. (The authority ADR is now
`docs/adr/0189-accepted-wp07-mapping-is-the-module-instance-authority.md`, WP-48.9.)

Two different ADRs use 0177; the authority ADR is still Proposed and specifies an
`ends_with` rule/shared resolver while the implementation uses duplicated `contains`
matchers. WP-42's queue still says storage/editor not started despite their source;
LS-02 says catalogue delivery pending despite `c079ec8d6` being on main. These are
contradictions, not evidence that the incomplete features are fully accepted.

Acceptance: unique ADR identity and explicit dated amendments, backlink sweep, queue/detail
and maintained status reconciliation against actual delivered commits and remaining gaps.
Do not rewrite historical gate reports or turn the above bugs into completed features.

Implementation note (WP-48.2 and WP-48.9): the shared resolver part landed in 48.2
(`knx_core::resolve_module_owner`, PR #129). WP-48.9 resolved the rest:
- **ADR identity.** The authority ADR was created later the same day (10:15 against 07:16)
  and is now [ADR-0189](../adr/0189-accepted-wp07-mapping-is-the-module-instance-authority.md);
  ADR-0177 is the grid-axis caption ADR only. A dated note in ADR-0189 records the former
  number. Source comments, maintained docs and the ADR index were switched (42 textual
  references in 22 files plus the OPEN_WORK link). Dated records (the ledger, status
  records, the retained KL-068 dossier) keep their original "ADR-0177" wording; the note in
  ADR-0189 and the index row say which ADR they mean. No redirect stub: no link to the old
  file name remained outside the audit anchors above.
- **Authority ADR status stays Proposed.** The 17 November owner decision accepted the
  counted mapping witness as evidence and lifted the block; it did not accept the ADR text
  (OWN-01 records no acceptance of it). ADR-0189 carries an amendment saying so and
  describing the implemented `_<RefId>` boundary/counter rule.
- **Queue/status.** The WP-42 and LS-02 rows were stale restorations (a stale-base merge,
  `d18687ba1`, re-added rows that `c7e61f09a` had removed). WP-42.2 storage and WP-42.3
  editor are on main (`df896afe1`, `af51c2983`, merge `3a72996b4`, PR !57); LS-02 is
  delivered as `c079ec8d6` (digest-bound cache availability in 48.8). The LS-02 and
  delivered WP-43 rows are removed from OPEN_WORK; their detail files carry the state. The
  WP-42 row stays with the corrected delivered-scope text that the concurrent
  stored-export track (PR #137) published; its physical download is a separate follow-up.

## Catalogue and reconciliation findings

### F16 — An old CLI report can clear newer programming changes (P1)

Commit `f6b14780f`; independent source review, confirmed by the controller's static trace.
Anchors: `apps/knx-server/src/programming_reconcile.rs:226–258,273–318`.

Reconciliation checks source/program/target, but compares the current plan with the report
only by `(steps, data_octets, group_links)`. Those dimensions are not content identity.
After a fixed-width parameter edit, or replacing a link while preserving the link count,
an old successful report can still match the new plan dimensions. Supplying a fresh
snapshot binding satisfies the revision guard without proving that those new bytes were
ever downloaded. `SetDeviceProgramming` then clears the affected pending parts; the
report's parameter values are not compared to the current values.

Acceptance: bind a report to the canonical content of the actual scoped configuration/
write plan, not counts alone. An old report must be refused after a same-width parameter
change and a same-count link replacement, even with a fresh request snapshot binding;
state, marks, baselines and history must remain unchanged. A matching unchanged report
must remain an explicitly supported control. Legacy reports lacking the necessary binding
must fail closed or enter a separately labelled manual assertion flow, never claim proof.
No new device read/write is needed to reproduce these offline admission cases.

### F17 — Same-size catalogue revisions look already downloaded (P2)

Commit `c079ec8d6`; independent source review, confirmed by the controller's static trace.
Anchors: `apps/knx-server/src/online_catalogue/download.rs:116–128`,
`apps/knx-server/src/online_catalogue/service.rs:307–329`,
`apps/knx-web/src/OnlineCatalogue.tsx:595`.

Search determines `downloaded` using entry id, ordinary-file/link metadata and length,
not the current SHA-256. After a manual index update with the same entry id and length
but new content/digest, the UI offers Import for the old cached revision. The import's
fresh hash check correctly refuses those bytes. The existing Verify cached action can
verify/remove a stale copy and acquire the selected revision; the defect is the misleading
availability classification and premature Import offer, not an absent acquisition path.

Acceptance: bind cache availability to the verified content identity (e.g. retained
verified digest metadata or a digest-addressed cache), without hashing every large file
on every search page. Same-id/same-size/different-digest updates must be unavailable
until the new revision is acquired; the current matching cache must stay reusable.
Keep import-time size/hash checks. This is not an unverified-byte installation finding:
modern streaming imports check the digest, and legacy imports rehash the cached file.

Implementation note (WP-48.8): a verified download atomically writes `downloads/<id>.sha256`
next to the cached file (same symlink/hardlink/size-bound open as the file); search reports
`downloaded` only when file metadata and that record match the current index digest, with no
hashing. Legacy files without a record report not-downloaded until the download action
rehashes and adopts them (no network) or removes them; verification failure and eviction
remove both files. Import-time checks are unchanged.

## Verification leads — dispositions (22 November 2026)

None of these was promoted to a runtime defect without evidence. Original lead text is
summarised; the disposition is what holds now.

1. **Module image authority — resolved (WP-48.7 review).** The image builder admits
   base-offset module images (`b4d141288`/`e0738472a`): a module-local `Memory/@BaseOffset`
   resolves per instantiation. It refuses, fail-closed, devices whose instances carry
   stored module-qualified parameter values (`no such ParameterRef`, `ParameterValue`
   category) and still refuses a module `ComObjectRef` (`a module instance`). "Module images
   refuse wholesale" and "module images are admitted wholesale" are both wrong; stale
   statements were corrected in WP-48.9 (WP-37 detail file, IMPLEMENTATION_STATUS, REL-01,
   one dated ledger entry annotated). Owner: [WP-07](WP-07-module-instance-authority.md)
   (per-instance image values), [KL-068](../limitations/KL-068-repeated-module-activation.md).
2. **`CollapseIfEmpty` value domain — open, owner [WP-05](WP-05-parameter-layout-fidelity.md).**
   The repository holds no primary evidence for the attribute's value type: the research
   note cites the public schema-20 grammar for the attribute's existence and the 762
   declaring rows only, and the PSR rules document is not in the corpus. The shipped rule
   is documented in the parameter-workspace contract as truthiness of a non-empty string
   (`"false"` and `"0"` therefore collapse too); it is not claimed as ETS behaviour.
   Lift condition: a primary source, or an authentic project with an observed rendering,
   fixing the value domain; until then no boolean parsing is introduced.
3. **Rename scopes and axis fallback — dispositioned in WP-48.3** (rename scope and fill
   metadata fixed; axis fallback unchanged by decision; Button translation versus text
   parameter open). Open owner questions moved to WP-05 and OWN-01 (below).
4. **Historical catalogue rows — partly confirmed in WP-48.3.** Open: an authentic
   pre-change database to decide backfill versus a re-import notice. Moved to WP-05 and
   OWN-01.
5. **Other review leads — open, not confirmed defects.** Moved to their owners so they
   survive this closure: per-definition axis-query budget reuse → WP-05; last-download
   request fan-out → [WP-37](WP-37-pending-changes-across-ui.md); all-hidden table
   preferences and hidden selection controls → [OWN-01](OWN-01-owner-decisions.md)
   (no active UI owner exists; lift condition: a targeted consumer/keyboard check in a
   future UI package). The catalogue's search-field keyboard navigation stands; no claim of
   complete keyboard inaccessibility.
6. **Nested-binding refusal causes — not a defect** (WP-48.3); cause-specific reporting is
   an enhancement.

## WP-48.3 result (scoped captions and provenance)

Delivered in WP-48.3 (PR #130, merge `00a9338d0`).

- **F02 fixed.** A literal Button's caption/translation/fill is looked up with its exact
  `(scope.node_chain(), node_id)` key. HTTP fixture `passive_labels/module_buttons.rs`: a
  program Button and two module definitions whose Buttons share node 7 with it, one definition
  instantiated twice, each with its own translation; each keeps its own text, and an
  untranslated module Button no longer borrows the program Button's translation (both tests
  fail on the previous code).
  This is the scope-collision case (two module instances of one definition, distinct scope
  chains), not a `Repeat` expansion: `Repeat` is a dynamic container whose content is not
  expanded into scopes, so the spec's "Repeat expansion" acceptance item stays **remaining**
  (no Repeat-scoped Button coverage here).
- **F03 fixed.** Resolved translations are kept in a separate map from the positional fills
  (`CaptionFills::translations`). A plain translation sends no `textFill`; real
  stored/default/empty/unresolved substitutions keep their origin, a translated placeholder
  label's Button consumer now shows the filled text (it showed the unfilled template before).
  The wire DTO is unchanged, so no bindings regeneration; web consumers already render a fill
  only when present (tests: `parameterCaption.test.ts`, `ParameterWorkspace.test.tsx`).
- **Lead 3, rename scope: confirmed defect, fixed.** The overlay keyed accepted renames by
  `(module_def_id, target node)`, so an instance whose own branch was inactive adopted a sibling
  instance's replacement text. Now keyed by the rename's module scope chain; test
  `a_rename_active_in_one_module_instance_does_not_retitle_the_other`. The evaluator's
  settlement stays per tree (documented in the rename contract): two instances of one definition
  that both activate a rename for the same target are refused together with a diagnostic. That
  is a conservative refusal, not a wrong display; left unchanged.
- **Lead 3, rename fill metadata: confirmed, fixed.** A rename filled from its
  `@TextParameterRefId` dropped its fill; it now carries the real origin
  (`StoredValue`/`EmptyValue`/...), asserted in `a_rename_leaf_fills_its_placeholder_from_its_own_text_parameter`.
- **Lead 3, Button translation vs. text parameter: pre-existing, not changed.** A Button
  with `@TextParameterRefId` and a placeholder in its source `Text` takes the fill branch
  (`domain.rs`) and ignores a requested-language translation of that Button. Unchanged in
  this slice; whether translated templates should apply there is an open decision. (Covered
  instead: a translated placeholder Button without that branch shows the filled text, test
  `a_translated_placeholder_button_shows_the_filled_text_and_keeps_its_fill`.)
- **Lead 3, axis fallback: not changed.** `copy_axis_text` keeps the literal axis text and still
  reports the unresolved/empty fill so the reason for not using the parameter-driven caption is
  visible; ADR-0177's pinned tests require exactly this for unresolved references. Still open
  (owner wording decision): for `EmptyValue`/`TemplateDefault` the tooltip says the product
  text's default or emptiness applies while the literal is shown.
- **Lead 4, historical rows: partly confirmed.** Button literal labels still bind from the
  retained source without re-import. Only the requested-language overlay needs the persisted
  `element_id` (rows ingested before 19 Nov 2026 lack it): the label stays literal, nothing
  breaks (test `a_historical_button_row_without_element_id_keeps_its_literal_label`, rows
  simulated by clearing `element_id`, not an authentic old database). Whether to backfill or
  show a re-import notice needs an authentic pre-change database; still open.
- **Lead 6, nested-binding reporting: not a defect.** The generic `unshownContent` issue
  (`parameter_layout/grid.rs`, `parameter_grid.rs`) is reported for reached candidates;
  cause-specific reporting stays an enhancement. No change.

## Bounded corrective packages and actual status

All nine slices are delivered on `origin/main`. Each of 48.1–48.8 had an independent
review and a green change-scoped package gate before its merge; per-slice evidence is in
the implementation notes above and in the PR. Merge commits are the first-parent merges.

| Slice | User outcome / findings | PR | Merge commit |
| --- | --- | --- | --- |
| WP-48.1 | Old reports cannot clear new programming changes; F16 | #127 | `705d35abe` |
| WP-48.2 | Structural module ownership (shared `knx_core::resolve_module_owner`); F01 and the resolver part of F15 | #129 | `098f13de8` |
| WP-48.3 | Correct scoped captions and provenance; F02–F03; rename/axis/historical-row/nested-reporting leads dispositioned | #130 | `00a9338d0` |
| WP-48.4 | Configuration editor loads and edits stored documents, tracks revisions, server enforces the document contract; F04–F06 | #131 | `971653dda` |
| WP-48.5 | Usable, honest last-download address marks; F07–F08 | #132 | `54ca43ee8` |
| WP-48.6 | Chronological monitor ordering and stationary readable select controls; F09–F10 | #133 | `0e3e428dc` |
| WP-48.7 | Effective UDP-trap and settings regressions, per-instance image proof; F11–F12, F14 (producer gap recorded, KL-068 / ADR-0189 amendment / WP-07) | #134 | `e3ef3bff9` |
| WP-48.8 | Catalogue cache availability follows the selected digest; F17 | #135 | `9cd7c0d98` |
| WP-48.9 | Status closeout: ADR identity, WP-42/LS-02 status, lead and decision dispositions; rest of F15 | PR #136 | recorded in the merge |

## Closeout summary (22 November 2026)

Sixteen confirmed findings (F01–F12, F14–F17; F13 was demoted at intake):
- **Fixed in runtime code (12):** F01, F02, F03, F04, F05, F06, F07, F08, F09, F10, F16,
  F17.
- **Test/harness proof restored (3):** F11, F12, F14. F14 is a test gap closed with a
  recorded producer gap, not a fixed producer.
- **Documentation/status (1):** F15.
- **Remaining, with owner:**
  - Per-instance image values (devices with module-qualified stored values refuse
    download): [WP-07](WP-07-module-instance-authority.md), [KL-068](../limitations/KL-068-repeated-module-activation.md).
  - Repeat-expansion Button coverage (F02 acceptance item), `CollapseIfEmpty` value
    domain, Button translation versus `@TextParameterRefId`, axis-fallback tooltip
    wording, historical Button rows, axis-query budget: [WP-05](WP-05-parameter-layout-fidelity.md).
  - `-0` in device-configuration documents: [WP-42](WP-42-device-configuration-modules.md).
  - Last-download request fan-out: [WP-37](WP-37-pending-changes-across-ui.md).
  - ADR-0189 owner acceptance, all-hidden table preferences/hidden selection controls,
    and the four owner questions: [OWN-01](OWN-01-owner-decisions.md).
- No hardware, live-bus, ETS VM, provider login or release action was part of WP-48.

Per-slice completion requires regression coverage of the stated acceptance cases,
independent review, the maintained change-scoped package gate, updated owning docs and
verified integration/publication. Carry unknowns and unsupported inputs explicitly;
never weaken tests or safety gates to make the package green. No live bus/hardware,
private firmware work, manufacturer contact, provider login/download, release or deployment
is authorized by this backlog.

## Audit coverage and non-findings

- Four source-review groups were assigned. Three completed static reports; the catalogue/
  reconciliation reviewer was interrupted by a provider limit and transferred to another
  provider. A provider refusal itself is not review evidence or a defect in this repository.
- The large `229286818` formatting commit was checked against its parent: the reviewer
  found formatting/import-order changes, not hidden runtime logic. Schema-22 additions and
  configuration-profile undo/native snapshot plumbing were not reported as broken.
- The #51 address-picker handoff still requires an explicit Send action; no automatic bus
  write or bypass of the existing send guards was established.
- Source-asserting CSS tests are brittle, but are not by themselves a demonstrated runtime
  failure. Co-author trailers violate repository instructions but are historical metadata,
  not a runtime correction target; rewriting published history is not authorized.
- No broad runtime suite, private-corpus campaign, mounted-browser run or hardware test
  was executed for this audit. Helper probes establish only their named bounded cases.
- The replacement catalogue/reconciliation review found F16–F17. Catalogue acquisition
  and import consumers plus reconciliation received focused source review; #52 and PRG-01
  N1/N3/N7 were inspected only within the bounded review. N13's test-only changes,
  exhaustive fixture coverage and independent protocol-source validation remain unreviewed.
- Counts at intake: 16 confirmed source findings (3 P1, 12 P2, 1 P3), plus six explicitly
  unconfirmed verification-lead groups. Four module-authority cases, the invalid-JSON
  constructor, chronological sorting and the partial-summary branch were exercised as
  helper probes; the matrix probe reports observed strings, not established external semantics.
