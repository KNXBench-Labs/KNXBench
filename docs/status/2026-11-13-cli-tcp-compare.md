# CLI device comparison over explicit UDP/TCP — 13 November 2026

## Scope and unchanged policies

Bounded WP-29 source candidate: only the read-only `knx device compare` caller
adds `--tunnel-transport udp|tcp`. UDP omission remains the default; strict shared
parsing rejects invalid, missing and duplicate values before project, product or
history I/O. The selected client is configured before optional diagnostic tapping.
There is no fallback, reconnect or replay and no new gateway lease or timeout.
Original gateway admission/order remains unchanged.

The same preparation service, plan and optional partial selection feed the same
read-only comparison recipe: DD0/manufacturer and exact five-octet application
identity, APDU-limit query, planned load states and planned memory regions. No
access key, Verify Mode write, memory/property write or restart is introduced.
Protected/failed reads remain failures, never known-empty comparisons. Legacy
exits remain same **0**, different **2**, failed **1**. Optional activity history
still marks a completed difference `finished` and a failed read `failed`; no
history is created by omission. History settlement and best-effort outer cleanup
retain their existing order and meaning.

Stderr names the opened transport and assigned individual address. Positive TCP
skip counts are sampled before outer disconnect consumes the tunnel and reported
once with the existing helper, including failed reads/EOF without diagnostic
recording. Zero is silent; these are unknown/oversized frames whose contents were
not kept, not lost telegrams or final-wire totals.

## Maintained public-CLI evidence

The real built `knx` binary is driven against bounded loopback peers. A saved
synthetic MV-0701 project/product fixture is admitted through the real reading
planner before peers start. Literal expected memory bytes and literal different
bytes are independent of the planner's output. The peer checks the exact outgoing
T_Connect, descriptor/manufacturer/application/APDU queries, load-state and memory
reads, numbered controls, management disconnect, outer disconnect and actual EOF.
Any extra write/key/restart/Verify Mode service fails that sequence. Retained TCP
listener and UDP traps are checked again after the child exits to exclude late
fallback/reconnect/replay.

Coverage includes same/different exits and data, real activity-history readback
for finished differences and failed reads, skipped frames followed by a valid
reply and by EOF with recording disabled, zero-skip silence, refused memory reads,
a refused TCP socket held bound/non-listening through child exit, strict CLI
admission with absent inputs/history, and unchanged unrelated-command refusals.
Default and explicit UDP are **handshake-selection controls only**: the peer
returns `E_NO_MORE_CONNECTIONS` before acquisition. They do not prove a complete
UDP comparison; complete acquisition in this package is TCP only.

## Verification status and limits

Author-focused verification: the final focused integration run passed all **71**
registered transport/history leaves, including **11** public-CLI comparison tests;
the binary-local comparison run passed **7** tests. Counts are not summed across
overlapping runs. An earlier 70-test focused attempt remained failed (68 passed /
2 failed). Only the new test oracles needed correction: partial selection removes
the allocation record and therefore its load-state read; the unrelated-command
control must name the existing `service-control` command, not nonexistent `unload`.
The initial failed/corrected comparison runs used the same CLI binary; the final
focused run rebuilt it with the candidate revision metadata and unchanged CLI
source. Earlier fixture
admission failures (missing segment Data, then an incorrect low-first literal
expectation for default high-first numeric encoding) are retained separately;
none required a production-policy change.

Focused package Clippy (`--all-targets -- -D warnings`), formatting, headers,
four Rust test-layout tooling self-tests and documentation/local-information
checks passed. Independent whole-feature review, the complete selected package
gate, integration and publication remain parent-owned. This source record does
not claim those steps have passed. No real bus, device, gateway, serial interface or installation was
accessed. No release, hardware interoperability, KNX IP Secure, programming,
download/restore, server/Web comparison transport or wider WP-29 closure follows.

[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md) ·
[CLI manual](../manual/user-guide/10-command-line.md)
