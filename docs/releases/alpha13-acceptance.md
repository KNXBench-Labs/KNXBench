# Alpha 13 release acceptance — 16 October 2026

Frozen candidate: `6105976f1a960dd34205638df501f8c3ede60a6e` on
`release/alpha13-20261016`, now immutably tagged `v0.1.0-alpha.13`.
Owner explicitly requested preparation and publication. This result-only record
does not change the accepted product or move its tag.

## Evidence and review boundary

All **59 ordered stages** pass, bound to **2,965 source paths** and **516 original
input commitments**. Complete source modes/content/HEAD and original inputs are
unchanged. This is a source acceptance record, not by itself public artifact or
physical-device evidence. Review is a separate **in-session self-review**, not
independent-agent approval.

The first attempt stopped when isolated XDG settings redirected Firefox into an
empty cache. Neither of its two Firefox assertions executed. The failed receipt
is retained. A private harness-only cache binding was corrected; ten admission
controls reject changed source/HEAD, incomplete or failed/missing/reordered stages,
missing logs and missing freeze evidence. The same-source continuation retains
**32 original successful stages** and actually executes **27 remaining stages**.
This is not represented as one uninterrupted all-green execution.

## Measured results

| Check | Result |
|---|---|
| Rust workspace | 4,724 passed; 0 failed; 210 ordinarily ignored |
| Component tests | 3,632 passed; 0 failed or pending |
| Default fixture Chromium | 578 passed |
| Additional browser configurations | 369 passed in 22 profiles, including 2 Firefox cases |
| Native evidence | 12 passed |
| Native naming / Functions / naming recipe | 4 / 4 / 4 passed |
| Conventional private corpus | 152/152 in 33 targets |
| Explicit product-corpus scope | 16/16 in 6 targets |
| Tooling | 189 run; 1 inherited skip; 0 failures |
| Build, all TypeScript configs, fmt, Clippy | Passed |
| Generated bindings | Complete nonempty producer/consumer set, literal-preserving token equality; original bytes restored |
| Website/Story tests and release-mode build | Passed |
| Documentation and five repository checks | Passed |
| Source/original inputs | Unchanged |

Ignored tests are not claimed as executed. Conventional and explicit corpus
scopes are reported separately; neither establishes complete manufacturer
semantics, ETS certification or device support. Native-evidence/edit checks use
an isolated software server and synthetic projects, not a native WebKitGTK window.

## Packaging without publication

- AppImage/MCP run **38954005373**: successful; **Publish tag release** skipped.
- Docker amd64/arm64 run **38954008662**: successful; **Publish multi-arch image** skipped.
- Both runs are bound to the exact candidate. Downloaded dry-run files were hashed;
  the actual downloaded MCP version/revision was verified in network isolation.
  Public tag-build artifacts are verified independently, not assumed byte-identical.

## Scope and immutable publication

[Release scope](alpha13.md) records real manufacturer-mode editing, parameter
blocks, atomic missing-object refresh, independent navigation folds, grouped File
actions and offline image integrity/codecs. Native/model16 and product23 are
unchanged from Alpha 12; prior-reader compatibility remains unverified. MV-0025,
new module construction and full source/procedure/recovery paths remain open.

[Structured source evidence](../evidence/alpha13-acceptance-2026-10-16.json).

## Ordered gate inventory

| Stage | Exit | Provenance |
|---|---|---|
| `controls` | 0 | Original successful prefix |
| `versions` | 0 | Original successful prefix |
| `diff-check` | 0 | Original successful prefix |
| `fmt` | 0 | Original successful prefix |
| `isolation` | 0 | Original successful prefix |
| `tools` | 0 | Original successful prefix |
| `website-tests` | 0 | Original successful prefix |
| `story-tests` | 0 | Original successful prefix |
| `website-build` | 0 | Original successful prefix |
| `web-build` | 0 | Original successful prefix |
| `web-types` | 0 | Original successful prefix |
| `vitest` | 0 | Original successful prefix |
| `browser-fixtures` | 0 | Original successful prefix |
| `browser-ux-feedback` | 0 | Original successful prefix |
| `browser-communication` | 0 | Original successful prefix |
| `browser-ga-members-download` | 0 | Original successful prefix |
| `browser-group-addresses` | 0 | Original successful prefix |
| `browser-dialogs` | 0 | Original successful prefix |
| `browser-devices` | 0 | Original successful prefix |
| `browser-notes` | 0 | Original successful prefix |
| `browser-instructions` | 0 | Original successful prefix |
| `browser-parameters` | 0 | Original successful prefix |
| `browser-catalogue` | 0 | Original successful prefix |
| `browser-issue09` | 0 | Original successful prefix |
| `browser-structure` | 0 | Original successful prefix |
| `browser-updates` | 0 | Original successful prefix |
| `browser-monitor-control` | 0 | Original successful prefix |
| `browser-address-unavailable` | 0 | Original successful prefix |
| `browser-device-checks` | 0 | Original successful prefix |
| `browser-service-control` | 0 | Original successful prefix |
| `browser-site` | 0 | Original successful prefix |
| `browser-device-download-support` | 0 | Original successful prefix |
| `browser-reload-guard` | 0 | Executed continuation |
| `browser-support` | 0 | Executed continuation |
| `browser-task-status` | 0 | Executed continuation |
| `clippy` | 0 | Executed continuation |
| `rust` | 0 | Executed continuation |
| `server-build` | 0 | Executed continuation |
| `native-evidence` | 0 | Executed continuation |
| `native-naming` | 0 | Executed continuation |
| `native-functions` | 0 | Executed continuation |
| `native-recipe` | 0 | Executed continuation |
| `bindings` | 0 | Executed continuation |
| `bindings-compare` | 0 | Executed continuation |
| `docs` | 0 | Executed continuation |
| `xtask-check-layering` | 0 | Executed continuation |
| `xtask-check-headers` | 0 | Executed continuation |
| `xtask-check-anchors` | 0 | Executed continuation |
| `xtask-check-ledger` | 0 | Executed continuation |
| `xtask-check-corpus-gates` | 0 | Executed continuation |
| `corpus-inventory` | 0 | Executed continuation |
| `corpus` | 0 | Executed continuation |
| `product-dynamic_tree` | 0 | Executed continuation |
| `product-corpus_compatibility_matrix` | 0 | Executed continuation |
| `product-legacy_member_names_corpus` | 0 | Executed continuation |
| `product-parameter_views_corpus` | 0 | Executed continuation |
| `product-program_code` | 0 | Executed continuation |
| `product-standalone_packages` | 0 | Executed continuation |
| `final-whitespace` | 0 | Executed continuation |
