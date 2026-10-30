# Backup prior-application identity: a restore needs the application the bytes came from

Bounded source safety fix of 2026-10-25, on top of
[partial-identity-comparison](partial-identity-comparison.md) and ADR-0049.

**Additional replay rule, 29 October 2026 source candidate:** the
[exemplar-binding contract](regular-restore-exemplar-binding.md) adds mandatory
captured, bindable serial evidence and a live executor precondition. All format
1/2 files therefore remain readable but cannot restore, including partial files.
The application-identity rule below still applies independently; its partial-plan
precondition is not an exemption from the additional exemplar guard. Format 3
preserves required nullable `prior_application` and adds required nullable
`exemplar_serial`; older format-2 readers reject it. The current format-4 producer
retains both rules and adds explicit write-generator identity; see
[inactive whole-byte selection](inactive-byte-write-selection.md). Legacy formats
keep their historical generator and do not gain an exemplar exemption.

## Defect

`restore_plan` swapped only the `WriteMemory` octets and kept every `LoadRecord` and task step of
the plan it was handed. A complete backup read from application X before a complete download of
application Y could therefore be prepared with Y's load procedure wrapped around X's bytes. The
shape, target, mask, manufacturer and load-state checks did not notice. A partial plan was safe
only because it carries an exact five-octet identity `CompareProperty` that the executor runs.

## Contract

- `DeviceBackup.prior_application` is `PriorApplication::Recorded([u8; 5])` or `NotRecorded`.
  `Recorded` is one exact five-octet answer for object index 3, PID 13
  (`MM MM TT TT VV`, [RES §4.2.13](partial-identity-comparison.md)), read on the same connection,
  after the mask/manufacturer checks and first in the backup (before the load-state and region
  reads), before the durable `keep` callback and before the first change.
- Only the backed-up execution reads it (`run_memory_download_with_backup`). Unbacked runs,
  `read_what_the_plan_overwrites` (preview) and `compare_with_plan` keep their traffic; the
  read snapshots default to `NotRecorded`. One read, no retry, no reconnect.
- A refusal (`nr_of_elem = 0`), an answer of any width other than five, and acknowledged silence
  on a still-live connection are `NotRecorded`. A complete download over an unloaded, different
  or unreadable application stays possible; its backup just cannot be restored. Transport,
  encode, connection-lost, connection-released, lagged and every other session error abort like
  any backup read: disconnect, no `keep`, no write.
- No identity matching precondition is added to complete downloads. A partial plan's own
  precondition stays first and strict.
- `restore_plan` (the lowest public derivation, so `prepare_restore`, the CLI and any other
  caller) refuses unless the plan names exactly one five-octet identity
  (`application_identity`; a padded or longer compare is not one, and no prefix rule is invented)
  and either the backup `Recorded` the same identity, or it is `NotRecorded` **and** the plan
  carries an exact five-octet identity `CompareProperty` equal to that identity ahead of the
  plan's first device-changing step (the legacy partial route, enforced by the executor against
  the live device). A matching compare placed after a change is no precondition; the executor
  refuses such a plan anyway (`CheckAfterWrite`, before any traffic). A `Recorded` identity that
  differs from the plan's refuses even behind such a precondition; `StoredBackup.partial` is
  never consulted.
- Errors: `PlanApplicationUnknown`, `PriorApplicationNotRecorded`, `OtherApplication`. The CLI
  prints them through its existing `no restore of <file> prepared: ...` path before any connector
  opens; the server has no restore route.

## File format 2

`prior_application` is an explicit nullable ten-digit hex string (`"0083002715"`) or `null`.
Format 2 requires the field; `null` means `NotRecorded`. Format 1 files are still read, never
rewritten or migrated, and read as `NotRecorded`; a format 1 file that carries the field is
refused, as are a missing field in format 2, a non-string, short, long or non-hex value, unknown
or duplicate fields and unsupported versions. Create-new, sync, read-back, directory sync and
owner-only permissions are unchanged. **Downgrade limitation:** builds that only know format 1
refuse format 2 files; there is no bidirectional compatibility. Existing complete backups without
evidence now read but no longer restore; legacy partial backups continue through their plan
precondition. The durable file stores the pre-write observation, not a later post-download one.

## Limits

This is a necessary guard, not proof. It does not show that X's bytes fit Y's allocations, that a
physical device is the same exemplar, that the final identity after a restore is X, or that a
whole-device recovery works; that stays unproved and admission is unchanged. Matching identity
does not bypass the shape, target, mask, manufacturer, load-state or procedure checks.
Simulator and synthetic product data only; no bus, network or hardware access.

## Tests

- Core: complete plan restores only the recorded same application; other, unrecorded, unknown,
  conflicting and padded identities refuse; precondition admits unrecorded but never conflicting;
  an identical compare after the first change is no precondition; matching identity does not
  bypass shape/target checks.
- Executor (simulator): original-source regression (X backed up under a Y download refuses),
  same application restores the original octets, absent/truncated/oversized/empty/silent
  PID 13 download but never restore, dropped, released, transport-failed and lagged identity
  reads abort before keep and write with their original error, event order (plan compare and
  `RequireLoaded`, then PID 13, then load states and regions, then `keep`, then the first
  write), a late identity compare refused as `CheckAfterWrite` with no traffic and no `keep`,
  unbacked and preview runs read no PID 13, legacy partial restore stops at a wrong live device.
  `Encode` cannot come out of a transport, so the result classifier is tested directly. These
  characterize behaviour that already aborted; they do not fix a defect.
- App: format 2 round trip, explicit null, format 1 read-only compatibility and smuggling,
  malformed/missing/duplicate refusals; restore preparation against a synthetic product database
  (a forged `partial` selection on a complete plan stops at the procedure guard,
  `DifferentProcedure`; the identity guard is covered in core).
- CLI: `knx device restore` refuses before any connector (loopback socket sees no traffic);
  opt-in corpus tests assert the durable file's evidence and the refusals; the server's corpus
  HTTP test asserts the durable file's `prior_application`.
