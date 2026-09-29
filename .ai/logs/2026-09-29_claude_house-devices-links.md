# 2026-09-29 — Claude — the house, device by device; `Links` is positional

## Question
The maintainer did not know what a "modular device" is and asked to be told
in terms of his own devices (`devices.md`, "Unser Zuhause" project).

## Answer
- A modular application program declares a channel once (`ModuleDef`) and
  stamps it per channel (`Module`). The house's 12 programs contain none.
- Planned every address of the ETS 6.3.0 project (plan only, nothing sent):
  17 of 32 plan; the rest are refused per program (RESEARCH §19.12).

## Bug found
`knx-etsproj` mapped every schema ≥21 `Links` entry to `Send`. The direct
Project Schema23 PDF says the first entry is the sending one. Two of the
house's objects have two links, so the planner refused 1.1.22 and 1.1.24
with "object sends on 2 group addresses". Fixed positionally; cross-checked
against the ETS4 export of the same house, which states directions
explicitly: 543/543 comparable senders agree, 2 of them multi-link.

## Tests
- `crates/knx-etsproj/tests/links_direction.rs`: 4 synthetic (first sends,
  single link, order is the document's not numeric, dangling first entry
  does not promote the second) + 1 corpus cross-check.
- `map.rs` corpus test renamed/rewritten from the old "default to Send"
  assumption to the positional rule.

## Not done
- 1.1.11–13 union overlap, presence detectors `LsmIdx 5`, Merten
  `LdCtrlTaskCtrl1`, Gira `MV-0012` remain refused.
