# Catalogue batch creation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-catalogue-batch-creation)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## U11 catalog batch scope (ISSUE-07)

The product catalog can request 1–32 devices in one project command/undo
step. Generated names use the entered base followed by a one-based index for
multiple devices; by default the application does not deduplicate against
pre-existing names. A selected line places the devices in that topology line,
and by default **no physical addresses are allocated**. Two opt-in request
flags (MODEL-04, server side) change that: `uniqueNames` skips names already
used in the project, and `allocateAddresses` (requires a line) gives each new
device the lowest free device octet 1–255 on that line in the same undo step,
skipping octet 0, every address used anywhere in the project and the project
exclusion list (`knx_core::free_line_addresses`). Too few free addresses
refuse the whole batch before any ID is reserved. The allocator knows only the
project, not devices on the real bus. Since 2026-10-04 the web catalog offers
both flags as unchecked checkboxes; address allocation is disabled without a
target line. This is a local project edit, not a KNX download or
ETS-compatibility claim.

An older server may ignore the additive `quantity` field and return a legacy
single-device response. The web client then refreshes the returned project,
warns that the requested batch was not confirmed, and does not retry. It
cannot roll back a partial change already made by a different server. For a
current server, quantity/name/line refusal happens before IDs are reserved;
the core `Batch` command rolls back project changes if one child fails.
If a response is lost after the request was sent or the server responds with
an internal error, the client cannot know whether the batch was committed;
it blocks a blind retry and directs the user to inspect or reload the project.
Since ADR-0069 the server accepts an optional `requestId`: an identical resend
replays the recorded outcome (`replayed: true`) instead of applying again, and
the same ID with other content is refused. The record is in memory, bounded to
256 requests and cleared when the project is replaced; a server restart forgets
it. Since 2026-10-04 the web client sends one `requestId` per submit. After a
lost or 5xx response it treats the batch as unconfirmed for any quantity and
offers **Retry safely** with the same request, but only after checking that
the server's `server_incarnation` is unchanged; a restarted server gets no
retry, only the request to inspect the project. A newer web client against a
pre-ADR-0069 server would have its retried batch applied again; the desktop
app and `knx-server` serving its own bundle always pair matching versions.
Successful batch responses carry per-device diagnostics; a late core batch
failure carries the zero-based child command index and its typed cause, which
the catalog maps to the one-based device number (the reservation is child 0).

Catalog creation checks the available `u32` device and communication-object
ID ranges for the full requested quantity before calling the core allocator.
AR02 also makes all nine shared core allocators fallible: `u32::MAX` is the
last valid ID, and later allocation returns `IdAllocationError` without
changing a counter. General creation, CSV planning, offline reconciliation
and ETS mapping propagate refusal rather than wrapping or panicking. CSV and
reconciliation discard a refused detached batch; editing an existing parameter
does not allocate a replacement ID. Maximum-ID native roundtrip and monotonic
undo/redo regressions are in `knx-app/tests/id_exhaustion.rs`. This does not
activate parked ADR-0039 phases 3–5 or claim every failed single edit leaves
its successfully reserved high-water mark unchanged.

Headless Chromium checked the catalog at 1440×900 and 400×700 against a
local Vite page with mock catalog data, not a real manufacturer package. A
native WebKitGTK run and real screen-reader announcements are not verified.
<!-- END retained catalogue record -->
