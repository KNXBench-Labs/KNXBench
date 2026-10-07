# AR18 independent re-check, round 4

- Agent: codex in Hermes, independent reviewer, not release owner.
- Timestamp: 2026-10-07 06:49 CEST.
- Base: `b8724d66acad05c30328ef54fd61cdd799e87cea`, freshly fetched
  `origin/main`; product-source equivalence to `2254eed0` verified.
- Requested delivery: review report on `review/alpha-recheck-4`, commit and
  push that branch; no merge or push to main.

## Result

**READY.** N11, N12, N13 verified; R6/R7 met. New nonblocking MINOR N14:
empty directory CRC/stream validation is incomplete; no useful content loss
or substituted project member reproduced. Release/tag decision remains
with the user.

Report: `docs/review/2026-10-07-alpha-recheck-round4.md`.

## Executed evidence

- Own main fixture manifest: 181 unique synthetic inputs, 142 refusals,
  33 valid controls, 6 exploratory observations.
- Ran on independently built release CLI, release standalone server, and
  copied AppImage's own server. All 142 structural refusals hold on all
  three; all 33 acceptance controls import the genuine sample.
- Additional CLI matrix: 16/16 expected guard refusals, including a valid
  embedded-header overlap and high 32 bits in Zip64 descriptors.
- Actual writers: Info-ZIP 3.0 plain/encrypted nested, Python 3.14.7
  streaming/seekable with/without force_zip64, Java jar 26.0.2.1.
- Both API smokes: 13 steps; actual area rename preserved after missing or
  malformed discard-import refusal; documented 409/422/relative Open 400.
- Gate script: 24 stages, every expected exit, 06:23:53–06:46:16 CEST.
  Rust 3402/0/178, 192 result blocks; Vitest 2076/117; Chromium 142;
  repository gates ×5; deny; npm audit 0; corpus helper unit tests 4.
- Missing-corpus control: expected exit 2; selected corpus: 143/143 in
  31 targets. Corpus and oracle links removed after the run.
- Read-only real census: 3 projects (2 exit 0, 1 report-loss exit 2), 3/3
  via product ingest, 103/103 product packages. 106 input files unchanged
  (size/mtime/SHA-256). No private identities retained in the receipt.
- AppImage copied before execution; digest
  `138444b4cf7f5664ce2f64e88b82574f88dcc5b382dca7f2e49ff1eae7d6c3fc`,
  stamp 2254eed0; check-appimage passes. Native window visually inspected
  offline via the accepted Wayland recipe; v0.1.0-alpha.4, no crash dialog,
  loopback sockets only. Screenshot is not a permanent artifact.

## Scope and provenance

No product changes, subagents, bus/device contact or hardware writes.
Own isolated worktree, fresh target, short private scratch and isolated XDG
state; alpha/workspace/browser leases for the full gate. Setup dependency
fetch/audit may use network; actual tests and case runs do not.

Protected products-only ingest does not forward a password; its 87 nested
refusals are classified as that existing boundary, not inner-guard proof.
Initial generator/native smoke harness failures were corrected and the
complete final matrix rerun; no failed attempt was relabeled a pass.

The literal docs-only delta clause in the brief is qualified: the cleanup
revert restores agent tooling/config/root notes, but no product sources or
Cargo manifests differ from the code pin. The review gates the actual tip.

Compact scripts and aggregate receipts: maintainer evidence
`ar18-recheck-round4-20261007`; no private archive/content/database or
screenshot is kept. The root main checkout remains untouched. See the
report's coverage limits before treating this as broader ETS/hardware
compatibility evidence.
