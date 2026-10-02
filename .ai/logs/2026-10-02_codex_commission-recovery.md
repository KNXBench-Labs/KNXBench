# Commissioning recovery gate and review — 2026-10-02 22:11

## Integrated acceptance addendum

Source `743c3d29` was integrated as `09cd951df77bee212c70cf0006b6e6ce220fa1f5`
on the current upstream baseline. The retry process `proc_9fcff7e4dcdd`
completed with exit 0: all 15 receipts accepted, including workspace
2929 passed / 0 failed / 164 ignored, Web 1559 passed in 89 files,
17 matching shadow bindings and all 606 frozen code/configuration inputs
unchanged. The earlier integration attempt exited 1 and is not accepted
evidence. Fresh-worktree audit scope was nonempty: 448 packages, 400 headered
files, 333 Rust files for corpus policy, and 376 links across 239 documents.

An additional explicit offline private-fixture target ran against this exact
integrated revision: `cargo test -p knx-server --test http_device_download --
--include-ignored`. All 13 registered tests passed, with no ignored tests and
no unknown skip signals. The target injects an in-process SimTunnel; no socket
or real device is used. All 108 original project/product/database fixture
identities and hashes were unchanged. Raw private output was not retained;
only aggregate counters were collected. These 13 passes are additional
selected coverage, not a relabelling of the workspace's 164 ignored tests.

The public address/reset recovery gates, whole-device recovery, durable action
history, cancelled-future cleanup and hardware compatibility remain open.
No frontend source/binding or another owner's lock was changed. Publication
is a separate ref-readback step; this addendum records acceptance, not a
release or a premature publication claim.

## Scope and review

Base: d62baef4d3ec5d0311fb558d18b31ca4038c4826.
Owned worktree: iaw-alpha-commission. No delegated reviewer, Web source edits,
private-corpus read, live bus operation, new hardware authorization or release
waiver. Full original and corrected code diffs were reviewed separately.

Findings closed:
- IMPORTANT: connection setup could mutate PID 14 before a failed backup;
  postpone Verify Mode, record both originals and test every PropertyWrite.
- IMPORTANT: Device Control reads silently accepted trailing octets;
  require exactly one byte before deriving or persisting a recovery snapshot.
- IMPORTANT: simulator disconnect paths erased unrelated PID 14 bits;
  clear only Verify Mode in ordinary, forced, descriptor and read-drop paths.
- IMPORTANT: wrong-scope refusal sent an unsolicited cleanup Disconnect;
  pre-admit the exact scope before the connected operation/cleanup block.
- MINOR: CLI plan omitted Verify Mode and refusal tests checked only PID 8;
  list both writes and assert persisted nonzero originals in CLI/HTTP.

Final in-session verdict: approve this bounded recovery correction. Remaining
whole-device, crash recovery, native UI and durable-history gaps are not waived.

## Real verification

RED assertion failures were witnessed for backup ordering, malformed width,
format-2 persistence, CLI plan, ordinary and forced-disconnect preservation.
Wrong-scope no-frame regression also failed before its fix.
Focused GREEN: net 425/0/0; backup 4/0/0; CLI 8/0/0; HTTP 12/0/0.
Workspace totals include ordinary tests and doctests; 164 ignored tests were
not executed and are not private/hardware evidence. Web: 1559 in 89 files.
Gate runner held both Git-common-directory gate locks, refused competing
workspace Cargo jobs, used this worktree's target and redirected TS exports
to shadow bindings (17 compared using CI whitespace semantics).

```json
[
  {
    "step": "fmt",
    "command": [
      "cargo",
      "fmt",
      "--all",
      "--",
      "--check"
    ],
    "exit": 0
  },
  {
    "step": "clippy",
    "command": [
      "cargo",
      "clippy",
      "--workspace",
      "--all-targets",
      "--",
      "-D",
      "warnings"
    ],
    "exit": 0
  },
  {
    "step": "workspace",
    "command": [
      "cargo",
      "test",
      "--workspace",
      "--no-fail-fast"
    ],
    "exit": 0,
    "totals": [
      2929,
      0,
      164
    ],
    "result_blocks": 147
  },
  {
    "step": "build",
    "command": [
      "cargo",
      "build",
      "--workspace"
    ],
    "exit": 0
  },
  {
    "step": "layering",
    "command": [
      "cargo",
      "run",
      "-p",
      "xtask",
      "--",
      "check-layering"
    ],
    "exit": 0
  },
  {
    "step": "headers",
    "command": [
      "cargo",
      "run",
      "-p",
      "xtask",
      "--",
      "check-headers"
    ],
    "exit": 0
  },
  {
    "step": "anchors",
    "command": [
      "cargo",
      "run",
      "-p",
      "xtask",
      "--",
      "check-anchors"
    ],
    "exit": 0
  },
  {
    "step": "corpus-policy",
    "command": [
      "cargo",
      "run",
      "-p",
      "xtask",
      "--",
      "check-corpus-gates"
    ],
    "exit": 0
  },
  {
    "step": "deny",
    "command": [
      "cargo",
      "deny",
      "check"
    ],
    "exit": 0
  },
  {
    "step": "web-types",
    "command": [
      "npx",
      "tsc",
      "--noEmit"
    ],
    "exit": 0
  },
  {
    "step": "web-tests",
    "command": [
      "npx",
      "vitest",
      "run"
    ],
    "exit": 0
  },
  {
    "step": "web-build",
    "command": [
      "npm",
      "run",
      "build"
    ],
    "exit": 0
  },
  {
    "step": "whitespace",
    "command": [
      "git",
      "diff",
      "--check"
    ],
    "exit": 0
  },
  {
    "step": "shadow-bindings",
    "exit": 0,
    "files": 17
  },
  {
    "step": "frozen-inputs",
    "exit": 0,
    "files": 606,
    "changed": []
  }
]
```

## Compiled behavioral mutants

Each mutation returned test exit 101 with an actual FAILED test, not a compile
error. try/finally restoration and whole-source equality were verified.

```json
[
  {
    "mutant": "early-verify",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "commissioning::service_control::tests::backup_callback_precedes_every_property_write",
      "commissioning::service_control::tests::a_device_without_the_property_is_refused_by_name",
      "commissioning::service_control::tests::mask_0021h_is_refused_before_anything_is_written",
      "commissioning::service_control::tests::snapshot_preserves_both_properties_before_verify_mode",
      "commissioning::service_control::tests::an_already_set_bit_is_not_written_again",
      "commissioning::service_control::tests::failed_prewrite_backup_refuses_the_property_write"
    ]
  },
  {
    "mutant": "control-width",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "commissioning::service_control::tests::malformed_device_control_refuses_before_backup_or_write"
    ]
  },
  {
    "mutant": "backup-error-ignored",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "commissioning::service_control::tests::failed_prewrite_backup_refuses_the_property_write"
    ]
  },
  {
    "mutant": "no-op-writes",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "commissioning::service_control::tests::an_already_set_bit_is_not_written_again"
    ]
  },
  {
    "mutant": "scope-cleanup",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "commissioning::service_control::tests::another_scope_does_not_cover_the_write"
    ]
  },
  {
    "mutant": "snapshot-not-original",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "service_control_backup::tests::saves_the_entire_original_property_without_overwriting"
    ]
  },
  {
    "mutant": "cli-original-control",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "device_service_control::tests::read_then_enable_prints_what_the_device_holds"
    ]
  },
  {
    "mutant": "http-original-control",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "enabled_it_reads_sets_bit_2_but_serial_write_still_requires_recovery"
    ]
  },
  {
    "mutant": "simulator-clears-whole-property",
    "exit": 101,
    "compiled": true,
    "failed_tests": [
      "commissioning::service_control::tests::simulator_disconnect_paths_preserve_other_device_control_bits",
      "commissioning::service_control::tests::disconnect_clears_verify_mode_without_erasing_unrelated_bits"
    ]
  }
]
```

## Delivery

PENDING commit/publication and post-document anchor/whitespace gate. The
42-source reconciliation and remaining runtime/history work stay open.
