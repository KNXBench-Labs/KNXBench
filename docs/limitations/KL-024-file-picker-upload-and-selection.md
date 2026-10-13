# File picker upload and selection

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-024)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 24. `FsPicker` has no drag-and-drop or multi-select

**Status, reconciled 2026-10-01 (AR00): resolved for multi-file upload/drop.**
The historical heading remains for fragment links. Single-project selection
is intentional and does not make the implemented upload gestures absent.

The historical description moved verbatim to [resolved limitations](../history/KNOWN_LIMITATIONS_resolved.md#24-fspicker-has-no-drag-and-drop-or-multi-select) (AR14D D5).
<!-- END retained catalogue record -->

<a id="earlier-resolved-description"></a>

<!-- BEGIN earlier resolved record -->
## 24. `FsPicker` has no drag-and-drop or multi-select

**Status, reconciled 2026-10-01 (AR00): resolved for multi-file upload/drop.**
The historical heading remains for fragment links. Single-project selection
is intentional and does not make the implemented upload gestures absent.

**Final-review hardening, 2026-09-22.** Duplicate basenames are explicit 409
conflicts, including pre-existing upload files; no destination is overwritten.
The earlier successful count and filename/error remain visible on partial
failure. Closing/selecting/unmounting stops the remaining queue. A reopened
picker waits for the previous in-flight request, which may still finish on the
server; closing is not a rollback of that request.

**Resolution, 2026-09-22.** The browser picker still returns one
`Promise<string | null>` path because project open/import remains a singular
human choice. Its local file input now accepts multiple files, and its upload
label accepts native file drops. A shared routine sends each file to the
existing one-file `/api/fs/upload` route sequentially, refreshes the
`uploads` listing after successful requests, and announces a completed batch
as a polite status. It never auto-selects an uploaded project.

**Failure handling.** The first failed request stops that batch, keeps earlier
successful uploads intact, and names the failed file plus server error and
completed count. It does not announce batch success. During protected-mode
dragover the picker inspects only `DataTransfer.types`; it reads dropped files
only at drop time.
<!-- END earlier resolved record -->
