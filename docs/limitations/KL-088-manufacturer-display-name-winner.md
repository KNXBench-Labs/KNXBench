# Manufacturer display-name winner

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-088)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 88. A manufacturer's display name is last-writer-wins, and that is on purpose

**Limitation.** `ingest_master_data`'s `"Manufacturer"` arm
(`crates/knx-productdb/src/parse/master.rs`) writes `name` with `INSERT ...
ON CONFLICT(id) DO UPDATE SET name = excluded.name` — whichever
`knx_master.xml` is ingested last overwrites the name every earlier one
wrote. Every other id-collision path this crate has is first-writer-wins
instead, plus a recorded `IdConflict` when the losing row's file differs:
`first_winner` in `parse/mod.rs`, shared by `hardware.rs` and `catalog.rs`
since 2026-09-13 and by `program.rs` since 2026-09-17 (§86). Since PDB-11
(schema v17, §135) every such candidate element is also recorded with its
digest; manufacturer names are master data and are not. Manufacturer names
update silently and take the opposite side.

**Cause.** `hardware.rs` and `catalog.rs` can each create a manufacturer row
stub (`id`, `name = NULL`) before any `knx_master.xml` naming it has been
ingested — order between the two is not guaranteed. `first_winner` semantics
applied literally would make the first arrival "win", `NULL`-name stub
included, and the name would then never get filled in by a later,
better-informed `knx_master.xml`. The existing test
`a_manufacturer_seen_during_ingest_first_gets_its_name_later` pins the
required outcome: a `NULL` stub inserted first still ends up with a real name
after `ingest_master_data` runs, in either arrival order. That stub is a
constraint on any rule chosen here, not an argument for this one — a
first-writer-wins variant can satisfy it, and the Ruling below names the one
that does and says why it lost anyway.

**Measured against the real corpus.** Swept 69 real `knx_master.xml` files
(pattern search across the filesystem, not one remembered path) for
manufacturer ids whose declared `Name` differs between files. Of 832 distinct
manufacturer ids seen, 52 have more than one `Name` on record — real ETS
rebrandings, not typos. Quoted exactly as the corpus spells them, in no
particular order, because the corpus offers no way to order them: `M-0007`
is either `"Busch-Jaeger Elektro"` or `"ABB AG - BUSCH-JAEGER"`, `M-003D`
either `"WAGO Kontakttechnik"` or `"WAGO GmbH & Co.KG"`, `M-0085` either
`"Video-Star"` or `"GVS"`, and 49 further ids are the same shape **[V]**.
Which spelling is the newer one is not stated anywhere this ingest can read:
no file carries a timestamp or version marker inside the `Manufacturer`
element itself that would let it tell "the newer file" from "the one that
merely happened to be read second" — file mtimes and ingest order are the
only signal available, and mtimes are not part of the KNX master-data
grammar, so they are not read at all. Every id above has exactly two
spellings on record; no id in this corpus has three.

**Consequence.** Ingesting an old package after a new one silently reverts a
manufacturer's display name to its old spelling. There is no `IdConflict` and
no `ingest_unknown` row, because this was never a data-loss path in the sense
those exist for (`kept_sha256`/`other_sha256`) — no id-scoped row is ever
dropped, only overwritten, and every overwrite has the exact same
justification: some later file's opinion of the correct spelling.

**Ruling, 2026-09-13, with its reasoning corrected 2026-09-13.** Aligning
this with `first_winner` was considered and rejected, and the conclusion
stands — but not on the argument first written down here, which claimed
first-writer-wins *must* strand the `NULL` stub. It need not.
`ON CONFLICT(id) DO UPDATE SET name = COALESCE(name, excluded.name)`
satisfies both `a_manufacturer_seen_during_ingest_first_gets_its_name_later`
and first-writer-wins for real names, in one statement, and was the option
this entry should have named and did not.

What actually decides it is that `COALESCE` does not buy what it looks like
it buys. It does not remove the order-dependence, it relocates it: the
*first real* spelling wins forever instead of the last one, and which
spelling that is still depends on which file the ingest opened first — with
52 ids in this corpus carrying two spellings apiece, that is the same coin,
flipped earlier. It then removes the only repair the user has: once a real
name is in the row, no later `knx_master.xml` can correct it, so shipping a
package that renames a manufacturer would have no effect on an existing
database and no report to say so. A manufacturer's display name is not a
fact fixed at first sight the way a hardware id or a catalog item is — ABB
really did rename Busch-Jaeger's `knx_master.xml` entry, and the two
spellings sit side by side in this corpus with nothing to rank them. Given
two order-dependent rules and no recency signal, the one that lets newly
ingested master data have an opinion is the more useful, and it is one
statement rather than one statement with a hidden third state. That is the
whole of the case; it is a preference with a reason, not a proof.
`manufacturer_names_are_filled_in` and
`a_manufacturer_seen_during_ingest_first_gets_its_name_later` lock the stub
behaviour in; the characterization test
`a_later_ingested_master_file_updates_the_name_the_earlier_one_wrote` pins
the last-writer-wins case explicitly, order-dependence named in the test's
own name so nobody mistakes it for an invariant.

**Lifted when.** Never, unless `knx_master.xml` grows a field this crate can
use to actually rank two spellings by recency (a schema/edition attribute
would do it) — at which point "last ingested" could become "provably newer",
and this section would describe that instead.

**A second scan order, not covered above (2026-09-14, T13 fix round 2).**
Everything above is about first *ingest* order. The v9→v10 backfill
(`backfill_function_and_space_data`, `migration.rs`) replays this same
last-writer-wins `Manufacturer` write over every blob a database already
holds, ordered by `source_file.rowid` — the order distinct blobs were
first *written* to that table, which is not always the order they were
first *ingested*. `store_source_file` (`blob.rs`) returns `false` without
inserting a row when a blob's sha256 is already on record, but
`install_package` (`package.rs`) calls `ingest_master_data` on that blob
regardless, so a master blob installed by two different packages is
ingested twice but occupies one rowid — the backfill then replays it once,
at its *first* install's position, which can differ from its *last*
install's position (the one whose names actually won under last-writer-wins
at real install time). Measured **[V]**: installed Weinzierl 730 ETS4, then
MDT KP AMI/AMS 03, then the Weinzierl archive repacked with one XML comment
appended to `M-00C5/Catalog.xml` (package hash differs, `knx_master.xml`
byte-identical to the first install). Rolled back to `user_version = 9`,
dropped the three v10 tables, reopened through `open_and_migrate`: **21 of
799** manufacturer display names changed relative to the pre-rollback
database — `M-0002` from `ABB` to `ABB AG - STOTZ-KONTAKT`, `M-0007` from
`Busch-Jaeger Elektro` to `ABB AG - BUSCH-JAEGER`, `M-000A` from
`INSTA ELEKTRO` to `Insta GmbH`; `translation`, `datapoint_type` and
`ingest_unknown` counts were unchanged. `ORDER BY rowid` is still the right
order to hold — it is the only order `source_file` actually records — but
it reproduces first-install's own answer only when no master blob in the
database was ever installed by more than one package; the third install
above was constructed specifically to violate that, to make the residual
measurable rather than asserted.

**Alpha decision (user, 2026-10-04).** Accepted as an Alpha boundary.
<!-- END retained catalogue record -->
