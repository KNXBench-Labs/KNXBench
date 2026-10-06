# AR18 conditions C1–C5 (Claude, 2026-10-06)

Input: independent review `37dd613a` (branch review/alpha-independent),
READY_WITH_CONDITIONS, F1–F4 IMPORTANT, M1–M9 MINOR.

Fixes (TDD, RED shown first for each):
- F2/F3/M4/M5 `7e606e55`: container.rs `check_member_names` (raw central
  directory walk, exact + ASCII-case duplicates of file members),
  `read_declared` (take(declared+1), refuse if longer), `check_total_size`
  (MAX_ARCHIVE_UNCOMPRESSED = 512 MiB, pub), `ContainerError::TooLarge`;
  test file crates/knx-etsproj/tests/archive_member_integrity.rs. Corpus
  measured first: 3 projects, max 22.3 MiB total, 5.5 MiB entry, 39 entries,
  no duplicates or case collisions.
- F2 nested `64badb99`: same check on the protected payload (found while
  drafting the re-check brief; first gate run stopped and restarted).
- F4 `55badf3c`: `--replace`; refusal before open/migrate/prompt.
- F1 `94bdd7bd`: domain `has_unsaved_changes` early check +
  `replace_unless_unsaved` (transaction guard), LoadFailure kind
  `projectUnsavedChanges` → ApiError::conflict (409 + kind); PathBody/
  ImportBody `discardChanges`; web dialog `.replace-confirm`, api
  `isUnsavedProjectConflict`, discard kept for the password retry. Two
  App race tests needed `mockReset` plus the dialog confirmation (their
  first project is modified by design).

Mutants: mut.sh 12 killed + S1/S3 survived (layered guards), S2 NOMATCH;
added the broken-file 409 assertions and the inner-guard domain test;
mut_server.sh S1–S3 killed; R7 (nested check) killed. 16/16.

Gate g/gate.sh on 64badb99: see docs/ALPHA_FINAL_GATES.md §7. Evidence
`ar18-conditions-20261006` (AppImage, manifest, logs, mutants, smoke).
