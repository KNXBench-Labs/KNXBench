# KNXBench Alpha — brief for the independent re-check of the AR18 conditions

You are checking whether four findings from the independent AR18 review were
really fixed. The fixes were made by the release owner. **You did not make
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
| Revision | `origin/main` at the commit that added this brief. The product code is `64badb9956a036affdf955677ff38f91a44fccfb`; `git diff --name-only 64badb99 HEAD` lists only documentation and handover files |
| Fix commits | `7e606e55` (F2, F3, M4, M5), `55badf3c` (F4), `94bdd7bd` (F1), `64badb99` (F2 inside a protected payload) |
| AppImage | `/home/knxbench/.hermes/profiles/knxbench/evidence/alpha-release/ar18-conditions-20261006/KNXBench_0.1.0-alpha.4_amd64.AppImage`, SHA-256 `235b00704cbde86c79c48ae3c202731fa0cfbe00a700c86e38d3d25f692d6bf7` (copy it before running it) |
| Owner's gate dossier | [ALPHA_FINAL_GATES §7](../ALPHA_FINAL_GATES.md#7-ar18-conditions-c1c5-fixed-and-re-gated) |

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
4. **MINOR findings M1–M3 and M6–M9** were not addressed, apart from the part
   of M1 that F4 and F1 now cover. Confirm they are disclosed or listed, and
   say whether any of them should block.

## 3. Output

Write `docs/review/<date>-alpha-conditions-recheck.md` on a branch
`review/alpha-recheck`. It gives a verdict for the candidate: `READY`,
`READY_WITH_CONDITIONS` or `NOT_READY`. It contains:
- a table of F1–F4 with fixed or not fixed and the evidence;
- new findings, rated CRITICAL, IMPORTANT or MINOR, each with `file:line`;
- the gate table;
- what you did not check.
