# Project-comparison parity

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-051)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 51. Project diff (T14) has no ETS-comparison parity, and none can currently be measured

**Limitation.** `crates/knx-diff`'s output — a "KNXBench project diff" —
is KNXBench's own comparison. It is not, and cannot currently be shown to
be, similar in matching rules, content, or presentation to whatever
ETS's own project-compare feature produces.

**Cause.** No ETS-produced comparison output — no screenshot, no exported
report, no printed diff — exists anywhere in this repository, the same
evidence gap [§44](../KNOWN_LIMITATIONS.md#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured)
records for T13's HTML report and [§38](../KNOWN_LIMITATIONS.md#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
records for T12's CSV format: project comparison is an ETS *application*
feature, not something the KNX Association standardizes, so there is
nothing to read except a real sample, and none has been obtained.

**Impact.** Nothing in the UI, CLI output, or this documentation set may
say "ETS compare" or imply compatibility with it, and none of it does —
`knx-diff`'s own module doc and its design spec (§1) state this
explicitly. A user expecting the diff to match what ETS's own compare
screen would show — which entities it matches, which fields it compares,
how it presents a rename — has no basis for that expectation from
anything KNXBench ships.

**Lifted when.** A genuine ETS-produced comparison sample is obtained. At
that point a content-set comparison becomes possible for the first time;
whether that motivates changes to the matching rules or the rendered
output is a separate decision to make once evidence exists.
<!-- END retained catalogue record -->
