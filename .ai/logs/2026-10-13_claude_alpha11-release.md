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
  docs/releases/alpha11.md). Gate: PENDING.
