# ADR 0172: Programming all pending devices is one server-held batch that stops at the first doubt

Date: 2026-11-11
Status: Proposed (design only; no code, no hardware run)
Session: issue intake #42, step 2 ([WP-38](../open-work/WP-38-pending-devices-panel.md))

## Context

Issue #42 asks for an action that loads every pending change into its device in
sequence, the way the ETS pending-devices list offers it. Step 1, the read-only
**Pending programming** block (main `638563715`), lists those devices and jumps
to the single-device download. The triage reply promised a design before any
code: readiness per device before anything starts, one confirmation for the
batch, stop at the first failure, a result per device.

Verified state of `main` on 2026-11-11 (source reading, no new run):

- **One device at a time.** `/api/device-download/{plan,start,status,report}`
  (`apps/knx-server/src/device_download_routes.rs`) keep exactly one shown plan
  (`device_download_plan`, a single slot) and one run (`device_download`).
  Planning a second device replaces the first device's shown plan; a started
  plan leaves the slot.
- **What `start` already guarantees per device**, in this order: stop admission
  ([ADR-0153](0153-regular-download-stop-ends-at-first-change.md)), the plan id
  being the one shown last, the device's confirmation phrase
  ([ADR-0045](0045-device-download-route-demands-phrase-and-plan.md) §1–2), the
  tunnel reservation in the order download → address programming → monitor →
  line scan, the plan prepared again (for `scope: "changes"` with the advised
  scope re-checked), the typed acknowledgement for an `untested` download
  ([ADR-0049](0049-download-readiness-is-per-plan-and-backups-are-pre-write.md) §2),
  equality of the whole prepared plan with the shown one (ADR-0045 §3), a
  durable activity-history start before the connection
  ([ADR-0067](0067-download-lifecycle-preserves-uncertainty.md),
  [ADR-0075](0075-shared-commissioning-activity-lifecycle.md)). During the run:
  a backup of every region the plan overwrites before the first change
  (ADR-0049 §3), and afterwards the settlement of the device's pending flags
  ([ADR-0129](0129-per-part-pending-programming.md)).
- **Offline readiness exists, for the complete download.** `GET
  /api/device-readiness` grades every project device as `verified`,
  `untested`, `unsupported`, `excluded` or `no-address` without a tunnel. It
  prepares the *complete* download (`knx_app::project_readiness`). Evidence is
  recorded per operation (ADR-0049 §2), so a partial download of the same
  device can grade differently or be refused.
- **Scope advice** (`knx_core::commissioning::scope_advice`): application
  pending → complete download; only parameters and/or group addresses →
  partial download of exactly those; otherwise nothing. No memory download
  writes the individual address or the medium configuration; a pending address
  is reported as `AddressNeed::Pending`, to be closed by address programming.
- **The identity check is a type check.** Before the first write the executor
  compares mask, manufacturer, the application identity where the plan needs
  it, and the comparisons the plan declares (`knx-net` `memory_download.rs`).
  The backup reads and records the serial number, but compares it only when the
  plan pins an exemplar; the post-download check reads none
  (`knx_app::post_download_check`). Without a pinned exemplar, a different
  device of the same type at the same address passes.
- **Each run ends in a typed result:** `Finished { written, restart,
  restart_note, post_download_check }` or `Failed { written, stopped_in_step,
  error, hint, failure }`, with `written` ∈ `yes | no | partially` and
  `restart` ∈ `acknowledged | notInPlan | unconfirmed`.
- **The post-download check has many outcomes short of a match:** not checked,
  unsupported (no verified identity layout or application identity), not
  performed, interrupted, target not reached or reach unknown, identity
  unavailable or mismatched, and a load-state verdict of `matchesPlan`,
  `differsFromPlan` or `unknown`.
- **Busy refusals.** Besides the four tunnel holders above, other bus routes
  (System-2, device compare, serial address, service control, group write)
  refuse while a download runs.
- **System-2 (mask 0025h)** runs on its own routes with its own best-effort
  phrase ([ADR-0152](0152-gated-hardware-system2-download.md)).
- **UI consent** is remembered per release stage
  ([ADR-0040](0040-programming-requires-release-stage-consent.md) §3); a
  remembered answer skips the dialog.
- **No evidence about sequences.** No hardware run, ETS observation or
  specification text tells us how a gateway or a line behaves when several
  devices are programmed back to back. Nothing below claims it.

## Decision

1. **Candidates and order.** The batch offers the devices the pending block
   lists, in the same address order. A device takes part only when all of
   these hold; every other device appears in the review as *not in this batch*
   with its reason and is never dropped silently:
   - it has an individual address, and the project does **not** mark that
     address pending. A download to an address the device does not hold yet
     reaches whichever device answers there, and the type check cannot tell an
     exemplar apart. Address programming needs a button press per device
     ([ADR-0046](0046-address-programming-route-one-phrase-stoppable-wait.md),
     [ADR-0059](0059-button-address-programming-requires-durable-recovery.md)),
     and the serial-number route refuses every confirmed write today
     ([ADR-0057](0057-serial-address-writes-require-durable-recovery.md)); it
     stays outside the batch;
   - its advised write is `complete` or a partial download; a device with only
     address or medium configuration pending has nothing to download;
   - the plan of its advised scope, prepared and graded by the batch review
     (2), is `verified` or `untested` on the regular memory path, and the
     device is not on the project's exclusion list. The project-wide readiness
     route grades the complete download only and does not decide this. A
     System-2 device keeps its own panel and phrase;
   - the post-download check can be performed for that plan (a verified
     identity layout and application identity). Otherwise its result could
     never qualify the next device (5); the device is listed as *not in this
     batch* with that reason instead of ending the batch midway.

   Each device is written in its advised scope (`scope: "changes"`); the batch
   has no scope picker. The order is the address order and nothing else: no
   topology rule (couplers first, line by line) is verified, so none is
   invented. The user can deselect devices in the review; a deselected device
   is reported as *deselected*, not as excluded.

2. **Readiness before anything starts.** `POST /api/device-download/batch/plan`
   prepares the plan of every selected device from one project snapshot,
   grades it and returns per device the scope, step count, data octets,
   support level, confirmation phrase and, for `untested`, the acknowledgement
   phrase, or the reason it is not in the batch. It keeps the plans under one
   batch id. Nothing is sent and no tunnel opens. The shown-plan slot holds
   either one single plan or one batch; each replaces the other, so a stale
   single plan can never start beside a batch. `batch/start` moves the plans
   out of the slot into the run, as a single `start` clears it; a later
   `/plan` cannot change a running batch.

3. **One confirmation.** The review lists every device of the batch. **Start**
   opens one confirmation that names the release stage (ADR-0040), the number
   of devices and their list. It goes through `useProgrammingConsent`, the
   single entry point of ADR-0040 §1, as a batch request that is shown even
   when the stage consent is remembered: a remembered answer accepts the
   stage's risk, not this list of devices. This narrows ADR-0040 §3 for
   batches only; the single-device dialog is unchanged, and ADR-0040 gets a
   *Narrowed by ADR-0172* note when this ADR is accepted. Each `untested`
   device keeps its own typed acknowledgement in the review (ADR-0049 §2);
   there is no "acknowledge all". Start stays disabled until every listed
   `untested` device is acknowledged or deselected.

   `POST /api/device-download/batch/start` carries the batch id, the gateway,
   and per device the confirmation phrase and the acknowledgement where one is
   needed. ADR-0045 §2 holds as written: the UI builds the phrases only after
   the batch request resolved `true`. The server checks every phrase with
   `WriteAuthorisation::for_hardware` and every acknowledgement **before** it
   takes the reservation; one mismatch refuses the whole batch with nothing
   started.

4. **The server runs the batch.** One batch run holds the download reservation
   from start to end, also between devices; every route that refuses beside a
   running download refuses beside a batch, and the busy message names the
   batch as holder. Per device the run does exactly what `start` does for a
   single device today: prepare again from the then-current project and compare
   with the reviewed plan and the advised scope, write the history start, open
   its own tunnel, check identity, back up, write, restart, run the
   post-download check, settle the flags, disconnect. The settlement of an
   earlier device in the batch changes only that device's flags and does not
   count as a change to a later device's plan; any other project edit that
   changes a later plan stops the batch before that device. One tunnel per
   device keeps the verified single-device lifecycle unchanged; sharing a
   tunnel across devices is not part of this decision. Closing the browser
   does not stop the batch; its status stays readable.

5. **Stop at the first doubt.** The next device starts only when the previous
   one ended `Finished` with `written: yes`, `restart` `acknowledged` or
   `notInPlan`, and a post-download check that reached the device, matched its
   identity and found every load state `matchesPlan`. Every other outcome ends
   the batch, the unknown ones included: a failure, `written: partially` or
   `no`, an unconfirmed restart, a check that was not performed, interrupted,
   unsupported, did not reach the device or could not tell, an identity that
   was unavailable or differed, a load-state verdict `differsFromPlan` or
   `unknown`, a plan that changed since the review, a gateway that does not
   connect, an activity history that cannot record. The remaining devices are
   `notStarted`. There is no retry, no skip-and-continue and no automatic
   restore; a restore stays a separate, confirmed download (ADR-0049 §4).

6. **Stopping by the user and by the server.** `POST
   /api/device-download/batch/stop` means *stop after this device*: the device
   in progress runs to its end, the rest become `notStarted`. It does not stop
   a device mid-download; there is still no step boundary for that after the
   first change (ADR-0045 §5, ADR-0153). A graceful server stop applies ADR-0153
   to the current device and starts no further device.

7. **A result per device.** `GET /api/device-download/batch/status` reports per
   device `done`, `failed` (written, step, error, hint, backup file),
   `stopped` (ADR-0153, before the first change), `notStarted` (with the
   reason: earlier device, stop request, server stop), `notInBatch` (with the
   reason from 1) or `deselected`. Every device run keeps its own
   activity-history row, operation recording and download report. The link to
   the batch is the operation recording's existing related-value mechanism
   (`related("batchId", …)`); the activity-history format does not change.
   The batch summary lives in server memory until the next batch or a
   restart; it is not a durable record, the per-device history is. A crash
   leaves the device in progress `interrupted/unknown` (ADR-0067) and starts
   nothing more. A batch never resumes: a new batch needs a new review, plans
   and confirmation. Devices already done have their flags settled and drop
   out of it.

8. **First slice.** Web server and UI only, regular memory-mapped downloads
   only, tested with simulated transports and fake tunnels. No CLI batch
   command, no hardware claim. Copy follows WP-34: English **Download all…**,
   German **Alle programmieren…**.

## Alternatives considered

- **A loop in the browser over the single-device routes.** Rejected. The slot
  holds one plan, so the user would confirm a list whose plans are shown only
  one at a time; the exact-plan rule would bind each device at its own start,
  not at the confirmation. The reservation would be released between devices,
  so a monitor or scan from another window could interleave, and closing the
  tab would leave the batch somewhere undefined.
- **One tunnel for the whole batch.** Not now. Connection and disconnect belong
  to the verified single-run lifecycle and its history; sharing them changes
  that lifecycle without a measured benefit.
- **Continue after a failure or an unknown check.** Rejected, and promised
  otherwise in the issue. A failure can be the gateway or the line rather than
  the device, and writing on regardless multiplies a partial state.
- **"Acknowledge all untested".** Rejected: ADR-0049 asks for the device's
  address, and a checkbox would turn it back into a blanket consent.
- **Programming pending addresses first, inside the batch.** Rejected: it needs
  a person at each device's programming button and has its own recovery rules.
- **Topology-aware ordering.** Deferred: no verified rule exists. Revisit with
  evidence, not with a guess.

## Consequences

- New routes `batch/plan`, `batch/start`, `batch/status`, `batch/stop` under
  `/api/device-download/`; the shown-plan state becomes *single or batch*; the
  busy messages name a batch holder. Single-device behaviour, phrases,
  activity-history format and backups stay as they are.
- The pending block and the download tab gain **Download all…** / **Alle
  programmieren…**, which opens a batch review; nothing plans or connects until
  the user asks for the review.
- Devices whose plan has no verified identity layout cannot take part. That is
  the price of rule 5; the single-device download stays available for them.
- Tests to write before the feature counts as done, all on simulated
  transports: every *not in this batch* reason (address pending, missing
  address, nothing to download, advised-scope plan `unsupported`, excluded,
  System-2, no performable post-download check) and *deselected*; refusal of
  the whole batch before any tunnel on one wrong phrase or acknowledgement; a
  project edit between two devices stops the batch before the second, while
  the batch's own flag settlement does not; each outcome listed in 5 stops the
  batch with the rest `notStarted`, and only the full match continues; busy
  refusals while the batch is between devices; stop-after-this-device; server
  stop mid-batch; one history row, one recording with the batch id and one
  flag settlement per finished device; a single plan and a batch replacing
  each other, and `/plan` during a running batch leaving it untouched.
- Known limit: the batch is only as safe as each single download. It adds no
  device-identity guarantee beyond the existing checks and no knowledge of
  line or coupler ordering.
