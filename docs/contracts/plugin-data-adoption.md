# Plug-in data adoption

Status: implemented for the CLI and the application layer (17 October 2026).
Decision: [ADR-0136](../adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md).
Limits: [KL-186](../KNOWN_LIMITATIONS.md#catalogue-kl-186).

## What it does

A manufacturer plug-in editor (for example the econ SmartSensor editor) keeps
its configuration in ETS in two places. KNXBench retains both byte for byte on
import and never interprets or executes them ([ADR-0025](../adr/0025-extension-is-data-not-code.md)):

| Channel | Where | How the device refers to it |
| --- | --- | --- |
| `BinaryData` | `<P-xxxx>/BinaryData/<guid>.dat` | the device's `BinaryData` reference (`DeviceInstance.binary_data`) |
| `ExtraData` | `<P-xxxx>/ExtraData/<n>.<ext>` | a device parameter whose value is the number `<n>` (observed, not a documented ETS rule) |

*Adoption* replaces the destination project's bytes for those entries with the
bytes of the same entries in a source `.knxproj` of the same lineage, for one
chosen device pair. Nothing is decoded, converted or merged; the bytes are
copied verbatim and their SHA-256 is recomputed.

```text
knx adopt-plugin-data <source.knxproj> --project <target.knxdb>
    # lists device pairs (same ETS device id) and why each can or cannot take bytes
knx adopt-plugin-data <source.knxproj> --project <target.knxdb> \
    --source-device <id> --target-device <id>            # preview + confirmation token
knx adopt-plugin-data ... --confirm <token>               # apply
```

Listing and preview write nothing. The token binds the source bytes, the
request, the destination's exact state and its file; any change makes it stale.

## When it refuses

Because the bytes are opaque, only what the model knows can be checked. The
plan refuses, naming the one failed fact, unless the two devices have

1. the same product reference and the same application program reference,
2. the same `BinaryData` references (id and name),
3. the same ExtraData associations (parameter reference and file number),
4. the same group-address links (by communication object number, group address
   value and direction; internal ids are not compared),

and every named entry exists exactly once on both sides. It also refuses when
the destination already holds these bytes (nothing to do). A different
individual address is reported in the preview notes, not refused.

Remapping a blob id or ExtraData number would be a guess about the plug-in's
format, so it is never attempted.

## What stays

- Every other entry of the destination is carried unchanged; the model is not
  touched.
- The bytes a replacement supersedes remain in the project as
  `KnxBenchPluginDataSuperseded` entries under
  `knxbench-native/plugin-data/superseded/<sha256>/…`.
- One `KnxBenchPluginDataAdoption` record per adoption
  (`knxbench-native/plugin-data/adoption-<sha256>.json`) names the source
  archive digest, both devices and each replacement by digest — not the bytes.
- **Rollback is the saved version, not command undo.** The opaque store is
  outside the command undo stack. Saving an adoption records the previous
  saved state (with its bytes) as a project version; restore that version to
  go back.

## What it does not claim

- That the plug-in, ETS or a device accepts the adopted bytes. Not verified.
- That the source was produced by ETS. A source made with the original ETS6
  libraries outside the ETS application (no licence signature) is accepted
  like any other archive; see its provenance note.
- Any `.knxproj` output: KNXBench writes none ([ADR-0028](../adr/0028-no-knxproj-export.md)).
  The adopted bytes can be read back from the project.

## Tests

- `crates/knx-app/tests/plugin_data_adoption.rs` — 17 synthetic cases: exact
  replacement, retention, record, save/reopen, each refusal, stale apply,
  confirmation token, candidate listing.
- `apps/knx-cli/tests/plugin_data_cli.rs` — the real binary: listing, preview
  without writes, confirmed apply with a restorable version, wrong token,
  incompatible devices, usage errors.
- `crates/knx-app/tests/ets6_plugin_data_pair.rs`
  (`the_edited_plugin_bytes_can_be_adopted_into_the_unchanged_project`,
  ignored, needs the private corpus): on the real ETS6 export pair exactly one
  device is adoptable, with exactly its two plug-in entries replaced by the
  edited archive's bytes.
