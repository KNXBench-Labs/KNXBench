# Alpha 14 release acceptance — 18 October 2026

Frozen candidate: `8d104e2f91cf7f29fa9bdbf146a9e75e80ba72d1` on
`release/alpha14-20261018-r6`, now immutably tagged `v0.1.0-alpha.14` and
fast-forwarded into `main`. The owner asked to wait for P1 and the
product-database implementation, then prepare and publish. This result-only
record does not change the accepted product or move its tag.

## Evidence and review boundary

The final gate ran **36 of 36 ordered stages green** in one uninterrupted run
(97 minutes) with its inputs frozen (HEAD, index manifest and clean tree equal
before and after). It is a source acceptance record, not by itself public
artifact or physical-device evidence. Review is in-session self-review, not
independent-agent approval.

Five earlier candidates are retained, not tagged. Each stopped on a real finding,
fixed on `main` before the next cutoff; none of their green stages is carried
into this acceptance:

| Candidate | Finding | Fix |
|---|---|---|
| `b08ed794` | Docker dry run failed on both architectures: the image's frontend stage did not copy `tools/browser_tmp.cjs`, which the web build's typecheck reaches. | `a4ba4193`, guard test |
| `f59ace9b` | Native rename/Functions specs not adapted to closed-by-default branches; reopen race. | `8fd01af4` |
| `54010888` | ux-feedback import race and knx-net IGMP snapshot flake (both under heavy load); opt-in product matrix still pinned to v23. | `6ee142b2`, `482f0fcd`, `a0c2537d` |
| `bb7c896e` | Superseded before its gate. | — |
| `ec52706e` | Green through web gate, Rust, native and corpus; superseded by the matrix re-pin. | — |

Two gate-runner (harness) attempts on the first candidate were stopped for
temporary-directory placement, not product findings.

## Measured results

| Check | Result |
|---|---|
| Versions, diff-check, fmt, no stray specs | Passed |
| Website and Story tests, release-mode website build | Passed |
| Repository web gate (`tools/run_web_gate.py --profile web`) | 63 stages, all exit 0, inputs frozen |
| Component tests (Vitest) | 3,742 passed; 0 failed |
| Default fixture Chromium | 596 passed; 0 failed; 0 flaky |
| Production browser configurations | 341 passed in 15 configs |
| Further browser configurations | 41 passed in 8 configs, including the diagnostic export against the loopback simulator (5) |
| Clippy (`--workspace --all-targets -D warnings`) | Passed |
| Rust workspace | 4,882 passed; 0 failed; 212 ordinarily ignored |
| Native evidence | 12 passed |
| Native rename / Functions / naming recipe | 4 / 4 / 4 passed; demo original unchanged |
| Generated bindings | Unchanged; original bytes restored |
| Tooling | 223 run; 0 failures |
| Conventional private corpus | 153/153 in 33 targets |
| Explicit product-corpus scope | 16/16 in 6 targets (dynamic tree 6, matrix 1, legacy names 1, parameter views 1, program code 4, standalone 3) |

The product matrix measures the pinned 115-instance composition from an owned
copy of its two private scopes without the one vendor ZIP added after Alpha 13;
originals were read-only and their digests unchanged
([re-pin](../IMPORT_EXPORT.md)).

## Packaging dry runs

AppImage `39118142434` and Docker `39118144327` on the exact candidate: builds
successful (Docker on `linux/amd64` and `linux/arm64`), publish jobs skipped, no
tag present at the time.

## Publication

Published on 18 October 2026 from the gated commit; tag workflows AppImage
`39124698698` and Docker `39124698730` (event `push`, head `8d104e2f`) succeeded
including their publish jobs. External verification of the public artifacts:
[publication record](../evidence/release-alpha14-2026-10-18.json) — SHA256SUMS,
binary and OCI revision binding, registry `latest` equality, an isolated
authenticated Docker smoke, a native-13 → 16 migration probe and a headless
AppImage startup. Its scope labels say what each check does not prove.
