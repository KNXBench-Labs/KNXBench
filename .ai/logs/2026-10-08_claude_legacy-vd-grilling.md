# 2026-10-08 Claude — grill-me: legacy VD3/VD4 product database import

User request: support importing old product databases; user has a KNX device only available as VD3/VD4.

## Facts gathered (no code changed)
- Existing research/design: docs/VD4_PRODUCT_DATABASE_IMPORT.md, docs/superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md (status proposed; Board decisions B-1..B-6 open; phase 1 maps identity only, params/COs retained-unmapped).
- Current behavior: typed filename refusal for .vd3-.vd5/.pr3-.pr5 (AR06, KL-128 DONE); .vd2 permanently refused (KL §11).
- Re-measured local Eibmarkt .vd4 (structure only): 37 tables incl. parameter 520, parameter_list_of_values 2192, communication_object 56, s19_block 44, text_attribute 11053; visibility via PAR_PARAMETER_ID/PARENT_PARM_VALUE; mask MASK_VERSION 1793 = 0x0701 (same family as verified memory download path, ADR-0048); s19_block is the image source vs. knxprod XML (ADR-0044).
- Product DB has relational tables (parameter_ref, com_object, dynamic_node, ...) plus stored-XML reads for download.

## Round 1 asked (Q1-Q6), recommendations
Q1 depth: (c) offline params now, download separate package. Q2 file/device + existing ETS5/6 project check. Q3 password: per-import + optional local remember (needs explicit OK; never built-in). Q4 accept 2026-09-26 design as base, new ADR, Claude implements under GPL firewall. Q5 .pr* out of scope. Q6 content detection; vd3/vd5 unevidenced until sample; vd2 stays refused.

Status: awaiting answers. No code/tests/build/commit/deploy/bus contact.

## Round 1 answers
User accepted Q1-Q6 recommendations: (c) offline params now, download later package; Q2 file supplied (EIBMARKT.VD3); Q3 (b) per-import + optional local remember; Q4 design accepted, Claude implements under GPL firewall, new ADR; Q5 .pr* out; Q6 content detection, vd3/vd5 unevidenced until sample, vd2 refused.

## New facts (measured 2026-10-08)
- EIBMARKT.VD3: 68,559 B, sha256 6d4cc9d1eb62d6d10d182c03e271693ddaed1a762f3690158e50e67c3ec3b96f; member PRJ-EIBM/EIBDB/ets.vd_; same password decrypts; payload sha256 a0202da31ac597d28363bfee01059aee9266ec06c59f2330bd8fb80ac87e10c8; header V 5.10 (2006); 37 tables, 4,214 rows, 0 structural anomalies, 980 continuation lines; type codes identical to vd4; no bytes 0x80-0x9F; no PASSWORD/KEY columns (also vd4).
- VD3 content: SA.8.16, JA.4.8.230, IF-RS232 (Eibmarkt, original manufacturer 102 Bischoff); masks 0x0020/0x0021 (BCU1); 572 params, 148 COs.
- Column drift between versions (vd3-only address_fixup, mask_entry; vd4-only MinEtsVersion, OBJECT_READONINIT*, s19 Record/MERGE_ID/PROC_MASK, ApplicationProgramAttributes, program_to_mask_feature) -> name-driven parser.
- House project (ets4 + ets6.3) embeds ETS-converted VD4 program N000520_IRBM_20 v34 (M-006A_A-0001-22-617E/26C0-O0079, ConvertedFromPreEts4Data=1, MV-0701, Dynamic present) -> differential oracle. Manufacturer pairs 131->M-0083, 106->M-006A, 121->M-0079.
- Parameter tree/visibility evaluator reads dynamic_node table (dynamic/evaluate.rs:386); only download reads stored XML (ADR-0044).

## Round 2 asked (Q7-Q13)
Q7 target device; Q8 direct table mapping (A) + oracle diff; Q9 ids M-xxxx_LX-<sha8>_A-<id>; Q10 store original + decrypted payload blob, never password; Q11 cp1252 [A]; Q12 single remembered password 0600 in server XDG config; Q13 no merge, hint only.

## Round 2 answers
Q7 target = Präsenzmelder (VD4). Q8-Q13 accepted as recommended (direct table mapping + oracle diff; ids M-xxxx_LX-<sha8>_A-<id>; store original + decrypted payload, never password; cp1252 [A]; single remembered password 0600 in server XDG config, CLI+web set/forget; no merge, hint only).

## Round-3 facts
- ETS conversion (oracle) of N000520_IRBM_20: 260 ParameterRef == 260 VD params of program 63558; Parameter ids P-<PARAMETER_NUMBER>; 28 ComObjectRef == 28 VD COs (22 ComObjects); 9 ParameterBlocks (pages = atomic type 0 "none" params); 56 choose/75 when; empty PARENT_PARM_VALUE child -> when default="true", non-empty -> when test=value; Access None 71 / ReadWrite 96; 5 languages (LCID 1031/1033/1036/1040/2057).
- Local product DB (~/.local/share/knx/products.sqlite) already holds catalog items for N000520 (both ETS4 26C0 and ETS6 617E conversions) from house-project ingest. VD4 also contains N000530 "Präsenzmelder 360 KLR" (program 63559) which is NOT in any project.
- VD4 programs: 63558 N000520 (260 params/28 COs), 63559 N000530 KLR (260/28).

## Round 3 asked (Q14-Q18)
Q14 variant N000520 (already placeable via house-project catalog) vs N000530 KLR (VD4-only) -> importer still built, KLR acceptance target. Q15 semantic equivalence via KNXBench evaluator vs oracle (structure, types, enums, defaults, memory, COs, visibility sweep), deviations fixed or named. Q16 sub-ids P-<PARAMETER_NUMBER>, O-<OBJECT_NUMBER>_R-<OBJECT_UNIQUE_NUMBER>. Q17 all 5 languages, LCID->BCP47, COLUMN_ID meaning [A] cross-checked vs oracle. Q18 slicing L1 parse/report, L2 publish + oracle diff, L3 CLI/server/web password flow + deploy, L4 download separate package.

## Round 3 answers
Q14: device is only an example; importer must be generic (no product-specific target). Q15-Q18 accepted as recommended.

## Synthesis (frontier empty)
Scope: generic, content-detected import of legacy ETS3-era EX-IM product databases (.vd3-.vd5 by content; vd3/vd4 evidenced, vd5 unevidenced) into knx-productdb, offline parameterisation (catalog, hardware, program, params/types/enums/defaults/memory, COs, visibility via dynamic_node, translations). Out: .pr*, .vd2, download (L4 later package), merging with modern data, built-in password, GPL code.
Decisions: Q1 (c); Q3 per-import + optional remember; Q4 design 2026-09-26 as base + new ADR, Claude implements under GPL firewall; Q6 content detection; Q8 direct table mapping; Q9/Q16 ids M-xxxx_LX-<sha8>_A-<PROGRAM_ID>_P-<PARAMETER_NUMBER> / _O-<OBJECT_NUMBER>_R-<OBJECT_UNIQUE_NUMBER>; Q10 store original + decrypted payload blob, never password, blank+report non-empty secret columns; Q11 cp1252 [A]; Q12 single remembered password, 0600, server XDG config, CLI/web set+forget, never a candidate list; Q13 no merge, equivalence hint; Q15 semantic equivalence vs ETS-converted oracle via KNXBench evaluator incl. visibility sweep, deviations fixed or named; Q17 all languages, LCID->BCP47, COLUMN_ID meaning [A] cross-checked; Q18 slices L1 read/report, L2 atomic publish + ADR + corpus/oracle tests, L3 CLI/server/web + deploy, L4 download separate; each slice reviewed/green/merged/pushed.
Status: awaiting explicit go.
