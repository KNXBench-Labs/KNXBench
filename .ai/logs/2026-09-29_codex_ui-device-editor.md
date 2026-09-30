# U11 / ISSUE-09 device editor — 2026-09-29

## Scope and evidence

The KNX Association *Project Schema23* §1.2.4–1.2.5 nests a device octet below its area and line; the Association's offline project check reserves a device octet of zero for an identified coupler. `docs/RESEARCH.md` §20.2 records the primary sources, the existing importer mapping, the absence of a normalized coupler discriminator, and the resulting fail-closed editor policy. This is a project edit, not a bus transaction.

- `Inspector.tsx` displays full localized communication-flag names beside the standard letters, plus a line-derived, read-only area.line prefix and editable device octet. An imported mismatch remains visible and repairable; ambiguous ownership disables the control. The prefix is accessible as the number input's description.
- `domain.rs` accepts additive `Both` on existing link/unlink routes, applying separate Send and Receive domain links as one `Command::Batch`. Core tests prove single undo and late-failure rollback; HTTP and UI tests pin the contract and independent directions.
- `command.rs` and `validation.rs` reject a new unclassified line-bound `.0`, duplicate address, out-of-line address, line with two owning areas, or a device in multiple topology placements (two lines, repeated in one line, both assigned and unassigned, or repeated unassigned across installations). A move validates the old address, unique placement and target line before mutation and never readdresses automatically. The undo-only `RestoreIndividualAddress` inverse preserves imported `.0` and mismatched addresses; `knx-store/src/command_sync.rs` handles that inverse when incremental syncing is used.
- `styles.css` wraps long translated tabs and group-link rows; `e2e/device-editor-layout.e2e.ts` mounts the real Inspector through a Vite-only static fixture, intercepts `/api/**`, and checks English/German at 360, 640 and 1440 px with long labels. Run from `apps/knx-web` with `npx playwright test --config playwright.issue09.config.ts`; it starts only local Vite on 127.0.0.1:4796. No server/tunnel or device was used.

## RED/GREEN and review

The paired-link HTTP regression was RED before server support. The line-owner ambiguity tests were RED before `unique_line_owner`; an accessible-prefix test was RED before `aria-describedby`. The mixed line/unassigned and cross-installation unassigned tests failed on the formerly accepted mutations, then passed with placement validation. The repeated-reference regression detected deliberate guard removal in both core and UI, then passed after restoration. Focused green runs: `knx-core --lib` 610, `knx-store --lib` 80, the full `http_edit_routes` suite 19 (including paired-link and address-repair/undo), Inspector 24, Chromium six. The 78-file web suite passed 1,213 tests plus TypeScript and production build. Earlier negative mutations of the area-owner, multiple-line, later-installation, prefix, zero-octet, move-preflight, narrow-tab and UI ambiguous-owner guards also failed as intended; each guard was restored and rerun green.

The in-session cross-layer review found and closed an ambiguous two-area topology owner, the missing inverse storage-sync arm, narrow German tab overflow, and the inaccessible prefix. A later read-only core/UI diff review found **one important data-integrity defect**: a device listed both on a line and as unassigned could be moved into a second placement. The new core preflight counts placements across all installations and the UI disables the ambiguous editor; RED/GREEN tests also cover repeated references within one line. No unrelated refactor or silent topology repair was added. Full post-review branch and merged-result gates are still pending. External independent reviewers were not spawned (the user prohibits subagents in this KNXBench session). Static added-line scan before the last guard change: 0 sensitive literals, 0 unsafe HTML, 0 dangerous eval, 0 shell injection, 0 interpolated SQL; repeat it before commit. Existing limitations: line moves and new group links are still restricted to the first installation; native WebKitGTK and real screen-reader runs remain unverified.

The four repository gates also passed on this worktree: `check-layering`,
`check-headers` (311 well-formed / 161 headerless, at ceiling),
`check-anchors` (397 links / 215 files, none dead), and
`check-corpus-gates`. The corpus symlink points at the read-only root
product corpus; no package data was modified. The pre-review
workspace gate passed fmt, strict Clippy and 125 Rust suites / 2,581 tests
(0 failures, 148 ignored, no corpus-skip notices). It predates the
mixed-placement fix, so it does **not** replace the final branch gate.

## Pending before delivery

Run the final corpus-backed workspace fmt/Clippy/tests and four `xtask` gates on the finished branch. Fetch/rebase against current `origin/main`, retain both sides of any documentation/handover conflict, re-run gates on the actual fast-forward result, publish and read back the remote ref. Only then release the web lock, clean this worktree/target/scratch, and start U12 (ISSUE-05 then ISSUE-08 UI half) once its prerequisites are verified. There is no need for a quota check per the user's later instruction.
