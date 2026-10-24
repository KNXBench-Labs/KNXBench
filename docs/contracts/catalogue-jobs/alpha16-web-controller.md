# Alpha16 WP04.3 web controller

Status, 24 October 2026: implemented frontend/client slice, with bounded production
catalogue software outcomes recorded in the
[current release matrix](../../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026).
This supersedes earlier candidate-pending wording below. Original mocked
transport/component evidence stays separate from actual mounted server outcomes;
it is not retroactively runtime proof. No universal rollback, performance,
physical-device or overall release acceptance is claimed.

## Actual ownership and consumers

`App` creates one `CatalogueJobsProvider` above `AppBody`. It does not live in a
catalogue workspace, a wizard instance or a wizard product step. The injected
`CatalogueJobsController` privately retains the capability, original admission
offer/create metadata, one upload attempt, apply/cancel command identities,
origin wizard-run ID, latest admitted status and one bounded detail page.
There is no module-global operation, persistent browser storage or telemetry.

Concrete consumers are:

- `AppBody` / `CatalogueJobsRootNotice`: persistent toolbar discovery and a
  body-portalled report dialog, even after workspace or whole-wizard dismissal;
  commit-driven product-language refresh with a separate read/retry warning.
- `CatalogBrowser` / `ProductInstallControl`: actual catalogue file selection,
  per-File profile/acknowledgement, root-job reattachment and list invalidation.
  Search/manufacturer filters are preserved; stale list paints are guarded
  independently from catalogue-wide committed evidence.
- `CatalogUpdatePanel`: the already-open backup listing consumes the catalogue
  browser's install revision. New backup lists remain explicitly opened/lazy.
  Existing ask-first restore and identity-bound delete are unchanged.
- `DeviceWizard` / `DeviceWizardProduct` / `ProductInstallControl`: a distinct
  wizard-run ID is provided above the conditional product step. Product →
  placement → product does not recreate or upload. Whole-wizard close offers
  continue-in-background or request-cancellation-and-leave. A commit while the
  product step is detached sets that run's installed-package discard warning;
  a commit after whole-wizard dismissal remains visible at root and does not
  mark an unrelated new wizard as its origin.

The optional context path preserves standalone synchronous component consumers
and their existing compatibility tests. The production `App` always supplies
its root owner; an HTTP refusal never switches a root job to the synchronous
installer, a different profile or another source. A renamed legacy database is
handed to the existing password flow only by an explicit, once-only action
following `legacyProductDatabase`, proved `notCommitted` and complete cleanup.

## Transport and strict parsing

`catalogueJobsApi.ts` uses exactly the contract's
`/api/catalog/update/jobs[/admission]`, `/{id}`, `/{id}/upload`, `/{id}/details`,
`/{id}/apply` and `/{id}/cancel` routes. Create has capability-only headers;
other operations carry the exact capability/incarnation/serial header trio.
Details bind artifact ID, full public digest, canonical offset and limit 200.
Nothing puts capability data into a URL, snapshot, report, log or storage.

The existing `api.request` returns already-parsed JSON. Exporting that function
alone cannot support duplicate-key checks, fatal UTF-8 decoding or response-byte
admission. This slice therefore reads bounded raw responses in its dedicated
client and reuses the existing exported `noteRefusal` authentication notification.
It does not edit `api.ts`. If parent integration requires shared request usage,
it must first supply a raw-response transport seam; passing its parsed return
value to the strict parser would weaken the protocol and is not equivalent.

The catalogue-only parser consumes the checked-in wire/record schemas and
explicit policy state rules. It rejects decoded duplicate keys before object
construction, BOM, invalid UTF-8/surrogates, depth above eight, unknown/missing
keys, invalid nulls/enums, numeric bounds and cross-field inconsistencies.
Canonical U64 strings use grammar checks and `BigInt`, never `Number` or
`parseInt`. Status and error envelopes are limited to 16 KiB; detail pages to
256 KiB/200 records, with actual individual-record spans limited to 64 KiB.
Source/profile, exact handle, immutable preview/result/descriptors, command,
revision and lease continuity are checked before publishing status. Validated
lower revisions cannot repaint a newer snapshot; changed equal revisions fail
closed. Cancellation replies must agree with their actual gate/terminal state.

## User-visible authority and bounded lifetime

- Standard/large selection and LARGE risk acknowledgement precede allocation;
  new File selections reset opt-in even with equal names, sizes and timestamps.
  The 8 GiB input / 64 GiB main-growth cliff and non-aggregate disk/RSS and
  cooperative-only cancellation boundaries are disclosed before consent.
- Start uploads once. Neither effects, StrictMode, a dropped reply nor a remount
  starts another create/upload/apply. Explicit lost-create recovery repeats only
  the exact original offer/capability/metadata, without uploading afterward.
- Cancellation intent during create is retained locally and labelled as **not
  yet acknowledged**. If creation is already in flight, its known handle is
  cancelled instead of uploaded; a lost create still requires explicit exact
  recovery. Before create allocation, a local cancellation simply abandons
  preparation. No local intent is a server cancellation witness.
- The apply checkbox binds the currently displayed immutable preview; apply
  uses one command and exact consent/artifact/digest. A failed/lost response
  permits status-only inspection, never an automatic reapply.
- A tracked `commitReturnedOk` increments one root catalogue revision once,
  regardless of leaf subscriptions. This is separate from selected-view paint.
  The notification bit lives only in the current retained record; no eternal
  delivered-ID set exists. Disposal rejects late replies by operation identity.
- The report exposes all 17 public record kinds, including conflicts,
  provenance withholding, unavailable history and full baggage evidence. It
  uses existing importer/report CSS shells, not lossy conversion to older
  number-based DTOs. Paging retains only one page, exposes exact offset/total
  and next/previous navigation, and never calls the first page the full report.
  Signature members explicitly mean stored, **not cryptographically verified**.
- Commit, cancellation request, settlement, cleanup/quarantine and backup
  publication/retention are separate labels. Unknown commit is not rollback;
  witnessed commit survives cleanup failure. Explicit report disposal never
  deletes a user-managed recovery backup.
- View unmount stops subscriptions only. Root-owner teardown aborts its HTTP
  transport and polling, not server work. `beforeunload` warns while active.
  App/process loss, restart and retention forgetting can lose volatile outcome
  authority. Explicitly forgetting unavailable history keeps a visible warning
  before a new selection; it does not recreate, resume or stop a former worker.

## Acceptance boundary

Focused tests use synthetic DTOs and intercepted fetches only. They cover strict
wire admission, large-profile acknowledgement, lost responses, cancellation
before a create reply, truthful settlement/quarantine, exact operation identity,
StrictMode, actual wizard step/whole-parent detachment, origin warnings and
hidden catalogue/open-backup invalidation. Existing picker, wizard, catalogue,
backup and German du-form tests remain relevant compatibility controls.

Parent integration must mount and exercise the real runtime routes with the
production managed catalogue owner, complete artifact paging, actual native
commit/cancel/cleanup evidence and server/desktop lifetime drain. Its one final
integrated review/gate and built-browser layout acceptance remain separate;
frontend mocks and a successful frontend build do not establish them.
