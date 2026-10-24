# OpenAI takeover — 24 October 2026

## Reason and ownership

Claude reached its session limit. The owner authorised an OpenAI fallback.
Five live Claude workers received stop requests; committed candidates and
existing gate subprocesses were preserved. This is a model/ownership
handover, not feature acceptance or a new programming approval.

The owner previously authorised work on blocked packages. Missing evidence,
source contracts, recovery requirements and explicit runtime confirmations
still apply. The existing global excluded-device policy is unchanged. No
further bus operations are part of this takeover.

## Source candidates

| Package | Branch | Head at takeover |
| --- | --- | --- |
| Vendor-plugin download refusal | `fix/plugin-download-refusal-20261024` | `f6b938c66` |
| Frontend native dialog directory | `fix/frontend-dialog-dir-20261024` | `8c320abbd` |
| Comparison link adoption | `feat/wp19-link-adoption-20261024` | `bdc50f7c4` |
| Project object folders | `feat/wp06a-object-folders-20261024` | `06f416f49` |
| Cargo development homes | `fix/test-home-isolation-20261024` | `e243ea70b` |
| SIGTERM fixture | `fix/browser-tmp-sigterm-20261024` | `8b2a50721` |

All six worktrees were clean when inspected. Main was `b75242b28`.
SIGTERM and WP-19 each have a candidate-bound successful package receipt;
that is not independent review or merged-tree acceptance. Other attempts
include incomplete, interrupted or lease-expired receipts. Plugin and
Cargo-home gates were running at the takeover inspection. Preserve all
original attempts; do not report an outer shell's zero exit as gate success.

## Continuation

Three read-only Codex CLI reviews were launched with invocation-local
`--ignore-user-config`, official provider `openai`, and `gpt-5.6-sol` from
the installed model catalogue. Startup output confirms High effort for
safety/dialog and mapping reviews, Medium for hygiene. Persistent provider
configuration was not changed. The workers may inspect code but cannot
edit, build, test, publish or contact a bus.

Review outcomes, actual gate receipts and integrated-source verification
must precede publication. Run new gates serially: concurrent gates have
already exhausted shared workspace-lease waits. Raw field evidence stays
private; a comparison difference is not by itself an encoding defect and a
read-only observation does not admit a download.
