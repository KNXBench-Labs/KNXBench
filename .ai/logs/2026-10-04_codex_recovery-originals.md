# Offline recovery-record admission and original-byte preservation

## Scope and retained work

The independent `iaw-recovery-originals-20261004` checkout starts from
`5f017a29798ea3179c155390e3e44e85cbf0bb8b`. The original stash
`9d14c93ced6fb6018f3dc8aeebf520cd1caec11d` is retained unchanged. Its baseline
production prefix was compared byte-for-byte before tests were reintroduced;
the validator was restored only after a current-source registered runtime RED.
No Web, root/main or hardware source state is modified by this work.

## Actual evidence

- `semantic-red-1/receipt.json`: compile0, exactly1 registered semantic test;
  separate original-byte preservation positive1/0/0; named runtime101 at the
  intended format assertion. All stage source manifests remained stable.
- `semantic-green-1/receipt.json`: five admitted stages on the restored
  validator: fmt, six backup tests, App97/0/21, strict Clippy and whitespace.
  The changed `knx-app0.1.0-alpha.3` crate was actually rebuilt. Counts are
  separate scopes, not summed into whole-goal coverage.
- Original `PID_DEVICE_CONTROL` tests cover every u8 value crossed with four
  representative full `PID_SERVICE_CONTROL` values. Lowercase lexical octets
  remain lowercase after roundtrip. Validation constrains syntax and record
  identity; it does not interpret or discard unknown bit values.

## In-session review

**IMPORTANT — writer-admission witness missing (service_control_backup.rs):**
The retained validator runs before directory creation, but the original
semantic regression covers deserialization only. Add a separate seeded-file
regression that requires malformed format, unsafe path, excluded target and
signed original-octet records to fail before creating directories or reaching
sync, preserving existing recovery evidence. The test has been added and
formatted; it subsequently registered and passed in all four control batches and the fresh seven-test gate. A separate source control
must remove the writer's admission call and reach that exact filesystem
assertion before restoring canonical bytes. This is a coverage gap, not a
claim that the current production guard is missing.

The13-control inventory includes format/kind/time, address syntax, excluded
target, mask, object index, both property IDs, both original-octet fields,
unknown-field refusal and writer admission. After the earlier actual lease
refusal, all13 controls executed in four admitted batches: compile0, list1,
Runtime101 at the intended named assertion, canonical bytes restored. Each
batch reran backup7/0/0. The original refused lease remains infrastructure
rejection, not one of these13 executed controls.

## Boundaries and next steps

App program version advances to0.1.0-alpha.3; recovery wire format remains2.
Unknown fields or unsupported records are explicitly refused, not silently
coerced. This is a property-only backup, not a device image, identity proof,
write authorization, automated replay, ETS compatibility, or guaranteed
power-loss recovery. No hardware/bus/vendor/ETS/power-loss experiment is
required or performed. Broader abort/restore behavior and safe Unsupported
boundaries remain in the commissioning goal.

Fresh semantic-green-2 subsequently passed five stages: backup7/0/0,
App98/0/21, fmt/strict Clippy/whitespace, actual alpha.3 compilation and stable
source throughout. docs/evidence/service-control-backup-validation-2026-10-05.json
binds this local dirty snapshot, not unchanged base-commit source.
Next: review/commit, integrate current main and Caller, run source-bound gates,
then publish/read back. No Web source edit permission exists even though the
latest U21 owner reservation is released.
In-session review is not independent model or owner approval.
