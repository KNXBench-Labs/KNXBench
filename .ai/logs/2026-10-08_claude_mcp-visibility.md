# 2026-10-08 — Claude: MCP parameter visibility, paging and real-client tests (ADR-0090 amendment)

## Scope (user: "finish MCP: parameter visibility (lift KL §165) plus a test with real clients")

1. Move the parameter panel's stored-value evaluation out of `knx-server` so
   `knx-mcp` can report visibility with the same code.
2. Test `knx-mcp` with Claude Code and Hermes.

## Implementation

- `crates/knx-productdb/src/device_evaluation.rs` (new): `evaluate_device`,
  `take_digits`, `decompose_module_qualified`, `resolve_mi_authority`,
  `MiAuthority`, `DeviceEvaluation` moved verbatim from
  `apps/knx-server/src/domain.rs`; the two duplicate diagnostics became
  `EvaluationFinding` data; new `scoped_ids`, `ValueStatus`,
  `value_status`, `traversal_complete`.
- `apps/knx-server/src/domain.rs`: thin wrapper keeps the old
  `DeviceEvaluation` DTO shape; `finding_dto` keeps the wording (new unit
  test pins both texts — they had no test before).
- `apps/knx-mcp/src/tools/device.rs`: visibility per stored value and in
  `explain_parameter`, `activeInModules`, module declarations, paging and
  filters for `get_device`, `resolve_product` (placeholder fix).
  `schemaVersion` 2.

## Facts worth keeping

- Hermes caps MCP tool results at 50,000 chars
  (`tools/budget_config.py: DEFAULT_MCP_RESULT_SIZE_CHARS`) and spills larger
  results to a file the agent cannot read without file tools. `get_device`
  was unbounded: 202 KB for the 577-object device #36 in `project123`.
- Hermes `-t <x>` filters MCP discovery by **server name**; `mcp-<name>` is
  only the alias after connection. A temp `HERMES_HOME` config needs
  `_config_version`, or chat mode connects zero servers.
- `~/.local/bin/claude` runs `mise use -g claude` each call and hung; the
  installed binary `~/.local/share/mise/installs/claude/2.1.289/claude` works.
- `knx_projection::DeviceProductNode::resolution` is a placeholder every
  consumer must overwrite; the first `knx-mcp` release leaked `NoDatabase`.
- Hidden ("inactive") parameters: whether they are downloaded is
  program-dependent (`Options/@DownloadInvisibleParameters`, research
  `knxnet-ip-and-bus.md`); tool notes must not claim "no effect".
- `knx import` of a project with embedded master data rewrites manufacturer
  names in the installed product DB (documented: last `knx_master.xml`
  wins). Happened during this package's KV-demo import; restored byte-exact
  from the pre-import copy (sha256 dab0c5db…).

## Evidence

- knx-productdb `device_evaluation` 3/3; knx-mcp 16 tool + 2 stdio + 6 unit;
  knx-server 699 + 1 new unit test.
- Mutation sweep 11/11 killed (two survivors fixed by new tests first).
- Live probe (copies, installed product DB copy): 36/35/4 devices
  evaluated; 1,262 active + 81 inactive of 1,343 per real project; KV 9/9
  module-scoped active; product `Resolved` 36/36; largest response 25.6 KB.
- Claude Code 2.1.289: visibility question correct (device #10, 12
  inactive), write refused, KV modules correct, 577-object device answered by
  paging (verified in SQLite: 577 objects, 0 links, 0 values).
- Hermes: `hermes mcp test` 8 tools; chat with `-t knx`: visibility question
  correct using `parameterVisibility: inactive` (4.8 KB), write refused.
- Full gate on the candidate (inputs hash 68deb302…, before a comment-only
  header fix in the new test file and the status-entry gate line): web
  build, fmt, diff-check, workspace Clippy, workspace tests 3,546/0/178,
  corpus via the moved evaluation (server 5/5 + lib 2/2, productdb 7/7
  release with the product corpus), xtask layering/anchors/ledger/
  corpus-gates green; headers red (156 > 155) → fixed, rerun green
  (642/155). Committed as `cc78c173`.
- Correction: the 2026-10-07 gate's "4,376" double-counted knx-store and
  knx-productdb (818 passed) via the `--include-ignored` run in the same log.
