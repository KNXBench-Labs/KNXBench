# Product file-stream extent investigation

## Claimed boundary

The owner requested the next open tasks. Claim `c7bf2662` reserves only
ordinary-file DEFLATE extent investigation in `knx-productdb` and its wrapper
consumers. No project-reader, encryption-header, schema, dependency, protocol,
real-device, desktop, release, deployment or third-party operation is included.

## Baseline and preceding delivery

Product-directory repair `9c79634b` and result-only tail `87fea45e` are on `main`.
The controller verified all 14 gate stages: 3,034 Rust tests, 97 selected corpus
cases and 323 tool tests (one existing optional skip), frozen sources and empty
user-data guard lists. Private evidence archive verified against 181 member
digests before retiring the package's owned worktree/branch/rebuildable scratch.
Root stale/dirty and unrelated files were preserved.

## Current evidence

Primary ZIP/DEFLATE/decoder sources are recorded in
[the research note](../../docs/research/2026-10-20-product-file-stream-integrity.md).
The lockfile and installed source identify zip 8.6.0 and flate2 1.1.10; raw member
reads are compressed-size bounded, while decoded EOF alone does not prove exact
compressed extent consumption. Standalone raw-archive retention, normalized
member bytes and reported-only wrapper extras are separate preservation claims.

## Evidence and decision

Both independent investigations completed. The controller reran the census
verifier against actual inputs/extents: 54 controls, 15,029 ordinary-file rows,
15,015 measured extent hashes and 171 unchanged inputs reconcile. All 12,772
measured DEFLATE extents are exact; fourteen encrypted entries remain unmeasured.
Original-production characterization passes 21 new plus forty preserved tests.
Attempt one failed setup before Cargo due to one inserted newline per old-source
reconstruction; attempt two corrects exact insertion boundaries, not whitespace
normalization. The failed setup remains uncredited.

The [whole-stream contract](../../docs/contracts/product-file-stream-integrity.md)
was committed before implementation, after source research and corpus evidence.
It chooses complete/exact ordinary-file admission with valid descriptors and
block forms unchanged, using the existing fixed-buffer secondary checker rather
than replacing the normal CRC decoder. Raw-archive retention versus normalized
member/report omission is explicit. Thirteen characterization acceptance pins
are deliberately superseded by named refusal tests; original results remain
retained. Original-production replay reaches25intended failures/53passes, then
candidate78GREEN and owning selections/Clippy pass. Parent reran focused tests
and all-targets Clippy, checked source hashes, reconciled actual patch content
(the only explicit-base diff difference was Git mnemonic path prefixes), and
recomputed all four synthetic benchmark medians from80samples.

The extra fixed-buffer decoder costs1.38–1.84times the original direct-reader
median in the four synthetic workloads. This is not installation latency or
peak-RSS evidence. Initial incomplete historical fixtures and zero-test benchmark
filter attempts remain retained/uncredited; final source-bound runs correct them.

## Pending

Independent full-diff review accepted with zero Critical/Important findings;
parent verified verdict and all three final source hashes. Production/tests stay
frozen under those hashes. Maintained committed-head delivery gate and publication
remain pending. No new compatibility/release claim yet.
