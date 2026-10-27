# ADR 0155: Production functional cases may run in parallel within one file

Date: 2026-10-27
Status: Accepted
Amends: [ADR-0154](0154-production-browser-suites-use-tag-lanes.md), decision's
file-level restriction and the deferred case-level alternative.

## Context

The production tag lanes still took 681.7 s in the integrated gate of
`745932994`. Single-file suites could not use the four-worker budget. The owner
approved the next independence audit and implementation on 27 October 2026.

The reviewed scope is 35 production spec files and their 11 recursively imported
test-side helper modules. No module-level mutable binding, ordering hook or
mutation of a module/imported binding was found by the lexical checks. Specs
already use per-case browser contexts and route fixtures. File output goes to
the per-case Playwright output path or to capture switches removed by the gate.
There is no new shared server or application state. This audit is bounded,
not a proof that arbitrary future JavaScript is independent.

## Decision

Only `run_production`'s **functional** lane passes `--fully-parallel`. The
worker budget stays four by default. The inventory, exclusive lane, default
browser lane, standalone Playwright configs, suite ordering, assertions,
retries and identity checks are unchanged. The exclusive lane remains one
worker without the new flag; production suites still execute sequentially.

`tools/production_lanes.py` provides a lexical regression safety net. An owning
tool test runs it over every production config and recursive test-side helper
in every package gate. It reuses the existing browser-lane masker and import
resolver rather than adding another JavaScript lexer. It flags module-scope
`let`/`var`, `beforeAll`/`afterAll`, `describe.serial`/`describe.configure`,
and common mutations of module-level or imported bindings. Config selection
must consist entirely of explicit spec names; mixed globs or expressions are
refused instead of silently reducing the checked selection. Direct developer
runs can also invoke `python3 tools/production_lanes.py`; a standalone targeted
web measurement does not itself run the package gate's tool-test stage.

A new spec or dependency needs independence review, not just a green scanner.
Known blind spots include destructuring, aliases, describe-local shared state,
mutations hidden in called functions, computed imports and external/application
modules. Scope shadowing can produce false positives. Do not suppress a finding
by weakening assertions, removing cases or moving shared state behind an alias.

## Evidence

Two frozen, successful targeted runs selected all 16 production suites and no
default specs: 557 cases each (528 functional, 29 exclusive), with exact per-lane
case identity equality against the previous integrated gate. Production stages,
including inventories: **438.95 s** and **439.84 s**, versus **681.7 s** before.
The new runs did not follow the full default suite, unlike that baseline; these
are single-host observations, not a general throughput guarantee. Guard-only
hardening followed these measurements; final integrated acceptance is separate.
[Research and acceptance record](../research/2026-10-27-production-case-parallelism.md).

## Alternatives and consequences

- Keep file-local ordering: least change, but the measured single-file bottleneck
  remains. Retained for the default lane and standalone commands.
- Split the 35 specs into more files: unnecessary test/source churn to achieve
  scheduling already supported by the locked runner.
- Run production suites concurrently: not part of this decision. Leases, build
  outputs and CPU budgeting need a separate review.

The tool uses standard-library Python plus the existing browser-lane module;
there is no new runtime or dependency. No application, KNX protocol, corpus,
bus access or compatibility claim changes.
