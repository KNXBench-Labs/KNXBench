# CLI monitor formatting

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-029)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 29. `apps/knx-cli bus monitor` has formatting limitations

**Limitation.** The `knx bus monitor` subcommand, added in Session 6 Cycle 1,
always formats group addresses as three-level (e.g. `1/2/3`) regardless of
the project's configured style, and merges group-address names from all
installations into one flat namespace (last-seen wins on collision).

**Cause.** Deliberate scope decision for Cycle 1: the tool is built for
dev/smoke-testing use against the reference project, which has one installation
and uses three-level addressing throughout. Generalizing to multi-installation
projects and honouring the configured style requires mapping infrastructure
not needed for this cycle's verification workflow.

**Impact.** A project with multiple installations or a non-three-level
group-address style will see misformatted or incorrectly-merged names in the
monitor output. Data is not lost — telegrams still resolve by address internally
— only the human-readable label is approximate.

**Lifted when.** A future cycle adds full formatting respect and per-installation
name resolution, either bundled into a general bus-monitor redesign or as a
targeted enhancement to the CLI subcommand.

**Update, 2026-09-11 (T29).** `bus monitor --project <path>` now also
resolves and prints a DPT-decoded value for each telegram, via
`resolve_project_group_address_dpts` (§61 below has the full accounting of
what that resolution covers and does not). That resolution takes a
different position on the exact problem this limitation already describes:
where a group address's *name* is merged across installations with
last-seen-wins on collision, a group address's *DPT* is merged across
installations by reporting a `Conflict` and refusing to pick one. Two
answers to the same shape of problem inside the same subcommand, arrived at
in different sessions. This entry is left as-is rather than silently
rewritten to match the newer, stricter behaviour — a future cycle that
reconciles the two should treat that as its own decision, not an
accidental side effect of a DPT codec landing.

**Update, 2026-09-11 (T15).** The same hardcoding exists on the *write*
side, not just the display side this entry originally described:
`apps/knx-cli/src/main.rs`'s `bus write`/`route write` parse a
caller-typed destination with `knx_core::GroupAddress::parse(&str,
knx_core::GroupAddressStyle::ThreeLevel)` — the style is a literal, never
the open project's own `info.group_address_style`. T15's own `/write`
route (`apps/knx-server/src/bus_routes.rs`) had the identical bug and was
fixed to parse in the session's project's actual style (commit
`b540264`); the CLI's copy was deliberately left as-is, since fixing it
was not this branch's scope and CLAUDE.md asks that unrelated changes not
ride along with a feature branch. See
[§62](../KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence)
item 13 for the full account.

**Update, 2026-10-04 (AR14) — lifted for the CLI.** `bus monitor`,
`route-monitor` and `bus write` now parse and print group addresses in the
style of the `--project` they were given; without `--project` (and always
for `route-send`, which has none) three-level stays the default. A raw
address named differently in several installations shows every distinct
name, in installation order, joined by ` | ` — both here and in the
server's Group Monitor — through the shared
`knx_core::resolve_project_group_address_names` instead of last-read-wins.
The joined text is for display only: a name containing ` | ` itself is not
escaped. Tests: `apps/knx-cli/tests/cli_bus_address_style.rs`,
`monitor_line_uses_the_project_style_and_all_names`,
`names_shared_across_installations_are_all_shown`. See
[ALPHA_READINESS](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/ALPHA_READINESS.md#ar14-offline-buscli-contract-dossier).
<!-- END retained catalogue record -->

## Update 2026-10-14: the short summary caught up with AR14 (Goal B8)

The catalogue summary still described the original limitation ("three-level
address formatting and flattened names"), which AR14 lifted on 2026-10-04 (above;
ledger `KL-29` is `DONE`). Checked against the code on this date: `bus monitor`,
`route-monitor` and `bus write` use the `--project` style and show every distinct
name through `knx_core::resolve_project_group_address_names`; what remains
three-level is the default without `--project` and `route-send`
(`run_bus_route_send` parses and prints `ThreeLevel`, it takes no project). A
name containing ` | ` is not escaped in the joined display.
