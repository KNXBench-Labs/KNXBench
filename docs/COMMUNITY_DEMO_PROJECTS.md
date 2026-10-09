# English community demo projects

Status: original local acceptance below remains dated 2026-10-08; repository publication followed on 2026-10-09. The owner approved website linking on 2026-10-09; its separate handoff verification is in [WEBSITE](WEBSITE.md). These are original fictional offline exploration projects, not ETS-compatible output or executable building plans.

## Contract

Three native `.knxdb` projects: Single-Family Home (25–40 devices), six-flat Multi-Unit Residential Building (80–120), three-floor Office Building (150–250). All project text/docs English. Common functions: switch/dim/blind/room-temperature-heating signals, separate command/status; home central-off/scenes, residential apartment/common areas, office presence/zones/meeting scenes. Three-level groups use function/zone/signal. Clean baseline, explained reserve channels and five supported UI exercises per project.

Ship one shared fictional `.knxprod` if catalogue/parameters require it, installable offline with the current catalogue UI, without replacing the user's database. Topology is instructional: explain main/secondary TP lines and illustrative couplers/interface, never claim electrical design, isolation or verified filtering. No firmware, vendor baggage, signatures, certification or real-device bus operation.

Acceptance requires a clean catalogue, real UI exercises, duplicate/invalid address/reference/DPT checks and exact native model save/load equality. Record actual tested source/build/schema, reproducible generation and package checksums. Inspect locally first; repository/website publication is a separately reserved owner decision.

## Architectural/format findings

- [ADR-0005](adr/0005-separate-product-database.md) separates catalogue data from projects. A `.knxdb` opening successfully does not establish parameter/program availability in an empty catalogue.
- `knx-store/src/manifest.rs` records missing manufacturer references; references are not the programme database. `knx-app/src/import.rs` can retain manufacturer files opaque, but retention is not parameter functionality.
- [ADR-0028](adr/0028-no-knxproj-export.md) remains binding. Existing `tools/manual_sample_project.py` is a screenshot fixture, not a production exporter; these demos do not produce `.knxproj`.
- The generation tool writes the native model through `knx-core` and `knx-store`, never SQL shaped by external format. Product data remain separate, generated from shared original declarations rather than hard-coded into app code. No production core/API/UI/schema dependency change.
- The fictional product package uses the already-tested scheme-11 standalone adapter (`knx-productdb/tests/standalone_packages.rs`). This is tested KNXBench synthetic input, not XSD validation, signatures, ETS admission or manufacturer compatibility evidence. The synthetic `M-7FF1` identity is not a claim of an allocated manufacturer number.

## Observed scope adjustments

The real UI has a device Description edit but no device-name rename command.
Exercise 2 therefore edits the description rather than adding a new app feature.
The group-creation form has no declared-DPT input; exercise 4 creates an address
and then verifies the DPT 1.001 inferred from its receiver link. The baseline
addresses keep explicit declarations and matching sender/receiver DPTs.
Some native edits auto-save once a store path exists: guides require an
untouched reset copy, not merely a final Save-as instruction.

The initial synthetic master declared `SizeInBit`, `KnxManufacturerId` and
`MediumTypes`, which the bounded catalogue adapter recorded as unknown/
unsupported. These unnecessary authoring fields were removed from the original
fixture rather than widening parser admission or hiding warnings. The final
clean-catalogue UI install reports **0 unknown, 0 conflicts**, and all nine
fictional application programs/290 device parameter views resolve.

## Reproduce generation and verification

From the repository root with the pinned Rust/Cargo and Python toolchain:

```sh
python3 -m unittest discover -s tools/tests -v
cargo test -p knx-app --all-targets
cargo clippy -p knx-app --example community_demos -- -D warnings
python3 tools/community_demos.py build NEW_OUTPUT_DIRECTORY
```

The output path must not already exist. The builder makes a private temporary
stage, installs the fictional catalogue in a disposable product database,
evaluates all programs, writes through the typed native store, compares the
entire reopened model and then copies the completed output to an exclusive
new directory. No consumer app, product database or bus is contacted. Original
source descriptors/generator and the unchanged repository LICENSE are included
in each ZIP; SHA256SUMS covers every archive member. No new licensing/CLA policy.

### Real browser acceptance

Use a freshly built `knx-server` and production `apps/knx-web/dist`. Prepare a
fresh task-owned data directory with copies of the three native files; point
`KNX_DATA_DIR`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME` into it. Use port **4826**.
Run the server and Chromium **inside one user/network namespace with only
loopback enabled** (`unshare --user --map-root-user --net`; bring `lo` up).
Verify the namespace differs from the host and no non-loopback interface exists.
Never use the owner's live server/catalogue or loosen the namespace to make
startup discovery succeed.

Bind the CLI script to those exact candidate/evidence paths:

```sh
python3 tools/community_demos.py ui-script CANDIDATE_DIRECTORY EVIDENCE_DIRECTORY VERIFY.js
```

Run `playwright-cli` in the same namespace, with a Chromium config naming the
verified distribution executable, 1440×1000 viewport and English locale. Open
`about:blank`, then `run-code --filename=VERIFY.js`. This opens the actual app,
dismisses asynchronous onboarding, proves the catalogue is empty, imports the
package through its file input, confirms idempotent re-import, opens each native
file through the real picker, checks all device parameter panels, performs all
five exercises and saves/reopens the three working copies. It uses no response
mocks. Read `window.__communityDemoVerification` through `eval` and validate its
`success`, three project results, named assertions and request/error arrays;
a CLI exit by itself is insufficient evidence.

`/api/project/save-as` is a void response, unlike the model-returning edit
routes. Catalogue conflicts are a scalar count, not the core report's vector.
Native `<option>` elements need DOM assertions, not a visibility-based option
count. These distinctions were observed and corrected in the verifier, not
changed in production.

For full edited-state fidelity, with the runtime's practice files still present:

```sh
cargo run -p knx-app --example community_demos -- CANDIDATE_DIRECTORY PRACTICE_DIRECTORY
```

This reads each practice store without changing it and compares the full model
to the independently generated baseline plus the four intended core commands
(description, parameter, new group, receive link). Only the legitimate save
modification timestamp is normalized; allocator, order, provenance, topology,
flags and all unrelated content must remain equal. Each complete edited model
also saves/reopens exactly in a separate in-memory store.

### Acceptance evidence

- [UI receipt](evidence/community-demos/ui-verification.json): **966 named
  checks**, 15 exercises, **290** device parameter evaluations, no external
  browser requests or page exceptions. No bus operation other than the
  existing startup-discovery attempt, which fails **502** in the loopback-only
  namespace. No physical gateway can be reached.
- [Self-review](evidence/community-demos/self-review.json): exact source digests
  and bounded static scan; this is **self-review**, not independent approval.
- Nine [actual screenshots](evidence/community-demos/screenshots/): clean
  baseline clones, reset scrolling, dismissed toasts and a filtered participant
  trace. Three representative images were visually inspected. The buildings
  view uses horizontal columns; large hierarchies require scrolling. No UI
  redesign was performed to manufacture better demo evidence.
- The receipt records application base `33db32e94b7281787331c7bb0ebf3c8a1c4043ab`,
  server `0.1.0-alpha.2+g33db32e9` and frontend `0.1.0-alpha.5`. Those existing
  differing labels are preserved and disclosed, not corrected as unrelated
  work. Native schema **10**, product database **22**, synthetic package scheme
  **11**. No blanket published/older-alpha or native-shell claim.

See [the download index](../demos/README.md) for exact measured project sizes.

### Final local delivery gates

[The delivery receipt](evidence/community-demos/delivery-verification.json)
records 162 affected application tests passed (25 explicitly ignored), 42
Python tooling tests, five final authoring-example tests, Clippy and format
checks. All five repository gates pass on the explicit worktree target:
layering, headers, anchors, ledger and corpus-gate declarations. Corpus-gate
lint is not execution of the ignored private-corpus tests. Documentation
validation covers 202 Markdown files, 1,854 local targets and 33 manual chapters;
external URLs are not checked.

All four final ZIPs pass CRC/member/outer checksum verification, contain the
current original source files and unchanged licence, and preserve exactly the
three native files and shared catalogue accepted by the real browser. A
header-only correction required rebuilding the packaged source/manifest but
changed none of those accepted inputs. Earlier header/import-path/log-footer
diagnostics are retained as rejected attempts, not green gates.

The isolated test server and browser are stopped. Local creation and acceptance
are complete; public repository/website delivery remains an explicitly separate
owner decision. No commit, push, release, root synchronization or website
publication was performed, and no parallel worktree is incorporated.
