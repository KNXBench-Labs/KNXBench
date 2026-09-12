# T34 — one overlay per device fetch, and three riders

Closes **T34** in [GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md)'s Tier 6:
the four non-blocking findings T33's whole-branch review left open on
2026-09-12. None was Critical, all four are real.

## Context

`knx_productdb::query::com_object_view(conn, program_id, ref_id, language)`
resolves one `ComObjectRef` against its `ComObject` and folds every
attribute through `pick()`. When `language` is `Some`, it loads the
program's *entire* translation overlay — every `translation` row for that
program and language — and then uses at most six entries from it.

`apps/knx-server/src/domain.rs:~845` calls it in a loop, once per
communication object of the device being fetched. So a device fetch costs
O(com objects × overlay rows). The T33 review measured the largest program
actually attached to a device in the reference project at 3,876 overlay
rows / ~1.26 ms of SQL per load, against up to 66 com objects per device —
paid again on every device click, every edit-triggered refetch, and every
language change. `parameter_views` already has the right shape for this
(one query for the whole program, one overlay for the whole call); the com
object path never got it.

**Correction, added 2026-09-12 after Task 2 measured it.** The "3,876
overlay rows / ~1.26 ms" above is T33's figure and it does not reproduce
against any denominator this branch could construct — not the language-
filtered overlay, not the program's rows across all languages, not the
whole database. The measured cost is 1,249 `de-DE` `translation` rows per
load on `M-0083_A-0317-31-7DC6`, loaded 104 times per device fetch before
this change and once after. The paragraph above is left standing as the
argument the plan was written from; the number in it is withdrawn. See
[GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md)'s T34 entry.

The other three findings are independent of the performance one and ride
along because they live in the same T33 surface.

## Global Constraints

1. **No behaviour change where a language is not requested.**
   `language: None` must still issue **zero** `translation` queries and
   produce byte-identical results. This is T26's and T33's own standing
   constraint and it survives verbatim.
2. **Only a `Program`/`ProgramRef`-layer value may be overlaid.** The
   layer consulted is the one stored on the project's own
   `ComObjectInstance`, never the layer `ComObjectView` reports —
   `Layer::Instance`, `Layer::Inferred` and `Layer::UserEdit` are
   project-authored and are always shown verbatim. This is T33's
   load-bearing invariant, it has a regression test, and nothing in this
   plan may weaken it.
3. **Translation stays display-only.** Device creation and `enrich()` keep
   baking untranslated text into the project file. No task here may store
   a translated string.
4. **Evidence markers.** `[D]` documented in the KNX Standard with exact
   file and section, `[V]` locally verified, `[A]` assumption. No
   promotion between them.
5. Corpus tests must skip, not fail, when `OriginalData/` is absent — it
   is gitignored and exists only in the main checkout. Override the
   product corpus with `KNXBENCH_PRODUCT_CORPUS`.
6. Every gate green before a task is called done: `cargo fmt --all
   --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `cargo run -p xtask -- check-layering`,
   `cargo run -p xtask -- check-headers`, `cargo deny check`, and from
   `apps/knx-web`: `npm test -- --run` and `./node_modules/.bin/tsc
   --noEmit`. Baseline on `main`: **1080 passed / 0 failed / 3 ignored**
   summed across all 77 `test result:` lines; npm **339 passed across 31
   files**; headers **74 well-formed / 169 without a header at ceiling
   169 / 22 generated skipped**.

## Task 1 — `com_object_views`, and a translated-or-not answer

`crates/knx-productdb/src/query.rs` only. Two changes, one function.

**(a) The batch query.** Add

```rust
pub fn com_object_views(
    conn: &Connection,
    program_id: &str,
    com_object_ref_ids: &[&str],
    language: Option<&str>,
) -> Result<HashMap<String, ComObjectView>, ProductDbError>
```

keyed by the `com_object_ref.id` each view was resolved from. One
`SELECT … WHERE cor.program_id = ?1 AND cor.id IN (…)` for the whole
slice, and the overlay loaded exactly once for the whole call — mirroring
`parameter_views`'s shape, including its comment style about *why* the
per-row shape is wrong.

Ref ids absent from the database are simply absent from the map; the
caller decides what that means. An empty slice returns an empty map
without touching the database at all — not one query with an empty `IN
()`, which SQLite accepts but which would still load the overlay.

**Chunk the `IN` list.** SQLite's default `SQLITE_MAX_VARIABLE_NUMBER` is
999 and a program can declare more refs than that (the corpus has a
program with 543 `ParameterRef`s, so the order of magnitude is real).
Chunk at 900 ids per statement, prepare once, and load the overlay once
across all chunks. A test must cover a slice larger than one chunk —
build it synthetically, do not require the corpus for it.

`com_object_view` stays, with its signature and its public behaviour
unchanged, reimplemented as a one-element call into
`com_object_views`. Its doc comment gains one sentence pointing callers
that resolve more than one ref at the batch function. Every existing
`com_object_view` test must still pass untouched — if one needs editing,
stop and report, because that means the wrapper changed behaviour.

**(b) Translated-or-not.** `ComObjectView` gains three `bool` fields:

```rust
pub text_translated: bool,
pub function_text_translated: bool,
pub visible_description_translated: bool,
```

each `true` **iff** the value `pick()` finally selected for that field came
out of the translation overlay rather than out of the product database's
own column. With `language: None` all three are always `false`.

This exists because of finding M6 below: the server currently cannot tell
"the overlay gave me a translation" from "the overlay had nothing and I am
handing you today's untranslated column", and it overwrites
project-resolved text in the second case. Three fields rather than one for
the two the server happens to read today: a per-overlaid-field flag is
symmetric with the three fields that are overlaid at all, and the
asymmetric version would only raise the question of why `function_text`
was left out.

Tests: overlay hit sets the flag; overlay miss leaves it `false` while the
untranslated value still comes through; `language: None` leaves all three
`false`; a `ComObjectRef`-scope translation winning over a
`ComObject`-scope one reports `true` and still reports layer
`ProgramRef`.

## Task 2 — the server stops paying per com object, and stops overwriting on a miss

`apps/knx-server/src/domain.rs` only (plus its tests).

**(a)** The `device_detail` overlay loop calls `com_object_views` **once**
with every lookup id it is about to resolve, then reads the map. Collect
the lookup ids with `com_object_lookup_id(&program_id, ref_id,
module_based)` exactly as today — module-based ids are synthesised and the
map must be keyed by the same id that was queried.

**(b) Finding M6.** `com.name` is overwritten only when the stored layer
is `Program`/`ProgramRef` (Global Constraint 2, unchanged) **and**
`view.text_translated` is `true`. Same for `com.description` against
`view.visible_description_translated`. On an overlay miss the value the
project already resolved stays exactly as it was.

Regression test, and it must fail against the current code: a program
with a com object that has **no** translation row for the requested
language, whose project-side text differs from the product database's
current column. Today the response shows the product database's column;
after the fix it shows the project's own resolved value.

**(c)** The `create_device` call site keeps `com_object_view` with
`language: None` — it resolves refs one at a time for a reason
(`ok_or_else` per ref, with the ref id in the error) and it pays no
overlay cost. Leave it alone. If you convert it anyway, the review will
ask why, and "it was nearby" is not an answer.

**(d) Measurement.** Report, against
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`'s largest
device-attached program, the number of `translation` rows the overlay
loads and how many times it is loaded per device fetch, before and after.
`[V]`, with the command that produced each number. The T33 review's
figures (3,876 rows, ~1.26 ms, up to 66 com objects) are the "before" to
reproduce or correct — if they do not reproduce, say so and give yours.

## Task 3 — one request-id guard for all three device-detail fetches

`apps/knx-web/src/App.tsx` and its tests only.

`App.tsx` has three `api.deviceDetail` call sites: `selectEntity` (device
pick), `handleTreeUpdate` (post-command refetch) and the
`[productLanguage]` effect (language change). Only the third carries
`languageRequestIdRef`, so the guard protects language-vs-language races
and nothing else: a language reply can still land after — and overwrite —
a later edit-triggered refetch, because the selection-identity check the
other two rely on is true in that case.

Replace `languageRequestIdRef` with **one** counter shared by all three
sites: every site increments it before issuing its request and applies
its reply only if its own id is still the newest. Keep the existing
selection-identity checks as well — they answer a different question
("did the user move to another device"), and the id answers "is this
still the newest request for it".

Tests, on the existing `App.test.tsx` fixtures: an edit-triggered refetch
issued after a language change applies, and the older language reply that
resolves afterwards does not; and the existing out-of-order
language-vs-language test keeps passing unchanged.

Do not touch `api.ts`, the Inspector, or the product-language setting
plumbing.

## Task 4 — finding M5 ruled, and the documents reconciled

**(a) M5, ruled here so no one has to guess.** `GET /api/device/{id}`
gained a `Query<ParameterLanguageQuery>` extractor in T33, which turns a
malformed query string into a `400` where it was previously ignored
outright. **Ruling: keep the 400.** The same extractor already guards
`GET /api/parameters/{id}` and `POST /api/parameters/{id}/value` since
T26, so leniency here would be the inconsistency, and silently ignoring a
query string the client meant something by is how a language setting goes
missing without a single log line. Cost if wrong: a client sending
`?language[]=de` gets a 400 instead of untranslated text — visible
immediately, and fixed by sending a scalar.

Pin it with one test in `apps/knx-server/tests/`: a malformed `language`
query on `GET /api/device/{id}` returns 400, and an absent one returns
the untranslated detail with 200.

**(b) Documents.** `docs/GAP_ANALYSIS_ETS.md`'s **T34** entry moves from
`Open` to `Done (2026-09-12)` and records what each of the four findings
turned into, with Task 2's measured before/after numbers as `[V]`.
`docs/IMPLEMENTATION_STATUS.md` and `docs/KNOWN_LIMITATIONS.md` §37 get
whatever the change actually invalidates — read them first, and change
nothing that stayed true. The 400 ruling belongs in the documented API
behaviour wherever the other two routes' query handling is already
described; if it is described nowhere, that is worth one sentence in the
gap-analysis entry rather than a new document.

No source file outside `docs/` in this task.

## Out of scope

* Widening translation to hardware- or master-scope rows
  ([KNOWN_LIMITATIONS.md §64](../../KNOWN_LIMITATIONS.md)) — still D10's
  residue, still not this task.
* Translating `object_size`, `priority`, `dpt_list`, `number` or the four
  flags. They are values, not display text.
* Storing any translated string anywhere (Global Constraint 3).
* Caching an overlay across calls. One load per call is the fix; a cache
  is a lifetime and invalidation problem nobody asked for.
