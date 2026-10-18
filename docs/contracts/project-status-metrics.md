# Project status storage metrics (UI-04)

The Overview combines two read sources. Project content counts come from the
accepted `ProjectTree` snapshot already used by the workspace. The dashboard
walks each installation once: areas, lines, group addresses, assigned and
unassigned device instances, and each device's communication-object count.
Building placement does not count a device again. These are project entities,
not catalogue products.

`GET /api/project/status-metrics` is an authenticated, read-only measurement.
Its `project` result names the `projectIncarnation` and `snapshotRevision` of
the open project. The client displays it only beside that same tree snapshot.
`available.bytes` is the metadata length of the open project's current native
`.knxdb` file on disk, selected by the last successful Save As or native open.
The unsaved-change flag comes separately from `ProjectTree.is_modified`. A
recovery journal may change the native file's size before a formal save; the
figure remains the measured file size at read time, never an estimate of a
future save. A new or ETS-imported project without a native save path reports
`notSavedYet` with no bytes. A missing file reports `error` with no bytes.

The independent `catalogue` result describes only the active product database
connection. `available.bytes` sums the main SQLite file and any currently
present `-wal` and `-shm` sidecars. It does not include source archives,
catalogue backups or other databases. The measurement reads file metadata and
never checkpoints, vacuums or writes. Counts use one read query over the
canonical `manufacturer`, `product` and `application_program` tables; a
`catalog_item` placement is not a product. The query does not enumerate rows.
An absent database reports `unavailable`; a database whose metadata or count
query fails reports `error`. Empty tables are `available` with zero counts.

No path or raw file error leaves the server in this response. The UI also shows
`loading` while the request is pending, a localized error if it fails, and an
explicit unavailable state if the answer belongs to a different project
snapshot.
Sizes use decimal MB (`bytes / 1,000,000`) with one locale-formatted decimal;
values below 100,000 bytes display as less than 0.1 MB. The endpoint is
refreshed on an Overview mount and an accepted project snapshot change, so
catalogue changes made in the catalogue workspace appear when Overview opens.
