# “Who talks to whom?” research decision

## Outcome

The final `goal.md` §7 research item now has a written decision in
`docs/RESEARCH.md` §16:

- build a selected-telegram flow inspector before any animation;
- resolve the observed source against configured `Send` links, preserving
  exact, ambiguous, and unresolved outcomes;
- describe `Receive` links only as configured recipients;
- treat reads and later responses as separate telegrams, with no timing
  heuristic presented as causality;
- keep the bus session's interpretation snapshot authoritative when the open
  project changes, extending the current group-address/name/DPT fingerprint to
  cover every device/object/link fact the flow uses;
- add no topology coordinates, persistence, domain entity, or KNX write.

The protocol boundary was checked directly against local KNX Standard v3.0.0
PDFs: Network Layer §2.2.2, Application Layer §3.1.3, Interworking Model
§3.2.3.1, and Data Link Layer General §2.2.1.

## Repository evidence

`GroupAddressNode.links` already projects both sides of each configured link.
`BusTelegramRow` already carries source, destination, service and decoded value,
and the monitor already has selected-row details. What is missing is immutable
per-session flow evidence and its honest presentation, not another domain
graph.

No product code, bus access, schema, endpoint, or UI changed.
