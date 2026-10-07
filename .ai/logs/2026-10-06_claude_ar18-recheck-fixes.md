# AR18 re-check fixes (Claude, 2026-10-06)

Re-check b4a23730 (branch review/alpha-recheck): READY_WITH_CONDITIONS,
R1 = N1, R2 = N2, R3 = regate. MINOR N3–N6.

- 79bee3c6 container.rs: `check_member_names` returns the record count;
  `check_decoded_names` (archive.len() == records, else NameCollision;
  ASCII-case-insensitive uniqueness of decoded file names) in open_raw and
  for the nested payload; nested declared total checked before the loop.
  Test file decoded_member_names.rs has its own stored-ZIP writer (flags,
  extra fields, declared sizes independent). 5/7 RED first.
- 30a5cfb4 server: `AppState::new` = no product DB; `with_user_product_db`
  in server main + desktop lib; `load_failure_kind` maps file-caused
  ImportFailure to `projectNotImportable` (422).
- 9118c884 CLI restore: record_never_connected.
- 3ede4817 test header width (first gate red only on check-headers).

N6 not changed: mount-time GET /api/project in App.tsx would shift many
App.test.tsx call-count assertions — UI owner. KL §82 dated update.
Mutants 6/6. Gate 3ede4817 green; evidence ar18-recheck-fixes-20261006.
