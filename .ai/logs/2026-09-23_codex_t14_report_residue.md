# T14 — project documentation report residue

## Scope

Completed the remaining architecture-owned documentation-report work without claiming ETS report parity. `knx-report` stays a pure renderer; `knx-app` composes project data with optional, hardware-consistent product-catalog context; CLI and HTTP expose the resulting diagnostics. No KNX, LAN, multicast, gateway, or hardware operation was performed.

## Delivered

- Optional report sections and English/German report language through one renderer.
- HTML preview using the same immutable project snapshot and renderer as export.
- Manufacturer, product, application-program, parameter-enum, and module-argument enrichment where catalog provenance is safe.
- Raw-value/reference fallback and explicit warnings for blank, absent, unsupported, or mismatched data.
- Hardware-consistency guard preventing data from another program/product being attributed to a device.
- Honest documentation of partial localization, unresolved semantics, and the deferred frontend preview/selection controls.
- Regression tests for catalog mismatches, raw fallback, preview non-mutation, warnings, and corpus rendering.

## Review and fixes

The first independent review found six correctness/data-integrity issues. The correction round fixed cross-attribution, blank-label fallback, non-scalar parameter behavior, module-argument naming, preview mutation coverage, and untranslated headings/warnings. Follow-up review found only stale documentation, which was corrected. Final review reported 0 Critical and 0 Important findings. Its sole Minor table-rendering issue was fixed before integration.

After merge, the local-only real-project corpus exposed a test defect: the row-balance assertion counted only literal `<tr>` openings while valid rows can carry attributes. The assertion now counts `<tr` and the corpus test passes; production rendering was not faulty.

## Integration and verification

Integrated into `main` with merge commit `cd10132` and follow-up test commit `253fd77`.

Fresh gates on merged `main`:

- Rustfmt: pass
- Clippy workspace/all targets with warnings denied: pass
- Rust workspace: 1,990 passed, 0 failed, 5 ignored across 93 result blocks
- Layering, headers, anchors, cargo-deny, and diff checks: pass
- TypeScript: pass
- Vitest: 977 passed
- Production frontend build: pass

The existing uncommitted `.ai/CURRENT_STATE.md` history was preserved and not folded into the product commits.
