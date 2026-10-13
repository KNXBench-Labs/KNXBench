# Indeterminate import progress

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-097)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 97. Progress is a phase label far more often than it is a percentage

**Limitation.** Of the seventeen phases a load reports, exactly two carry a
`completed`/`total` pair: reading the container entries the exporter cannot
regenerate, and ingesting manufacturer files. Every other phase — including
`parseTopology`, which is the longest one in the maintainer's reference
project — shows an indeterminate indicator and its name. The bar does not
fill smoothly from 0 % to 100 %, because for most of a load there is no
honest number to fill it with.

**Cause.** A percentage needs a total that is known before the work starts.
Topology parsing is a streaming XML pass with no element count in hand;
enrichment visits whatever the product database happens to resolve. The
alternatives — elapsed time, compressed size, the phase ordinal — are all
forbidden by ADR-0023 for the same reason: they would report something
other than progress while looking exactly like progress.

**Consequence.** Users see "Parsing the topology" with a moving indicator
rather than "43 %". This is the intended trade, not an unfinished feature.

**Lifted when.** A phase gains a total that is genuinely known in advance —
counting topology elements in a cheap first pass would be one way, and
would have to pay for itself in measured time before it is worth it.
<!-- END retained catalogue record -->
