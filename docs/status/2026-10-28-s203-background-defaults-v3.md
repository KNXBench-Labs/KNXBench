# S2-03 delivery: defaults v3 and explicit-Background admission (28 October 2026)

Result record for [S2-03](../open-work/S2-03-up258-image-interpretation.md).
Published main-first as `d39668393` on base `6c2247161`: feature `3ed49015a`
(defaults profile v3) and follow-up `4f462822f` (explicit-`Background`
admission plus review test gaps).

## What changed

- Defaults profile v3 keeps the source segment `Data` for inactive parameters,
  like the configured path. Owner decision of 28 October 2026 after a renewed
  public source search that found no image algorithm for
  `DownloadInvisibleParameters`
  ([source search](../research/2026-10-28-background-precedence-source-search.md)).
  This is an observation-supported hypothesis, not a defined rule.
- New physical execution of a v1 or v2 bundle refuses before any socket;
  recovery of an existing operation and simulator reproduction stay available
  ([contract](../contracts/up258-system2-validation.md)).
- Template retention (configured path and v3) requires an explicit
  `DownloadInvisibleParameters="Background"`. `DefaultValue`, an `Options`
  element without the attribute, an absent `Options` element, `None` and
  unknown values refuse before an image is returned
  ([contract](../contracts/project-system2-offline-preparation.md)).

## Evidence

- On the pinned public UP258 package (simulator only), v3 writes the same 775
  octets as the configured defaults bundle. It differs from v2 in 94 octets,
  each equal to the stored source `Data`. Every changed bit lies in an
  inactive parameter field and no active bit changes; a test checks this
  independently of the image builder.
- Admission fix: test-first, 3/3 semantic failures before the fix. Behavioural
  mutants (grammar binding, v3 reading, v2 refusal, v3 bundle suffix) were each
  caught; sources restored byte-identical.
- Read-only review of `3ed49015a`: no findings; its three test gaps (v2 recovery
  over the CLI gateway path, direct/deserialized v3 bundle admission, the
  inactive-field mapping above) are closed in `4f462822f`.
- Full package gate on the merge: 10/10 stages accepted (format, web gate,
  clean desktop, clippy, Rust tests 7,113 passed / 236 ignored summed over
  `test result` lines, bindings, corpus 155/155, website, story), frozen
  inputs, no user data touched. Publication readback: local, tracking and live
  `main` equal.
- A first full gate on the feature-only merge `24bdcf19d` was red in one
  browser measurement (`devices-navigation.e2e.ts:425`, 120 s timeout) while the
  machine ran concurrent own cargo jobs; that merge changed no web file and the
  spec passed 3/3 in isolation. That merge was superseded, not published.

## Boundaries

Simulator and offline evidence only. No bus contact, ETS equivalence, hardware
approval or `DefaultValue` image algorithm follows. The pre-existing, ignored
`pinned_public_http_worker_cancellation_reopens_original_once_only_recovery`
also times out on the unmodified base and is unrelated
([earlier record](2026-10-26-system2-safe-diagnostic-projection.md)).
