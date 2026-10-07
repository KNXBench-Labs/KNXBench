# UI track U0 — baseline (2026-09-28)

- Created isolated worktree `ui-baseline` from `origin/main` at `0f7ed0b`; the root checkout has only the foreign untracked `docs/paperclip-shutdown/`, left untouched.
- `npm ci`, `npx tsc --noEmit`, `npx vitest run`: 71 files / 1105 passing tests.
- Built `apps/knx-web/dist` before the workspace gate. The initial Clippy attempt failed because a fresh worktree has no `dist` for Tauri's build script; retry after `npm run build` passed without source changes.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast -j 4`: 121 suites, 2350 passed, 0 failed, 135 ignored. The default run does not exercise ignored corpus tests.
- `xtask` layering, headers (267 valid / 161 unheaded at ceiling), anchors (393), corpus gates: passed. `git diff --check`: passed.
- No application or hardware operation. Next: U1 independent ADR-0038 review and user decision; then U2 diagnosis/research. `apps/knx-web` lock remains released.
