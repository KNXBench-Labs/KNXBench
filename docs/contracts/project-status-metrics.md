# Project status storage metrics (UI-04)

The Overview combines two read sources. Project content counts come from the
accepted `ProjectTree` snapshot already used by the workspace. The dashboard
walks each installation once: areas, lines, group addresses, assigned and
unassigned device instances, and each device's communication-object count.
Building placement does not count a device again. These are project entities,
not catalogue products.

`GET /api/project/status-metrics` is an authenticated, read-only measurement.
The response names `serverIncarnation`; its `project` result names the
`projectIncarnation` and `snapshotRevision` of the open project. The client
admits the reply only against all three tree identities. `available.currentOnDiskBytes`
is the current on-disk size of the open project's native file (may include
unsaved recovery-journal growth), selected by the last successful Save As or
native open. It does not mean the size at the last save.
The unsaved-change flag comes separately from `ProjectTree.is_modified`. A
recovery journal may change the native file's size before a formal save; the
figure remains the measured file size at read time, never an estimate of a
future save. A new or ETS-imported project without a native save path reports
`notSavedYet` with null `currentOnDiskBytes`. A missing file reports `error`
with null `currentOnDiskBytes`. With no project open, the state is `unavailable`
and both project identity fields are null.

A measurement can occur between a save's revision bump and its manufacturer-file
carry into the native file. That reply can contain the interim file size. A
single client receives the save response after the carry finishes and then
refreshes its tree, so its subsequent measurement sees the post-carry file.

The independent `catalogue` result describes only the active product database
connection. `available.bytes` sums the main SQLite file and any currently
present `-wal`, `-shm` and `-journal` SQLite side files. It does not include source archives,
catalogue backups or other databases. The measurement reads file metadata and
never checkpoints, vacuums or writes. Counts use one aggregate query, no rows
returned: manufacturers are distinct IDs owning a product or application
program, while products and application programs are canonical table counts.
A `catalog_item` placement is not a product.
An absent database reports `unavailable`; a database whose metadata or count
query fails reports `error`. Empty tables are `available` with zero counts.

No path or raw file error leaves the server in this response. The client
validates enum states, safe integer bytes/counts and null fields before
formatting. Malformed replies show an error. The UI shows `loading` on an
initial read and marks the group busy while fetching. It keeps measured values
during a same-incarnation revision refresh. A revision mismatch invalidates
the project-file value while independent catalogue values remain usable; a
different server or project incarnation shows one stale notice.
Sizes use decimal MB (`bytes / 1,000,000`) with one locale-formatted decimal;
values below 100,000 bytes display as less than 0.1 MB. The endpoint is
refreshed on an Overview mount and an accepted project snapshot change, so
catalogue changes made in the catalogue workspace appear when Overview opens.
