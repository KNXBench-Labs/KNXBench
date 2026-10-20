# ADR-0148: Repair released product-column promotions from retained source facts

Status: accepted; implemented and source-delivered as `8fa0d76b9fd41f81c4e135c29e54f2a98d516b38`
on 20 October 2026. [Acceptance record](../IMPLEMENTATION_STATUS.md#product-column-promotion-repair--source-delivery).
Date: 2026-10-20.
Amends: [ADR-0020](0020-migrations-may-rederive-from-stored-bytes.md),
[ADR-0126](0126-positional-text-placeholders.md),
[ADR-0146](0146-unknown-evidence-rows-have-an-identity.md).

## Context

WP-17 is delivered at product schema v27 and its shared migration/report claim
is released. Earlier v23–v25 promotions can choose a later same-file program
declaration for static object-caption references or parameter placement and
enumeration columns, and can retire unsupported evidence outside the promoted
parser context. Eight semantic regressions against unchanged v26 production
reproduced these defects; 24 existing baseline tests passed. Those early-schema
regressions alone do not establish repair of already-upgraded databases.

## Decision

Append **product schema v28**, a corrective data migration without a new domain
or project schema. Retain the released v23–v27 steps; the repair must work after
they have already executed. An empty new database performs no source work.

For the exact scope, invariants and acceptance matrix see the
[repair contract](../contracts/product-promotion-repair.md).

1. Derive only the affected columns and unsupported constructs from authentic,
   retained program sources. Keep the normalized program's stored source owner,
   first declaration, parser identity/defaults and traversal policy. Do not guess
   ownership from source sorting, captions or current values.
2. The repair is a narrow exception to ADR-0020's absent-slot policy: replace
   wrong **non-NULL** derived promotion values, and clear them when the winning
   declaration omitted the attribute. These columns are byte-derived catalogue
   metadata, not user edits or historical encounter facts. Unrelated values
   and all retained source bytes remain unchanged.
3. Reuse authoritative parser rules for unsupported evidence, including Dynamic,
   module paths, qualified names and scheme reconciliation. Do not add a parallel
   generic unknown scanner. When a scratch derivation is used, isolate it from
   durable state, preserve parser ownership context, bound one source at a time
   and avoid recursively invoking the corrective migration. Retain every
   unrelated unknown, conflict, master-provenance record and diagnostic.
4. Reconcile only the targeted unknown evidence in **existing measured** package
   reports, preserving every non-unknown count, source inventory, dropped-DPT
   count and historical diagnostic. Aggregate complete relevant member evidence
   in stored member order; do not use global-ledger additions as package deltas.
   Missing reports stay missing; unavailable reports stay unavailable. Validate
   persisted facts before and after any report rewrite using existing readers
   and persistence logic.
5. A missing, oversized, corrupt, unclassifiable or unparseable known source
   cannot authorize partial repair: preserve its original derived rows and
   evidence, record a named backfill diagnostic and downgrade affected measured
   reports through the existing unavailable mechanism. SQL failures roll back
   the enclosing migration. Never invent a successful install or historical
   observation by replaying sources. Missing/partial shared program aggregates
   cannot prove which member counter units were retired: preserve the package
   counter and mark ambiguous history unavailable. Complete aggregates and
   unambiguous single-member evidence remain repairable. Dispatch semantic work
   by stored member role/path; byte-identical opaque members stay opaque.
   Authenticate non-program target ledgers against retained-source parser facts
   before using them. Admit lexical/evidence work before semantic parsing and
   copies under the fixed conservative working-set policy; commit cumulative
   retained budget only after source success. Without a measurement marker,
   failure diagnostics surface as a reader error, not fabricated measurements.
6. Legacy EX-IM payloads retain their exclusion; opaque retained files are not
   XML inputs. Source identity and length/hash admission precede parsing.

## Compatibility and acceptance

Older builds refuse v28 product databases; retain an independent backup before
upgrade. Native/model schema remains unchanged. No download admission, bus
operation or new proprietary-format interpretation is introduced.

Test already-upgraded v27 state with contaminated non-NULL values and deleted
unsupported rows in addition to the earlier v22/v23/v24 entry fixtures. Require
fresh/migrated affected-surface equality, unchanged unrelated tables and master
provenance, shared-source package reconciliation, preserved missing/unavailable
history, corrupt-source preservation, SQL rollback and no-change reopen.
An independent review with no unresolved important findings and the maintained
change-scoped gate are required before delivery. Corpus changes require predicted
and inspected deltas, not blind re-pinning.
