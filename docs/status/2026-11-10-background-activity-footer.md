# Background activity footer — bounded source implementation (10 November 2026)

## Status

The bounded observer is implemented. Focused regressions, maintained TypeScript
configuration checks and actual App browser execution have run; the browser flow
passes at 1440/400 px in EN/DE, in both the default development lane and the
registered task-status production lane. This is frontend fixture integration,
not backend, release, deployment, hardware or whole-application background-job
completion. Publication remains guarded by the maintained change-scoped package
gate and its frozen-input, stage-proof and user-data checks.

## Accepted interaction

- A producer registers its operation immediately. Existing local loading text,
  disabled controls and result/decision panels remain authoritative. The footer
  inventory is immediately available; its summary waits **2 seconds**. Work that
  settles earlier never flashes a global shimmer or success summary.
- The footer is an expandable global inventory of concurrent and recent work.
  The bus connection remains directly before the version, which remains last.
  Completion never forces navigation to the initiating view.
- Ordinary successful visible work stays in the summary for **3 seconds**.
  The session keeps the latest **20 settled entries**, ordered by settlement;
  outstanding unacknowledged failures are retained separately from that bound.
  Closing the inventory does not acknowledge errors or dispose a server job.
- Consent/overwrite/password/unsaved-project questions are static decisions,
  never failures or animated work. An uncertain catalogue outcome is not success.
- Progress is only a producer-reported completed/total pair. No estimated
  percentages, synthetic progress, fake fetch wrapper, retry engine or durable
  job history is added. The existing `task-status-text` shimmer and its
  reduced-motion, motion-off, forced-colour and print fallbacks are reused.
- The new hub has no cancellation control. Existing owners keep all authority.
  In particular, accepted catalogue cancellation and comparison stopping remain
  visible until actual terminal settlement, not HTTP abort or view dismissal.

## Architecture and identity

`ActivityProvider` constructs one `ActivityController` for the mounted editor.
The store is an observer with guarded operation handles, project scope and
optional entity identity. Exact duplicate observers are deduplicated. Late
updates, double terminal events and updates through evicted handles cannot
modify a different operation. Its injected clock schedules only the next reveal
or success-expiry deadline; it does not update state every frame.

The App owns the catalogue subscription independently of catalogue/wizard
leaves. The catalogue controller exposes a non-secret per-attempt identity and
source filename; capabilities, polling, admission, upload, apply, cancellation,
retention and disposal stay in that controller. The footer's **Show details**
opens its existing App-retained result/consent dialog, only while the exact
operation is still retained by that owner. It never substitutes a result from a
later catalogue job with the same filename. Device-comparison details likewise
return only to the exact current retained review/run; starting or inspecting a
new review withdraws the previous result link.

Accepted project replacement withdraws stale project reads. Device-editor
reads retain request ordering and selection fences across all three callers.
An old load result is not adopted after replacement; a save's refreshed tree
must still belong to its captured project scope and incarnation. Mutating
requests are not declared cancelled by local scope withdrawal. Existing
save-before-replace, overwrite, unsaved changes, draft, quit and server ownership
guards remain in place; no new safe-exit capability is claimed.

## Reviewed corrections and regressions

- The parameter host acknowledges its own exact returned revision/value even
  when synchronous activity publication renders that committed revision before
  the leaf's Apply promise settles. Unrelated/newer revisions, wrong values and
  retired selections remain blocked; validation, undo history and App admission
  are unchanged. Forced host/leaf publication regressions cover this ordering.

- Resume reports success only after snapshot and cross-window publication
  admission. It settles at that seam before resetting its own initial read
  scope; a refused response is discarded, not successful.
- Save readback distinguishes **accepted-clean**, **accepted-dirty** and
  **discarded**. Only accepted readback reports completion or save achievements;
  dirty acceptance confirms a save but explicitly retains unsaved changes and
  refuses Save-and-quit/replacement. Discarded readback is not proof that the
  write failed or was cancelled. Autosave keeps its existing `Promise<void>`
  rescheduling/error contract; only accepted readback emits autosave success.
- Entries capture readable installation/project names and available device
  labels/individual addresses independently of opaque scope fences and source
  filenames. History keeps the initiating labels after rename/replacement;
  filenames do not expose full paths. Resume adds names only from an admitted
  tree; opening/importing may identify its source filename before admission.
- Settlement and scope withdrawal clear active phase/count data, while retaining
  errors. Neutral operation titles plus the actual terminal outcome replace
  misleading combinations such as a completed row still saying it is reading.
- Error acknowledgement first returns focus to the surviving Activities toggle,
  including when retention immediately evicts that old row.
- The collapsed comparison summary keeps one device and a localized remainder
  count, bounded to two lines even with long labels. Expanded history preserves
  every captured target. Its scrollable height subtracts the existing measured
  footer clearance rather than assuming a fixed-height footer. A real 500-target
  Compare-all regression covers both widths and languages.

Discriminating regressions cover deferred resume versus a newer
cross-window record; all three save callers against scope, revision and
cross-window refusals with accepted-clean/dirty neighbours; dirty/clean quit
outcomes; autosave rescheduling after refusal and a real error; original project
and actual device captions in App/component/comparison history; phase-bearing
completion, decision and error; and keyboard acknowledgement with row eviction.
The executed browser flow includes actual Tab/Enter focus checks for that
acknowledgement in both languages and both widths. Isolated semantic wrong
variants were detected for admission, save truthfulness, terminal wording and
acknowledgement focus, with the restored selected 18-case control passing. These are
regression-discrimination checks, not additional whole-application coverage.

## Production integration inventory

| Producer | Completion and ownership |
| --- | --- |
| ETS import / native open | App-owned; full JSON tree or exact-token recovery plus full current-project response. Existing load phase polling feeds the observer; terminal polls alone do not finish the owner. Password/unsaved refusals become a neutral decision outcome. |
| Editor resume | App-owned read; full current-project response plus local/cross-window admission before success, fenced against a newer load/project. No-project or refused admission is a neutral discarded read, not an error. |
| Save / Save As / save-before-replace / save-and-quit | App-owned; persistence response **and accepted** current-project response. Accepted-dirty is a saved snapshot with remaining unsaved edits; only accepted-clean permits exit/replacement. Refused readback is discarded, not successful. Overwrite is a static wait. |
| Autosave | Same persistence/readback seam; original autosave error propagation and rescheduling retained. |
| Device editor reads | Selection, post-command refresh and product-language refresh share a project/entity/request-fenced helper. Full detail response precedes terminal feedback. |
| Read-only device comparison | Existing App-owned queue, unchanged admission and result panels. True completed-target counts only. Stop/project withdrawal preserves the pending read owner until it settles. An explicit details link returns to its retained review. Completed means queue completion, not proof that devices match; findings remain in the comparison owner. |
| Catalogue package job | Existing retained App-owned controller; synchronous public snapshot observation, no additional fetch/poll/retry. Ready-for-consent is static; unknown outcome is uncertain; terminal cleanup and the actual job result determine outcome. Independent of project changes. |

## Deliberate exclusions and remaining work

- Ordinary `DeviceDownloadPanel` polling/settlement is panel-owned and can unmount
  on a Bus Diagnostics inner-tab change. It is **not** bridged: a footer row must
  not imply retention or worker completion after that unmount. Migrating that
  lifetime is separate work. No new ordinary-download cancellation is offered;
  see [pre-write stop boundary](../contracts/regular-download-prewrite-stop.md).
- System-2, device management, bus scans/monitoring, firmware and other device
  jobs keep their existing independent activity/status and safety owners. Stale
  or unavailable status is not reinterpreted as a successful footer result.
- `ProjectDiffPanel` has no operation/run identity or project fence and remains
  local. It needs those prerequisites and late-response regressions before a
  truthful retained bridge. Device comparison is a different, retained owner.
- Full-project validation/dashboard/catalogue list reads, undo/redo commands,
  new-project wizard, CSV/selective import/export, legacy product-database
  import, catalogue backup/restore and panel-owned parameter/notes/RTF writes
  remain with their existing local feedback. Post-command device-detail refresh
  is tracked, not the preceding domain command itself.
- No promise of job resumption after exit/reload, safe cancellation of arbitrary
  mutations or universal pending-operation exit guards. No storage/schema,
  protocol, hardware execution or real-installation access is introduced.

## Verification and visual recipe

- `activityController.test.ts`: injected clock/deadlines, no quick flash,
  concurrent states, brief success, bounded history, sticky failures,
  stale/double terminal identity and project withdrawal.
- `ActivityFooter.test.tsx`: real provider/controller, static terminal/decision
  rows, concurrent errors, acknowledgement, keyboard disclosure/Escape,
  translated copy, retained view-independent inventory and details routing,
  including compact/lossless 500-target context in EN/DE.
- `catalogueActivity.test.ts`: real catalogue owner with mocked transport,
  observer reattachment, real terminal cleanup, cancellation settlement,
  uncertain/forgotten outcomes and no extra polling/disposal.
- `App.test.tsx`: real open/import producers, terminal-poll-vs-body ordering,
  retained catalogue observer through Help removing its leaf, load/save
  replacement fences and sticky errors. Existing safety suites are retained.
- `useDeviceComparisons.test.tsx`: existing queue ownership regressions plus
  activity stopping/settlement and view-independent rerender coverage.
- `e2e/background-activity.e2e.ts`: actual App at 1440/400 px in EN/DE;
  project opening concurrent with catalogue work, producer-view unmount,
  keyboard disclosure, motion static fallbacks, result dialog, success history,
  footer bus/version order, bounded summary and popup, and a real 500-target
  Compare-all run stopped through its existing owner. The wide fixed-height shell
  keeps the footer inside the viewport; the established narrow shell scrolls.
  Every API is intercepted,
  including `POST /api/bus/discover` → `{ interfaces: [] }`; unexpected requests
  fail the test. This is frontend fixture integration, not backend acceptance.

The new browser spec is in the existing registered **task-status** production
lane and `tsconfig.task-status-fixtures.json`; it also runs in the default dev
fixture lane. No separate unregistered/demo-only config is used.

For visual inspection, create a fresh external capture directory and run the
owning lane with `KNX_ACTIVITY_CAPTURE_DIR=<capture-dir>`. The spec writes
`<production|development>-activity-<en|de>-<1440|400>-<concurrent|completed|many-targets>.png`.
Each lane has twelve distinct stills, so the later lane cannot overwrite the
earlier one. Inspect every captured still,
especially wrapping, bus visibility, text ink and disclosure focus; screenshots
are not an automatic visual acceptance. Do not regenerate released manual
screenshots. Routine acceptance uses the [package gate](../VERIFICATION.md#package-gate-targeted-by-default-complete-for-releases).
