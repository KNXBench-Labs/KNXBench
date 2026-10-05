# ADR 0083: Admit exact product scheme 10 through the strict namespace path

Date: 2026-10-05
Status: Accepted
Session: Alpha release (AR06P, KL-153)

## Context

146 of 853 public manufacturer downloads (crawler run of 2026-10-03, nearly
all ABB) use the ETS4-era master namespace `http://knx.org/xml/project/10` and
are refused as an unsupported namespace (KNOWN_LIMITATIONS §153). No scheme-10
XSD or specification is available locally or was recovered publicly
([research](../PRODUCT_SCHEME_10_RESEARCH.md)). Schemes 12–14 were admitted on
observed-vocabulary evidence against schemes 11/20 plus fixture tests
(PRODUCT_DATABASE_CORPUS "Scheme and producer observations"); scheme 13 added
no new observed names at all. The same standard is applied here.

Measured on 2026-10-05 over all 146 scheme-10 packages, streaming every XML
member without extraction (no names or values recorded):

- 1,391 XML members, all parse; every typed member (Master, Catalog, Hardware,
  ApplicationProgram, Baggages) has the exact scheme-10 root namespace; no
  foreign element namespace and no qualified attribute appears.
- Scheme 10 uses 176 element kinds, 438 element/attribute pairs and 188
  parent/child pairs per member role. **None is absent from scheme 11's
  observed vocabulary** (scheme 11 uses 29 element, 95 attribute and 39 edge
  kinds that scheme 10 does not).
- Of 244 attributes with a closed scheme-11 vocabulary (at most 8 values),
  scheme 10 differs only by (a) the `false` spelling of xs:boolean where scheme
  11 writes `0`, (b) other numbers in numeric fields (catalog numbers,
  load-control addresses), (c) a translation unit version `0`, and (d) other
  plug-in handler names in `Extension/@EtsDataHandler`. `bool_flag` already reads `0`/`1`/`true`/`false`, and the
  affected `Options`/`Legacy*` flags are kept verbatim. `ObjectSize` uses the
  same 17 values. Identifier attributes keep scheme 11's structural prefixes.

What this does not establish: scheme-10 semantics where equal names could mean
something else, defaults the missing XSD would define, or ETS4 parity.

## Decision

1. Admit exactly `http://knx.org/xml/project/10` in `master_scheme`, and in the
   master-language evidence scan (`parse/master_language.rs`) so retained
   masters re-derive the same way. Look-alike namespaces stay refused.
2. Validate every typed scheme-10 member against that exact namespace and
   refuse foreign elements and qualified attributes before any write — the
   strict path of ADR-0036/ADR-0072 — because the typed readers dispatch on
   local names.
3. Use the generic scheme-11 readers and their unknown reporting. The
   supplemental evidence scan stays limited to 12/14/21/23: scheme 10 has no
   field outside scheme 11's vocabulary for it to find.
4. Storage, ZIP and evidence limits, atomic rollback and byte retention are
   unchanged.

## Alternatives considered

- **Keep refusing** until a primary XSD appears: honest, but leaves the largest
  refusal class unusable although its observed grammar is a subset of an
  admitted one. The corpus evidence meets the bar used for 12–14.
- **Admit on the lenient scheme-11 path:** rejected; strict validation costs
  nothing for the corpus (no foreign content observed) and keeps unknown
  namespaces out of local-name readers.
- **A scheme-10-specific parser:** no evidence of a different grammar; it
  would duplicate the scheme-11 readers.

## Consequences

- Scheme-10 packages install with explicit unknown reporting; the release
  measurement is in [PRODUCT_SCHEME_10_RESEARCH.md](../PRODUCT_SCHEME_10_RESEARCH.md).
- Tests: `knx-productdb/tests/scheme10.rs` (install, queryability, master
  translations, replay, strict-namespace refusals, late rollback, look-alike
  namespaces). `scheme23.rs` no longer lists 10 among unresearched namespaces.
- A later primary scheme-10 specification that contradicts the corpus
  evidence reopens this decision.
