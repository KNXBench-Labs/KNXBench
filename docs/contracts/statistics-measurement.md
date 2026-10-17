# Statistics measurement contract — TOOLS-01 through TOOLS-05

2026-10-11. This contract maps the five original telemetry IDs without
reassigning them. Source inventory of record: Git object
`bca2d3336b9692e7a6ece18804d187ecdda6968f:docs/archive/OFFENE_PUNKTE.md`;
current state remains in the [ledger](../status/LEDGER.md). The formerly named
`AI_STATS_TELEMETRY_PLAN.md` is absent in this checkout; this is a focused new
contract, not a claim to have recovered that historical file.

## Location, invocation and exact evidence (TOOLS-01)

The actual standalone collector is the local Git repository
`<sources>/ProjectStats`. Entry point `ai-stats.py` dispatches
the explicit-project CLI in `ai_stats_cli.py`; structural measurement belongs
to `ai_stats_code.py`, observed collector-response bytes to `ai_stats_traffic.py`.
It is not a script inside KNXBench or an in-app dashboard component.

The clean canonical collector checkout was fast-forwarded from `e639f206`
to accepted source `2ac5dfb072fffc97e1622defd042b0a6adf41e19`. The five changed
paths are README, the two code/CLI modules and their CSS/CLI tests. **181 tests**
were executed on that accepted candidate; ignored or missing parser coverage
must not be relabeled as successful tests on a different environment.

Actual local invocation:

```sh
python3 <sources>/ProjectStats/ai-stats.py --project /path/to/KNXBench --json
```

This writes `docs/ProjectStats.md` in the **selected** checkout and reads its
`.projectstats.json`; it is not a no-write preview command. Use an owned isolated
checkout for evidence runs, not the shared project. Existing settings, profile
selection and `include_cloud=false` were copied unchanged for the admitted run;
no account/network instrumentation was enabled.

The earlier configured-origin `Repository not found` and authenticated404
results were access-context-specific. On 11 October 2026, the already-saved
credential matching that exact private repository owner established access;
no login, persistent account switch, new remote or repository was needed.
Existing commits `e639f206b92c46f95623c0f6891b0cea49e38401` and
`2ac5dfb072fffc97e1622defd042b0a6adf41e19` were pushed normally to its `main`.
Exact local/tracking/live tip and all accepted tracked source blobs were read
back. [Publication receipt](../evidence/collector-publication-2026-10-11.json).
The source-publication residue is delivered, not new telemetry coverage or a
new generated report. The original dated snapshot remains unchanged.

## Exact gap mapping, sources and retained limits

| Original ID | Source and measured boundary | Still unknown / not admitted |
| --- | --- | --- |
| TOOLS-01 | The actual Git checkout/entry point above; accepted source SHA and clean-root fast-forward verified. | Source publication is now verified separately; this does not close physical-traffic or attribution gaps. |
| TOOLS-02 | `ai_stats_traffic.py` optionally records the collector's own HTTP **response-body** bytes, request-attempt identity, outcome and observation UTC time in a private versioned SQLite database. Replay of one attempt is idempotent; different attempts are separate. No store/observations returns unavailable. | Those are not HTTP/TLS/on-wire totals, model-provider requests or remote-job traffic. No historical physical traffic can be reconstructed from tokens or cost. Instrumentation remains opt-in; no new real network measurement was performed here. |
| TOOLS-03 | Existing loaders read provider-recorded session boundaries, activity timestamps and reported durations where available; the Hermes loader reads `started_at`/`ended_at` and end reason from source rows. Outcome/duration meaning is provider-specific. Report pricing/cache savings are explicitly dated API-equivalent scenarios, not invoices. | A last activity time is not proven execution termination; session duration is not joined per-event model generation time. Full tool-outcome rates and actual billing/cache discounts are not available across all sources. No fabricated throughput, completion time or measured savings. Missing source completeness is not solved by this CSS package. |
| TOOLS-04 | Local path/repository matching, explicit profile opt-in and source session identity govern attribution. An opted-in profile can admit no-project sessions under its recorded policy; a populated unrelated project path does not become KNXBench by profile alone. Unknown providers are retained as Other in project-scoped totals, without assigning another provider's prices. | Cloud jobs without explicit project attribution and unknown source authorship remain unassigned/unknown. Git author names and AI-themed branch names do not prove AI authorship. An aggregate over selected available records is not total activity outside their scope. |
| TOOLS-05 | Existing language AST families remain separate. Added optional installed stable PostCSS8.x.y counts CSS rules, at-rules and declarations, with measured-file coverage and parser version. JSON and Markdown publish the same admitted CSS values. | Shell has no admitted structural parser. CSS/AST syntax observation is not browser/compiler validation or proof of all parser-version/plugin compatibility. Missing/partial/budget-refused CSS is unavailable, not zero or a partial total. |

These are **coverage boundaries**, not a blanket DONE disposition for the five
IDs. Existing regressions exercise traffic-attempt identity/schema refusal,
Hermes provider/attribution/source timing, unavailable ASTs and pricing/throughput
labels; their actual execution is in the collector's 181-test snapshot. New CSS
and CLI controls also include malformed parser responses, bad versions,
unreadable inputs and ignored previous source maps. Static source review and
synthetic regressions are not live provider or hardware evidence.

## Additive machine contract for Session1

CLI JSON now includes the actual `code_stats` producer result. CSS is
`code_stats.css_structure`, format version1, with:

- `files`, `measured`, `unavailable_files`: explicit coverage counts;
- `rules`, `at_rules`, `declarations`: integers only for complete coverage;
  otherwise JSON null / rendered N/A;
- `parser_version`: the observed admitted parser version or null.

No CSS sources means an absent/null family and no extra CSS table. This is
not the same as a successful measured zero-node file. Any partial file coverage
suppresses all CSS totals; successful-file count remains explicit. The generic
functions/loops/types/branches subtotal is unchanged. Do not add CSS declarations
to function/type totals or turn null into a dashboard zero.

Counting definitions: nested selector/keyframe steps are rule nodes; media and
other at-rule nodes are a different count; declarations include custom properties.
Comments and strings do not manufacture nodes. No plugins are run and previous
source maps/external resources are not loaded. Collector budgets are 256 files,
1MiB/file,8MiB/batch,100,000 syntax nodes/file,128MiB Node heap and30-second parser
timeout. Oversized source is unmeasured, not declared invalid CSS.

This package owns collection and the additive data contract. Session1 owns
visual integration; it must keep unavailable values distinguishable and must
not present response-body traffic or estimated cost/energy as physical measures.
No dashboard, global style, language catalogue or chart was changed here.

## Actual generated snapshot and reproducibility

The [payload-free receipt](../evidence/statistics-coverage-2026-10-11.json)
binds generator `2ac5dfb0`, project source `aeb5a386`, exact report SHA-256,
observed installed parser8.5.27 and six/six CSS files. The
[actual dated generated report](../evidence/statistics-report-2026-10-11.md)
is retained byte-exact from the real canonical entry point, not hand-written
example statistics. Source code inventory is the selected checkout; historical
Git additions are not surviving source lines. Session aggregates are selected
local source observations, with report-disclosed estimates/coverage limits.

Reproduction needs the same collector/project revisions, installed parser,
settings and source-log snapshot. Live local logs evolve, so session aggregates
can change on a later run even if CSS source counts do not. No fake fixed
answers are substituted for unavailable sources. Hash/date/source binding proves
what was observed, not reproducibility against a future mutable log stream.

The shared root's `docs/ProjectStats.md` had pre-existing uncommitted content.
It was not overwritten. A separately named snapshot avoids competing writes;
updating the shared report remains an explicit coordinated owner action.
No credentials, source session IDs, host user path or original log bodies are
published in this contract/receipt. This work performed no quota query, hardware
operation, release or deployment.
