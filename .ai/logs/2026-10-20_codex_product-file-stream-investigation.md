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

## Pending

Independent read-only ordinary-file census and synthetic original-production
characterization are running. The first characterization runner stopped before
Cargo at its old-test-byte guard. Controller comparison isolated one extra blank
line in each reconstructed old test file; exact range accounting is being fixed,
not relaxed. The failed setup is retained and establishes no runtime result.
No production policy change or new admission
acceptance is claimed. Future implementation requires an explicit policy,
named counterexamples, compatibility analysis and measured candidate costs.
