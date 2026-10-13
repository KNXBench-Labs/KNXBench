# Write-echo error branch

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-102)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->
## 102. The write echo's decode-failure branch has no known real trigger

**Limitation.** `POST /api/bus/write`'s `decodedEcho` (task 27) carries a
`kind: "error"` branch for when the bytes just sent fail to decode against
the DPT they were just encoded with. No shipped DPT has been found that can
actually reach it through the public API — the branch exists in the type and
in `decode_single`'s dispatch, but no test drives it end to end, because no
input was found that makes it fire honestly.

**Cause.** `crates/knx-core/src/dpt/codec.rs`'s codec is symmetric by
construction. Roughly fifteen main-type `encode`/`decode` pairs were audited
for task 27 looking for one where `encode` accepts a value `decode` then
rejects (main types 1, 5, 6, 9, 10, 11, 15, 16, 19, 20, 21/22/27/30, 24, 28).
Every validation that could produce such an asymmetry turned out to be either
skipped identically on both sides, or enforced by one function both
directions call (`char_set_is_ascii` for main 16, `mode3_code_is_assigned`
for main 6.020). Nothing in the audit suggests this was an accident to fix;
it reads as a design property worth keeping, not a gap.

**Consequence.** The error branch is exercised only indirectly: at the unit
level, by handing `decode_single` hand-crafted bytes no `encode` call would
ever produce (as the pre-existing telegram-decode tests already do for the
bus monitor), never by a write that round-trips through `knx_core::encode`
then `knx_core::decode` inside the same request. If a future DPT — or a
future edit to an existing one — introduces a real asymmetry, the write
handler will surface it correctly (response stays `200`, the mismatch travels
as text) without any code change, but no regression test will notice the day
that asymmetry appears, because none exists to break.

**Lifted when.** An encode-succeeds/decode-fails case is found or introduced
for some DPT, giving a server test something real to drive the branch with.
Until then, adding one anyway would assert nothing the codec's own contract
does not already guarantee some other way.

**AR14 (2026-10-04) — one correction.** "No regression test will notice"
is too strong: 41 per-type round-trip tests in `crates/knx-core/src/dpt/codec.rs`
(`*_round_trips_*`) would fail if an asymmetry appeared at their sampled
values (minimum, maximum, zero, interior). Values between those samples are
not covered. The branch itself is still not reachable through the public API.
<!-- END retained catalogue record -->
