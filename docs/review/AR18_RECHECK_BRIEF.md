# KNXBench Alpha — brief for the independent re-check of the AR18 conditions

You are checking whether the findings from the independent AR18 review —
the four IMPORTANT ones (F1–F4) and the MINOR ones (M1–M9) — were really
fixed. The fixes were made by the release owner. **You did not make
them**, and the reviewer asked that someone other than the owner check them.
Start fresh, with no history in this code.

Read first:
- [the review](2026-10-06-alpha-independent-review.md): verdict, findings
  F1–F4, conditions C1–C5;
- [the original brief](AR18_REVIEW_BRIEF.md) §2: ground rules (read-only,
  offline, no hardware, private corpus only in aggregate, both leases, no
  subagents, commit identity).

They all still apply.

## 1. Candidate

| Item | Value |
| --- | --- |
| Revision | `origin/main` at the commit that last changed this brief. The product code is `faa3955ffd382763e6a7e4ad3a2ef1700b38844e`; `git diff --name-only faa3955f HEAD` lists only documentation and handover files |
| Fix commits | `7e606e55` (F2, F3, M4, M5), `55badf3c` (F4), `94bdd7bd` (F1), `64badb99` (F2 inside a protected payload); `7bb3e12a` (M1, M2, M8), `9cb293d8` (M3), `a86b7ddd` (M6), `519633e0` (M7, M9), `faa3955f` (clippy). The UI owner's `5d648560` is merged too |
| AppImage | `/home/knxbench/.hermes/profiles/knxbench/evidence/alpha-release/ar18-minors-20261006/KNXBench_0.1.0-alpha.4_amd64.AppImage`, SHA-256 `41ad3880da01a8964deefbe80517faa9f85d2bf8b5cc63b3fc7beae8020285f7` (copy it before running it) |
| Owner's gate dossier | [ALPHA_FINAL_GATES §7](../ALPHA_FINAL_GATES.md#7-ar18-conditions-c1c5-fixed-and-re-gated) and [§8](../ALPHA_FINAL_GATES.md#8-ar18-minor-findings-m1m9-fixed-and-re-gated) |

## 2. What to check

1. **Each fix against its finding**, by running your own adversarial inputs
   again, not the owner's tests:
   - **F1:** Open and Import over unsaved edits, in the UI and through the
     API; the `discardChanges` path; a password-protected import after
     "discard"; and a refusal that arrives late.
   - **F2:** exact-duplicate and case-variant members, including in a
     ZipCrypto-protected nested payload.
   - **F3:** a member that lies about its size, the 512 MiB total, peak
     memory on your earlier bombs, and `knx products ingest` on the
     project-archive route.
   - **F4:** `--store` on an existing file with and without `--replace`;
     whether the refused file stays byte-identical.
2. **New problems the fixes may cause.** For example: is a legitimate real
   project now refused (run the corpus subset)? Does a script that relied on
   re-importing into one store break? Does the 409 collide with the other
   `409` (a load already running)? Does the dialog appear when the project
   is clean?
3. **Gates and artifact:** reproduce the dossier's gate table on the
   candidate, and the AppImage checks (`check-appimage`, no builder home
   path, offline start).
4. **MINOR findings M1–M9**, each against its fix (§8 of the dossier):
   - **M1:** a failed import into a new or old store; a foreign SQLite file
     with and without `user_version`; an empty or v3 store through *Open*,
     `doc-export` and `ga-export`; a mistyped path. Do the files stay
     byte-identical?
   - **M2:** a Save interrupted between the tables.
   - **M3:** a two-part archive's report.
   - **M6:** a download against an unreachable gateway, through the API and
     the CLI, and the history entry it leaves.
   - **M7:** `tools/run_corpus_tests.py` with and without the corpus.
   - **M8:** whether any test still reads `~/.local/share/knx`.
   - **M9:** `npm audit`.

   Also: does refusing foreign or empty files reject anything a user could
   legitimately open?

## 3. Output

Write `docs/review/<date>-alpha-conditions-recheck.md` on a branch
`review/alpha-recheck`. It gives a verdict for the candidate: `READY`,
`READY_WITH_CONDITIONS` or `NOT_READY`. It contains:
- a table of F1–F4 and M1–M9 with fixed or not fixed and the evidence;
- new findings, rated CRITICAL, IMPORTANT or MINOR, each with `file:line`;
- the gate table;
- what you did not check.

## Round 2: N1–N6

The [first re-check](2026-10-06-alpha-conditions-recheck.md) returned
`READY_WITH_CONDITIONS` with conditions R1–R3. The release owner fixed them,
and fixed N3–N5 too
([ALPHA_FINAL_GATES §9](../ALPHA_FINAL_GATES.md#9-ar18-re-check-conditions-r1r3-fixed-and-re-gated)).
Round 2 is narrower. The rules of §1–§3 above still apply.

| Item | Value |
| --- | --- |
| Product code | `3ede481741acbfcd5904f67c80e443077d1151c0`; `git diff --name-only 3ede4817 HEAD` lists only documentation and handover files |
| AppImage | `/home/knxbench/.hermes/profiles/knxbench/evidence/alpha-release/ar18-recheck-fixes-20261006/KNXBench_0.1.0-alpha.4_amd64.AppImage`, SHA-256 `ca3101fb3b4584dacb1e2d768f455f35bab772f38009a9dc504968aebc78591e` |

Check:

1. **N1:** your Unicode Path, CP437/UTF-8 and `0.xml` substitution
   archives, outer and nested. Also the identity tricks the first round did
   not try:
   - local-header names that differ from central-directory names;
   - zip64 records;
   - directory entries that collide with file names.
2. **N2:** your nested bombs. What is the peak memory now?
3. **N3–N5** against their fixes.
4. **N6:** it was not changed. Say whether it should block.
5. **Regressions:** the corpus subset; real projects; product packages
   through `knx products ingest`.

Write `docs/review/<date>-alpha-recheck-round2.md` on a branch
`review/alpha-recheck-2`. Give the same verdict scale and include a gate
table.
