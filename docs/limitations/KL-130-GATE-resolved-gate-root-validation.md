# Resolved gate-root validation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-130-gate)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## §130 A gate binary can verify a directory that no longer exists

**Status.** Resolved by AR01 (2026-10-01); historical heading/anchor retained.

The historical description moved verbatim to [resolved limitations](../history/KNOWN_LIMITATIONS_resolved.md#130-a-gate-binary-can-verify-a-directory-that-no-longer-exists) (AR14D D5).
<!-- END retained catalogue record -->

<a id="earlier-resolved-description"></a>

<!-- BEGIN earlier resolved record -->
## §130 A gate binary can verify a directory that no longer exists

**Status.** Resolved by AR01 (2026-10-01); historical heading/anchor retained.

Before AR01, `xtask`'s checks derived their repository root at **compile time** from
`env!("CARGO_MANIFEST_DIR")` (`xtask/src/main.rs:41`, `:234`, `:283`), not from
the working directory at run time. A cached `xtask` binary built inside a
different worktree therefore keeps checking *that* worktree's path. When the
worktree is deleted, `check-anchors` fails with `cannot read .../DIN-3`, while
`check-headers` reports `0 files with a well-formed header ... 0 without one`
and still **exits 0** — a gate that inspected nothing and called it success.

**Evidence.** After the `din-3-goal-migration` worktree was removed,
`strings target/debug/xtask` still contained
`<old-sources>/.paperclip-worktrees/KNXBench/DIN-3`. `git worktree
prune` did not help (the path is in the binary, not in git metadata), and
`touch xtask/src/main.rs && cargo build -p xtask` did **not** rebuild it on
this ntfs3 mount. A build with a fresh `CARGO_TARGET_DIR` produced a binary
carrying `<old-sources>/KNXBench`, after which the same three gates
reported real magnitudes: anchors **382 links / 214 files**, headers **215**,
layering ok, all exit 0 **[V]**.

**Cost.** Any documentation gate run from a stale binary is worthless but
looks green. This is the skip-vs-pass failure of §129's corpus tests one layer
up: exit code 0 is not evidence that work happened.

**Resolution.** Current gate binaries select the exact runtime workspace root
from CWD or an explicit leading `--root PATH`, validate its Cargo workspace and
named members, and print that canonical target. They never climb to a parent
or fall back to the build tree. Required source/documentation scan roots must
be nonempty; layering requires every checked policy root as a workspace member
and resolved node. Corpus-gate output now includes actual Rust-file coverage.
See [verification targets](../history/../VERIFICATION.md) and `xtask/tests/gate_scope.rs`.

AR01 reproduced the old success over zero sources after deleting its own build
worktree, then verified new deleted-target refusal, valid/wrong targets,
intentional fixtures and behavioral guard mutations. A pre-AR01 executable
still has the old bug and must be rebuilt. The caller must still verify that
the emitted target/revision is the intended candidate; nonempty coverage does
not certify completeness or concurrent-tree stability. No native UI or bus
claim is added; the separate zoom entry numbered 130 remains open.
<!-- END earlier resolved record -->
