# WP-10 — Import and model evidence (incl. issue #7)

**Status:** Blocked: input. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

**Evidence route (8 November 2026):** clean-room observation is admitted by the owner ([OWN-01](OWN-01-owner-decisions.md#decisions-recorded--8-november-2026), [rules](../RESEARCH.md#evidence-routes)). IMPORT-02 used it first: the harness ran on a project-made fictional ETS6 export set ([run record](../AUTHORIZED_RESTORE_EXPORTS.md#run-record-8-november-2026)).

## Task

Independent, input-gated import/model cases:

1. `IMPORT-02`: authorised ETS restore-point exports of distinct revisions
   plus independently reviewed baselines. Since the owner decision of
   8 November 2026, ETS exports of fictional projects made by the project count
   as authorised ([evidence routes](../RESEARCH.md#evidence-routes)).
   **Done for one scope on 8 November 2026:** ETS6 6.4.1, schema 23, revisions
   A/B plus an archive-restored A′, with baselines taken from ETS and the XML
   ([run record](../AUTHORIZED_RESTORE_EXPORTS.md#run-record-8-november-2026)).
   Other ETS versions, schemas and protected projects remain open.
2. `MODEL-05`: explicitly deferred by the owner on 27 October 2026; no current
   need for a KNXBench-native site spanning separate installations. No model
   change is commissioned or implemented. Reopen only on a new owner decision
   backed by a multi-installation export or an explicit native-site requirement
   ([ADR-0125](../adr/0125-multi-installation-site-is-blocked.md)).
3. KL-177: the project-side `SubModule` grammar, schema-22 samples with
   devices, and independent samples for other untested schemas (12/13/14).
4. Real CSV/ESF/report/compare references (KL-038/044/051) when supplied.
5. Issue #7: the reporter's log or report and acceptance.
   **Checked 12 November 2026:** an ETS 5.7 project (schema 20) imports since
   12 October 2026 (KL-177); this is not a silent failure. Synthetic
   always-on tests (`crates/knx-etsproj/tests/it/schema20_import.rs`) pin the
   import, the report's `unsupported` entry naming the three-public-sample
   evidence boundary, and the refusal of a schema without a table with a
   message naming it and the supported schemas (11, 20, 21, 23). A read-only
   inventory of the available private corpus found no schema-20 export among
   readable top-level XML; nested/protected archives were not classified. No
   real ETS 5.7 project was run; acceptance on the reporter's project remains open.
6. Primary `knx_cvexc` semantics before any runtime rule is enabled.
7. Installation instructions: other source schemas/encodings and an ETS
   export of edited overrides.

## Done when (per active evidence case)

MODEL-05 is deferred, not done; it has no current implementation task.

Production import/service/native reopen and reports match the independent
baseline, preserve unknowns, and refuse malformed or unsupported mappings
honestly. Synthetic exports do not replace authorised real exports. AES/Secure
samples are separate later scope ([LS-06](LS-06-wider-product-scope.md)).

## References

[Authorised-export harness](../AUTHORIZED_RESTORE_EXPORTS.md) ·
[ADR-0125](../adr/0125-multi-installation-site-is-blocked.md) ·
[KL-177](../limitations/KL-177-ets-57-schema-20-evidence.md) ·
[import expansion](../status/2026-10-10-import-expansion-integration.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-10--import-and-model-evidence)
