# Alpha 11 release (claude-release-alpha11)

- Owner standing goal 2026-10-13: cut v0.1.0-alpha.11 after KL-170 part B; stop before
  anything public (tag, GitHub release, AppImage upload, Docker push) for an explicit go.
- Waited for part B's web lock with a standalone-marker watcher; it fired at 7ce003d8
  (part B: af0daffe, merge 0d1e24bf). Scope note drafted meanwhile from
  `git log v0.1.0-alpha.10..origin/main` and the status/ADR/KL sections, not from memory.
- Part B reported a pre-existing flake on main (`onboarding-guide.e2e.ts`, "first alpha
  start"). Reproduced on 7ce003d8: 5/20 red, all `Received: 4` for 2. Cause: the spec
  predates release notifications (55384982); `useReleaseUpdates` (default on) also reads
  `/api/version` per load, and its api.github.com request escaped the `**/api/**` fixture.
  Web lock taken (cf87e630); test-only fix 30c4bec3 (`releaseChecksEnabled: false` + GitHub
  routed to a recorded abort, the other specs' pattern), merge 2a43ad5c. Fixed spec x20:
  80/80; mutant (checks on again) 10/10 red; full fixture Chromium 477/477, 13 tsconfigs,
  inputs frozen. Pitfall recorded in docs/DEVELOPER_PITFALLS.md.
- Cutoff main: 2a43ad5c. Schema unchanged since Alpha 10 (store and model 14; no
  knx-store change, knx-core only gained the byte-order decoder), so no migration.
- Candidate: this commit (version bump to 0.1.0-alpha.11 in the four crates, web package
  and lock; `cargo update -w` touched only the four workspace packages; scope note
  docs/releases/alpha11.md). The handover entry inside it reads 15:22; written ~15:01.
- Gate on 758a112b (15:02-15:45, 44/44 stages, inputs frozen): Rust 4281/0/197, Vitest
  3302/0, Chromium 477/0, 18 configs 255/0, native 12/12, naming 4/4, corpus 144/144 in 31
  targets with both OriginalData and project_dump.json linked. No harness corrections.
- Dry runs 38401821252 / 38401825245 green on 758a112b (publish jobs skipped).
- Main moved during the gate (plug-in pair test; Codex limitations catalogue move): not in
  the release. Merge of the release branch into main (f9ba9cff) had one handover conflict,
  both entries kept; the catalogue move kept the old KL anchors (973 links, none dead).
- Owner go 2026-10-13. Tag object 44824586 on 758a112b; release created with
  --verify-tag --prerelease from the notes file (11 links -> blob/v0.1.0-alpha.11, all
  checked with git cat-file). Tag workflows 38412319954 / 38412319727 green.
- External: assets re-downloaded, checksums + digests equal; knx-mcp version; AppImage
  extracted, no libwayland-client; Mesa-pinned startup survived 12 s. Docker version ==
  latest digest (latest compared in the registry, not pulled), both platforms, revision
  label, anonymous pull + smoke test; user container identity unchanged.
- Receipt docs/evidence/release-alpha11-2026-10-13.json; docs pass in alpha11-docs worktree.
