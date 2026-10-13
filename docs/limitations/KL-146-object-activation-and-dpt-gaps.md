# Object activation and DPT gaps

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-146)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="146-a-channel-without-text-has-no-name-of-its-own-and-some-activations-are-undetermined"></a>
## §146 Channel labels are shown; undetermined activation and missing DPT remain

**Owner-approved read-only differential 2026-10-11 (no lift).**
`private_saved_activation_compares_every_imported_tree_object_without_rounding_unknown_states`
compares source/import/HTTP/shared evaluation independently. Complete historical
comparison preserved input/project and found evaluated-object differences and
extra unscoped refs; unsupported scoped sets remain unadjudicated. Diagnostic
admission requires coverage, actual evaluation and unchanged input/project, not
agreement. All findings remain in private boolean evidence, with explicit
`parity_accepted=false`; no runtime changed and no ETS answer adopted. Owner chose
read-only delivery. Final source `f045390fdf7b` passed complete private diagnostic
replay, all non-desktop Rust3934/0, Web2612/0, build/repo/docs gates, full corpus
and independent RefId/instance/reconciliation/raw-evidence gates. Missing opt-in
and missing-file controls refuse. Eight synthetic controls and three compiled
mutants pin coverage, finding retention, unknowns and failed integrity.
Published no-ff merge `70c05fdc58e5`, HEAD/tracking/live equality and exact
receipt readback verified. Source/metadata-equivalence and outgoing privacy clear.
This delivered diagnostic does not lift the limitation.
[Contract](../research/schema23-activation-differential.md).

**Update 2026-10-10 (comparison source; no change).** ETS writes its own
activation result into the project: `ChannelInstance/@IsActive`,
`ComObjectInstanceRef/@ChannelId` and the object set listed in
`GroupObjectTree` (`Node` types `Channel`/`Folder`). The privately supplied
schema-23 project carries all three. Once that project imports completely they
can serve as a read-only differential reference for KNXBench's evaluated
activation. A difference is a finding to explain, not a reason to adopt ETS's
answer without a documented rule. Every channel instance in that sample is
active, so it says little about channel activation itself.

**Bounded text-refusal policy accepted (2026-10-03 13:32 CEST, ADR-0066).**
Scoped FunctionText substitution and channel-text rendering/cached copies share
one4,000,000-unit content/work context. Overflow refuses the whole DeviceDetail
through existing HTTP400, no clipped output or changed activation/write authority.
Actual0369a56a/22 commands passes: Rust2995/0/165, Web1702/Chromium72,
selected private9,699 exact inputs/17 equal bindings/420 unchanged originals.
No new generated/UI contract; dedicated language/refresh error/snapshot behavior
is UI-owner verification. Legacy String SDK helper, unscoped/raw metadata/query/
serializer allocations and total RSS remain outside this bound. Full AR07/ETS
and the newly found AR06P nested-definition sample are not accepted by this gate.
Final five document checks passed; scoped delivery 0b8ec935
was published/fetched/live-read back,699 inputs/7 docs exact at that checkpoint.
Whole AR07/Alpha and dedicated UI consumer acceptance remain open. See
[decision](../adr/0066-outside-walk-text-refusal.md).

**AR07 candidate (2026-10-02, ADR-0061).** A resolved controller kind outside
Number/Restriction (with the existing None exception retained) now emits
UnsupportedControlKind, skips/names branch refs and marks activation potentially
hidden. Public ingest/load, nested-scope, source-reopen, HTTP hidden-field refusal
and Undetermined projection checks pass; six behavioral mutants are detected.
This is a 1,118/0/57 public two-crate candidate, not new whole-corpus counts or
ETS parity. The warning token/English fallback is on the backend wire; Web's
manual union and English/German catalogue adopted it on 2026-10-05 (UI owner,
with ADR-0080's tokens).
The corrected broad candidate is independently verified at 13/13: workspace
2,924/0/164, Web 1,357 and six selected private Dynamic tests 6/0/0; all 103
original archive hashes unchanged, no genuine skips or private raw logs. This
selection is not blanket validation of every archive/opaque construct. Upstream
UI ancestry integrated at 03f18c95 and independently accepted: 17/17, workspace
2,924/0/164, Web 1,559, Chromium fixtures 61, selected private 6/0/0 and
seventeen shadow bindings equal. Published/read back at 2d9aaeb8;
full AR07, typed/localized token adoption and earlier corpus figures retain
their distinct dated scope. See [parameter boundary](../contracts/PARAMETER_SEMANTICS_BOUNDARY.md).

**Status (2026-09-29, ISSUE-08 P2, ADR-0050).**

- **Channel names.** `ComObjectChannel::text` is the element's `@Text`, and
  `None` when that is empty. In the corpus, 24 of 29 `Channel` elements
  have an empty `@Text`, including every one in the two house exports.
  **Data half lifted (2026-09-30, schema v18, ADR-0052):** they do state
  `@Name` and `@Number`, and `dynamic_node` now stores both verbatim.
  `ComObjectChannel` carries them as `name` and `number` (`None` when
  absent or empty), never composed into `text` and never translated, since
  no product translates `@Name`. `@Number` is text: 5 of 1,268 corpus
  channels hold a value that is not a number. What the standard means by
  either attribute is not documented in the schema text available here;
  KNXBench shows them as written. The UI now uses `@Name` as the heading
  when `@Text` is absent and shows `@Number` as text, without composing or
  translating either value.
- **`Undetermined`.** The evaluation cannot decide in these cases:
  - two module instances with one `RefId` (D40);
  - a scoped activation that no imported instance owns;
  - a value or definition the evaluation could not use.
  Objects in these cases stay `Undetermined`. None occur in the corpus
  (0 of 1,849).
- **Stale module rows.** A stored module row whose `_MI-` digits disagree
  with the imported instance is set aside as stale. Its parameter then runs
  on the program default. `Active` is not downgraded for it, the same as in
  the parameter panel, which evaluates identically. There are 0 stale rows
  in the corpus.
- **Typed UI boundary (2026-09-30, U12/ISSUE-08 UI half, published on main).**
  The previously skipped activation, owner channel, program DPT, translated
  DPT text and function text now have generated TypeScript bindings. The
  device editor groups by the supplied opaque key, shows all four evaluated
  states plus the stored claim, and leaves unmatched objects inspectable.
  **§146 channel-label follow-up (2026-09-30):** the UI shows `name` and
  `number` without changing the translated `text`: it uses `name` as the
  heading only if `text` is absent, shows a separate name fact when text is
  present, and always shows a present number as an uninterpreted string.
  The channel-label gap is lifted. Missing and multi-choice DPTs still have
  no chosen identifier; genuinely undetermined activations remain visible.
- **No DPT to show (P3).** 495 ETS4 and 456 ETS 6.3.0 objects have neither
  a stated DPT nor a program default. For 473 and 434 of them the product
  states none. For 22 each it states a list of several (`DatapointType`
  with more than one id) and neither the project nor the product picks
  one. KNXBench shows no DPT for them rather than choosing from the list.

**Channel-label condition lifted (2026-09-30).** The UI displays both
`name` and `number` when present (`DeviceWorkspace.test.tsx` and local
EN/DE browser fixture); the data half is ADR-0052. The remaining
`Undetermined`/missing-DPT conditions above stay explicit, not guessed.
<!-- END retained catalogue record -->
