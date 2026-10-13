# 8.7.1 Method, and why it changes what is citable

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="871-method-and-why-it-changes-what-is-citable"></a>
<!-- BEGIN preserved record-1-010 -->
#### 8.7.1 Method, and why it changes what is citable

`pdftotext -layout` on the **source PDFs** produces clean, row-by-row readable
tables where the Markdown extraction §8.4 assessed produces mush. This matters
for one specific claim. §8.4 says of `03_03_07`'s Table 1, the APCI code table,
*"No APCI value can be read out of that safely."* **That is true of the
Markdown extraction and not of the PDF.** `pdftotext -layout` renders Table 1
one row per line with its ten bit columns intact. Spot-checked against this
project's own shipped code: `A_DeviceDescriptor_Read` reads as `1100000000` =
`0x300`, which is exactly what `crates/knx-net/src/cemi.rs` encodes as
`0x300 | descriptor_type`. **[V]** 2026-09-13.

The practical rule for anyone extending this research: query the knowledge bases
to *find* the clause, then read the clause out of a `-layout` extraction of the
PDF. The bases point; the PDF quotes. §8.4's "reusable pointer" still holds —
this is an addition to it, not a replacement.

One extraction artefact, recorded so nobody files it as a spec gap: in
`03_03_07` the heading of §3.5.4 renders as `..  _ emory_rite-service` while the
clause body extracts intact. The body is what was quoted. **[V]**


<!-- END preserved record-1-010 -->
