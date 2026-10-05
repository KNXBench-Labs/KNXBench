# KNXBench goal — UI/UX track (issues, theme packs and telegram flow)

Written 2026-09-28 against `main` at `99e2a6d`. Use this file as the
instruction passed to `/goal` in the **UI session**. That session runs on
**GPT (Codex)**; this file is written to be agent-neutral. Creating the file
does not start a run.

There are now three goal files, and they do not overlap:

| File | Session | Owns |
|---|---|---|
| [`alpha-release-goal.md`](alpha-release-goal.md) (took over from `goal.md`, now [archived](docs/archive/goal.md)) | alpha session | everything else: data integrity, product database, import, docs hygiene, manual, alpha, final review |
| `goal-commission.md` | commissioning session (Claude) | T30 phase 3: device writes and their own programming UI |
| **`goal-ui.md`** (this file) | **UI session (GPT/Codex)** | the user-reported UX/UI issues of `goal.md` §11, the routed alpha follow-up, user-importable theme packs (U14–U18), and session-local telegram flow (U19–U21) |

§5 below is the exact boundary. §6 describes how work crosses between
sessions.

## Where things stand

Row status of the UI-owned source IDs lives in the
[source-ID ledger](docs/status/LEDGER.md) (owner `ui`), per-row evidence in
[UI_ALPHA_READINESS](docs/UI_ALPHA_READINESS.md), deliveries in
[IMPLEMENTATION_STATUS](docs/IMPLEMENTATION_STATUS.md). The status narrative
that stood here (as of 2026-10-02) moved verbatim to
[UI_ALPHA_READINESS](docs/UI_ALPHA_READINESS.md#owner-status-history) on
2026-10-04 (AR14D D5, agreed by the goal-ui owner). The current top of
`.ai/CURRENT_STATE.md` owns the Web-lock/publication state.

**Reconciliation 2026-10-05:** U0–U18 and the UA1–UA8 deliveries are
recorded below; U19 and U20 are delivered. U21's implementation receipt
`fb40a99a` was returned with three findings by AR21; see
[the acceptance review](docs/TELEGRAM_FLOW_VISUALIZATION.md#13-ar21-acceptance-review-alpha-2026-10-05).
The alpha session claimed those corrections under the Web lock on 2026-10-05.
Do not duplicate its active package or treat checked implementation boxes as
final acceptance. Closure still requires the findings to be fixed, the new
receipt reviewed, green integrated gates and a released lock (§4).

---

### Theme-pack extension (user request, 2026-10-02)

**U14–U18 delivered.** U17's management delivery is `4d9073ca`; U18's
closing delivery is `1964fd6b`, with the named evidence in their sections below.
The user requested theme follow-up tasks after
confirming the existing UI can switch built-in themes. U14–U18 add importable,
exportable declarative theme packs; they do not rebuild the existing palette
system. U0–U13 remain complete and the separately authorized alpha follow-up
above retains its own status, dependencies and evidence. This extension is
not an alpha release blocker unless the alpha owner explicitly adopts it.
Updating this goal does not start or resume a run.

Baseline: five built-in palettes plus System in
`apps/knx-web/src/theme.ts`, a selector in `SettingsPanel.tsx`, a versioned
settings record via `settingsStore.ts`, and the token/contrast boundary in
[ADR-0022](docs/adr/0022-theme-token-boundary.md). The built-in token guard is
not yet validation of user-supplied runtime packs. ADR-0022's original
localStorage persistence sentence is historical: the server's settings file
is authoritative and browser storage is only its cache.

## Telegram-flow Alpha addition — user decision 2026-10-04

The user confirmed [the nervous-system contract](docs/TELEGRAM_FLOW_VISUALIZATION.md)
and authorized the responsible Goal sessions to implement it. **This is required
Alpha scope**, unlike the earlier optional theme extension. U19–U21 were added
as open packages on 2026-10-04; their delivery evidence and the current AR21
follow-up are recorded below and in the reconciliation above. Completed U0–U18,
owner follow-up/UA receipts and another session's active work are not reopened
or interrupted. Reconcile current published owner evidence at startup; stale
local theme checkboxes do not overrule its delivery receipts.

The alpha session owns only AR20's read-only participant/event contract and
AR21's integrated acceptance. This UI session owns U19 design/handoff and all
rendering in U20/U21, under the existing Web lock. Commissioning gets no new
K-package. Geometry is session-local, not project/physical coordinates; values
update immediately, not when an illustrative pulse arrives. Approval does not
start a run, take/release a lock, authorize new bus contact or publish a release.

This addition extends the scope/completion/boundary rules below: this goal cannot
close its newly approved scope before U19–U21 have named, integrated evidence.
The alpha owner must adopt the U21 receipt in AR21 before final readiness.

## 0. Scope: what this goal owns

No UI implementation work remains in the original U0–U13 queue. The separately
authorized alpha follow-up is tracked above. The new theme-pack queue is
U14–U18 below. ISSUE-12 reconciles the
documented host-firewall correction and actual unicast-loopback discovery
tests; no wire capture or multicast-loopback proof is claimed. Native
WebKitGTK and real screen-reader checks remain verification gaps, not
permission for a KNX device write. Completed ISSUE-01–13 slices
and the accepted ADR-0038 are evidenced in
[the issue plan](docs/superpowers/plans/2026-09-21-user-reported-issues.md)
and [IMPLEMENTATION_STATUS](docs/IMPLEMENTATION_STATUS.md); they are not
implementation tasks here. Product/import data belongs to `goal.md`, live
programming and its safety gate to `goal-commission.md`.

## 1. Hard rules

1. **Read first.** Read `AGENTS.md` and follow it. `CLAUDE.md` applies
   where it states project facts, even though you are not Claude.
2. **Nothing reaches a KNX device.**
   - No write of any kind to a real device: no group-value send to the real
     bus, no programming, no restart.
   - All UI tests use the simulator, mocks or fakes.
   - Real multicast discovery for ISSUE-12 is read-only. Individual address
     `1.1.220` (an alarm panel) is never read, written or scanned.
   - The gateway accepts exactly one tunnel. Before any live read, make sure
     no other session's monitor, server or CLI holds it.
3. **Data integrity beats convenience.**
   - Save, autosave and bulk creation must be atomic and honestly reported.
   - Product-data fallbacks stay explicit diagnostics; never guess a name,
     DPT, channel or visibility.
   - Undoable project changes go through `Command`/`CommandStack`
     (ADR-0039). A multi-entity change is one batch command.
4. **One path per behaviour.**
   - Preferences go through the versioned settings document
     (`apps/knx-server/src/settings.rs`, `apps/knx-web/src/settingsStore.ts`),
     never directly into browser storage.
   - Existing discovery, filters and Send/Receive links are extended, not
     re-implemented. The issue plan names which ones already exist.
5. **Keyboard and screen reader.** Every new gesture has a keyboard
   equivalent: resize, zoom, drag and drop, filters, contextual help.
6. **Wording.** Claim nothing beyond "KNX-compatible". Claim no ETS
   compatibility and no certification.

## 2. Operating rules

1. **Start of every work cycle.**
   - `git fetch`, then read the top of `.ai/CURRENT_STATE.md`. The newest
     entry is first, and the web lock lives there (§3).
   - Then read this file and the issue plan's section for the next package.
     For U14–U18 use their checklists and the contract recorded by U14.
     Reconcile already-authorized alpha work first; do not interrupt another
     session's active package or infer permission to resume from this plan.
2. **Isolation.**
   - One worktree per package: `/mnt/daten-i/Sourcecode/KNXBench.worktrees/ui-<topic>`
     on branch `ui-<topic>`, created from the current `origin/main`.
   - Never work in the root checkout `/mnt/daten-i/Sourcecode/KNXBench`, which
     belongs to the goal.md session.
   - A fresh worktree has no `node_modules`: run `npm ci` in
     `apps/knx-web` first.
   - Corpus-backed tests (ISSUE-07/08) need the corpus. Link it:
     `ln -s /mnt/daten-i/Sourcecode/KNXBench/OriginalData OriginalData` in
     the worktree root (the link is gitignored), and set
     `KNXBENCH_PRODUCT_CORPUS` to
     `/mnt/daten-i/Sourcecode/KNXBench/OriginalData/ProductDatabases`.
     Without it those tests take their skip path, which proves nothing.
3. **Handover (the `.ai/` protocol in `AGENTS.md`/`CLAUDE.md`).**
   - Put a new entry at the **top** of `.ai/CURRENT_STATE.md` in your branch:
     `**Last Agent:** Codex (UI session)`, a timestamp from `date`, then
     Completed, Pending/Next Steps, and Notes for Codex or Claude.
   - Never delete or rewrite other sessions' entries.
   - Write a log for each package: `.ai/logs/YYYY-MM-DD_codex_ui-<topic>.md`.
   - `.ai/` is gitignored, although `.ai/CURRENT_STATE.md` and the logs are
     tracked. Stage them with `git add -f`. A plain `git add` refuses them,
     and the commit silently goes without them.
4. **Merging (you merge your own branch).**
   - Immediately before the merge, `git fetch` and rebase onto
     `origin/main`, or merge `origin/main` in.
   - For conflicts in `.ai/CURRENT_STATE.md`, `docs/IMPLEMENTATION_STATUS.md`
     and `docs/KNOWN_LIMITATIONS.md`, keep both sides. CURRENT_STATE entries
     go in chronological order.
   - A new KNOWN_LIMITATIONS number is the next free one at merge time.
   - Push after each reviewed, merged and fully green package. Then delete
     your worktree, your branch, and your scratch and build directories.
5. **Gates (run on the branch, then again on the merged result).**
   - Only one workspace `cargo` gate at a time on this machine, across all
     three sessions. Before `cargo test --workspace` or `cargo clippy`,
     check `pgrep -af cargo`. If another session's gate runs, wait.
   - Use a build directory of your own:
     `CARGO_TARGET_DIR=/mnt/daten-i/Sourcecode/KNXBench.worktrees/.target-ui`.
     Delete it when the track ends.
   - **Required gates:**
     - `cargo fmt --all --check`;
     - `cargo clippy --workspace --all-targets -- -D warnings`;
     - `cargo test --workspace --no-fail-fast`;
     - `cargo run -p xtask -- check-layering`, `check-headers`,
       `check-anchors` and `check-corpus-gates`;
     - `git diff --check`;
     - in `apps/knx-web`: `npx tsc --noEmit` and `npx vitest run`.
   - Judge a gate by its exit status **and** the amount of work in its log
     (tests counted, crates compiled). A wrapper's exit 0 alone is not
     evidence.
   - Every new file needs the licence header (`check-headers` has zero
     slack).
6. **Evidence.**
   - Write a failing test first (RED), then the fix.
   - Check every new guard or validation once by mutation: remove it, and
     the test must fail.
   - Tick the issue plan's checkboxes only with the covering test named.
7. **Review.** No subagents.
   - Before each merge, review your own full branch diff against the issue
     plan's *Review Focus* and `AGENTS.md`, in a separate pass after the
     implementation.
   - U13 records the completed original closing review; U18 closes the new
     theme-pack extension. Preserve review provenance and do not describe
     self-review as independent approval.
8. **Commits.**
   - Author: `KNXBench <github@knxbench.com>`, for example with
     `git -c user.name="KNXBench" -c user.email=github@knxbench.com commit …`.
   - **Never** add a `Co-Authored-By` trailer or any other co-author line.
   - Messages are concise and describe the change; a little humour is
     welcome.
   - Push normally. Never wait on GitHub Actions.
9. **Quota.** The user monitors quota directly; do not stop at a package
   boundary to ask for a quota reading. A user-requested pause still holds
   until their explicit "go".
10. **Scratch** goes under
    `~/.hermes/profiles/knxbench/cache/scratch/ui/` (or your own
    tool's scratch). Never touch `scratch/iaw/` (commissioning) or other
    sessions' files.
11. **Do not touch:**
    - `docs/paperclip-shutdown/` (untracked, foreign);
    - `ideas.md` (gitignored, never commit it unchecked);
    - worktrees and branches named `iaw-*`;
    - the root checkout.

## 3. The web lock (all sessions)

`apps/knx-web` is shared: `App.tsx`, `styles.css` and the message catalogues
collide on every task. Only one session edits it at a time.

- **Take it.**
  1. `git fetch` and read the top entries of `.ai/CURRENT_STATE.md`. The
     lock is free if the newest lock line says *released*, or if there is no
     lock line at all.
  2. Commit a handover-only entry **directly to `main`** and push it at
     once. Its first line after the header is
     `Web lock: taken by <session> for <package>`.
- **Release it.** The same line with `released`, in the entry that
  announces your merge.
- **Hold it for one package at a time.** Release it between packages, so
  that the commissioning session (K5/K6) can get in.
- **Current holder:** read the newest entry of `.ai/CURRENT_STATE.md` before
  touching `apps/knx-web`. A holder written in this plan is stale as soon as
  another package merges. U13's read-only review needs no Web lock.

---

## 3a. Work packages, in this order

Every package ends with: tests and mutation check, gates, docs, merge, push,
handover, cleanup. **[web]** means the package needs the web lock.

### Completed packages (U0–U12)

U0–U9, U10's host/port UI, U11's catalog/device editor and the bounded U12
surfaces are delivered;
the acceptance tests are in the
[issue plan](docs/superpowers/plans/2026-09-21-user-reported-issues.md) and
[implementation log](docs/IMPLEMENTATION_STATUS.md). The gateway's reply
was initially blocked by the host firewall; after the user's rule change,
CLI and HTTP discovery succeeded (RESEARCH §20.1). Native WebKitGTK Search
remains unverified. Do not redispatch the completed packages.

### U12 — Delivered within verified boundaries [web]

ISSUE-05 structure editing, ISSUE-08 grouping, §146 labels, §147 monitor
control, read-only **Device checks**, ADR-0051 Debug property UI and
ADR-0038 Site/Property actions are published. `GET /api/device-readiness`
is offline; `POST /api/device-compare` uses a confirmed read-only tunnel
and does not prove a later write. The Debug server gate defaults off and
records only the original property octets; it cannot restore an entire
device. The K6 address tab now checks the independent server recovery
precondition and fails closed before consent when unavailable (ADR-0059).
No new hardware run or full ETS site/project evidence was asserted by U12.
The global activity/status bar, partial-scope selector and K13 reset UI
remain commissioning follow-ups with their own safe contracts and Web lock,
not unfinished U12 checkboxes. See the implementation log and commissioning
handover.

### U13 — Close the UI track

**Done, 2026-10-01.** The operator explicitly accepted the independent
GPT-6.1-Sol report instead of unavailable Claude. The original report remains
`.ai/logs/2026-10-01_codex_ui-u13-gpt-review-received.md`, with its true
**changes required** verdict and independent provenance; it is not relabelled
as Claude approval. The historical refused Claude attempt is preserved in
`.ai/logs/2026-10-01_codex_ui-u13-review-blocked.md`.

- Device changes clear old detail; both editor surfaces reject mismatched IDs.
- Undo/Redo invalidates parameter GETs through the accepted project snapshot;
  older overlapping responses remain rejected.
- Completed saves cannot re-arm an obsolete autosave cycle after disabling,
  unmounting or cadence changes. Failures remain visible.
- ISSUE-12's host-firewall correction was accepted by the independent review.
  Real offline UDP loopback tests now exercise the shared transport and cover
  HPAI, filtering, duplicates, multiple reply sources, mapping and no-reply
  deadline. RESEARCH §20.1 and limitation §79 retain multicast/native boundaries.
- All §2.5 gates were rerun after integration of concurrent commissioning
  backup-safety changes. Web 1,312 tests, Rust 2,820 passed / zero failures /
  161 ignored / zero `SKIP:`; all six Chromium mock suites pass (30 tests).
- The implementation session's separate full-diff pass is self-review, not a
  newly claimed independent verdict. Each new identity/lifetime guard failed
  its mutation check and was restored before final GREEN.

Final evidence: `.ai/logs/2026-10-01_codex_ui-u13-fixes.md`. The final
handover releases this package's Web lock and transfers only the existing
native verification/global acceptance boundaries to their owners (§6).

### U14 — Specify a safe, versioned theme-pack contract

**Done, 2026-10-02 (contract only).** Inspected the published alpha-owner
receipt and current theme/settings/language code, retrieved primary evidence
and resolved the bounded v1 format, persistence/preview safety and negative
fixture contracts. [THEME_PACKS](docs/THEME_PACKS.md),
[ADR-0060](docs/adr/0060-versioned-declarative-theme-packs.md) and the U14 log
record actual document/research evidence. U15–U18 remain implementation work.

- [x] Inspect `theme.ts`, `themeTokens.ts`, the stylesheet/bootstrap,
  `appearance.ts`, settings storage/routes and the language-pack lifecycle;
  reuse applicable mechanisms without treating language and theme semantics
  as identical. Research security-sensitive CSS/DOM behavior in primary
  documentation and record evidence before choosing runtime application.
- [x] Document a minimal versioned JSON pack contract in an ADR and a linked
  theme-format document: identity, display name, format/token compatibility,
  complete token set and optional accent variations. Keep metadata bounded;
  built-in IDs cannot be shadowed. Specify duplicate JSON-key handling,
  unknown fields/tokens, unsupported/newer versions and deterministic export.
  Reject unsupported packs as a whole with named diagnostics rather than
  silently deleting fields or downgrading them.
- [x] Specify typed, bounded values for each token class, including palette,
  typography, shape, shadows and backdrop. Packs are data, not arbitrary
  CSS: no selectors, HTML, scripts, `@import`, `url()`, network assets or
  executable content; fonts come from the installed allow-list. Any token
  aliasing must have explicit missing-reference/cycle handling. Motion,
  density, project styling and all other preferences remain separate.
- [x] Specify import size/count/value-length limits, duplicate-ID replacement
  consent, active-pack removal, missing/corrupt-pack fallback, preview rollback
  and read-only/newer-settings behavior. Keep a known-good built-in fallback
  and preserve recoverable stored data; rejection must leave the existing
  pack and selection unchanged.
- [x] Carry ADR-0022's exact supported contrast-pair validation into the
  contract; do not promise all-component WCAG compliance from those pairs.
  Define stricter unsupported-value rejection rather than a bypass.

**Acceptance:** reviewed contract and negative-fixture matrix, linked sources
for security-critical decisions, no product implementation or compatibility
claim disguised as research. Record any deliberate change to ADR-0022.

### U15 — Validate packs and integrate the existing theme engine [web]

**Delivered as9d1ae19d; runtime foundation only. U16/U17/U18 subsequently delivered and read back; native/global-alpha acceptance remains separate.**
Depends on U14's resolved contract. Keep parsing/validation pure and
separate from DOM effects and persistence; do not add theme logic to KNX Core.

- [x] Write failing tests for the agreed valid and invalid pack fixtures;
  implement one typed parser/validator and explicit structured diagnostics.
  Cover malformed input, duplicate keys/IDs, missing/unknown tokens, unsupported
  versions, size limits, unsafe values, alias cycles and invalid accents.
- [x] Validate before any DOM application, including packs reloaded from
  settings or a stale browser cache. Extend the existing registry/resolver and
  token application path rather than building a second theme system. Keep
  built-ins and System unchanged; packs cannot override their IDs or selectors.
- [x] Reuse or narrowly extract the existing pure token/contrast evaluation;
  enforce it for every imported base palette and accent variation at runtime.
  Never round a failing contrast value into acceptance or skip an unsupported
  value. Mutation-test validation and safety guards.
- [x] Apply only validated token names/values via the ADR-approved bounded
  mechanism, remove obsolete overrides on switching back, and prove that
  theme changes cannot alter motion/density or other preference-owned tokens.

**Acceptance:** runtime and build-time invariants agree; all five built-ins,
System light/dark changes, valid imported palettes and rejected packs have
named tests. Invalid data produces neither injected rules nor asset requests.

**Evidence:** `themePack.test.ts`, `themePackStore.test.ts`,
`themePackDom.test.ts`, `themePackRuntime.test.tsx`, `themePackAgreement.test.ts`
and intercepted `theme-pack.e2e.ts`. Thirty-eight restored guard controls and
new-file TS2322 canary; combined proc_32a1aab25020 passes 22/22 steps, Web 1,559,
Chromium 61, Rust 2,916 / zero failed / 164 ignored, twenty selected offline
private cases and pinned 115-instance matrix. All 614 protected files unchanged;
exact remote ref/tree/21 artifacts/zero outgoing commits verified. No independent,
native/Orca/all-component WCAG, full import/export or release approval follows.

### U16 — Persist, import and export theme packs without losing settings [web]

**DONE — foundation delivered/read back as 1f94808d.** Depends on U15. Implemented file transport,
acknowledged plans and queue/API contracts; 144 focused tests, TypeScript,
restored unit/HTTP controls and new-file canary pass. Candidate4 passed all
16 checks. Actual-merged 22-step acceptance passed on 36e922bc, with six
explicitly selected offline suites/11 private cases, narrower than historical
U15's twenty-case scope. Web 1,665, intercepted Chromium 61, ordinary Rust
2,940/0/164 over 148 blocks, all 17 bindings equal, 622 protected inputs and
420 private files unchanged. Exact remote ref, 20 owned artifacts and all 622
gated inputs read back equal. These checkboxes attest U16 application/storage
and structured error contracts, not an implemented management panel. U17 owns visible
management/diagnostics/preview. This goal owns only the necessary theme/settings
application changes, not a new generic configuration subsystem.

- [x] Store installed packs and selected identity through the existing
  versioned settings document and `settingsStore.ts`; use no separate
  authoritative browser store, project file or manufacturer database.
  Add a settings-schema migration only if the chosen representation requires
  it. Preserve unknown/unrelated settings and newer-file refusal semantics.
- [x] Implement bounded file import/export with deterministic semantic
  roundtrip. Export must not include private project, bus or unrelated settings
  data. Clarify whether built-in palettes can be exported as independent packs;
  importing one must never shadow its built-in source.
- [x] Require explicit consent to replace an installed ID; commit pack and
  selection changes atomically where coupled. Do not report durable success
  from an optimistic cache update. On server/write failure, surface the failure
  and preserve or restore the last acknowledged state and original file.
- [x] Cover restart/hydration, cross-client settings refresh and races between
  import, selection, replacement and removal. Removing the active pack uses
  the documented built-in fallback; unsupported/corrupt stored packs remain
  recoverable and visibly diagnosed rather than silently erased.

**Acceptance:** import → select → persist → reload → export → reimport retains
all supported semantics. Failure, collision, newer settings and multi-client
regressions pass with unrelated settings unchanged; no new KNX API dependency.

### U17 — Add accessible theme management and reversible preview [web]

**DONE — management delivered/read back as 4d9073ca; U18 subsequently closed below.**
Actual merged chain proc_cdcd42b97d47 passed all23 commands:17 repository and
six offline inventory/execution commands. Web1,702/95 files, Chromium72 (zero
failed/skipped/flaky), Rust2,984/0/165 across149 blocks,17 equal bindings and697
unchanged source/config inputs. Compiled ignored inventory165; separately selected
six offline suites execute27 private cases plus the115-instance/113-unique matrix.
All420 private input files unchanged; transient corpus link removed.
31 actual parent/five root and eleven manager browser cases cover the management
contract. Eight original behavioral controls plus three added compiled browser
controls are caught/restored; diagnostic-module type canary is caught/restored.
Earlier cancelled/lifecycle-interrupted attempts remain non-acceptance evidence.
In-session self-review, not independent/native/Orca/global-alpha approval.
Depends on U15 and U16. Extend Settings → Appearance; do not create a
second settings screen or require a visual theme editor for this slice.

- [x] Show built-in and installed packs with understandable names, origin,
  version and any incompatibility diagnostic. Add Import, Export and Remove
  actions, replacement confirmation, and a clear reset to System/built-ins.
  Accent controls reflect the selected pack's actual capabilities.
- [x] Provide a temporary preview with explicit Apply/Cancel. Preview alone
  does not write settings; Cancel, Escape, dialog close/unmount or a failed
  apply restores the previous acknowledged theme without retaining overrides.
  Define/test what an authoritative cross-client change does during preview.
- [x] Keep reset/cancel usable for poor palettes. Use existing overlay/focus
  patterns; localize every label and structured error, announce outcomes and
  provide full keyboard operation without disrupting stacked modals.
- [x] Add behavioral component tests and fully mocked browser flows for valid
  import, rejection, ID collision, preview rollback, active removal, reset,
  persistence failure and System mode. Browser tests intercept all backend
  traffic; no live server, gateway or productive bus is involved.

**Acceptance:** named tests prove management/preview effects, not just mounted
controls. Disclose native WebKitGTK and real assistive-technology evidence
separately; mocked Chromium is not native/screen-reader acceptance.

Named evidence: `ThemePackManager.integration.test.tsx`, `themePreview.test.tsx`,
`SettingsPanel.test.tsx` and all eleven `theme-manager.e2e.ts` flows. The U17 log
records findings, negative controls, integration provenance and actual counts.

### U18 — Review and close the theme-pack extension

**DONE — source delivery1964fd6b published/read back; final metadata housekeeping in the handover.** U18 depends on the delivered U14–U17 extension, does not reopen U13 and does not close unrelated alpha/domain/native gates.

U18-R1 is closed by an opt-in, explicitly typed harness using actual
GroupAddressTable, Inspector, inline address validation, Overlay and structured
diagnostics. Nine palette paths cover five built-ins, System light/dark and two
admitted user palettes; the harness-presence case makes10/10. Three rendered-style
controls fail named assertions, not invented production-source mutation results.
The fixture-module TS2322 inclusion canary was detected and restored. Initial
shared-lease refusals remain infrastructure-only, not successful mutant evidence.

- [x] Semantic roundtrip, migration/settings and hostile-pack regressions:
  `themePackRoundtrip.test.ts`, `themePackFiles.test.ts`, `themePackStorage.test.ts`,
  settings client/HTTP regressions and U15/U16/U17 restored guard evidence.
  The actual Web1702/Rust2995 suites execute these contracts; U18 adds no new
  production rejection/persistence guard needing a separate source mutant.
- [x] Representative editor/table/inspector/dialog/diagnostic states:
  `e2e/theme-state.e2e.ts` passes10/10, full mocked Chromium82. Focus, selection,
  filtering, invalid editable drafts, disabled controls, dialog trap/Escape/restore,
  diagnostic kinds/paths and supported/disabled accent choices are asserted.
  Selection/focus/validation rendered-style controls fail their named assertions.
  Three contrast-pair guarantees remain separate from all-component acceptance.
- [x] Full extension reviewed in a separate in-session **self-review** pass,
  admission → plans → queue/API → root runtime → manager/selector/diagnostics →
  semantic roundtrip/real states. No new blocking production finding; no claimed
  independent approval or subagents. Source-freeze703 binds actual1660911b;
  18 repository plus six explicit offline commands pass24/24. Web1702,
  Chromium82, Rust2995/0/165 over150 blocks,17 equal bindings. Offline inventories
  reconcile11 product+16 injected-server+1 matrix=28; matrix115/113 unique;
  421 private files unchanged and own temporary inputs/matrix removed.
- [x] Status, roadmap, theme format/ADR and owned known-limitations section
  reconciled. `docs/THEME_PACKS.md` documents import/export, preview/Cancel,
  explicit replacement, recovery, System reset and unavailable-data fallback.
  Closing self-review also corrects live candidate/open U18-R1 documentation drift.
- [x] Source delivery1964fd6b published/read back: live/fetched refs, full tree,
  all703 gated inputs and15 owned artifacts exact; zero outgoing commits.
  Completion/remaining limitations handed to the goal.md owner; UI reservation
  released in the current handover.52 completed own scratch entries and own
  node_modules cleaned; final clean worktree/branch housekeeping follows the
  metadata readback. Root/foreign worktrees and all other reservations untouched.

**Evidence:** `.ai/logs/2026-10-03_codex_ui-theme-closing.md` and its aggregate
receipt; actual gate1660911b, normal Markdown-only upstream integrationef4cfb92,
all703 protected inputs exact. No private raw output is retained. Candidate18/18
and U17 historical23/23 are not substituted for actual24/24 acceptance.
Native WebKitGTK/Orca, general WCAG, Alpha/ETS and real KNX commissioning remain
separate; this theme extension never grants a hardware-write go.

---

## 3b. Alpha owner queue (Claude owner session, 2026-10-04)

The user made this Claude session the `goal-ui.md` owner and asked it to work
through the UI rows at the end of `alpha-release-goal.md`. Row statuses and the
decisions behind them are in that file's *UI owner checkpoint*. Native/live
evidence rows are `ACCEPTED_BOUNDARY` by user decision; these packages cover
the remaining absent behaviour. Domain/backend halves come first because the
commissioning session holds the Web lock at the time of writing.

| Package | Rows | Content | Status |
| --- | --- | --- | --- |
| UA1 | `MODEL-03`, `KL-127` | Research coupler addressing (`.0`) and ETS Site/Ground samples; implement on reliable evidence, otherwise record a known gap and close | backend done (RESEARCH §25); KL-127 closed as known gap; MODEL-03 web half done 2026-10-04 (`.0` submitted, server decides) |
| UA2 | `DATA-03` | Idempotent catalog batch: client request key, server replays the recorded outcome instead of applying twice; the client may then retry safely **[web]** for the client half | server half done (ADR-0069); client half done 2026-10-04 (same-id retry, same server only) |
| UA3 | `MODEL-04` | Opt-in address allocation and unique names for catalog batches, validated in the core **[web]** for the UI half | server half done; UI toggles done 2026-10-04 |
| UA4 | `MODEL-01` | Installation-scoped structure/move/link commands and installation rename **[web]** for the UI half | core/server half and web half done 2026-10-04 |
| UA5 | `MODEL-02` | Explicit, undoable repair of ambiguous imported topology without guessing **[web]** for the UI half | core/store/server half and web half done 2026-10-04 |
| UA6 | `UX-01` | Drag a group address onto a communication object (keyboard equivalent kept) **[web]** | no backend half needed (`POST /api/group-links`); web half done 2026-10-04 |
| UA7 | `KL-13` (AR08) | Project-password dialog for protected ETS imports: ask on `422` `projectPasswordRequired`, ask again on `projectPasswordWrong`, password only in that request **[web]** | handed over by the alpha session 2026-10-04 20:58; web half done 2026-10-04 (`9bc36499`) |
| UA8 | `KL-60` (AR11) | Diff view: virtualised long tables with search and status filter instead of 50-row paging; backend diff API unchanged **[web]** | user decision 2026-10-04 (AR11); done 2026-10-04 |

Every package follows §2: RED first, mutation check per new guard, gates,
docs, merge, push, handover, cleanup.

**Handoff 2026-10-04 11:27:** all backend halves are published. By user decision the
**[web]** halves of UA1–UA5 and all of UA6 are handed over to the Web-lock
holder (the commissioning session). Tasks, API contracts and acceptance
criteria are in `alpha-release-goal.md` → *UI owner handoff*. This owner
session keeps the backend contracts.

**Owner takeover 2026-10-04 15:38:** the previous owner session closed at 15:28. By user
decision, the Claude UI session that delivered the §133 desktop-shell fix
(`2ab5698f`) is now the `goal-ui.md` owner. It keeps the backend contracts
above and answers questions about them. The web halves stay with the Web-lock
holder as handed over, and this takeover neither takes nor releases the Web lock.

**User decision 2026-10-04 15:41:** the six web halves of UA1–UA6 and the AR13
`debugReport.privacyTelegrams` text move from the commissioning session back to
the `goal-ui.md` owner session. That session starts on them only after the
commissioning session releases the Web lock, then takes the lock through the
`goal-ui.md` §3 procedure. Until then the commissioning session keeps the lock
for its own package and owes nothing on these rows. As of this decision, no web
half had been started anywhere.

---

## 3c. Telegram-flow extension — U19–U21 (approved Alpha scope)

Execute after reconciling the owner's active package and outstanding assigned
work. Dependency order is **U19 → AR20 → U20 → U21 → AR21 → final Alpha gates**;
AR20/AR21 live only in `alpha-release-goal.md`. Nothing here reimplements capture,
DPT decoding, completed settings/themes or commissioning.

### U19 — Resolve the flow contract and evaluated visual slice

**Done 2026-10-04 (goal-ui owner).** Evidence and the exact AR20 handoff:
[TELEGRAM_FLOW_VISUALIZATION §9](docs/TELEGRAM_FLOW_VISUALIZATION.md#9-u19-resolution-goal-ui-owner-2026-10-04);
study code under `apps/knx-web/e2e/flow-study/`. Originally: dependencies:
latest owner/lock reconciliation and [ADR-0077](docs/adr/0077-session-local-telegram-flow-view.md).
Use the Web lock if editing any Web sources, fixtures or generated bindings.

- [x] Inspect current monitor/session/context/projection/Inspector and primary
  research; preserve observed versus configured versus ambiguous evidence.
  Send associations do not exclude receiving membership; no fabricated receipt.
- [x] Produce a bounded, visibly synthetic visual slice of direct devices,
  unknown group nodes, group-address labels, directed pulses and separate value
  badges, using existing themes/motion. Record an actually rendered screenshot
  and keyboard/static fallback; do not claim a study is productive integration.
- [x] Evaluate native SVG first and a bounded layout approach. Record real
  frame/lag/memory evidence for the specified workloads before selecting a
  solver/dependency/Canvas/worker. Specify readable resting edges, model/pulse
  limits, overflow disclosure, rolling window, damping and live tie behavior.
- [x] Freeze an exact handoff to AR20: session/context identities, typed raw
  addresses, participant evidence/flags/activation, value/error states,
  observation age, cursor/drops, stale/restart handling and legacy fallback.
  Proposed fields/routes are not claimed existing; no parallel polling/tunnel.
- [x] Record resolved contract/tuning and reproducible test inputs in the flow
  document/ADR and handover to the alpha owner. Preserve current owner work and
  original native/Orca/live accepted boundaries, not a blanket new certification.

**Acceptance:** an evaluated visual/semantic slice and exact AR20 input contract,
not a shipped graph or a hypothetical benchmark. Acquire no lock merely to plan.

### U20 — Integrate the shared monitor feed, graph and immediate values [web]

**Done (2026-10-04).** AR20 integrated (`85bfab88`, §10). Part 1 `4525c36e`
(wire validation, reducer), part 2 in the delivering commit (feed, view,
Inspector, keyboard, e2e); rules in TELEGRAM_FLOW_VISUALIZATION §11, residue in
KNOWN_LIMITATIONS §154. *Freeze* moves to U21: U20's layout is static, so a
freeze control would do nothing until U21 adds motion. Dependencies: U19 and **integrated AR20 contract/receipt**, with exact
wire names and revision. Reuse one session/event controller with existing
attachment/pause/cursor/context guards; do not implement another capture service.

- [x] Add a read-only Telegram flow view to the existing diagnostics/monitor
  navigation. Opening it does not connect/start/write; main/companion views
  share event/session handling and do not duplicate poll loops or tunnels.
- [x] Implement a pure, bounded session-keyed reducer and deterministic graph
  identities. Display exact/unresolved/ambiguous sources, configured target
  endpoints, group labels and a loss/context/overflow legend. No first-match
  device selection, formatted-address guessing or current-project relabelling
  of historical rows; preserve per-object evidence in the Inspector.
- [x] Immediately update source and configured-target per-group value slots
  from admitted value-bearing events, at most three current badges per device
  plus Inspector overflow. Expire at 7 seconds using trustworthy age/monotonic
  deadlines; newer same-slot sequences win. Reads/errors/old rows do not invent
  a value or renew stale TTL; pulse completion has no data-write authority.
- [x] Add keyboard selection/navigation/pan/zoom/freeze controls and semantic
  HTML/Inspector access, focus/selection styling, localized explanations and
  no screen-reader event storm. Follow admitted built-in/System/imported themes.
- [x] Prove fake-clock expiry boundaries, multi-group/multi-source replacement,
  duplicate poll delivery, delayed/out-of-order replies, capture loss, tab change,
  restart, no project, changed links/flags/addresses and historical reattachment
  with named RED/GREEN and restored behavioral guard controls.

**Acceptance:** actual application consumer/reducer/Inspector evidence using
intercepted synthetic traffic; no real bus, inferred receipt or new persistence.

### U21 — Make the nervous system dynamic, honest and bounded [web]

**Done (2026-10-04), returned by AR21 and corrected (2026-10-05).** Parts A
and B `9d432d17` (+ whitespace fix `deb6813a`), part C `fb40a99a` (activity
classes, 30 fps cap, production load figures in
docs/design/2026-10-04-telegram-flow-u21/). AR21 corrections in the delivering
commit of 2026-10-05: local per-node reheat, hub separation, §7-load envelope
(TELEGRAM_FLOW_VISUALIZATION §13, §14); awaiting the AR21 rerun. Rules:
TELEGRAM_FLOW_VISUALIZATION §12, §14; residue: KNOWN_LIMITATIONS §154 (one path
per pair, one machine measured, §7 load saturates motion, no WebKitGTK/Orca). Dependency: U20. Keep data admission independent of solver/rendering.

- [x] Add event-triggered directional traveling pulses, opposite-direction
  traffic, group-specific paths/labels and brief source feedback. Values stay
  immediate; no animation queue masquerades as physical bus timing or receipt.
- [x] Implement activity-dependent bounded distances, stable/damped layout and
  the observed-sender leader in the labelled rolling window. Window expiry and
  ties are deterministic; fan-out counts once. Freeze fixes geometry only;
  values/counts/pulses stay live, and the current leader remains labelled.
- [x] Fade quiet edges only to a theme-readable resting line for the session.
  Keep semantic graph membership independent of activity-window/value expiry.
  Coalesce high-load pulses with represented counts; visible capacity/loss
  diagnostics preserve existing edges, not silent eviction or complete-map claims.
- [x] Prove live theme changes, motion Off/OS-reduce changed mid-effect, hidden
  tab/resume/unmount cleanup, static directional fallback and continued value
  expiry. Stop actual solver/timers/frames, not just CSS. Record real dense-burst
  and long-session CPU/memory/frame/lag results, including reduced rendering.
- [x] Run integrated UI/full gates and a separate in-session review; restore and
  verify behavioral mutants, update focused docs/manual/limitations and publish
  the exact U21 receipt to the alpha owner for AR21. Keep evidence provenance
  and user-accepted native/Orca/live boundaries explicit; do not self-certify Alpha.

**Acceptance:** every flow-contract scenario has named, actual evidence;
no source/domain/store mutation or hardware action. AR21 adopts, not duplicates,
this owner's rendering implementation and acceptance.

---

## 4. Completion condition

Finish only when:

- U0 to U13 remain done with evidence, or the user has explicitly accepted an
  item out of scope;
- every issue-plan checkbox for the owned issues is ticked with a named test,
  or explicitly marked out of scope with the user's acceptance;
- every U14–U18 checklist item has named acceptance evidence or an explicit
  user-approved scope disposition; existing alpha follow-up is reconciled
  without treating this extension as alpha-release approval;
- U19–U21 have named integrated evidence and a handoff to AR21; the approved
  Telegram-flow Alpha feature is not silently deferred;
- all gates from §2.5 are green on the merged `main`;
- the closing review has run and its findings are fixed;
- the web lock is released.

Then report: what is done, the evidence, and what is left for the goal.md
session.

---

## 5. Boundaries (no overlap)

| Topic | Owner |
|---|---|
| ISSUE-01, 02, 03, 05, 07, 09, 10, 11, 12, 13, with their server and domain halves; the UI half of ISSUE-08; the ADR-0038 review; the File-menu rename | **this goal** |
| ISSUE-08 data half: trace, classification, language-aware name and DPT resolution, active/visible state and channel ownership in the projection, corpus regression counts | `goal.md` |
| Product database, KNX project import, domain/project schema, PDB-x | `goal.md` |
| U14–U18 theme-pack contract, runtime validation, preference persistence and narrowly required settings API changes, theme-management UI and theme-specific docs | **this goal**, under the Web lock when editing the web tree; no KNX-domain/storage schema changes |
| `docs/LIMITATION_TRIAGE.md` recount, `stats.md`, `goal.md`, the manual's T23 acceptance, doc hygiene `goal.md` §8, the alpha decision, the final whole-goal review | `goal.md` |
| Device programming: download, individual address, restart. Their UI (K5 download tab, K6 address dialog), their server routes, the server-side consent decision (ADR-0045) | `goal-commission.md` |
| The ADR-0040 consent hook and dialog | built and closed. The commissioning session calls it; this goal does not change it |
| Telegram-flow visual design/handoff, shared-feed UI, reducer, layout, pulses, theme/motion/keyboard/load evidence | **this goal**, U19–U21 under the Web lock; consumes integrated AR20 |
| Telegram-flow pure participant/event-type/age/context contract and whole-feature Alpha acceptance | `alpha-release-goal.md`, AR20/AR21; no UI rendering or commissioning work |
| `apps/knx-web` in general | this goal, under the web lock (§3). The commissioning session takes the lock for its own views only |

If a package turns up something from another column, do not do it here.
Hand it over (§6).

## 6. Handover between sessions

- **To the goal.md session:** in your handover entry, under the heading
  **"For the goal.md session:"**. That session adopts items into `goal.md`
  §12.3 and confirms in its next entry. Typical items:
  - new or renumbered KNOWN_LIMITATIONS entries (for the triage recount);
  - a `stats.md` refresh after each of your merges;
  - findings in product data or import (as findings, not fixes);
  - theme-extension completion and any new settings contract; theme tasks do
    not change alpha scope or overwrite existing owner dispositions.
- **To the commissioning session:** under **"For the commissioning
  session:"**, for example UI findings in their programming views.
- **From the goal.md session to you:** the chosen U13 independent review
  returns concrete findings and an actual verdict. A refused review request
  is not a verdict; do not close U13 until findings and gates are settled.

**Candidate gate verified (2026-10-03 13:48 CEST):** The notified runner completed normally and its complete receipt was checked:18/18 commands, Web1702, Chromium82 (zero failures/skips/flaky), final palette matrix10/10, Rust2984 passed/0 failed/165 ignored across149 result blocks; all701 frozen source/config inputs remain exact. This supersedes the earlier running-candidate observations, not the still-pending actual-integrated/offline/private acceptance or publication. In-session review found no new blocking production issue; it is not an independent third-party approval. Integration against fetched documentation-only upstream8af45464 is next.
