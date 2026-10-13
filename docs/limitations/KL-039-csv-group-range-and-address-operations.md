# CSV group-range and address operations

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-039)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="39-csv-import-never-re-addresses-deletes-or-manages-group-ranges"></a>
## 39. CSV import does not create or rename group ranges

**Limitation.** Explicit readdressing and deletion are implemented, but CSV
still does not create, rename, resize, or delete group ranges. New and moved
addresses are assigned to the innermost existing range whose bounds contain
their final address, or left without a range when none does. Removing a row
from a CSV still means nothing; deletion requires `Action=delete`. A
readdress target must be unoccupied in the pre-import project, so swaps and
"delete this target, then move into it" combinations are rejected rather
than made order-dependent.

**Cause.** Range CRUD is separately modelled structure
(`Command::CreateGroupRange`/`RenameGroupRange`), not a property of one
address row. Inferring range mutations from repeated `MainGroup` or
`MiddleGroup` text would introduce ordering, boundary, rename and conflict
ambiguities. Those columns therefore remain derived/read-only.

**Impact.** Bulk address creation, rename, flag edits, stable-id readdressing
and unreferenced deletion are supported. A spreadsheet cannot reshape the
group-range hierarchy; that still uses the dedicated project editing
commands. Delete is refused while communication-object links remain.

**Lifted when.** A dedicated, versioned range-exchange contract defines
stable range identity, hierarchy and bounds without guessing from names.
<!-- END retained catalogue record -->
