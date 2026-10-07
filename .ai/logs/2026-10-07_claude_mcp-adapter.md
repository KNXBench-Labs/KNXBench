# 2026-10-07 — Claude: AI integration (grill-me) and read-only MCP adapter (ADR-0090)

## Planning (grill-me, user-approved)

Question: "AI integration: MCP server, skill, chatbox — which of them?"

Facts established from the repository before asking:
- RESEARCH §13 / ROADMAP forbid any mutation-capable LLM/MCP surface until
  authorization, revision checks, audit and §63 concurrency exist; §13.6
  allows starting with bounded reads and proposals.
- Every existing store reader migrated in place
  (`knx_store::open_existing_and_migrate`, `knx_productdb::open_and_migrate`).
- `knx-cli` carries hardware commands, so a CLI-wrapping skill inherits them
  unless it restricts them.

Decisions (Q1–Q16 accepted as recommended): power users with their own
agent; v1 = read-only MCP server + agent skill, no chatbox; read-only, changes
only as group-address CSV proposals applied by a person (ADR-0033 path);
bus/hardware excluded absolutely; privacy by documentation + hard
exclusions; saved files by alias, read-only, older schema migrated in memory
only, newer refused; separate `apps/knx-mcp`, stdio only; `rmcp` with a
deny/tree check and a hand-written fallback; `validate_ga_csv` writes
nothing; eight tools; skill under `integrations/agent-skill/knxbench`;
product DB read-only like projects; separate release binary + SHA256SUMS;
experimental + `schemaVersion`; own package, independent of LCARS; ADR first.

## Implementation

Gated as `ea1f1645` (base `4c4f3b05`), rebased onto `c7859a7f` as `2029f5c1`
(code byte-identical; upstream changed only web and docs) plus docs `43ac1ddd`. See IMPLEMENTATION_STATUS 2026-10-07 entry for the file-level list.

Facts worth keeping:
- rusqlite already sets a 5 s busy timeout on every open
  (`inner_connection.rs:118`); the read-only openers now set it explicitly
  and a `BEGIN EXCLUSIVE` test pins it (mutant with a zero timeout fails).
- `Backup::run_to_completion` asserts `pages_per_step > 0`; `-1` panics.
  Use `i32::MAX` for "all pages in one step".
- xtask's layering graph includes dev edges: `knx-store`'s tests import
  `knx-etsproj` (→ `knx-secure`). The knx-mcp rule therefore uses a new
  production graph (`workspace_production_graph`) without dev edges; the
  existing rules keep the stricter combined graph.
- `knx_projection::DeviceDetail` serialises snake_case (`com_objects`); the
  adapter returns it verbatim inside its camelCase envelope.
- `knx-csv` requires a `Name` column and refuses deleting a linked group
  address while still listing the destructive preview.

## Evidence

- knx-store `read_only_open` 9, knx-productdb `read_only_open` 5,
  knx-mcp 6 unit + 13 tool + 2 real-binary stdio tests.
- Mutation sweep 8/8 killed by the intended tests.
- check-layering negative control: adding `knx-net` to knx-mcp fails the gate.
- `cargo deny check`: advisories, bans, licenses, sources ok (rmcp 3.5.1,
  Apache-2.0; 11 lock additions, nothing removed or changed).
- Live probe against copies of `data/project123.knxdb` (schema v9) and
  `data/project_migrated.knxdb` with the installed 107 MB product database:
  36/35 devices, 514 GAs, parameters decoded 3/3, smuggled `path` refused,
  startup+initialize 0.12 s, all five files (incl. originals) byte-identical.
- Full gate on `ea1f1645` (attempt 2; attempt 1 died on a full
  `/mnt/daten-i` — 100 MB free, linker bus errors, no test result): web build,
  fmt, diff-check, workspace Clippy, workspace tests 4,376 passed / 0 failed /
  178 ignored, cargo deny, five xtask checks — all exit 0.
- `--include-ignored` for knx-store/knx-productdb: everything ran green
  except six private-corpus tests that refuse without explicit setup. Rerun
  with the root corpus: `golden_reference_products` (xknxproject oracle) 5/5
  and `legacy_member_names_corpus` (full corpus, 280 s) green.
  `corpus_compatibility_matrix` stopped at its inventory pin (115 expected,
  117 found under the whole root; the pinned private scope list is not in the
  repository) — corpus inventory, before any install, not a code finding.
  Not run: the two AR05 tests (need task-local v18 baseline databases) and
  `nested_module_private` (needs a private package path).
- Rebased tree: anchors 687, headers 640/155, ledger 191, layering ok,
  diff-check ok.

## Self-review findings

- IMPORTANT (false alarm, kept as guard): missing busy timeout — rusqlite
  defaults to 5 s; made explicit + test.
- IMPORTANT (fixed): backup busy-spin with zero pause → one step, 20 ms
  retry pause.
- MINOR (fixed): skill claimed releases already ship knx-mcp.
- MINOR (fixed): KL §165 now states an older product DB is a startup snapshot.
