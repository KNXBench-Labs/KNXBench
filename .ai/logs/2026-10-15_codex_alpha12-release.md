# Alpha 12 release preparation — 15 October 2026

Owner request: release Alpha 12, refresh the main website with current UI media,
and review README/documentation without changing the established dry engineering tone.

## Cutoff and ownership

- Baseline: published main `bee4f016`; previous release `v0.1.0-alpha.11`.
- Sole admitted in-flight prerequisite: P4 parameter comparison plus its separately
  committed corpus corrections (`a6cdfd9d`). The package is already in final gates.
- All other sessions pause new integration/publication until release or explicit abort.
- Release branch/worktree owned by this task; dirty selected root is not synchronized.
- No KNX traffic, hardware permission, Story content changes or vendor contact.

## Required acceptance (pending)

Full source-bound release gate, actual browser media review and localized website
acceptance; AppImage/MCP and multi-architecture Docker workflow dry runs with publishing
skipped; exact candidate presented for final publication decision. Only after that:
annotated immutable tag, hand-written prerelease, artifact checksum/download verification,
registry version/latest digest and isolated published-image smoke, and live Pages readback.

Self-review will be labelled as such, not independent approval. Prior package gates
remain historical evidence, never relabelled as an Alpha 12 run.

## Integrated source and actual media

P4 feature `519eff4b`, corpus fix `a6cdfd9d` and tested merge `facdd62e`
are ancestors of published main `17cfd5b7`; its public delivered receipt records
full51/51, OriginalData152/152 and explicit product16/16 under the original
source identity. Current live gate leases were observed empty before capture.
The release merged that source as `6d8bf9bb`; later P4 publication metadata is
accepted only after checking its complete runtime delta is empty.

Actual real-server capture successor passes one test/five stages with frozen
tracked source. Ten final PNGs were inspected, including native instructions
created via the wizard and separate support setup/preview frames. Seven WebPs
are decoded-pixel-equal to the original PNGs. Media provenance binds actual
server binary, application commit, capture source/config, dimensions and hashes.
Three failed harness predecessors remain private and are not relabelled green;
an earlier successful nine-frame run was superseded only for better framing.
No production code, admission or old sample schema was changed for the camera.

Website24, current docs checker, capture TypeScript and public website build
pass after the media update. The actual release-variant browser review is a
separate running preflight; full frozen-candidate acceptance is still pending.

## Self-review (not independent approval)

Corrected stale native14/15 versus actual16 and product22 versus23 reader copy;
promoted delivered Functions/universal reporting in the feature inventory;
retained historical release/media boundaries and exact approved Story bytes.
Restored an older video's original declared height after replacing stills;
updated browser recipes to await real navigation and distinguish production
from private-preview banners. Native-origin instructions use their actual label,
not the imported-override badge. Unsupported schema-11 instructions remain
explicitly refused. No unresolved runtime/security finding has been identified;
this is not a release-ready verdict before the complete gate and packaging runs.

## Actual complete-gate refusal and scoped correction

The complete gate on `02d823c4` passed 54 of 72 declared stages, including
Vitest3626, ordinary Rust4597/0 (207 ignored), fixture browsers and native-evidence,
then failed in native naming. Remaining stages did not run. This is a real test
failure, not the separately identified finalizer issue, and it cannot be accepted
through the prepared receipt-reconciliation tool. Full source/mode inventory after
the refusal equals its original start inventory; the original failed receipt stays.

Reset opens an active empty project, so a welcome-only Open button is absent.
The first correction proved EN paths but exposed an unscoped-combobox/keyboard
startup race on DE reload. The final correction uses the rendered Commands button
and scopes the localized native Open command to its dialog in all three native
specs. No production code, mock response, limit, timeout or core assertion changed.
Targeted final proof passes12/12 (naming4, Functions4, naming recipe4), plus the
native TypeScript configuration. All four steps exit0, edited spec hashes stay
frozen, and each real-server suite verifies the untouched demo hash and teardown.
This uses the unchanged predecessor runtime and is not full release acceptance.

The successor private runner includes all three native specs and uses separate
ordered-stage and corpus-count variables. Seven controls pass after reproducing
the old runner's control failures. It will execute a fresh complete 74-stage gate
on a newly committed frozen candidate, rather than relabel the failed execution.
Both earlier exact-SHA packaging dry runs passed with publish jobs skipped; new
SHA-specific dry runs remain required. No tag, release, Pages or Docker cutover.
