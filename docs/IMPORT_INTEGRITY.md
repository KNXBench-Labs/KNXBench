# Import integrity implementation

Publication scope is the reviewed feature branch only. Main integration and
releases remain separate decisions; the verification below records local
acceptance before branch publication.

See [ADR-0107](adr/0107-import-source-integrity.md) for the decisions and acceptance
boundary. No project-specific data or private measurements belong in this file.

## Behaviour

- ZIP reads bind decoded names to validated physical member indices. ASCII case
  lookup stays unchanged. Raw/decoded collision, local/central record, CRC,
  encryption and expansion guards remain in force; declared UTF-8 is strict.
- Schema-23 object trees accept both independent root lists and recursive channel/
  folder node lists. Per-device encounter order is stable; repeated references
  are deduplicated within that device. Foreign-namespace nodes are retained,
  not interpreted as project object declarations.
- Modern unassigned devices reuse the normal device grammar and retain no-line/
  no-address placement rather than fabricating topology.
- Both parsed project XML files are retained byte-exact. Unknown metadata
  subtrees are also captured. Foreign namespaces cannot impersonate known
  project-information or audit-trace slots; their source is retained and reported.
  Mapper output separates metadata attribute
  provenance from topology; actual archive filename casing is preserved.
- Source observations come from the parser's own stream, including skipped
  subtrees, under a fixed lexical vocabulary. They are explicitly not semantic
  acceptance. Legacy read/mapped counters describe mapper visits, not a complete
  raw-source census. Unconsumed object overrides remain in retained source and
  are named as warnings; they are not activated outside the declared tree.
- Manufacturer unknown findings and identity conflicts use persisted source-bound
  evidence for fresh ingestion and retries. Unavailable evidence differs from
  measured zero. Report errors/unknowns project through existing CLI/HTTP/session
  diagnostics; session-log capacity limits remain explicit, not completeness.
- Baggage description is neutral at the project parser boundary. The application
  reuses `knx-productdb::sniff_media` to distinguish pictures/documents/opaque
  payloads from executable content without decoding or executing it.
- Documented metadata such as completion status no longer falls through a
  measured-sample table hole. Unsupported values are preserved and reported.
  Completion vocabulary adds FinishedCommissioning, Tested and Locked.

## Native compatibility

Native and normalized schema12 is a scalar-vocabulary version barrier; v11→v12
is an identity migration. It prevents older native decoders from silently
substituting Undefined for the newly documented states. Unknown stored states
fail closed. Back up native projects before upgrading; no downgrade is supplied.
No extra native tables or hardware capabilities are introduced. This unpublished
schema12 must be reconciled with any parallel unmerged schema work before future
integration. Historical opaque attributes are not guessed into newly typed fields.

## Verification and limits

Named synthetic tests cover the actual new branches, native persistence and
manufacturer retry. The explicit private-source regression was executed locally
through the production service, including idempotence, save/reopen/re-save,
product verification and unchanged input bytes; it was not a missing-data skip.
Independent bounded ZIP/XML observations were compared with the real native and
product rows and exact retained member payloads. Original-input production CLI
import, HTTP/session-log and frontend component contracts passed. Full Rust/
frontend tests and builds, warning-denied Clippy and repository/documentation
gates passed on frozen code. An authentic prior native11 reader and independent
synthetic input additionally verified row/schema preservation on upgrade and
older-reader refusal of native12 without mutation. These are local self-reviewed
results, not independent review, public release or whole-ETS acceptance.

Source/model/blob comparisons, detailed source bindings, rejected attempts and
final acceptance remain only in ignored local evidence. The original archive,
not a repackaged substitute, is the private acceptance input. Import reports and
session logs are not a newly persisted native report format: reopen retains
byte-exact source from which lexical observations remain independently
reconstructable; it does not manufacture a historic measured report. Native UI,
accessibility and hardware were not exercised by the API/component checks.

Full ETS semantics, signatures, vendor executable behaviour, original-format
export, native accessibility and hardware commissioning remain unverified.
F10-style extensions (channel declarations, additional addresses, IP configuration)
remain retained rather than fully modelled. F11-style parameter/loader execution
work is separately bounded and does not follow from successful import.

## Primary references inspected

- KNX Association, *Project Schema23 v01.00.00*, §§1.1.2.6, 1.2.4.6,
  1.2.5.21–23 and 1.2.5.26–27. Official document:
  https://support.knx.org/hc/en-us/article_attachments/17389755651474
- PKWARE APPNOTE, general-purpose encoding bit and Appendix D:
  https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT
- Dependency source for pinned `zip` 8.6.0: `SharedBuilder::build`,
  `index_for_name` and `central_header_to_zip_file_inner`; its index keys are
  raw bytes while displayed names are decoded.

These are documentation/source inspections, not XSD validation or certification.
