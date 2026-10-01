# K6 recovery: diagnostic-source scope check (2026-10-01)

## Scope and evidence

Offline reading only. KNX Association's Individual address, Device Info and
Device Reader documentation was compared with the already gated K6 CLI/HTTP
entry points and ADR-0059. Findings and primary links are in RESEARCH §24;
KNOWN_LIMITATIONS §116 and `goal-commission.md` now make the gap explicit.
No gateway, target device, bus write, key, or Web source was accessed.
Rebased over the ISSUE-04 UI lock handover and published as
`3993d6236a479d1c778fdf576b885ce95b8eabb8`; local and remote SHA
matched on readback. Anchors (405 links/225 files) and diff check passed
after the rebase.

- A programming-mode response count establishes a protocol recipient, not a
  complete device/product/application/storage identification. Line scans
  include address and mask; Device Info offers additional data but varies by
  mask and device.
- Device Reader accepts selected memory regions; unloaded array properties
  larger than 64 bytes can be exported blank. A saved diagnostic file cannot
  be treated as a complete pre-write image without proving scope and coverage.
- The consulted diagnostic pages do not specify the persistent areas changed
  by MP §2.3 for the historical MDT 0701h device or another button-selected
  device. This is an evidence gap, not proof that no manufacturer documentation
  exists or that a particular area is changed.

## Result and next input

K6 stays fail-closed before tunnel. The next design requires actual target
product/application identity, authoritative per-mask affected-storage and
access information, an exact persisted/read-back pre-send record, and an
abort/restore plan before asking for a fresh device-specific live go. No
access key may be guessed; no previous live result grants future permission.
Do not substitute a plan-scoped download backup or a Device Reader export
with missing arrays. The separate UI owner reacquired the Web lock for
ISSUE-04 while this check was in progress; a later K6 UI package can address
the misleading 412 consent affordance after that lock is released. This
package made no UI edits.
