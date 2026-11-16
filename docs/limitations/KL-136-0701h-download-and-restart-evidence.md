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

## 2026-11-13: the fourth program, complete and both

Two re-run reports on issue #38 (source build `e91da61ae`, same tester device)
add `complete` (28 steps) and `partial-both` (27 steps), 2477 octets each, for
`M-0006_A-E052-12-3406`. The program hash matched, an offline plan gives the
same shape, post-download identity and load states matched and the device
worked as configured. As in every earlier run, the closing restart was not
acknowledged. This makes it the fourth program verified for a complete
download. [Evidence record](../evidence/tester-report-hager-0701-2026-11-12.json).

## 2026-11-16: fifth program, Hager STCC510S in four scopes

Issue #53 supplies one report for each scope of `M-0009_A-207C-10-0210`
(STCC510S V1.0, application version 16, mask `0701h`). The source hash
matches the available application XML. Offline plans at the report-producing
`f6ef90fb4` revision reproduce 25/11/21/24 steps and 1084/64/1020/1084
written octets for complete/parameters/group-addresses/both respectively.
The reports record completed transfers, matching post-download identity and
load states and a running application; the tester confirms function in each
scope. This is one device, not general mask or manufacturer coverage.

Every restart remains unconfirmed. The later machine compares are the
reporter's observations, not independently inspected captures. A source
check against `origin/main` maps the literally named offsets `0x4404`,
`0x441C` and `0x4420` to descriptor type octets (codes 0–14 only), not to
the config/flag octets one byte earlier, so the reporter's flag attribution
is not supported as stated; the true cause and clear time remain unknown.
Reported differences do not change the write/read-back verdict or establish
their own cause; no comparison tolerance, encoder or executor rule changes.
The original MDT, Eibmarkt and Hager-button evidence remains unchanged.
[Report provenance and boundaries](../evidence/tester-report-hager-stcc510s-2026-11-16.json).
