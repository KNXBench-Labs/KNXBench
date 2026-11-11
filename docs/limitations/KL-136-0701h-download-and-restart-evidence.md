# 0701h download and restart evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-136)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## §136 `0701h` download verified on three programs; restart remains unconfirmed

The earlier claim that mask `0701h` could not be downloaded is withdrawn.
A memory download of `A-0027-15-0BAC` and a functional group-telegram check
on MDT `1.1.67` succeeded with a device-specific go (RESEARCH §19.4).
CLI/Web commands now expose this memory path, and partial scopes have their
own bounded evidence (§7/§142). The closing Basic Restart on this device
remains unacknowledged: `RestartOutcome::Unconfirmed` must not be presented
as a verified restart or a failed image read-back. **2026-10-09:** the
Eibmarkt presence detector `1.1.8` (`M-006A_A-0001-22-617E-O0079`) behaved
the same way: complete download read back, functional check, restore,
restart unacknowledged (RESEARCH §19.19). The MDT fourfold button `1.1.14`
(`M-0083_A-0026-15-3591`) completed a download and restore the same day,
with independent readback and light-function telegrams; neither closing
restart was acknowledged (§19.21). Three programs from two manufacturers
on one mask are not evidence for other masks or revisions.
<!-- END retained catalogue record -->

## 2026-11-11: a fourth program from a tester report

A tester report on issue #38 records a parameters-only download of the Hager
push button `M-0006_A-E052-12-3406` (`0701h`, application version 18) on the
tester's own device, with the post-download identity and load states matching
the plan and the device working as configured; the closing restart was not
acknowledged. The program file's SHA-256 matches the report. Only
`partial-parameters` is listed as verified: the complete and
parameters-plus-group-addresses runs described in the thread have no report
file yet. [Evidence record](../evidence/tester-report-hager-0701-2026-11-10.json).
