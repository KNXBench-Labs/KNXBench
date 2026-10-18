# Alpha 14 release (Claude, 2026-10-18)

Owner instruction: wait for P1 and the product-database implementation, then
prepare and publish Alpha 14.

- First candidate `b08ed794` (cutoff `bfc0a9c5`) is retained, not tagged: its
  Docker dry run 39102313839 failed on amd64 and arm64 because the web build's
  typecheck reaches `tools/browser_tmp.cjs` (moved there on 17 October) and the
  image's frontend stage copied only `apps/knx-web`; its AppImage dry run
  39102312358 passed. Fixed on `main` (`a4ba4193`) with a tools test; a local
  `docker build --target frontend` produced `dist/index.html`.
- Gate attempts on `b08ed794` were stopped: resolving TMPDIR (and a subfolder
  under the scratch root) left the deep-path tool tests no fitting anchor; the
  runner now uses the approved scratch root itself and has its own web build.
- Candidate 2 `f59ace9b` (cutoff `a4ba4193`) is retained, not tagged: AppImage
  39103033519 and Docker 39103034800 dry runs passed (publish skipped); its gate
  passed the full web gate, Rust 4,882/0/212, diagnostics 5, native evidence 12
  and naming recipe 4, but native rename and Functions were red: those specs run
  outside the web gate and were not adapted to closed-by-default branches (an
  explorer row used before its branch was opened; navigation sections folded again
  after a narrow remount) and their open helper raced a late reopen. Fixed on
  `main` (`8fd01af4`): rename 4/4, Functions 4/4.
- Candidate 3 `54010888` (cutoff `8fd01af4`) is retained, not tagged: AppImage
  39106380700 and Docker 39106382260 dry runs passed (publish skipped; dry-run
  `knx-mcp --version` = `0.1.0-alpha.14+g5401088`). Its web gate had one timeout
  in ux-feedback (de 1440) while the host was heavily loaded: the second of two
  refused imports started while the first was still running and was ignored.
  Not reproduced in 80 plain repeats; with refusals answered after 1.5 s it fails
  4/4, and waiting for the first refusal card fixes it (80/80, config 55/55),
  `6ee142b2`.
- Product matrix: the pinned scopes now hold 117 package instances instead of
  115 because one vendor ZIP with two inner packages was added to the collection
  after Alpha 13 (file birth time 17 October); Alpha 13's own unchanged test also
  counts 117. The gate measures the pinned composition from an owned copy of
  both scopes without that file (originals read-only, digests unchanged); the
  pins stay as they are.
- Candidate 3's Rust run also failed once in knx-net
  (`the_loopback_join_lands_on_lo_and_nowhere_else`): a membership change between
  the `/proc/net/igmp` read and its EOF probe looked like truncation and the
  after-read panicked. Reproduced under multicast churn at the page boundary (2
  panics in 160 runs); fixed with a bounded retry for a consistent snapshot and the
  documented skip path (`482f0fcd`): 0 of 200 failed while 139 moving-table reads
  were absorbed. Candidate 4 `bb7c896e` (cutoff `6ee142b2`) was superseded before
  its gate; its dry runs 39110004992/39110006523 are not used.
- Candidate 3's product stage also showed the opt-in corpus matrix still pinned
  to v23 (ADR-0139 had not run it). An Alpha 13 / current pair on the pinned
  115-instance composition moves exactly six of 102 projection leaves, each
  predicted by an independent XML census; re-pinned on `main` (`a0c2537d`),
  matrix target 11/11. Candidate 5 `ec52706e` (cutoff `482f0fcd`) is retained,
  not tagged: its gate was green through the web gate (incl. ux-feedback), seven
  extra configs, Clippy, Rust 4,882/0/212, diagnostics 5, native 12, rename,
  Functions, naming recipe, bindings and corpus 153/153, and was stopped at the
  product stage for the re-pin. Dry runs 39110286470/39110288344.
- A parallel session pushed two docs-only commits (census 3 and its claim for
  Alpha 15 work) before cutoff `a0c2537d`; they ship with Alpha 14 as docs. Their
  claim touches installer, size limits and unknown reporting, none of which this
  release changes.
- Cutoff `a0c2537d`: P1 delivered by the P1 session (`55bcb203`/`4b961475`),
  product-database series by this session (vendor ZIPs, packed TypeTime,
  IndentLevel, object size).
- Schema: `CURRENT_SCHEMA_VERSION` 16 unchanged (diff of `knx-store` migration
  is empty); `knx-core` changes are note-delete command/error types only. Product
  DB 23 → 24 (ADR-0139). Alpha 13 refuses newer product DBs (`FutureVersion`,
  read in its source, not executed).
- Gate runner: the release template now delegates docs/tools/build/tsconfigs/
  Vitest/fixture Chromium/15 production configs/xtask to `tools/run_web_gate.py
  --profile web` (it takes the gate leases itself, so the runner holds none while
  calling it) and adds the configs outside that list (issue09, monitor-control,
  address-unavailable, device-checks, service-control, site, support, and the new
  device-diagnostics against the loopback simulator), Rust, native verifiers,
  bindings, corpus and the six explicit product-corpus targets.
- Explicit product-corpus scopes come from an owned private file; the matrix
  output lives on a different filesystem than the corpus.
- Not gated as release stages: media captures (manual, readme, readme-hero,
  release-media, diagnostic-guide), load benchmark, study.
