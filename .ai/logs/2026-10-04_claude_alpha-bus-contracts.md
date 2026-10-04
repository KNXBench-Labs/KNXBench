# 2026-10-04 — Claude — AR14: offline bus/CLI contracts

## Scope
alpha-release-goal.md AR14 (KL-29, KL-31, KL-62, KL-72–78, KL-102, KL-126),
claimed by this Claude session on user request (`f8a23218`) while the Codex
alpha session works AR06/AR06P. Offline only: fakes, local adapters, loopback
multicast. No gateway, no bus traffic, no Web source change.

## Defects found and fixed
1. **CLI ignored the project's group-address style** (KL-29, KL-62 item 13).
   `bus write --project` refused a free-style project's `2049` and accepted
   its three-level spelling `1/0/1`; monitor lines were always three-level.
   Now `--project` supplies style, names and DPTs in one `ProjectBusView`;
   without a project, and for `route-send`, three-level stays.
2. **Names of one raw address across installations: last read wins** — in
   the CLI and in the server's Group Monitor. New
   `crates/knx-core/src/group_address_names.rs`
   (`resolve_project_group_address_names`): every distinct name, installation
   order, joined by ` | `; empty names only when all are empty. Both
   consumers use it.
3. **Linux delivered every host-joined multicast group to each routing
   socket** (KL-31). `RoutingClient` binds `0.0.0.0:3671`; with
   `IP_MULTICAST_ALL` on (ip(7) default), a default-group client received a
   custom group's telegram from another client on the same host. Fixed with
   `set_multicast_all_v4(false)` (`#[cfg(target_os = "linux")]`). Other
   platforms unverified.

## Evidence
- RED: `cli_bus_address_style` 3/5 failing (free/two-level refused,
  free-project `1/0/1` accepted); server names test returned `Home light`
  (the third installation's name had overwritten the second's); custom-group test failed
  with the leaked frame.
- GREEN: CLI 5/5 + existing `cli_bus_dpt` 10/10; knx-core names 4/4; server
  `bus::` 15/15; knx-net `scan::` 17/17; knx-net `client::` 22/22 three times
  without skip; `http_bus_scan` 11/11; `http_bus_write` three-style round trip.
- Mutants: AR14a 7/7, AR14b 4/4, AR14c 5/5 — every source restored and
  md5-verified. Three of the four scan pins and three of the four
  reconciliation refusals were caught only by the new tests.
- Full gate (fresh per-worktree target, both leases): fmt, clippy -D warnings, workspace tests 166 blocks / 3,124 passed / 0 failed / 177 ignored, layering (448 packages), headers 449/157 ceiling, anchors 391 links, corpus gates 361 files, cargo deny, Web tsc + vitest 1,739 + build, diff-check — all exit 0.

## Statuses
KL-29 DONE; KL-31 BLOCKED_EXTERNAL (real custom-group router run); KL-62,
72, 73, 74, 75, 76, 77, 78, 102, 126 ACCEPTED_BOUNDARY as retained documented
boundaries with offline subcases verified. Dossier:
`docs/ALPHA_READINESS.md#ar14-offline-buscli-contract-dossier`.

## Notes
- Intended `bus write` behaviour changes: a given `--project` is read even
  next to `--dpt` (its style decides how the address is read), so a broken
  project path now fails before the address is checked. `--dpt` still wins
  for the encoding.
- Manual updated: `docs/manual/user-guide/10-command-line.md`.
- §102 correction: 41 per-type codec round-trip tests do guard symmetry at
  sampled values; the earlier "no regression test will notice" was too strong.
- Test-only addition: `FakeTransport::delay_replies` in `knx-net` scan tests.
- Display caveat: a name containing ` | ` itself is not escaped.
