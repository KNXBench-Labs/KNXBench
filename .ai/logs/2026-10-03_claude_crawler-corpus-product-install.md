# 2026-10-03 — Claude: product install vs. public crawler corpus (test only)

- Input: 853 files from `knxprod-crawler` (Siemens 1, ABB 707, Hager 48, MDT 97; 1.25 GiB).
- Binary: release `knx` CLI from origin/main `c6b5a240`, own target dir on /home.
- Path: `knx products ingest <file> --product-db <shared fresh db>`, content (sha256) order;
  refusals re-run in fresh isolated DBs; then `products verify` / `coverage`; sqlite census.
- Result: 644 installed (608 new + 36 duplicates); refusals: namespace 147 (s10 145, s23 2),
  CLI case 46, size 13, evidence limit 1, nested ModuleDef crash 1, invalid ZIP (PDF) 1.
- Counterfactual: Hager with lowercase names: 43/46.
- DB 13.2 GiB; verify 0 mismatches; coverage 51/1167 plannable.
- Docs: PRODUCT_DATABASE_CORPUS, KNOWN_LIMITATIONS §149–§153, COMPATIBILITY, GAP_ANALYSIS_ETS A3,
  IMPLEMENTATION_STATUS. No code change.
- Lesson: an "other" error bucket hid three different causes. Classify every refusal by its exact
  message and cross-check the doc table against the raw results in code.
