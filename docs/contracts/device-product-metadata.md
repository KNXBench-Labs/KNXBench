# Device manufacturer names and product database metadata

## Read-only display contract

The Devices list and device Product data overview use the same product-database
name resolver. A nonempty installed manufacturer name wins unchanged. A missing,
empty or ID-shaped name (`M-xxxx` or its four hex digits) may use retained
master-content declarations for that exact, valid manufacturer ID, regardless of
the retained filename or package membership. One distinct
non-placeholder name resolves it; conflicting names, damaged masters and absent
matches leave the exact ID as fallback. There is no embedded vendor-name table,
network lookup, database rewrite or project mutation. Retained master bytes must
match their recorded length and SHA-256 before they can supply a name. The batch endpoint caches
a lazy, complete retained-name index only for its current read, shared across all
manufacturers in that request. Discovery makes one bounded source-table pass,
without source-by-membership scans. Before fetching payloads it admits at most
4096 sources, 8 MiB per source and 32 MiB in aggregate; XML examination also admits
at most 262144 events in aggregate, depth 128 and 256 attributes per element.
Exceeding any bound or an unreadable query refuses **all** optional fallback names,
not just the unexamined suffix. Real installed names still win, and all available
normalized product facts remain available. Thus a large unrelated source can make
fallback names unknown; a partial scan never establishes uniqueness. No schema or
persistent cache is added. Ordinary master-ingest last-writer-wins
policy is unchanged ([KL-088](../limitations/KL-088-manufacturer-display-name-winner.md)).

This also covers older databases whose generic `ingest_file` retained a master
without interpreting its Manufacturer declarations. It does not promise a name
when the database holds neither a real name nor unambiguous retained master data.

## Metadata coverage

`query::device_product_details` returns every non-NULL scalar column from the
winning normalized records, not a selected list of attractive attributes:

| Record | Coverage |
| --- | --- |
| Manufacturer | Exact ID and stored name; the resolved display name stays in the overview |
| Product | ID, manufacturer/hardware IDs, text, order number, rail flag, width, language, declared hash, registration status, source SHA-256 |
| Hardware | ID/manufacturer, name, serial/version, bus current, address/application/accessory/coupler/power/IP/repeater flags, original manufacturer and source SHA-256 |
| Hardware-to-program | ID/manufacturer/hardware/application references, media, hash, registration number/status/signature and source SHA-256 |
| Application | ID/manufacturer/name/number/version/type/mask/PEI/load style/language/hash/linkability/original manufacturer, all stored security/table-size/ETS/replacement literals and source SHA-256 |
| Catalogue | Every placement matching the product and hardware-to-program pair, all scalar placement metadata and each distinct ancestor section, including IDs/parent IDs/descriptions/references/languages/source SHA-256 |
| Sources | Winning records' source SHA-256, retained byte length, sanitized XML/package filenames (including recorded package aliases) |
| Source packages | Package SHA-256, sanitized original filename, schema version and byte size; aliases remain attached to the corresponding source |
| Additional attributes | All retained `ingest_unknown` rows for those sources: program scope, XML path, diagnostic kind/name/count/sample; explicitly **file-scoped**, not asserted as attributes of this particular device |

The internal `write_authority_recorded` ingestion-completeness marker is not
manufacturer metadata and is excluded, as are package ingestion-accounting counters. Binary source/package payloads, absolute
source paths, parameter/object tables, losing candidate definitions and full XML
introspection are outside this view. Parameters and communication objects retain
their own existing workspaces. Source records without retained bytes still expose
the reference SHA, without inventing filenames or sizes.

Mismatched program links contribute no program or catalogue metadata. Missing
hardware/program/section records leave the product's available facts intact.
Catalogue ancestor cycles stop on the first repeated ID. Multiple placements
remain separate and deterministic; the older compact overview uses the lowest
catalogue ID, not a claim about which placement was selected in the original
project. A programless pair also lists placements without a program reference.

## Presentation and limits

**More product data → Database records** contains expandable per-record details.
Overview texts retain the existing selected-product-language overlay; database
records are explicitly untranslated literals with translated EN/DE field labels.
Empty strings stay visible as empty values, NULL stays omitted, and zero/false
are not treated as absent. Flags and security values are declarations, not
verified runtime capabilities. The omission count describes only overview fields.
Long identifiers and prose wrap inside the current pane; database literals preserve
newlines and repeated whitespace. Scalar package/source metadata queries never
project discarded binary payloads.

No schema migration, device communication, catalogue installation or compatibility
policy is added. Full XML/baggage interpretation and manufacturer-name recency
ranking are not claimed. See [catalogue semantics](../limitations/catalogue-metadata-semantics.md).

## Verification boundary

The regression fixture is a fictional normalized SQLite database plus a retained,
otherwise uninterpreted master. The real HTTP handler's complete product response
is pinned to the typed browser fixture in `e2e/fixtures/product-details.ts`.
Product-query tests cover all scalar columns, duplicate/ambiguous names, unknown
IDs, renamed generic-retained masters/conflicts, bounded source/byte/XML refusal,
observed SQLite instruction work and payload-column projection denial, absent values,
multiple placements, cycles and mismatched links. HTTP tests
cover both device endpoints and unchanged project state. Mounted production and
development browser cases cover EN/DE at 1440/400 pixels and long literals; all
API requests, including automatic discovery, are intercepted inside an offline
network namespace. This is synthetic software verification, not a reproduction
against a private user database or real hardware. Independent review and the
complete change-scoped package gate remain the delivery owner's acceptance step.
