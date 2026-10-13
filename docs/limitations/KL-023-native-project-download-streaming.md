# Native project download streaming

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-023)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 23. `/api/project/download` buffers the whole `.knxdb` file in memory

**Status, reconciled 2026-10-01 (AR00): resolved for whole-file buffering.**
The historical heading is retained for fragment links. Temporary SQLite
serialization remains; it is not a whole-file response buffer.

The historical description moved verbatim to [resolved limitations](../history/KNOWN_LIMITATIONS_resolved.md#23-apiprojectdownload-buffers-the-whole-knxdb-file-in-memory) (AR14D D5).
<!-- END retained catalogue record -->

<a id="earlier-resolved-description"></a>

<!-- BEGIN earlier resolved record -->
## 23. `/api/project/download` buffers the whole `.knxdb` file in memory

**Status, reconciled 2026-10-01 (AR00): resolved for whole-file buffering.**
The historical heading is retained for fragment links. Temporary SQLite
serialization remains; it is not a whole-file response buffer.

**Resolution.** The route freshly serializes the current in-memory project,
including unsaved edits, opaque entries, and manufacturer references, into
one temporary SQLite file. `tower_http::services::ServeFile` streams that
file in bounded 64 KiB chunks instead of copying it into a whole-file
`Vec<u8>`. Content type and attachment filename remain unchanged.

The response body owns the temporary path until it is dropped, including
after the HTTP response is split into its headers and body. Both completed
and abandoned bodies remove their temporary file; response extensions alone
would not guarantee this lifetime.

**Proof.** Unit tests consume a file larger than three small test chunks,
require multiple non-empty frames bounded by the configured chunk size,
compare every byte, and verify cleanup after completed and abandoned
downloads. The HTTP regression downloads an unsaved project and opens the
result as a KNX store, preserving its latest edit, installation, opaque
entries, and manufacturer references. Serialization still creates one
temporary SQLite file before streaming begins.
<!-- END earlier resolved record -->
