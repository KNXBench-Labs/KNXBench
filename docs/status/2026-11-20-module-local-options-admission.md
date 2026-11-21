# PRG-01-N3: module-local Options admission — acceptance record

Observed: 20 November 2026. Bounded software acceptance on the PRG-01-N3
branch; simulator/loopback only — no live bus, no device, no hardware write.

## Scope

The [contract](../contracts/customer-adjustable-module-options.md) and
[ADR-0181](../adr/0181-module-local-options-customer-adjustable.md) admit
`CustomerAdjustableParameters` on a module definition's own
`Static/Options` as closed context class `ModuleOptions`. The parser walk is
unchanged; a `Static` nested in the root `Static` is not entered.
[Investigation evidence](../research/2026-11-20-module-local-options-admission.md)
records the observed parser gap and the probe results.

## Verification (real output)

Re-run 21 November 2026 after review narrowed the change (the nested-`Static`
parser arm was removed; the parser walk is unchanged). Exact commands, run with
scratch `XDG_DATA_HOME`/`XDG_CONFIG_HOME`:

- `cargo test -p knx-productdb --lib customer_adjustable` →
  `test result: ok. 13 passed; 0 failed; 0 ignored; 941 filtered out`
  (includes `module_local_options_are_an_admitted_context_in_both_anchors` and
  `excluded_descendants_still_do_not_acquire_the_new_category`, which now pins
  that `Static>Static` content is not observed).
- `cargo test -p knx-app --test it vendor_download_plugin` →
  `test result: ok. 23 passed; 0 failed; 696 filtered out`
  (includes `admitted_module_local_options_reach_the_public_writer_and_keep_reads_unchanged`
  and `excluded_descendants_and_foreign_prefixes_do_not_invent_the_new_refusal`).
- `python3 tools/run_package_gate.py --base origin/main`: fmt, documentation,
  check-layering, check-headers, check-anchors, check-ledger,
  check-corpus-gates, tool-tests, clean-desktop and clippy exit 0.
  rust-tests: every suite ok except 6 failures outside this diff:
  `c49_partial_group_addresses::the_49_report_plan_reproduces_offline`
  (`C49_PROGRAM_XML` not set) and 5 `http_parameter_grid` /
  `http_parameter_captions` tests. The gate is therefore not green; the
  knx-productdb and knx-app suites touched by this change passed. The
  `knx-desktop` web bundle was a stand-in copy, not a fresh build.

## Limits

Not hardware acceptance, not ETS customer-write semantics, not a preservation
algorithm, not universal flag detection, and no closure of PRG-01. Excluded
contexts remain matcher limits. `regular-070n-exclusive-inactive-storage-survival`
still needs hardware proof and is not claimed.
