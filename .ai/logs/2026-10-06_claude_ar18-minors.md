# AR18 minor findings M1–M9 (Claude, 2026-10-06)

User: "pruefe erneut und setzte den rest um". No re-check branch existed;
the UI owner's `origin/ui/style-hint-dpt-outcome` (5d648560) was merged into
the candidate first (d7b4b4fd, clean).

- 7bb3e12a store: `open_existing_and_migrate` (no create; refuses
  ForeignDatabase / NothingSaved / NotFound before migrating),
  `is_own_or_empty` also guards `migrate()` for writers,
  `save_project_with_passthrough` (one tx; `write_opaque`/
  `write_manufacturer_refs` crate-private bodies). Callers switched: CLI
  readers, comparison, device_serial, server `load_native` (LoadFailure kind
  `projectNotOpenable` → 422). CLI import stages in memory, writes --store
  via `write_imported_store`. `AppState::default` → `with_product_db(.., None)`.
- 9cb293d8 etsproj: `Container::other_project_parts` + report line.
- a86b7ddd download: `DownloadGuard::record_never_connected` (failed/no/
  returnedError), wired in route and CLI; harness `refuse_connect`.
  Web history validator already accepts failed+no+returnedError.
  NOTE for the commissioning owner: new terminal path, same schema.
- 519633e0 tools/run_corpus_tests.py (+tests), VERIFICATION section, npm
  audit fix (source-map-js 1.2.2).
- faa3955f clippy needless_borrow (first gate run red only on clippy).

Mutants: 15 first run, K3/K6 survived → two store tests → 17/17.
Gate g/gate.sh on faa3955f all green; evidence `ar18-minors-20261006`.
