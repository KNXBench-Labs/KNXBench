# 2026-10-04 — Claude — AR13: privacy, authentication and provenance

## Scope
alpha-release-goal.md AR13 (KL-106, KL-22, KL-65), claimed by this Claude
session (`dd22d143`) on user request. Offline only: no TLS service, role
system, host/firewall configuration, credential or bus action. AR06/AR06P
untouched, no Web source change.

## Privacy (KL-106)
- Audit: five bundle files; `report.md`, `environment.json`, `log.json`
  redacted; `project-summary.json` counts only (existing test);
  `bus-telegrams.json` unredacted by design.
- Finding: the four classes were redacted in every channel, but `report.md`
  (also the GitHub issue body) did not say what survives, and described the
  telegram file as carrying addresses only — not its values (text DPTs
  included) and timestamps, which together reveal when the installation was
  used.
- Fix: `report.md` names every kept class and the telegram content;
  `describe_file` updated. Test
  `every_privacy_class_is_either_redacted_or_named_in_the_report` (one
  synthetic fixture per class through description, client facts, log
  message/location/detail). RED only on the warning; GREEN after.
- No MAC class added: no server path writes a MAC into the bundle; a MAC
  typed by the user is now named as kept.

## Authentication (KL-22)
- `every_declared_route_refuses_a_caller_without_a_session_except_the_documented_four`
  parses all `.route(` declarations in `apps/knx-server/src` (97 method/path
  pairs, 89 paths) and expects 401 except `/healthz` and `/api/auth/*`. The
  parser must also find the seven hand-listed and four open routes (no
  vacuous pass). All routes were guarded already.
- `main.rs` binds only `bind_address(auth_required)`; no env override.
  Manual covers TLS proxy, `KNX_AUTH_COOKIE_SECURE`, loopback fallback.

## Provenance (KL-65)
- New `crates/knx-build-stamp` (zero deps, build-dependency of knx-cli and
  knx-server) replaces two identical `build.rs` bodies. Development builds
  unchanged. `KNX_REQUIRE_CLEAN_TREE=1`: re-run every build (watch a path
  that never exists), refuse unless git confirms this workspace, `HEAD`
  resolves, `git status --porcelain` is empty (ignored paths excluded) and an
  explicit `KNX_BUILD_SHA` matches.
- End-to-end on the worktree: clean → `knx 0.1.0-alpha.1+g345d0bc5`; append
  one line to a source file → next release build fails naming the path;
  revert → green. With the always-re-run watch removed (committed mutant in
  a temporary local commit, reset afterwards) the edited tree built and was
  stamped with the clean commit — false provenance. Now pinned by
  `release_re_runs_every_build_and_development_follows_head`.
- ADR-0018 amendment; VERIFICATION.md "Release build provenance";
  ARCHITECTURE crate list.

## Mutants
Privacy 3/3, auth 1/1 (only the new exhaustive test catches an added
unguarded route), stamp 4/4 scripted + rerun-path 1/1. Every source restored
and md5-verified.

## Gates
Fresh per-worktree target, both leases: fmt, clippy -D warnings, workspace tests 169 blocks / 3,136 passed / 0 failed / 177 ignored, layering, headers 451/157 ceiling, anchors 392 links, corpus gates 363 files, cargo deny, Web tsc + vitest 1,739 + build, diff-check — all exit 0.

## Statuses and handoffs
KL-65 DONE; KL-22, KL-106 ACCEPTED_BOUNDARY. Web-lock holder: dialog string
`debugReport.privacyTelegrams` should mention values and timestamps. AR17:
build the candidate with `KNX_REQUIRE_CLEAN_TREE=1`.

## Process notes
- One patch round partly applied after `cargo fmt` had reformatted the
  target block; a parallel test job ran against that broken intermediate
  state (compile error, no false result). Fixed by a line-range replacement.
