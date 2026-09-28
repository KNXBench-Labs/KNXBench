# KNXBench cloud-session rules

You are running in a Claude Code **cloud session**: a fresh clone of the
GitHub repository on an Anthropic VM with 4 vCPUs, 16 GB RAM and 30 GB disk.
These rules add to `AGENTS.md`, which still applies in full. The task briefs
are in `docs/CLOUD_SESSIONS.md`.

1. **No private corpus.** `OriginalData/` is gitignored and absent. Corpus
   tests report as *ignored*, and that is correct. Never mark such a test
   "verified", never copy corpus-derived data into the repository, and never
   change a corpus test so that it passes without the corpus.
2. **No KNX bus, no LAN, no gateway.** Do not run live tests (`live_*`,
   `KNX_GATEWAY`, `KNX_WRITE_*`). Commissioning code is out of scope.
3. **One task, one branch, one pull request.** Work only on the task named in
   your prompt. Do not merge into `main`, do not push to `main`, do not
   rebase or delete other branches. Open a **draft** pull request whose title
   starts with the task id (for example `CT-1: …`).
4. **Handover.**
   - Do not edit `.ai/CURRENT_STATE.md`; it belongs to the local session
     that integrates your branch.
   - Write one log file `.ai/logs/<YYYY-MM-DD>_claude-cloud_<task-id>.md`.
     `.ai/` is gitignored, so add it with `git add -f`.
   - The log says what changed, which gates ran with which result, and what
     remains open.
5. **Commits.** Author is `KNXBench <github@knxbench.com>` (already
   configured). No `Co-Authored-By` or other AI trailer. Messages are
   concise, describe the actual change, and may carry a little humour.
6. **Gates before you call the task done.** Run them in this order and
   report each exit status honestly:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace -j 2`, which lowers linker memory pressure
     (16 GB)
   - `cargo run -p xtask -- check-layering`
   - `cargo run -p xtask -- check-headers`: grep for `headers ok` or
     `violation`
   - `cargo run -p xtask -- check-anchors`
   - `cargo run -p xtask -- check-corpus-gates`
   - For web changes, in `apps/knx-web`: `npm test` and `npm run build`
   - `git diff --check origin/main...HEAD`

   A command that runs longer than the tool timeout moves to the
   background: wait for it and read its real exit status. If a gate cannot
   run here, say so; never report it as passed.
7. **Documentation.**
   - Update `docs/IMPLEMENTATION_STATUS.md` and `docs/KNOWN_LIMITATIONS.md`
     when your change affects them.
   - Mark a limitation lifted only when the change actually lifts it.
   - Expect those files to change on `main` in parallel. Keep your edits
     local to the sections you own.
8. **Honesty.** Distinguish verified facts from assumptions. Never claim ETS
   compatibility. If the task turns out larger or different than its brief,
   stop at a clean boundary and describe the rest in the PR, rather than
   silently widening scope.
