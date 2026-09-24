# Product database corpus

## PDB-3 evidence contract

Product-package compatibility measurements are reported through the schema-v12
install ledger. A measured install exposes deterministic category/disposition
count rows, all six dispositions (`read`, `stored`, `deduplicated`,
`retained-but-uninterpreted`, `unsupported`, `dropped`), unknown constructs
with distinct and occurrence counts, and unsupported diagnostics. Paths in
those diagnostics are archive-relative/XML-relative; host `source_name` is
never part of the server, CLI, or web projection.

A `facts: null` response means the package predates the v12 ledger and its
encounter facts are historically unavailable. It must not be interpreted as a
measured zero. This evidence describes KNXBench importer behavior only: it does
not establish ETS parity and does not verify package signatures. PDB-8 is the
future typed master-data coverage slice; PDB-10 is the future safe baggage
inventory and index-to-payload resolution slice.

Corpus measurements must remain opt-in and confined to an explicitly supplied
local corpus root. No private corpus content, manufacturer identity, or host
path belongs in committed reports.

The PDB-3 matrix remeasurement records legacy `InstallReport::unknown` totals of
22,404 for isolated successful attempts, 22,279 for installed shared-order
attempts, and 22,404 for all successful shared-order attempts. These are counts
of distinct unknown rows in each report, not the new ledger's occurrence totals.
The baseline changed deliberately. The isolated decrement comes from classifying
`Baggages.xml` as unsupported baggage-index evidence instead of a generic
unrecognized package member. The shared-order increases come from retaining
unknown evidence encountered below a declaration whose storage parent is
rejected. These totals are importer evidence, not a claim that the constructs
are understood.
