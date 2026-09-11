# 2026-09-11 — R5: what the KNX Standard actually says about commissioning

Branch `r5-commissioning-research`, four commits, merged to `main` as
`173bef7`. Five documents changed, 558 insertions, 14 deletions, and not one
line of Rust or TypeScript. No device, bus or network was contacted at any
point.

This was a research spike, so the deliverable is a document rather than a
capability. That makes it cheap to get wrong invisibly, which is why most of
the effort below went into verifying quotations rather than finding them.

## Why now

The user's scope ruling of 2026-09-11 kept commissioning in scope and
attached a condition to it: *"E1 Commissioning — dauerhaft raus? nein - das
muss auch funktionieren - aber warten bis knx specs db fertig ist"*. That
database now exists. Two of them, in fact, and the second arrived while this
spike was already running.

## The two knowledge bases

Both live under `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/`, share
schema 1.3, and expose an FTS5 index `factsSearch(title, content, keywords)`
with the `unicode61` tokenizer — so searches are in English, and `content`
is full of `|` characters from Markdown tables.

| Base | Facts | PDFs | Figures |
| --- | --- | --- | --- |
| `knx_spec_kb_programming.sqlite` | 2207 | 27 | **Yes** — 317 vision-model figure descriptions, plus `figuresSearch` |
| `knx_spec_kb_full179_clean.sqlite` | 16536 | 177 | **No** — text only: no image text, no captions, no footnotes |

`knx_spec_kb_full179_unfiltered_reference.sqlite` exists for comparison and
must never be queried or cited.

They are **complementary, not ranked**. The programming base is narrower but
richer — it alone recovers text that lives inside images, and 640 of its
facts come from material the plain Markdown export loses. The full base is
six times larger and covers the whole Standard.

The practical consequence is a rule, and it is the single most transferable
thing this cycle produced:

> Any claim that the specification does not document something must name
> which base was searched.

The measurement that forces it: `"functional block"` returns **206** hits in
the full base and **1** in the programming base. A gap measured in the narrow
base alone is not a gap in the Standard. This spike's Q9 had already been
drafted from the programming base when the second one arrived; it was
re-measured against both rather than published.

## What changed in the architecture of our knowledge, not the code

Nothing in `crates/` or `apps/` moved. What moved is a claim the repository
had been carrying since Session 0.

`RESEARCH.md` §8.3 listed four blockers on commissioning. Blocker 2 read:

> The `Legacy*` option matrix and partial-download rules are undocumented
> publicly.

Half of that was false, and had been for as long as it had been written down.

**Partial download is documented.** `03_05_03 Configuration Procedures` §3.5.3
specifies it: the CRC comparison, the base-pointer identity requirement
across the Unload/Load/Alloc sequence, the `PID_MCB`/`PID_MCB_TABLE`
mechanism. `03_01_02 Glossary` defines "Differential Download" as a formal
term. Five distinct facts, all quotable.

**The `Legacy*` matrix is not documented.** A bare `Legacy` query returns
exactly one hit per base, and both are unrelated — legacy-implementation
command handling in the Application Layer, and legacy-compatible RF hardware
modes. It is a product-data construct, not a Standard one.

So blocker 2 was split, and only the wrong half retired. Blockers 1, 3 and 4
are untouched, because they were never documentary problems:

1. Writing a wrong memory image to a real device bricks it. A database does
   not make hardware unbrickable.
2. *(corrected as above)*
3. Vendor `Baggage` DLLs participate in download for some devices. `Baggage`
   returns **zero** hits in both bases — the mechanism is genuinely outside
   the Standard.
4. KNX Secure devices need key material we do not have.

## §8.4, and the shape of the evidence

The new section answers nine questions. Compressed to their load-bearing
claims:

- **Individual addresses.** `NM_IndividualAddress_Read` detects conflicts by
  *counting responses*; `NM_IndividualAddress_Write` runs Read → Write →
  `A_DeviceDescriptor_Read` verify → `A_Restart` (which deactivates
  Programming Mode) → `A_Connect`. `03_05_02 Management Procedures` §2.2/§2.3.
- **The Load State Machine** is normative in `03_05_01 Resources` §4.23, and
  there is **more than one per device** — one per Interface Object. Cross-LSM
  ordering lives in `06 Profiles`, not in Resources.
- **Complete download** is a fully ordered numbered procedure per segment:
  unload dependents → Loading → allocate → write → CRC-check → Load
  Completed. `03_05_03 Configuration Procedures` §3.5.2.
- **Resources and PIDs.** The Application Program Interface Object's property
  table is reproduced from Resources Tables 89/90/91.
- **Memory services.** `A_Memory_Write`/`A_Memory_Read`, 1-63 octets, 16-bit
  addressing, confirmed versus acknowledged depending on Verify Mode (off by
  default), and a write touching any protected or nonexistent area fails
  *as a whole*, silently. `03_03_07 Application Layer` §3.5.4/3.5.5.
- **Unload mirrors load**, and Master Reset — previously an unresolved
  cross-reference out of `AN194` — is at Management Procedures §3.7.1.2, with
  the full Erase Code table (01h-08h). Finding it required the full base; the
  programming base could not close that reference.
- **KNX Secure wraps, it does not replace.** The same APDUs travel inside S-AL
  frames authenticated under the Tool Key (the FDSK for a factory-fresh
  device), with sequence-number replay protection. This is a structural claim
  only, and it does not lift T19's deferral by one inch.

## Two reviews, three defects, one family

Two reviewers verified 25 citations against the databases between them, and
every defect they found was the same kind: **prose drifting while wearing
quotation marks.**

1. Q3's CRC quote had grown a bracketed continuation — `"...MaC shall us[e it
   as the precondition check]"` — that the Standard never wrote. The real
   continuation is `"use differential download algorithm"`, which is not a
   paraphrase of the invention but a different fact, and is the very mechanism
   Q4 cites correctly two subsections later.
2. Q2's `"More than one Load State Machine is possible."` silently dropped the
   source's trailing `in one device` — meaning intact, discipline not.
3. §8.4's own opening claimed a citation convention stricter than the document
   followed: four `[D]` quotes were accurate against the extracted Markdown
   but had no `evidenceText` row in either base.

The third is the interesting one. The cause is structural: the extraction
pipeline chunks and samples documents rather than indexing every sentence, so
a sentence can be present, correct and attributable while having no fact row.
The fix was not to delete the quotes but to mark them `[D, corpus]` and name
the file they come from. That marker withholds exactly one claim relative to
plain `[D]` — *this sentence is indexed as a database fact row* — and states
the withholding openly. A document whose value is that its quotes are quotes
cannot advertise a discipline it does not keep.

A fourth, smaller correction came from counting: the convention paragraph
called the corpus-sourced citations "a minority", and there are eleven of them
against twelve fact-row citations. Technically a minority; rhetorically a
flattery. Replaced with a description that survives being checked.

## What this deliberately is not

It is not an implementation, not a plan for one, and not a verification.
`KNOWN_LIMITATIONS.md` §7 still says, in the same words it used yesterday,
that the application does not program devices — its "Limitation." and
"Impact." lines are textually unchanged, and only an "Updated" paragraph was
added beside them. `GAP_ANALYSIS_ETS.md` E1 stays open. T30 stays Tier 5.

The distinction the whole section is built around is between *documented* and
*verified*. §8.4 opens and closes by saying that nothing in it has been run
against a device, and every cross-reference added to `ARCHITECTURE.md`,
`COMPATIBILITY.md`, `GAP_ANALYSIS_ETS.md` and `KNOWN_LIMITATIONS.md` repeats
it. Both reviewers were asked specifically whether a KNX integrator skimming
§8.4 could come away believing otherwise. Both read it end to end and said no.

`docs/superpowers/plans/` and `docs/superpowers/specs/` still carry the old
`Legacy*` wording in a few places. Those are dated session artefacts; editing
them to match today's findings would falsify the record of what was known
when. Left alone deliberately.

## Verification

Six gates on the merged result, all measured, all matching the pre-merge
baseline exactly — a documentation change that moves a test count has done
something it should not have:

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **951 passed / 0 failed / 3 ignored** |
| `cargo run -p xtask -- check-layering` | ok |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| `npm run test` (`apps/knx-web`) | **179 passed across 17 files** |

## One operational note

rtk compresses shell output and silently drops words. An `evidenceText` read
back through a pipe is **not** verbatim, and comparing piped output against a
document will manufacture citation findings that do not exist. Every exact
comparison in this cycle — by the spike and by both reviewers — went through
a file and the Read tool. This is now written into the reusable pointer block
in §8.4 itself, because the next person to audit a quotation here will hit it
within ten minutes otherwise.
