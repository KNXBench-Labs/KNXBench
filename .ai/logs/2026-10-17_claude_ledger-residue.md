# Ledger residue: MODEL-07 closed, twelve rows sharpened

Date: 17 October 2026. Agent: Claude. Worktree `KNXBench-ledger-residue-20261017`
(branch `docs/ledger-residue-20261017`), base `01c9e47b`, claim `0133044b`,
fast-forwarded to `8d33f967` after the leftovers merge.

## Scope

Close or sharpen every non-terminal ledger row: 1 `IN_PROGRESS` (MODEL-07),
2 `BLOCKED_EXTERNAL` (MODEL-05, IMPORT-02), 16 `LATER`. A scripted parse of
`docs/status/LEDGER.md` confirmed exactly those 19 rows. TOOLS-01…05 and
FUTURE-08 were out of scope and untouched. The project drive moved from
`<old-sources>` to `<sources>` mid-session; only this worktree's link was
repaired (`git worktree repair <path>`), other sessions' worktree links untouched.

## Phase 1 — MODEL-07 (`236c64ae`)

- `knx-diff`: `InstallationDiff::functions` (`FunctionKey`, `FunctionFields`,
  `FunctionMemberFields`). Matching through the existing engine: ETS id, then
  (building-part path, name); twins are ambiguous. Members compare by raw
  address/name/role. The old `functions` info line now only covers unplaced
  Functions (`unplaced_functions`), which a valid project cannot have.
- Renderers: CLI `~ function <name> in <path> (<id>)`, server DTO
  (`FunctionKeyDto`, `FunctionFieldsDto`), web `projectDiffView`/panel summary,
  MCP `diff_projects` entity `function`.
- MCP `list_functions` (ninth read-only tool) over `build_functions_view`.
- Desktop: ARCHITECTURE rule 3 forbids domain IPC, so the surface is the
  embedded server. `functions_surface_tests` calls the shell's `spawn_server`
  on a loopback port and drives the routes with `std::net` HTTP.
- Review (in session, before the gate): no CRITICAL/IMPORTANT. MINOR: key text
  duplicated in CLI and MCP (same as the existing device key; `knx-diff`
  renders nothing by design); Functions of a wholly added/removed installation
  are covered by the installation status, as for every other entity.
- Mutation sweep: M1–M8, first run 7/8; M4 (room dropped from the natural key)
  survived and was killed by the new test
  `same_named_functions_in_different_rooms_match_their_own_room`: 8/8.
- Focused results before the gate: knx-diff 50+10+4+3, knx-mcp 19 tools + stdio
  + writer, server diff 5 + functions 15, CLI diff 9, desktop 24, Vitest diff
  files 40, `tsc` + 20 derived configs, Clippy `-D warnings` on six crates.

## Phase 0/2 (`10262a6e`)

See `docs/status/2026-10-17-ledger-residue-review.md`. Findings worth keeping:

- `checked-parameter-editors` summary "web still sends unbound writes" was
  stale since B1a `d705a2b8`; corrected in `catalogue.json` + dossier, view
  regenerated.
- KNX Standard v3.0.5 (15 Sep 2026) exists but is manufacturer-only (MyKNX).
- support.knx.org "Differential Download" article: 403, API 401, no archive.
- The 13 Oct plugin-pair `.knxproj` exports are host-made (ToolVersion 0.0.0.0,
  no licence, no restore point): not IMPORT-02 evidence; one installation each:
  no MODEL-05 lift.

## Gate history

1. `ledger-gate1`: queued behind the parameter-workspace merged web gate; stopped
   by its recorded PID with zero stages run (that session pushes web runtime
   inputs on success).
2. `ledger-gate2` on merge `6dcbc04e` (base `faa9ba95`): fmt and workspace Clippy
   pass; workspace tests 4763 passed, 1 failed:
   `http_catalogue_update::a_backup_that_changed_since_it_was_listed_is_kept`.
   Reproduced on clean `main` `9a3acd3b` on ext4, passes on tmpfs. Cause: the
   backup `file_identity` hashed dev/ino/size/mtime and ext4 reuses a freed inode
   immediately (200/200 probe). Fixed in its own commit `270984b5` (adds ctime);
   5/5 green on ext4 and tmpfs. The only other `ino()` user
   (`activity_history_paths.rs`) compares two simultaneously existing files.
3. `ledger-gate3` on merge `b20d5b1e` (base `9a3acd3b`): fmt, Clippy, workspace
   tests 4764/0/211, ts-rs bindings, layering, headers, anchors, ledger,
   corpus-gates and docs all exit 0; tools tests failed 6+1 (2 in a plain
   environment). Cause: environment, not code: the profile scratch became a
   symlink into the new drive at 12:56, and two `browser_tmp` tests need
   `TMPDIR` to resolve under a directory named `scratch` (198/198 OK with a real
   one; `tools/` byte-identical to main). The parallel
   `fix/browser-tmp-symlinked-scratch-20261017` session fixes exactly that; the
   full web stage had not run yet.
4. `ledger-gate4` on `52d56d8b` (pre-rewrite id; with race fix `384c54ea`): Rust
   and repository stages green; tools tests failed 6 because this wrapper gave
   them a private leaf longer than the runner's own (deep-path fixtures no
   longer fit). Wrapper now uses `browser_directory`.
5. `ledger-gate5`: the web runner nested its leaf inside the wrapper's: same
   failure inside the web profile. Wrapper now starts the web runner with the
   profile's TMPDIR.
6. `ledger-gate6` on `52d56d8b`: every stage green, web profile 63/63, inputs
   frozen; push withheld: `accepted-not-pushed-main-moved`. Main had been
   rewritten (KL-185). Branch moved with `reset --keep` onto
   `refs/kl185-rewritten/heads/docs/ledger-residue-20261017` (`384c54ea`, code
   identical but one placeholder test string), merged main `a7e44714` as
   `867f5b23` (ledger: main's rows + my notes; ids my texts introduced remapped
   via COMMIT_MAP). Delta acceptance on that tree recorded in the review.

Lesson: a gate wrapper must not wrap the web runner's temp directory; let it
allocate below the profile's TMPDIR itself.
