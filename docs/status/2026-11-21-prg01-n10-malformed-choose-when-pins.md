# PRG-01-N10 acceptance — 21 November 2026

Test-only regression pin for the delivered
[inactive whole-byte selection](../contracts/inactive-byte-write-selection.md): how
malformed `choose`/`when` input is handled. Simulator and synthetic catalogue only;
no bus, device, VM or hardware contact. No source behaviour changed; the existing
program-wide rule ("a failed proof omits nothing, the regular plan stands") already
held, it was only unpinned for these shapes.

| Shape (synthetic catalogue, source default Mode 1) | Observed outcome | Pinned as |
| --- | --- | --- |
| Two `isDefault` branches | evaluator accepts; proof fails | regular plan, `source-mask-only-v1`, nothing omitted |
| `isDefault` branch that also has `test` | same | same |
| `ParameterRefRef` without `RefId` | same | same |
| Empty `choose` beside the real one | same | same |
| `choose` with unknown controller reference | refused: `ImageError::Evaluation` / `UnresolvedParamRef` | refused by write preparation, read-only preparation, image build and offline plan (`ParameterEvaluation`) |
| Same `ParameterRefRef` listed twice in one active branch | admitted | identical plan and generator (`source-default-inactive-whole-bytes-v1`) to the single reference; ownership is set-based, so the duplicate cannot change which byte is exclusively inactive |

The duplicate-reference switch to the omitting generator is not drift: the
single-reference baseline in the same catalogue omits the same byte 4401h and binds
the same generator; the test asserts equality with that baseline.

Tests: `malformed_choose_when_shapes_omit_nothing_and_write_the_regular_plan`,
`a_duplicate_reference_in_one_branch_keeps_the_positive_selection_unchanged`,
`an_unknown_choose_controller_is_refused_in_every_consumer` in
`apps/knx-cli/src/device_download_inactive_policy_tests.rs`. Hardware-untested;
nothing here claims storage survival or ETS parity.
