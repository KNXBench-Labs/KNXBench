# Product scheme 23: bounded grammar research

## Status and scope — 2026-10-04

Research only, at `575a2d1d414bf747a5cda4e09429829e9295b52f`.
No product namespace admission, schema migration, runtime interpretation, UI
contract or compatibility claim is changed. Scheme 10 remains a separate,
later evidence question. This document records facts before implementation.

## Primary public source

**[D]** KNX Association's *Project Schema23 v01.00.00*, dated **2024-03-01**,
identifies the namespace `http://knx.org/xml/project/23` and associates its
schema 2.3 documentation with ETS 6.2.0. That is the document's dated scope,
not a claim about the latest ETS version. [1] (cover and §§1.4–1.5)

**[D]** Its stated purpose is ETS6 project interchange. It explicitly excludes
the manufacturer definition of products, parameter/group-object dependencies,
visibility and download-image creation. It references the v23 XSD as part of
Manufacturer Tool 6.2.0 rather than furnishing complete manufacturer grammar.
The public project document is therefore not sufficient evidence to claim
standalone product/application or runtime compatibility. [1] (§§1.1 and 2)

Retrieved directly from the official attachment on 2026-10-04. The configured
text-extraction backend refused URL extraction; bounded direct HTTP retrieval
and local PDF text extraction succeeded instead. No SDK credentials, member
portal access or manufacturer scripts were used. Public document SHA-256 is
recorded in the retained `ar06s` research receipt; the PDF is scratch evidence,
not a redistributed repository fixture.

## Read-only structural evidence

**[V]** A net-isolated, descriptor-relative/no-follow scan rechecked all 853
original hashes. There are 852 readable master XML documents and one explicit
`BadZipFile` scan refusal. Canonical namespace counts are 11:395, 20:298,
10:146, 14:4, 13:4, 23:2, 21:3. These are structural census units, not
production-import acceptance counts or a market compatibility percentage.

**[V]** The two namespace-23 packages have eight completely scanned XML
members: two each of Master, Catalog, Hardware and ApplicationProgram.
There are no observed cross-document namespace mismatches, foreign element
namespaces or qualified attributes in this bounded sample. Four positive and
three negative controls exercise default/prefixed namespaces, foreign content,
value non-disclosure, DTD/entity refusal, malformed nesting and duplicate
attributes. Independent header-only reselection confirmed two exact roots;
all 853 original hashes were independently checked again.

**[V]** XML was read without extraction; no private names, paths, field values,
per-item vectors, configuration files, copies or databases were persisted.
Only combined counts, closed structural categories and aggregate identity are
retained. Inputs were supplied through a sealed RAM descriptor; the parent
handle was closed after consumption. Python omitted the seal constants, so
local Linux headers and compiler definitions were verified before sealing;
the first unsealed descriptor was closed without starting a private run.

**[A]** Name/namespace agreement does not prove enum/default/reference
semantics. The research ZIP/XML reader is not the Rust package preflight,
current install API, a complete XSD validator, retained-row evidence, a
roundtrip test or runtime acceptance. No namespace is admitted from these
counters. A fresh unchanged Rust importer build and real atomic-refusal probe
were checked separately: three fresh public stages passed (631 Rust tests,
zero failed, 25 ignored in 28 result blocks; strict ProductDB Clippy and a
fresh Release CLI build). The real original-name CLI rejects both selected
packages with the exact namespace-23 error and empty user-data tables. All 853
original hashes were independently rechecked after the probe; private temp0.
This confirms the existing boundary, not new namespace or semantic support.

## Existing implementation boundaries

**[V]** `crates/knx-productdb/src/package.rs:1179` accepts only exact product
master namespaces 11/12/13/14/20/21. Its extra namespace/qualified-attribute
validation is explicitly applied to scheme 21 (`package.rs:2310`). Adding a
new master match alone would bypass that established admission boundary.

**[V]** `parse/master_language.rs:51` already recognizes namespace 23 for
bounded retained-source language evidence. Its contract explicitly says this
is not typed product namespace admission. Package acceptance and evidence
re-derivation must not be conflated. Scheme-evidence/report consumers must
also be traced before changing the typed install boundary.

Before any production change, require public synthetic RED/GREEN tests for
exact namespace admission, foreign/qualified namespace rejection, malformed
and mixed documents, unknown-field reporting, retained archives, atomic
rollback and replay. Verify real original-name Release imports, queryability,
unchanged older namespaces and current full-corpus/caller evidence separately.
Unsupported semantics stay explicit; no vendor script or bus operation is
part of this task.

## Sources

[1] https://support.knx.org/hc/en-us/article_attachments/17389755651474 — KNX Association: Project Schema23 v01.00.00 (2024-03-01)
