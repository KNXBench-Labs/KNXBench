- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 11:20
- **Completed:** Task 4 of `docs/superpowers/plans/2026-09-09-standalone-product-database-install.md`
  ("Make catalog creation and diagnostics honest in the web flow"), on
  branch `codex/catalog-creation-diagnostics` (base `41cb144`, Tasks 1-3
  already committed there: first-winner catalog provenance, the atomic
  standalone `.knxprod` package installer, CLI/HTTP install endpoints).
  Resumed and finished a prior agent's uncommitted Task-4 draft rather
  than restarting: `knx_productdb::query::resolve_catalog_item_program`
  now validates the full catalog item → product → hardware →
  hardware2program → program chain before `create_device_impl` builds a
  `Command::CreateDevice`; only a hardware row that explicitly declares
  itself programless may skip program seeding, every other dangling
  relation is a typed 400 with no command ever applied. `POST
  /api/devices` now returns `CreateDeviceResponse { tree, diagnostics }`
  covering `ProgramlessProduct`/`AmbiguousDpt`/`ComObjectRefMissing`/
  `DynamicOrModuleNotEvaluated`. `CatalogBrowser.tsx` gained an install
  file-picker (`installProductPackage`), install-report/error display,
  a post-install catalog refresh, and in-modal diagnostics rendering
  with a "Done" button instead of auto-close when diagnostics exist.
  Fixed one incorrect test assertion in `http_device_routes.rs`
  (`!can_undo()` after CreateDevice-then-DeleteDevice — `do_command`
  always pushes an inverse, so two undoable entries remain; confirmed
  by reading `CommandStack::do_command` and by reproducing the same
  result unmodified at HEAD `41cb144`). `Command::CreateDevice`/
  `DeleteDevice` semantics themselves were **not** touched.
  Tests: `cargo test -p knx-server --test http_device_routes --test
  http_product_install` → 9/9 + 3/3 green;
  `cd apps/knx-web && npm test -- CatalogBrowser api.test.ts && npm run
  build` → 34/34 tests + tsc/vite build green; `npm test` (full web
  suite) → 96/96 green. Committed as three focused commits: `style:
  cargo fmt incidental workspace formatting`, `feat(catalog): report
  product install and creation diagnostics` (server/productdb),
  `feat(catalog): install product packages and show diagnostics in the
  browser` (web).
- **Pending/Next Steps:** Task 5 of the same plan — reconcile
  `docs/IMPLEMENTATION_STATUS.md`, `docs/KNOWN_LIMITATIONS.md`,
  `docs/ROADMAP.md`, `docs/GAP_ANALYSIS_ETS.md`, `docs/COMPATIBILITY.md`
  against Tasks 1-4's actual, tested behavior, then the full gate:
  `cargo fmt --all --check && cargo clippy --workspace --all-targets --
  -D warnings && cargo test --workspace && cargo run -p xtask --
  check-layering && cd apps/knx-web && npm test && npm run build`.
- **Notes for Codex:** Two pre-existing, unrelated gaps noticed while
  verifying this task (neither introduced by Task 4, both reproduced
  identically at base commit `41cb144` before this branch's work):
  (1) `cargo clippy --workspace --all-targets -- -D warnings` fails on
  `crates/knx-core/src/command.rs` (`bool_assert_comparison` at three
  pre-existing test assertions) and on `crates/knx-etsproj/src/parse/
  installation.rs` / `installation_v21.rs` (`large_enum_variant` on
  `Frame`) — Task 5's full-gate step will need to fix these or they'll
  block it; (2) `cargo test -p knx-productdb --test standalone_packages
  installs_the_readable_corpus` fails locally with "corpus fixture ...
  unavailable" because this worktree has no `KNXBENCH_PRODUCT_CORPUS`
  env var / `OriginalData/ProductDatabases` fixture directory — needs
  that path set before Task 5's `cargo test --workspace` gate, or the
  gate will report a false regression. This worktree's `apps/knx-web/
  dist/` is a build artifact directory with only a tracked `.gitkeep` —
  running `npm run build` deletes it; restore with `git checkout --
  apps/knx-web/dist/.gitkeep` before committing if you build locally.
