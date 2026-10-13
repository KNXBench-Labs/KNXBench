# Structural validation without authoritative XSD

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-002)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 2. No authoritative XSD is publicly available

**Limitation.** Imports are validated structurally, not against a schema (risk
R2). There *is* a validation stage — `knx_etsproj::validate`, stage 4 of the
import, between parse and mapping, exactly where `CLAUDE.md`'s data-flow
prescribes it — but it enforces a hand-built list of structural rules, not
conformance to anything authoritative, and that list is deliberately bounded by
what the installed corpus can attest.

**Cause.** The official schemas ship with the Manufacturer Tool via the KNX
GitLab account, which requires KNX membership **[D]**. Without them there is no
document to check a file *against*, so every rule the importer applies has to be
derived from real files and argued for one at a time.

**Impact.** We still cannot tell "this file is invalid" from "this file uses
something we do not know", and a malformed file is read as far as it parses,
with the rest reported rather than rejected. That is the tolerant-parser
half, and it is unchanged. What is narrower than this entry used to claim is
the validation half. As of **T06 (2026-09-21)** stage 4 checks:

* duplicate `@Id` across `Area`, `Line`, `DeviceInstance`, `BinaryData`,
  `GroupRange`, `GroupAddress` and `BuildingPart` — an error. Until
  2026-10-04 a repeat collapsed two entities onto one internal id; since
  ADR-0073 every element keeps its own id, and a reference to the repeated
  `@Id` is reported as `AmbiguousReference` instead of being resolved;
* dangling communication-object → group-address links, in **both** spellings:
  schema 11's `Connectors/Send|Receive/@GroupAddressRefId` and schema ≥21's
  flat `Links` attribute of short ids — an error;
* two devices on one individual address, two group addresses on one address,
  and a group address outside its enclosing `GroupRange`'s bounds — warnings,
  since the mapper can use all three.

Validation never modifies the document and never aborts an import: a file that
parses still imports, and every problem above is a report entry.

**How bounded "bounded by the corpus" is.** All three corpus projects validate
clean **[V]** — 0 errors and 0 warnings each, for "Unser Zuhause" at schema 11
and schema 23 and for the KNX Association "KV v2.5" demo at schema 21. The
checks are therefore motivated by measured *coverage*, not by measured
violations, and two measurements from T06 are worth keeping:

* Before T06, stage 4 resolved 596 of the corpus's 1,218 communication-object
  links **[V]** — every one of the 622 written in schema ≥21's `Links` form was
  invisible to it, so for any project a current ETS writes, the stage performed
  no referential check whatsoever. It now resolves both forms.
* `ModuleInstance/@Id` is *not* checked for uniqueness, and that is a
  measurement rather than an omission: the KV project carries 32 of them of
  which only 16 are distinct **[V]**, because three of its four devices run one
  application program and repeat its module-instance ids verbatim. The id is
  device-scoped. Checking it would invent 16 errors about a valid file.

Two classes are deliberately left to other stages so that each has exactly one
reporting channel. `Installation/@DefaultLine`, `BuildingPart/@DefaultLine` and
`BuildingPart`'s `DeviceInstanceRef`s are reported by the mapper as
`MapProblemDetail::UnresolvedReference` — the corpus does contain an
unresolvable case (the KV project writes `DefaultLine=""` **[V]**), and it is
already reported. Anything requiring the manufacturer/application-program
database is reported by `knx_productdb::enrich`: `knx-etsproj` does not depend
on `knx-productdb`, stage 4 is a pure function of one `SourceDocument`, and
product resolution is a property of the machine's installed database rather
than of the file.

**Lifted when.** Authoritative schemas become available to the project. Note
that the tolerant parser would still be worth keeping — it is what turns a new
schema version into a report instead of a crash — and so would the structural
checks: an XSD says a `Links` attribute is a string of the right shape, not
that the group address it names exists.
<!-- END retained catalogue record -->
