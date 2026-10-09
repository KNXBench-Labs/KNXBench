# 2026-10-09 — Claude: legacy TypeNone fix, VD5 bounds, L4 download plans

Three packages from one request ("TypeNone-Platzhalter …; Speicherlimit für
.vd5 messen und anheben; Download von Legacy-Programmen aufs Gerät (L4).
Eigener Worktree"), each in its own worktree, gated and pushed separately.

## P1 — TypeNone rows (KL §128), `7f1efbb2` + `34026607`

- Server: `domain.rs` `carries_value` — kind `None` is `editable:false`, no
  `writeEtsId`, no warning; writes stay refused by name.
- Web: `ParameterPanel` decides by kind (old servers cannot resurrect the
  input): heading = bold label from `text`, empty text = `aria-hidden`
  spacer.
- Real `.vd4` probe in an offline namespace, screenshot reviewed.

## P2 — the real `.vd5`, `28d8f6af` + `ae0b4e8d`

- Measured: 67.5 MB file, 173 MB payload, four members (installer tree:
  three mask images + `ets.vd_`), 18.6 MB single value over 233,164
  continuation lines, 8.19 M values.
- Layout rule: exactly one EX-IM member, others listed as `unread-member`
  (never decrypted), records must tile the file. Bounds 128/256 MiB, parser
  defaults ~2.5–4× the measurement. Parsed document dropped before the
  transaction. Web report folds notes by kind.

## P3 — L4 download plans, `5dd021bf` (+ receipt)

- `legacy::code::legacy_program_code`: `s19_block` rows → `ProgramCode`.
  `CONTROL_CODE` = record's first octet = `(LsmIdx<<4)|event`; `0E/0F/0C/07`
  connect/disconnect/restart/compare. Records cross-checked with columns
  (allocation record holds the END address, Cookbook ADM1 format; plan still
  sends MP's length). Masks `01h`→`FFh`. TaskCtrl1 from record. Tables from
  LSM1's first data segment / `ASSOCTAB_ADDRESS` / `COMMSTAB_ADDRESS`,
  limits `(ADDRESS_TAB_SIZE−1)/2−1`, `(ASSOCTAB_SIZE−1)/2`.
- Identity: the task segment record of N000520 says `0079 0001 01`, the
  nine house PMs report `006A 0001 22` → identity from the program row (what
  the planner already did for XML).
- Oracles: house project (ETS 6.3 conversion, 9 PMs, plans equal except
  `4196h–4197h`, L2 deviation 3 / P-5008) and
  `SIEMENS_KNX_PDB_Nov_2016_ETS4.knxprod` (9 `070nh` pairs, equal code apart
  from 4 unmapped string parameters).
- L2 bug found on the way: `PARAMETER_ADDRESS` 0 = no memory. It merged
  unrelated parameters into one cell (one shared value!). `.vd5` 38,453 →
  41,817 parameters, `.vd3` 302 → 452.
- Mutation sweep 14/14 killed. No live run: the next step needs the
  maintainer's explicit device go (a scratch device with a legacy program,
  e.g. one house PM re-based onto the `.vd4` program — but note the
  P-5008 deviation, it would change `4196h`).
