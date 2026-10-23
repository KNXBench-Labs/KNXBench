# Project-bound experimental System-2 offline preparation

Status: scoped source delivered on main-first merge `25b5b6a88`; maintained branch and integrated gates accepted. This is offline preparation only, not hardware or general ETS compatibility acceptance.

## Scope

This is a separate, opt-in App/ProductDB path for preparing an offline candidate from a supplied domain Project and selected DeviceId. The ordinary image builder, existing defaults-only experimental preparation, and existing hardware-consent rules do not gain additional admission.

Project-bound describes the desired configuration, not a previous installation. The supplied project may be newly created and unsaved. A newly added catalogue device, a previously configured device, and a replacement instance for reused hardware use the same preparation boundary once the desired application/configuration and address are explicit. Preparation does not create a missing instance or contact hardware. The old contents of reused hardware remain unknown, not inferred from product defaults or the desired project.

## Input and binding contract

Resolve the application and derive parameter values, group links and exported instance flag overrides through the existing project-to-request mapping. Do not construct an empty/default request in place of the chosen project device. Ambiguous aliases, unconsumed explicit overrides and unsupported required encodings refuse rather than disappearing.

The project-specific policy requires explicit versioned interpretation names. Only exact source namespaces admitted by that policy are considered. Namespace admission is not complete support for every construct in that namespace. Unknown mandatory source information still refuses.

Product input is an explicit choice: an actual standalone package identified by its package digest, or a retained-source closure identified by its own descriptor digest. The latter retains exactly two ordered originals belonging to the selected Hardware2Program and ApplicationProgram owners, with current successful parsing and unique-occurrence witnesses. It does not invent package membership, historical common-archive membership or manufacturer authenticity. Missing package evidence does not silently downgrade a package selection into source mode. A project archive is not an installable product package.

Discovery, preparation and retained-input reopening/replay enforce the same bounded XML source boundary. Qualified non-namespace-declaration attributes refuse rather than gaining canonical meaning through a local-name fallback. Namespace declarations cannot supply missing ordinary semantic attributes: a declaration matching a consumed key requires the same unqualified attribute on that element. Ordinary unrelated namespace prefixes remain allowed. This consumer-aware admission table must be reviewed whenever relevant ProductDB attribute consumers change; it is not an exhaustive future-schema guarantee. XML names are checked independently of namespace admission; this remains structural validation, not full XSD validation. Both product-input modes must reproduce resolution, effective request, image, normalized plan and coverage through fresh production ingestion, without falling back to a live catalogue.

Retain enough exact inputs to reproduce the Project-to-request-to-image-to-plan path offline, including the supplied domain snapshot, selection, effective request, source/package and profile. Bind preparation identity separately from plan identity: two project inputs can have the same effective plan without being interchangeable operation evidence. This identity detects accidental mismatch, not authenticity or provenance.

The retained domain snapshot is not an original project archive and does not claim to carry infrastructure-owned opaque data absent from that domain model. Preparation does not mutate the input Project or product database. Evidence and recovery records remain private and bounded; a record that cannot fit the existing persistence contract must refuse before execution.

In the unpublished project-v2 record grammar, retained snapshot, profile and product-original bytes use canonical padded STANDARD base64 JSON strings. The retained-source pair is exactly two ordered strings. Numeric byte arrays and noncanonical encodings refuse; this is not a change to v1 records or ordinary plan/recovery octets. Decoded originals and their identity hashes remain unchanged. Encoded and decoded scalar bounds, decoded pair aggregate bounds, the existing outer persistence limit and recovery reservation all apply independently. Compact representation does not authorize raising a limit, trimming the project or altering source originals.

## Explicit experimental image interpretation

The project policy starts from the retained source template, applies explicitly selected inactive legacy patch rules, then overlays active configured references through the existing evaluator and parameter codecs. It validates inactive placements and alternative types as well as selected fields. Masks, field bounds, exact Union membership and table protection remain enforced. Coverage must distinguish retained template fields from fields actually initialized or overlaid.

This is a named experimental interpretation, **not a universal definition** of invisible-parameter or legacy-patch semantics. Agreement with synthetic tests or a simulator does not establish engineering-tool equivalence or physical-device support. The existing defaults-only policy retains its own interpretation and output.

Computed group-table content and declared table capacity are different intervals. In the project policy, a source byte is selected only if its source mask permits writing and it is either outside the address/association reservation or inside the validated computed table interval. Unused capacity is not padding to transmit; independent source content outside the reservation is not silently discarded. All computed intervals must have an unambiguous valid source placement. Table values are payload, not transport write-mask instructions. Declared allocations and recovery coverage remain separate from direct-write selection.

## Execution and backup boundary

Project candidates are intrinsically simulator-only and cannot be rebound into physical approval. The existing simulator executor and durable recovery machinery exercise the normalized plan; no new bus route is introduced.

Before a device-changing operation, a fresh target- and operation-bound backup of the actual affected state is required regardless of task, prior project membership, load state or whether a device is new or used. It must precede implicit setup mutations too. Persist and verify the backup before mutation; missing required coverage or failed persistence refuses the operation. Offline plans and source defaults are not actual-device backups. Keep the original pre-state through failed operations and recovery attempts. A valid backup is necessary, not proof that every indirect device effect can be restored.

This package does not audit or certify every other device-changing operation. Hardware backup acquisition, application compatibility and physical restoration remain separate from offline preparation and simulator acceptance.

## Acceptance and remaining boundaries

Required software evidence is synthetic source installation, real Project-to-request mapping, both admitted namespace variants, retained-input reopening/reproduction, unchanged default behavior, refusal of unconsumed/ambiguous configuration, exact mask/capacity/source-content selection, and same-device simulator success/failure/cancellation/explicit recovery with an independent OLD state. A destructive-Unload simulator is a counter-hypothesis, not a firmware observation.

No general descriptor-flag normalization or alternate association placement algorithm is introduced. Compare association meaning, including indexed sending selection, separately from byte order and layout. Unresolved external-tool differences are not permission to change a codec until its rule is established. No full ETS compatibility, physical recovery guarantee, hardware release or deployment is claimed.

## Independently inspected document boundary

The KNX Association's *Project Scheme20* v01.00.00, printed page 4, explicitly excludes manufacturer product-dependency/download-image construction from its project-format scope. Printed page 22 lists `None`, `Background` and `DefaultValue` as invisible-parameter behavior names; it does not establish the image-initialization precedence adopted above. The document was independently downloaded and text-inspected. Its scope limitation is retained; it is not used to relabel an experimental interpretation as a standard-defined rule.

Public primary document: https://support.knx.org/hc/en-us/article_attachments/360024169360

Related existing contracts: [source-bound simulator validation](up258-system2-validation.md) and [ADR-0149](../adr/0149-source-bound-system2-validation.md); gated hardware [ADR-0152](../adr/0152-gated-hardware-system2-download.md). Their historical defaults-only results are not re-labelled as execution of this project-specific path.
