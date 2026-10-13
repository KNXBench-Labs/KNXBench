# Project-local name editing

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-project-local-name-editing)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Project-local name editing is not bulk authoring (ADR-0101)

Current source adds guarded device-instance and individual group-address names
in editor/Properties and Explorer/list F2/context menus. This is newer than
alpha.6, not a published release/deployment. New names are bounded to 1,024
Unicode scalar values and exclude blank/control/line-separator input; valid text
is not trimmed or normalized, display names may repeat, and imported exceptions
stay preserved and exactly undo-restorable. Old/offline projections without a
server/project/revision context cannot author names.

No bulk name templates/find-replace, new CLI authoring, MCP writes or .knxproj
export. Renames never change numeric addresses, flags, DPTs, links, product data
or hardware evidence. Frozen captures/exports stay historical. Failed drafts
remain visible; read-only refresh and explicit reapplication are not a blind
network retry. Existing native history limits/save semantics remain binding.
See [ADR-0101](../adr/0101-project-local-device-and-ga-names.md) and
[the contract](../contracts/project-name-editing.md); no native/Orca/private-corpus
or hardware acceptance follows from browser tests.
<!-- END retained catalogue record -->
