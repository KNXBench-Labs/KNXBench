# 2026-10-04 — Claude (goal-ui owner) — UA1: coupler `.0` and KL-127

## Scope
Alpha rows MODEL-03 and KL-127, `goal-ui.md` §3b UA1. User decision: research
first, implement on reliable evidence, otherwise known gap and close.

## Research
- KNX Association offline project check: device octet 0 allowed only when
  `IsCoupler` (already cited in RESEARCH, recorded again in §25).
- Product DB already stores `Hardware/@IsCoupler` (`hardware.is_coupler`).
- Private corpus census: 103 `.knxprod`, 308 Hardware elements, 8 `IsCoupler="true"`
  (all line/RF couplers or IP routers), 2 `"0"`, 298 absent.
- KL-127: no `Ground`/`Site` space in the 3 corpus projects; 8 public
  xknxproject fixtures (6 readable) only have Building/Floor/Room. Scratch only.

## Implementation (backend)
- `knx-core`: `CouplerEvidence`, `Command::SetCouplerIndividualAddress`,
  `RestoreIndividualAddress.redo_coupler`, `CommandError::CouplerEvidenceMismatch`.
- `knx-productdb`: `query::product_hardware_is_coupler`.
- `knx-server`: `set_individual_address_impl` uses the coupler command only
  with `Some(true)` evidence; locks taken one after the other.

## Evidence
- RED: `apps/knx-server/tests/coupler_address.rs` 2/5 failed with the
  classification refusal before the change; GREEN 5/5 after.
- Core: `coupler_evidence_permits_zero_only_for_the_evidenced_product`,
  `coupler_evidence_does_not_bypass_duplicate_addresses`; productdb:
  `hardware_coupler_flag_is_evidence_only_when_true`.
- Mutants (6/6 caught by assertions, files restored byte-exactly): evidence
  product check, redo evidence, coupler line prefix, coupler duplicate check,
  NULL-as-coupler, server flag check.
- Gates on the candidate (shared alpha+workspace leases): fmt, clippy -D warnings,
  workspace tests 154 result blocks / 3,036 passed / 0 failed / 176 ignored /
  0 SKIP lines (with corpus link), layering, headers, anchors, corpus-gates,
  diff --check. First test run was SIGKILLed while linking (memory pressure
  from other sessions); rerun with `-j 6` passed. Web gates not run: no web file
  changed (frontend only built for the Tauri build script).

## Not done
- Web editor half of MODEL-03 (needs Web lock, held by commissioning).
- Imported projects' embedded `Hardware.xml` flag is not used.
