# CT-6 — Project diff against a raw `.knxproj` in the web UI (cloud session)

Branch: `claude/tender-mendel-d52odl`. Brief: `docs/CLOUD_SESSIONS.md` CT-6.
Environment report: rustc 1.98.0, webkit2gtk-4.1 present, OriginalData absent,
identity KNXBench. Nothing MISSING.

## What changed

- `crates/knx-etsproj/src/report.rs`: `ImportReport::error_count()`.
- `crates/knx-app/src/comparison.rs`: `ComparisonInputKind` (`NativeStore` |
  `EtsProject`) with `of_path`; `load_comparison_input` uses it.
- `apps/knx-cli/src/main.rs`: private `error_count` delegates to the new method.
- `apps/knx-server/src/domain.rs`: `diff_project_impl(state, path, requested_kind)`
  now loads through `knx_app::comparison::load_comparison_input` (no second
  import path; replaces `load_native`), returns `ProjectDiffOutcome` or
  `DiffProjectError::{Rejected, ImportRefused}`. Loads without the project lock.
- `apps/knx-server/src/routes.rs`: `DiffBody {path, inputKind?}`; response =
  `{inputKind, importReport, importDiagnostics, infoChanges, installations}`
  (diff fields flattened, unchanged). Error-level import → 422 with report.
  Unknown/contradicting kind or unsupported extension → 400.
- `apps/knx-web`: picker filters (combined, knxdb, knxproj) via
  `projectDiffView.compareFilters`; `ProjectDiffImportDiagnostics.tsx`
  (collapsed `<details>`, count summary); refused import shown in panel;
  `api.importRefusal`; `requestError` keeps the parsed body. 9 EN/DE keys, CSS.
- Docs: KNOWN_LIMITATIONS §57 lifted (with remaining gaps), §60 note,
  IMPLEMENTATION_STATUS entry, manual 08-reports-and-diff, manual
  implementation-status and known-issues.

## Gates

All run in this cloud session, in order, on the final code:

| Gate | Exit | Result |
| --- | --- | --- |
| `cargo fmt --all --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean (first run flagged `result_large_err`; fixed by boxing the refused report) |
| `cargo test --workspace -j 2` | 0 | 2161 passed, 0 failed; corpus tests ignored (no `OriginalData/`) |
| `xtask check-layering` | 0 | layering ok |
| `xtask check-headers` | 0 | headers ok (245 with header, 161 without, ceiling 161) |
| `xtask check-anchors` | 0 | 389 links, none dead |
| `xtask check-corpus-gates` | 0 | ok |
| `npm test` (apps/knx-web) | 0 | 70 files, 1095 tests |
| `npm run build` (apps/knx-web) | 0 | built; the usual chunk-size advisory |
| `git diff --check origin/main...HEAD` | 0 | clean |

No `--exclude knx-desktop` was needed (WebKit present).

## Open

- Real ETS exports not tried (no corpus); browser and Tauri dialog not run.
- `importDiagnostics` omits inferred values / namespace disagreement (in raw
  `importReport` only). Comparison diagnostics are not written to the session log.
