# Retained diagnostic admission boundary

## Question

Which already-retained device-diagnostics and universal-support results can the
diagnostic-bundle consumer copy without turning producer archive validity into an
unsupported credential-safety claim?

## Source-backed findings

- `device_diagnostics::validate` closes format 1 over one descriptor read and
  Device Object properties 12 and 78. It validates exact query order, service
  identity, response lengths, bounded error tokens, timestamps, build token and
  completion sequence. `bundle_export` then writes fixed member names and
  `read_export` independently rejects duplicate, missing, oversized, unknown or
  inconsistent ZIP members.
- The producer grammar deliberately preserves bounded bytes for `partial` and
  `refused-ambiguous` replies. Their service outcome is known, but their payload
  is not classified as credential-free. Retained admission therefore refuses
  non-empty payloads in those states and requires the exact producer-owned
  format-1 memory-reason sentence before invoking the writer.
- Format 2 adds `memory.json` with raw device-configuration octets. The existing
  validator proves the declared-range acquisition grammar and archive integrity;
  it does not classify arbitrary opaque or encoded octets as credential-free.
- `SupportReport::bundle` limits size and names two fixed members, but the public
  report fields remain mutable. Bundle success therefore cannot admit a report.
  Safe copying additionally requires the producer's closed kind/version/build,
  expected producer build, support levels, facts, origins, statuses, structural
  program IDs, numeric firmware versions, missing-evidence codes
  and withholding codes. Context step text and prior-run material are outside the
  currently proven subset.
- Retained server slots are exact single-result stores. Their token match must be
  checked before admission; a missing token may use only a recording carrying the
  same related token, never the slot's newest result. A historical device result
  is usable only from a complete, bounded, known format-1 end summary that parses
  as the same typed producer export.
- Project context and numeric device identity are separate provenance edges. A
  matching context plus a conflicting target is a conflict. Missing or foreign
  attribution may still expose the result's own admitted report, but cannot
  authorize a current-project device or catalogue projection.

## Implemented bounded subset

The retained admission wrapper owns exactly one fixed file vector and exposes it
only by consuming the wrapper. Generic identity results pass the source classifier,
then use the existing producer writer whose own strict independent readback runs
once before `report.json`, `properties.json` and the README are extracted. Support
reports pass the closed whitelist and invoke their existing writer once before its
two fixed members are moved downstream. The support identity digest retains its
compact serialization semantics through a streaming hash performed only after
admission. The request collector moves each admitted file vector once into the
final sections reused for preview, text, review fingerprint and ZIP output.

Direct retained results work independently of recording enablement or deletion.
Known historical format-1 identity summaries can supply the same admitted files
after the bounded slot has moved on. Their file read is capped at 8 MiB plus one
overflow byte; header and end are parsed from the same owned snapshot and matched
to the observed summary, token, target and optional project context. Missing or
unproven context is preserved as absent: the independently admitted report remains
available while device and catalogue projections do not. Direct counted controls
cover exact-limit, truncation, growth overflow and operation-id path rejection at
the limited reader itself. Unknown versions and malformed summaries are not interpreted.

**Owner decision, 31 October 2026.** Declared-memory evidence and raw
non-System-2 recordings stay unclassified but may leave after the bundle's
explicit consent, with a visible "possibly passwords; keep private" note.
Recognised keys are already redacted by the recorder at capture time. This is a
consent decision, not a credential classification. Device and catalogue extracts
are projected only for a result whose attribution equals the unchanged current
context (its own device), or for a user-chosen device when no source is selected.

The source correction follows actual focused policy REDs (two App privacy cases
and six Server history/availability cases). Focused GREEN, defaults, lint and
static verification, including the new direct reader controls, remain pending
outer-controller execution.

## Remaining proof obligations

- an independent credential classifier for declared memory and raw recordings
  (future work that could replace the consent-only warning of the owner decision);
- focused and default runtime verification, lint/static gates and independent
  integration review of the candidate.
