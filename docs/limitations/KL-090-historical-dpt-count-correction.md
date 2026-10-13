# Historical DPT count correction

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-090)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## 90. There is no DPT main type 46; 46 is a *count* of main types in one ETS master-data file

**Limitation.** Not a limitation of the code — a limitation of a number that
has been circulating through this repository's own planning documents, and
that reached a task brief as a requirement. `goal.md` §3 row **E4** briefed
the DPT-codec work as "main types 20, 21-30 and the rest of the 46
`knx_master.xml` main types", and task 5's brief shortened that to "main
types 20-30 and 46". There is nothing numbered 46 to implement. Numbers 88
and 89 are claimed by two other branches in flight alongside this one; this
entry is 90 for that reason and no other.

**Where 46 actually comes from.** `docs/RESEARCH.md` §5 measured the DPT
catalogue of one specific master-data file — the one inside
`OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj` — and
found 46 `DatapointType` elements. That file's 46 main-type ids are `DPT-1`
through `DPT-23`, then `DPT-25`, `DPT-26`, `DPT-27`, `DPT-29`, `DPT-30`,
then eighteen LTE/system types in the 200-series (`DPT-206`, `DPT-217`,
`DPT-219`, `DPT-222`, `DPT-229`, `DPT-230`, `DPT-232`, `DPT-234`,
`DPT-235`, `DPT-237`, `DPT-238`, `DPT-240`, `DPT-241`, `DPT-244`,
`DPT-245`, `DPT-249`, `DPT-250`, `DPT-251`) — re-measured on 2026-09-14
with

```sh
unzip -p "OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj" \
  knx_master.xml | grep -oE 'DatapointType Id="DPT-[0-9]+"' \
  | sed 's/.*DPT-//;s/"//' | sort -n
```

RESEARCH.md §5 already warns, in so many words, that this is "a property of
*this one* master-data file, not a fixed constant". The warning was right
and was read past anyway: the other `knx_master.xml` copies under
`OriginalData/` carry 29, 49 and 55 main types, and the two other demo
projects carry 57 and 63. A count that changes with the ETS vintage cannot
be an identifier.

**What the Standard says.** DPT-AS (`03_07_02 Datapoint Types v02.02.01
AS`) §2's overview table enumerates main types 1 to 31 and then jumps
straight to the 200-series; nothing between 32 and 199 exists. The nearest
thing to a "46" in that document is *clause* §3.46 "Datatypes A8A8A8A8",
whose datapoint type is `231.001 DPT_Locale_ASCII` — a clause number, not a
main type. Searched explicitly so the next reader does not have to:
`46.001` against `knowledge_base/knx_spec_kb_full179_clean.sqlite` (177
PDFs) returns exactly one hit, and it is a Connection Code table row from
`03_07_03 Standardized Identifier Tables v01.04.01 AS` —
`46 | 2Eh CC_FanSpeed | 5.001` — where 46 is a connection-code number and
the datapoint type is 5.001. `DPST-46` returns `[]`.

```sh
cd <old-sources>/knx-spec-kb && .venv/bin/python \
  scripts/05_knowledge_base_v1.py -o knowledge_base/knx_spec_kb_full179_clean.sqlite \
  --query "46.001" --limit 3
```

**Consequence.** No semantics were invented to fill the gap, and
`codec.rs`'s `a_main_type_above_the_implemented_range_is_unsupported_not_a_panic`
test pins main type 46 among the numbers that must return
`DptCodecError::UnsupportedDpt` rather than anything cleverer. Main type
**31** *does* exist (31.101 `DPT_PB_Action_HVAC_Extended`, DPT-AS §4.7.1)
and is deliberately not implemented: its own entry reads "This DPT shall
not be used for runtime communication. This DPT shall only be used for
encoding Parameter values in CH_PB_HVAC_Mode_1", so a group-value codec has
no honest use for it. It is also absent from every `knx_master.xml` in the
repository.

**Lifted when.** Never, as stated — there is nothing to lift. This entry
exists so the number stops being re-derived. If a future brief asks for
"main type 46" again, it means "the remaining main types in some
`knx_master.xml`", and the right first step is to measure the file in front
of you.
<!-- END retained catalogue record -->
