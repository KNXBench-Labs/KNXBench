# Issue #19 — link the public board, leave the snapshot out

## Scope and evidence

Owner requested the new issue be checked/fixed, then explicitly preferred a link
only to the GitHub document rather than another bundled build dependency.
Baseline frontend Docker target failed with UNRESOLVED_IMPORT for
../../../docs/community/hall-of-fame.json?raw in bundledHallOfFame.ts.
The replacement About component renders only the existing canonical board link;
no JSON is imported or fetched and no participant data is parsed or displayed.
Repository generator, policy, awards and public JSON are untouched.

## Self-review (not independent approval)

- CRITICAL: none.
- IMPORTANT: none remaining. Production imports stay inside the app; URL comes
  from the fixed repository constant, not project/input data; target=_blank is
  paired with noopener noreferrer. No project/domain/protocol behavior changes.
- MINOR: removed misleading claim about the native external-browser route from
  the manual; native link opening is not newly certified by Chromium coverage.
- Removed snapshot-specific tests cover deleted functionality, not relaxed
  assertions. Retained six unrelated About tests and added the link-only
  regression; RED was six passes/one expected failure, GREEN eight targeted
  passes including the canonical URL test. Browser coverage checks both locales,
  both widths, both themes, no background request and keyboard link activation.
- Static added-line scan: no secret literals, dynamic evaluation, unsafe HTML,
  shell/SQL execution or serialization added.
- Parallel P1 edits to this browser fixture only add its no-open-project bootstrap
  answer; preserve that separate change if P1 publishes before integration.

## Delivery boundaries

All 31 branch web/docs stages accepted with frozen inputs: Vitest 3500/0,
Chromium 531/0, production dialogs/link 18/0, repeats 12/0, tools 122 (one skip),
all builds/types/documentation/xtask checks zero. Four actual production EN/DE
1440/400 frames inspected; no clipping or overflow. The full precommit Docker
image builds, responds with HTTP 200 for version/index/asset, contains the canonical
link, and stops with exit zero under network=none and ephemeral tmpfs data.
The probe was removed. Original queued gate was cancelled before its first stage
because the external production config lacked explicit webServer.cwd; replacement
used separate stage-specific screenshot directories and passed.
Merged-result acceptance on `655f056e` (base `871cc285`, including P1, #20 and
#21): 31 stages zero/frozen, Vitest 3547/0, Chromium 546/0, production 18/0,
repeats 12/0, tools 123 (one skip); Docker image build, network-none link smoke
and the repository `smoke-test.sh` (isolated port 18419) pass. The session crash
interrupted the gate on `04ca7cf2`; its rerun (530/1, P1 toast spec under load)
and the gate on `71b9a346` (537/5, communication-objects timeouts at load ~50,
70/70 alone on candidate and main) are recorded as superseded, not accepted.
Interleaved toast runs at normal load: main 340/340, candidate 340/340.
Publication and closure follow this commit.
No running-container replacement, bus access, release/tag or research publication.
