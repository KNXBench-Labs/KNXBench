# Commissioning public-write route audit — 2026-10-01

This was an **offline, read-only audit** of public CLI/HTTP write entry
points after the K6 fail-closed gate. No gateway, device, socket, Web source,
credential or access key was used.

## Verified boundaries

- CLI `run_device_program_address`, `run_device_address_by_serial`, and
  `run_device_reset_address` and the corresponding published HTTP K6/serial
  starts fail before a tunnel when durable recovery is absent. There is no
  public HTTP address-reset route.
- CLI device download and restore share `device_download::execute`, which
  calls `run_memory_download_with_backup` before the first mutation. The
  on-disk backup reads the plan-overwritten regions and touched load states,
  syncs the file and directory, reads back and compares. This is **not** a
  complete device image or a universal rollback guarantee (KNOWN_LIMITATIONS
  "Commissioning readiness and pre-write backup boundary").
- Opt-in CLI/HTTP service-control bit-2 writes use the exact property-element
  backup before writing. Its receipt is not a backup of other memory. K14
  hardware erase remains refused. Group-value sends are assigned to
  `goal.md` (§5) and remain untracked for receiver effect; no scope transfer
  or code change is claimed here.
- Web `AddressProgrammingPanel` still shows the program action and asks for
  consent after fetching a phrase. An HTTP 412 from the server is rendered
  in `role=alert` via `api.errorMessage`; the panel has no focused 412
  regression and cannot currently program a device. This is an affordance
  and test handoff to the **Web lock owner**, not a request to weaken the
  server's fail-closed gate.

Added the bounded source-based findings to RESEARCH §24 and K6 status in
`goal-commission.md`. No manufacturer-specific effect or complete K6
storage scope was inferred. Rebased over the UI Site/Property closeout,
retained both handover entries, reran anchors (405 links/225 files) and diff
check, then pushed audit commit `f92114c248c65613f400f2a9cba9d322a2abb2f6`.
Local and `origin/main` SHA matched on readback. The UI lock was released by
its owner during this audit; a future Web change requires taking a new lock.
The next actual K6 work needs per-device/mask
identification and verified pre-send persistence/readback and recovery,
followed by a fresh device-specific go before any live write. No access key
may be guessed; never query `1.1.220`.

## For the goal.md / UI session:

When the Web lock is available, give the Program address tab an honest
unavailable-state affordance and a 412 regression rather than asking for
consent for an action the backend always refuses. Its current error is
visible, so this is a UX/contract handoff rather than a safety bypass.
