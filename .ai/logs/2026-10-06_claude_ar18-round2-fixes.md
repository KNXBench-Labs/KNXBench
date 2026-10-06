# AR18 re-check round 2 fixes (Claude, 2026-10-06)

Round 2 c4ac6b37/a6aa0063 (branch review/alpha-recheck-2):
READY_WITH_CONDITIONS, R4 = N7, R5 = regate; MINOR N8–N10.

- 901933a1 container.rs: `check_member_names` returns raw central names;
  `check_decoded_names(archive, raw_names, bytes)` adds: directory record
  with data → InconsistentRecord; directory named like a file →
  InconsistentRecord; local header name (at zip's header_start, which
  includes the prefix offset) != raw central name → InconsistentRecord.
  Outer check now runs on the probe archive. The test writer gained
  `local_name`, `alias_of` and `zip_after(prefix)`. 6 RED first.
  A separate shared-local-record check was removed: its mutant survived
  because the local-name check already covers it.
- a16141c6 server: Io(NotFound) → projectNotImportable; doc comment.
- 754a66dd three server tests had pinned the 500 of a missing file.
  LESSON: after changing an HTTP status, run the whole crate, not one file.

Gate 754a66dd green; evidence ar18-round2-fixes-20261006.
