# ADR-0059: Button-based address programming needs durable pre-write recovery

- Status: Accepted (2026-10-01)
- Scope: public CLI `program-address` and HTTP `POST /api/device-address/start`, offline change only

## Evidence and problem

MP §2.3 uses a programming-button broadcast read to find the one device that
will receive the address write. The CLI and HTTP route previously accepted a
new-address-bound phrase and opened a tunnel before identifying that device;
neither captured and verified a durable pre-write backup of its affected
storage. A valid phrase, exact one-button count, new-address occupancy check,
post-write readback and restart status do not substitute for such a backup.
The generic device backup is tied to a download plan, not to an address change
(ADR-0057/0058; RESEARCH §§22–23). No access key may be guessed.

A prior user-authorized `1.1.67 → 1.1.68 → 1.1.67` hardware round trip
(RESEARCH §19; KNOWN_LIMITATIONS §116) is historical evidence for that device,
not an automatic per-device durable backup contract for arbitrary future
button presses. The physical target is not even known to these entry points
until after the tunnel opens, which makes a naive caller-provided backup path
or address-only receipt especially misleading.

## Decision

1. A shared application-layer recovery precondition rejects **both** public
   confirmed entry points after address, wait and phrase validation but before
   the CLI runtime or server tunnel/lock acquisition. CLI exits unsuccessfully;
   HTTP returns `412 Precondition Failed`. Neither has a bypass flag or
   environment variable. Invalid input still fails first without a gateway.
2. CLI plan-only mode stays usable and says confirmed writes are blocked.
   HTTP `GET /api/device-address/phrase` remains read-only; the UI must not
   construe the returned phrase as permission to program. `status` and `stop`
   remain valid for any session already held by the server, but no new public
   session can start while this gate is active.
3. Keep the MP §2.3 implementation, `AddressProgrammingSession` and direct
   simulator coverage. HTTP status/stop, activity and exclusion tests inject
   a simulated session *inside the test harness* rather than reopening a
   public write route. ADR-0046's one-phrase and one-tunnel semantics remain
   a future contract, not current write availability. The low-level Rust
   protocol/session APIs are outside this CLI/HTTP guarantee.

## Reopening criteria (not authorization)

Identify the actual programming-mode device and supported mask before the
write; prove the complete affected-storage scope for that device; durably
persist and read back an exact, versioned, identity-bound pre-send recovery
record, with an operator-supplied key where necessary. Refuse ambiguous,
unsupported or inaccessible devices before any write. Test cancellation,
multiple buttons, partial transport delivery and deliberate restore/abort.
Only a fresh device-specific go may authorize a later hardware run. A prior
application dump, confirmation phrase or post-write response is insufficient.

## Verification boundary

Offline CLI integration binds a local UDP listener and checks no datagram.
Simulated HTTP tests check `412`, no connector call and unchanged device.
No Web source, real gateway, hardware write or secret content was touched.
