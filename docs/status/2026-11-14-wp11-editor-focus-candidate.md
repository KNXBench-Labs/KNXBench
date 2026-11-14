# WP-11 E4: editor focus candidate, not native acceptance

Status: source and regression-test authorship only, 14 November 2026.
No tests, builds, browser/native runs or gates have been executed for this
candidate. No fix verification, delivery or whole-WP-11 completion is claimed.

## Reproduced predecessor and diagnosis

Retained actual GTK/WebKitGTK execution on source `38d7ad5f5` found two failed
focus predicates in each of six Notes/Instructions cells (80/100/150%): after
trusted native Return on Preview, and after trusted native Return on Cancel,
no actionable web control retained focus. The old E1/E2 persistence results
remain bound to that old runtime, not this unbuilt candidate.

The relevant editor source is byte-identical between `38d7ad5f5` and candidate
base `9ea3c5a9c`. Both Preview buttons stay mounted but become natively disabled
during the asynchronous request; neither editor restored focus after re-enable.
Cancel removes the focused inline review control without restoring its owner.
Retained full native trees show the enabled Preview at the same accessible
path before and after each transition, and successful pre-action/button and
nearby dialog focus observations. This supports missing restoration over a
whole-editor remount or a generally blind collector. The intermediate disabled
blur itself was not captured separately; a new source-bound native run is
required to establish that this candidate repairs the observed failure.

## Candidate changes

- Capture the exact initiating button per request; restore only after its
  re-enable commit, or explicit Cancel's removal commit.
- Retire focus authority on navigation, owner/client/revision changes, passive
  revocation and unmount. Require the original connected, enabled, non-hidden
  target, without an Apply/Confirm or generic Preview fallback.
- Preserve the existing draft, request/cancellation, busy, conversion and
  mutation rules; no persistence/server/schema/dependency/CSS changes.
- Author mounted component regressions for held response, error, stale owner,
  abort/unmount, moved focus, passive revocation and action-specific Cancel.
  Author production-browser fixtures for real Tab/Enter Preview and Cancel,
  visible keyboard focus, held-response and no-Apply-request controls in EN/DE,
  Porcelain/Graphite, wide/narrow layouts. All remain unexecuted.

The [focus contract](../contracts/editor-preview-focus.md) defines the bounded
behaviour. Private raw failure evidence is preserved outside the repository.
No machine-specific paths or real project data are published here.

## Remaining acceptance

After the parent-owned running gate releases resources: independently review,
build and execute the focused component/production-browser checks, then repeat
the six failed native E4 editor cells against the actual new build. Retain the
same 12 focus predicates with real native Return, plus bounded Tab, moved-focus,
error/stale-owner and secondary-initiator controls. Preserve the old failed
record and bind every new result to its actual executable/frontend/source.
Do not replay the 36 unchanged persistence cells merely to repair focus, or
carry their old passes forward as new-source evidence. Wider checked-parameter,
screen-reader and other native acceptance boundaries remain unchanged.
