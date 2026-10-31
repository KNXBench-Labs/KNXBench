# ADR 0156: Record every device operation; report problems through one consented bundle

Date: 2026-10-29
Status: Accepted
Amends: [ADR-0121](0121-tester-download-reports.md),
[ADR-0131](0131-universal-device-support.md) and
[ADR-0134](0134-generic-read-only-diagnostic-export.md), privacy clauses, for the
mail bundle only.

## Context

Field report #30 (System-2 download failing at the final step) could not be
diagnosed from the material users could send: each panel had its own report
button with its own minimised content, none kept the telegrams, and the System-2
verification threw away the bytes that differed. Six report entry points existed
with different consent texts. The owner decided the package in three rounds on
28 and 29 October 2026 (work package decisions 1-18, tables O1 and O2).

Verified facts this design relies on: every device operation in server and CLI
opens its tunnel through `KnxNetIpClient`, so one observation point in
`knx-net` sees all datagrams; the server data directory is the persistent volume
in Docker; `A_Authorize_Request` and `A_Key_Write` are the only services in the
current code paths whose payload carries key material.

## Decision

1. **Recorder.** `knx-net` offers a non-blocking wire tap on the tunnel client.
   `knx-app` turns it into one JSONL recording per operation with a bounded queue
   and one writer thread; recorder trouble becomes a gap line and never an error of
   the operation. Recording is on by default in the server (data directory
   `diagnostics/`, 100 recordings / 512 MiB, failed kept longest) and can be
   disabled only by the operator variable `KNX_DIAGNOSTIC_RECORDING=off`. The CLI
   records only when the user asks for a bundle.
2. **Redaction at the source.** Key octets are zeroed inside `knx-net` before any
   sink sees them and named by a `redacted` marker. This is unconditional, also
   with consent. Project copies are made secret-free before they enter a bundle.
3. **One bundle, two tiers.** Everything else (addresses, names, parameter values,
   gateway endpoints, telegrams) may enter the **mail bundle** after a per-section
   preview and an explicit consent checkbox, because the receiver is the project
   mailbox and the user chose what to send. **Public GitHub issues** keep the
   minimised reports of ADR-0121/0131/0134 unchanged. For the mail bundle only,
   this ADR replaces those ADRs' rule that addresses and names never leave the
   machine.
4. **One entry point.** The UI has exactly one "Report problem" wizard; the former
   report buttons become shortcuts that open it with the problem type, device and
   operation preselected. Defaults per problem type come from one table in
   `knx_app::diagnostic_bundle` (owner table O1).
5. **No upload.** KNXBench saves the ZIP and offers a `mailto:` draft without
   content. There is no receiver service.

The formats are specified in [diagnostic-bundle.md](../contracts/diagnostic-bundle.md).

## Alternatives considered

- **Record only failed operations.** Rejected: the success that preceded a
  failure is often the comparison needed, and the outcome is unknown until the
  end. Retention evicts successful recordings first instead.
- **pcap as primary format.** Rejected for now: JSONL carries decoded fields,
  tunnel events and notes next to the bytes and is readable without tools. A
  converter may follow.
- **Redact on export.** Rejected: secrets would sit in files on disk, and every
  future exporter would have to repeat the rule. Redaction happens once, before
  the writer.
- **Upload endpoint.** Rejected by the owner: it needs a receiver service,
  retention policy and abuse handling; mail is sufficient for the expected volume.
- **A UI toggle for recording.** Rejected by the owner: recordings are only
  useful when they exist before the problem; deletion is available instead.

## Consequences

- Tunnel code must keep calling the tap for every datagram it sends or receives;
  `client_tap_tests` and the server and CLI loopback tests compare recordings
  with the datagrams a simulator gateway observed, byte for byte.
- New services that carry key material must be added to the redaction table in
  `wire_tap`, with a sentinel test.
- New report-like features plug into the wizard as sections instead of adding a
  button; O1 changes are owner decisions and change one table and its test.
- Recordings contain installation details. They stay on the machine until the
  user exports them; the bundle preview names what is included and what is not
  removed.
- The data directory needs room for up to 512 MiB of recordings.
