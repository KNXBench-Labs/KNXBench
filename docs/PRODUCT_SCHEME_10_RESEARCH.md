# Product scheme 10: bounded research, no namespace admission

## Scope and status — 2026-10-04

AR06U continues KL-153/AR06P after the delivered bounded exact23 adapter.
This is an in-progress research checkpoint on `c17b0f36`, not a compatibility
approval, manufacturer grammar implementation or new private corpus result.
Scheme10 remains explicitly refused. No production code, database schema,
budget, UI contract, vendor execution or bus behavior has changed.

## Public evidence and its limits

**[I — indexed official reference]** KNX Association's *Import/Add products* describes `.knxprod` as XML
product data beginning with ETS4. Its registration/certification file-format
reference likewise associates that extension with ETS4, ETS5 and ETS6. [1][2]
Those statements establish a format-family boundary, not equivalence between
individual XML namespaces or compatibility with an independent importer.

**[unverified candidate]** A mirror URL was probed, but neither its cover
nor complete body was recovered. No author, document revision, date, page count
or XSD filename is accepted from that candidate.

The configured extraction tool reports a search-only backend and cannot extract
URL content. Bounded direct HTTP retrieval returned 403 for both official
references and an empty 202 response for the mirror candidate. The browser
attempt had no readable body; a Wayback availability check found no snapshot.
The two official format-family facts above are supported by retained search
index descriptions, **not** a reviewed full article or specification body.
No recovered specification hash, cover validation or full-document review is
claimed. Newer project schemas and third-party examples are not grammar proof.

The targeted search did not produce a validated scheme10 XSD. That is a limit
of this search, **not** a statement that none exists publicly. No member portal,
SDK credentials, manufacturer script or access-controlled resource was used.

## Current implementation facts

**[V]** `crates/knx-productdb/src/package.rs`, `master_scheme`, admits the exact
master namespaces 11/12/13/14/20/21/23; 10 is absent. The first unsupported root
returns `PackageError::UnsupportedNamespace`.

**[V]** The package's extended member gate rejects foreign element namespaces
and qualified attributes for 21/23 before local-name domain readers can turn
extension content into typed rows. Copying only a master allowlist entry would
not prove an equivalent boundary for 10.

**[V]** `parse/master_language.rs`, `supported_namespace`, also omits 10.
That byte-only evidence path has independent 64MiB/262144-item limits and is
shared with retained-source re-derivation. Package admission and language
re-derivation therefore require separate ownership/caller analysis; neither
may be broadened just because an XML root looks familiar.

**[H]** Existing parsers may cover a bounded part of scheme10, but no complete
ancestor/expanded-name or field-ownership comparison has yet established that.
The historical 146-package census in the owning goal is selection context,
not a new measurement, installed count or compatibility percentage.

## Next evidence before any implementation

1. Recover a verifiable primary schema10 description where available; keep
   manufacturer semantics outside any project-only description's proven scope.
2. Define the allowed public structural vocabulary from repository source and
   synthetic fixtures before examining authorized corpus XML. Preserve unknown
   categories explicitly; do not publish private names, values or item vectors.
3. Use bounded, net-isolated, no-extraction scans of complete ancestor paths and
   expanded names, separately for Master/Catalog/Hardware/ApplicationProgram
   and other package members. Recheck original hashes and count refusals.
4. Trace specialized parsers, package-only supplemental evidence, master
   language scanning and retained-source re-derivation independently. Document
   field ownership before proposing any namespace or scanner change.
5. Only if the differences are demonstrably bounded, define synthetic
   RED/GREEN acceptance, foreign/qualified/mixed namespace controls, late
   nonempty-database rollback, byte retention, replay and real CLI/HTTP callers.
   A later original-name release comparison and complete regression acceptance
   would be separate work, not implied by research counters.

No full ETS compatibility, signature verification, commissioning, runtime
semantics, KL-153 completion or Alpha completion follows from this checkpoint.

Retrieved/discovered 2026-10-04. Official references below are confirmed
through indexed descriptions only; no full article/specification review.

## Sources

[1] https://support.knx.org/hc/en-us/articles/360022205039-Import-Add-products — Import/Add products - KNX Association
[2] https://support.knx.org/hc/en-us/articles/4659247971346-File-formats-used-during-registration-certification — File formats used during registration/certification
