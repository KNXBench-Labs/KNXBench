# CLI System-2 validation TCP transport — 13 November 2026

## Bounded source candidate

`device system2-validation` socket-backed execution and recovery admit strict
`--tunnel-transport udp|tcp`. Omission retains the existing patient UDP ACK
policy; TCP keeps the stream policy without fallback, reconnect or replay.
Preview and `--simulate` refuse explicit selection before input/history/output
I/O. Invalid gateways are refused early. Actual opened transport/assigned address
and positive TCP skipped-frame counts sampled before disconnect appear on ordinary
stderr, also on failure and without recording. Unavailable is never invented as zero.

Every existing exact confirmation, physical-source and identity admission, fresh
project/device/product binding, immutable backup before any mutation (including
setup), durable history/report classification, no-overwrite and cooperative stop
boundary remains. No Net/Core/ProductDB/schema/policy changes are part of this slice.

## Verification ownership and limits

Maintained public-CLI regressions use the existing fictional saved inputs and the
shared bounded TCP/cEMI simulator bridge. The package exercises literal configured
NEW, independently seeded OLD, failed execution, then a fresh CLI recovery process
against the same retained device and genuine operation. Whole modeled OLD equality,
original immutable evidence and separate result/history assertions are the recovery
oracles; a successful process exit alone is insufficient. Snapshot equality is the
existing simulator's persistent-state model, not arbitrary allocator rollback or
physical recovery. UDP controls are deliberately refused handshakes, not completed
UDP acquisitions.

Focused author execution passed by source-bound composition: the initial owning
integration run passed **31 tests** (legacy validation, new TCP validation and
the reused observation fixture's sibling tests). The supplement passed **7 tests**
after adding recovery UDP/refused-TCP controls, recovery phrase/EOF refusal and
the exact wrong-device outgoing suffix. The strengthened original-evidence test
then passed **1 exact test**, with its immutable checkpoint bytes detached before
the first wire mutation. These overlap: **32 unique integration leaves**, not 39.
The source-bound patient/diagnostic-finalization binary tests passed **3 tests**;
the Rust test-layout checker passed **4 tests**. Scoped all-target Clippy with
`-D warnings`, formatting and diff whitespace checks passed. Actual argv, direct
exits, source identities, full native output and copied CLI/test binaries are
retained in the private author evidence; no failed native test run was hidden.

The real public CLI completed configured TCP NEW, a deliberate final readback
mismatch with recoverable original evidence, and a separately confirmed fresh
CLI recovery against the same device and original store. The test requires whole
modeled OLD equality (including unowned bytes), unchanged original backup/binding/
download facts/immutable checkpoints, consumed recovery admission and independently
reopened history states. Wrong actual hardware identity emits exactly the pinned
read/control-only recovery suffix; changed retained source refuses before contact.
Execution and recovery both disclose positive skips on EOF, without recording.
Backup-persistence injection blocks every mutation including setup. Default and
explicit UDP cover **handshake selection only** in both execution and recovery.

This is author evidence and self-review, not independent approval. Independent
review, final maintained package acceptance and publication belong to the parent.
No hardware, release, deployment, broader programming or wider WP-29 closure claim.

[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md) ·
[CLI manual](../manual/user-guide/10-command-line.md)
