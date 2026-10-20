# Project ordinary-file compressed-stream integrity

## Decision (20 October 2026)

New project-container reads admit exactly one complete DEFLATE stream in each
validated ordinary-file compressed extent. Refuse whole zero/nonzero trailers
and concatenated streams. Preserve alternative valid blocks, unused alignment
bits in consumed bytes, signed/unsigned descriptors and existing ZIP64 handling.
This is a stricter KNXBench integrity policy, not a claim of universal ZIP or ETS
compatibility. No private real-project extent census has been performed.

[Source boundaries and original behavior](../research/2026-10-20-project-file-stream-boundaries.md)
are established before implementation. Thirty independently encoded synthetic
characterizations pass with the original production; fifteen acceptance pins
will deliberately become refusal tests. Retain their original bytes/results.
Source-file nonmutation, returned decoded members and selective raw-archive
retention are different preservation claims.

## Enforced read boundaries

- Keep lazy outer opening and validated physical-index lookup. An index-based
  read helper used by both `Container::read` and direct nested ZIP acquisition
  must check the ordinary-file extent after existing bounded decoded reading.
  Merely changing `Container::read` misses the nested carrier.
- Plain inner files are checked before insertion into the prepared-entry map.
- For supported legacy ZipCrypto, validate the decrypted compressed plaintext
  **after** the twelve-byte header has been removed. The declared ZIP compressed
  extent includes that header; the descriptor lies outside it. Neither belongs
  in the DEFLATE consumption comparison. No cipher or header-check convention
  changes, new AES support or real credential use.
- Prepared-entry rereads may clone already validated bytes. Do not eagerly decode
  unrelated outer files during open or recursively open arbitrary opaque ZIPs.

Keep the established normal decoder, output bounds and CRC behavior. For outer
members, a fixed-buffer secondary check can verify status and consumed bytes
without retaining a second expanded copy; bound its output by the actual normal
reader result plus one byte. For nested plaintext, retaining the existing
DEFLATE decoder's consumed-input counter after bounded decode may avoid another
pass. Require a complete stream and exact input consumption; read-ahead positions
are not consumed-input counters. Preserve existing decoded-size policies: outer
upper-bound behavior and inner equal-size checks are not unified by this repair.

## Error contract

Outer and plain inner failures remain `ContainerError::Read` with the validated
leaf or outer carrier path and a technical stream reason. Existing CRC/codec
refusal controls stay pinned. Correct-password ZipCrypto extent failures remain
mapped to `WrongPassword { nested_entry }`, just like existing encrypted DEFLATE
corruption. This preserves the established ambiguous-password retry behavior;
it does not diagnose all damaged ciphertext as an actual wrong user credential.
AES, missing/wrong password, unsupported codecs and truncated encryption-header
behavior stay unchanged. No new HTTP/CLI category or error variant is needed.

## Persistence and historical evidence

Full library import must finish reading the container before application
manufacturer/opaque/manifest writes; refuse a late damaged opaque/manufacturer
member before any new persistence. Test fresh and already migrated/populated
project/product destinations, source bytes, database/sidecar snapshots and native
reopen. This proves early refusal, not cross-database atomicity for later failures.

Selective `source_from_bytes` must refuse a newly invalid source before returning
new source/archive/report evidence. Preserve historical native decoded rows,
original primary/scoped reports and retained selective archives during normal
load/save/reopen. Do not retroactively reimport, relabel or certify old reports,
and do not invent raw bytes never retained. Project imports have no product-style
archive-hash retry/source-name shortcut; test actual same-byte new import instead.

Manufacturer restoration remains selected-member, per-member continuation: a new
read refusal is reported for that member, not a reason to roll back earlier
successful restoration or inspect already-present members. Test that consumer
separately rather than asserting whole-archive atomicity it never promised.

## Required evidence before delivery

- Named original-source RED for each independently changed admission boundary:
  outer, compressed nested carrier, plain nested leaf and actual synthetic
  ZipCrypto leaf, including bare/rooted names, empty/nonempty output, trailers,
  concatenation, descriptors, multiple input/output chunks and incomplete finals.
- Valid alternate/descriptor/ZIP64 controls and the existing directory/CRC/password
  identities remain green. Fifteen old suffix-acceptance pins are explicitly
  superseded, not silently called historical refusals.
- Actual application refusal/reopen and selective/restore consumer tests support
  only their measured boundaries. No live WAL concurrency or blanket durability
  guarantee follows from closed-database byte comparisons.
- Measure the candidate's added direct-read work on the project paths with source-
  bound original/candidate inputs; product-reader timing is not project timing.
  Disclose workload limits and any added cost rather than claiming a speedup.
- Independent full-diff review and maintained committed-head scoped gate with
  selected existing corpus regressions precede delivery. Corpus test success is
  not a new exhaustive real-project compressed-extent or encryption census.

## Status

Policy recorded before production changes. Implementation, original-source REDs,
consumer tests, measurements and delivery acceptance pending.
