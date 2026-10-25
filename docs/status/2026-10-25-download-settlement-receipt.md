# Download preparation and settlement receipts — 25 October 2026

## Accepted source and final integration

The receipt implementation (`e6835f100`) and exclusive browser-lane correction
(`3431aabbc`) passed the maintained integrated package gate on `3431aabbc`.
The final main-first merge preserves later upstream community documentation and
carries these results only through an explicit unchanged-runtime proof and the
maintained documentation-delta gate. This is source delivery, not a release,
deployment, packaged desktop or physical-device acceptance.

## Behaviour and boundaries

The [receipt contract](../contracts/download-settlement-receipt.md) adds two
optional, versioned display objects to existing regular-download history rows:
preparation under the same catalogue lease that built the plan, then settlement
before the read-only post-download check. Planned identity and counts are bounded;
unavailable preparation retains the requested scope. The history page remains
format 2, with strict Rust and web readers rejecting malformed or unknown objects
without rewriting stored bytes. Older builds can refuse rows with these additions.

A persisted or session-only settlement shows applied parts and its settlement
instant. A persistence failure shows computed parts **not applied**, without an
applied timestamp. The UI distinguishes these from the separate post-download
check, including an interrupted check after an applied settlement. A missing or
unavailable receipt does not establish that project state was unchanged.

The receipt changes no programming flags, request admission, transport or recovery
policy. It is not a device prestate, physical-exemplar identity, delta/restore
baseline, source-file-equality proof or observation of a running application.
CLI and System-2 paths do not record these objects. Project and history persistence
are separate, not atomic: a crash can leave applied project state without a receipt.

## Review and regression evidence

- Original HTTP worker/history test failed semantically on the old implementation;
  the parent replay retained that failure separately and passed four final HTTP
  cases. No new-API compilation failure was counted as the feature RED.
- Independent Opus source review and bounded amendment/capture reviews approved.
  Applied/computed captions, reopened-project wording and size claims were refined.
- The size regression enumerated valid rows and selected four-machine post-check
  variants; its largest enumerated row was 1,950 bytes against the 4,096-byte ceiling.
  This is not a universal maximum established by exhaustive enumeration.
- Original CSS plus the independent word-range oracle failed two narrow EN/DE cases.
  Corrected tables preserve normal words and scroll locally rather than overflowing
  the page. All four responsive cases and eight existing history cases passed.
- Actual pixel inspection accepted six mobile and six corrected desktop fixture
  captures. The desktop blank/clipped regions came from capturing entries taller
  than a hidden-overflow fixture ancestor. Only optional capture geometry changed;
  no application CSS workaround was introduced. These are intercepted-API component
  fixture images, not screenshots proving the packaged backend.

## Integrated gate on `3431aabbc`

All fifteen selected stages passed, with equal start/end input commitments and no
user-data paths touched or left unchecked:

- Rust: 2,626 passed, zero failed, 122 ignored across fourteen result blocks.
- Selected private corpus: 91/91 in five targets, including 49/49 server integration
  cases. The exact corpus manifest contains all four receipt/cleanup HTTP cases;
  the successful corpus log intentionally prints aggregate counts, not each name.
- Web gate: all 32 stages passed; Vitest 4,390 tests in 301 files; Chromium 890 cases
  (823 functional, 67 exclusive), zero failed, skipped or flaky cases.
- Formatting, Clippy, documentation/local-information, browser-lane classification,
  layering, headers, anchors, ledger, corpus registration, tooling and diff checks
  passed. A fresh frontend bundle preceded workspace Rust steps.

The first integrated attempt stopped at browser-lane admission: the frame-based
capture helper needed the existing `@exclusive` tag. Only that test metadata was
corrected. The second package gate succeeded; its outer controller then failed
while searching for a corpus test name in an aggregate-only log. Finalization
validated exact manifest membership plus complete per-target counts, preserving the
original failed controller output and successful gate rather than inventing a rerun.

No real bus or device was contacted. The selected regression corpus establishes
neither general product compatibility nor hardware success. Parameter policy,
complete effects and whole-device recovery remain open under
[PRG-01](../open-work/PRG-01-programming-gaps.md) and
[KL-007](../limitations/KL-007-bounded-commissioning-support.md).
