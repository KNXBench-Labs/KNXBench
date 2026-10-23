# Actual-connection main-growth increment

## Scope

Implements the existing P5 D/W main-growth policy without changing architecture,
schema, quiet API, binding version or external capability. Canonical behaviour
and exclusions: [live growth](../../docs/contracts/catalogue-jobs/live-growth-increment.md).

## Executed evidence

- Implementation handoff source hashes verified for all seven owned Rust files.
- Five retained owning executables rehashed; 18 exact focused tests independently
  re-executed on the retained library binary. This is retained-binary execution,
  not fresh compilation or 18 additional unique tests.
- Reported owning results verified against retained receipt: 708 unit passes,
  772 integration passes/31 ignored, separate large-member measurement ignored,
  two 13-test examples, two doctests. Focused populations overlap.
- Maintained headers and four test-layout tests independently pass.
- Native finite caps distinguish actual D and W allocation failure, partial
  setters/readback and restoration failure after successful COMMIT.
- Nine compiled critical variants are retained with final byte-exact restoration;
  original-runtime and development-harness failures remain separately classified.

## Pending acceptance

Independent source/evidence review ACCEPT has no blockers; all seven runtime/test
files match the reviewed hashes. Maintained branch/integrated gates remain pending.
No publication or whole-WP completion is claimed. B10, real large-database
acceptance, artifact lifecycle, production routes/UI and WP-19 remain separate.
Private evidence lives under `<profile-scratch>/wp043-live-growth-20261023/`;
no raw machine details or private corpus contents are committed here.
