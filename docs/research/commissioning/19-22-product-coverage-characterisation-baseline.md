# 19.22 Product coverage characterisation baseline (2026-11-08)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

## Scope and input identity

**[V]** This is a new forward characterisation baseline, authorised by the owner
on 8 November 2026 ([PRG-01](../../open-work/PRG-01-programming-gaps.md)). It replaces
the ignored coverage test's 105-package / 467-program expectations. The original
105-package selection could not be reconstructed; nothing here reconstructs it
or attributes its growth package by package. [Section 19.20](19-20-full-product-corpus-coverage-with-bounded-large-admission.md)
remains the historical observation, not a present compatibility claim.

The named input set is **product-coverage-2026-11-08**: all 122 loose `.knxprod`
files in the private collection at measurement time, recursively discovered by
`knx_testsupport::walk_corpus_files` in its sorted traversal order. Symlinks and
packages inside ZIP bundles are not followed by this test. Every package is
installed in that order into one fresh database, using `PackageLimits::LARGE`;
no package is excluded, and an installation error fails the test. There are
758 distinct installed application programs.

The private `manifest-pre.tsv` and identical `manifest-post.tsv` name each file,
its byte length and SHA-256. The test also pins a public, name-free input
commitment: SHA-256 of the concatenation, **in installation order, retaining
duplicates**, of each line `lowercase-package-sha256<TAB>decimal-byte-length<LF>`:

```text
3df573f3af8dc37f81e6602b531402443e0c1ead6ba0472d8455109a4aa780a6
```

A same-count replacement or changed installation order therefore fails, rather
than silently redefining the sample. Private filenames, program identities,
source XML and detailed refusals stay outside Git.

## Measured code and outcomes

**[V]** Two measurements at each of the following code versions gave identical
per-program classifications and plan sizes. The input manifests were unchanged
before and after the four runs:

- **C0 `8433a2d48`:** the 9 October 2026 code that introduced the old pins,
  evaluated on **today's** 122-file input set, not the lost 105-file set.
- **B `ac8f00556`:** the current-code base of this package, on that same set.

Both intentionally fail the old package-count assertion (122 versus 105),
after recording every outcome; those failures are retained, not credited as
passing regression tests. Additional detailed runs reproduce the classifications
and supply the private, per-program refusal attribution.

| Outcome | C0 on new inputs | B on new inputs | New pinned baseline |
| --- | ---: | ---: | ---: |
| Verified | 1 | 1 | 1 |
| Untested, offline plan builds | 101 | 112 | 112 |
| `not-memory-mapped` | 507 | 372 | 507 |
| `unmodelled-step` | 3 | 25 | 25 |
| `procedure-contents` | 1 | 1 | 1 |
| `parameter-evaluation` | 51 | 51 | 51 |
| `parameter-value` | 9 | 3 | 3 |
| `image-structure` | 84 | 50 | 50 |
| `product-data` | 1 | 136 | 1 |
| `vendor-download-plugin` | 0 | 7 | 7 |
| **Total** | **758** | **758** | **758** |

Masks `MV-0701` and `MV-0705` respectively contain 103 / 148 programs, of
which 42 / 71 plan in the new baseline. The existing single verified-program
assertion, including its plan size, remains unchanged. No new hardware evidence
is added.

## Attribution of every changed classification

**[V]** Matching program identities between C0 and B gives exactly these
transitions; all other 572 classifications are unchanged:

| C0 → B | Programs | Observed reason |
| --- | ---: | --- |
| `image-structure` → `unmodelled-step` | 25 | LittleEndian images now build (ADR-0159, `e5306e023`); the next refusal names `LdCtrlCompareProp`. |
| `image-structure` → `untested` | 13 | Inactive choose branches no longer select Float fields (ADR-0160, `e5306e023`). This is **not Float encoding support**. |
| `parameter-value` → `untested` | 2 | The same inactive-controller evaluation no longer selects conflicting parameter references. |
| `parameter-value` → `image-structure` | 4 | The same conflict disappears; the next refusal is the object's non-default `Priority="High"`. |
| `unmodelled-step` → `vendor-download-plugin` | 3 | The declared plug-in guard (`4e722bfd0`) now refuses before the former `LdCtrlWriteProp` refusal. |
| `untested` → `vendor-download-plugin` | 4 | The same declared plug-in guard, not a manufacturer list. |
| `not-memory-mapped` → `product-data` | 135 | Source parsing now rejects `RelativeSegment` payloads shorter than `Size` (`ba46467a9`); parsing fails before the mask is examined. |

The 13 Float and six reference transitions were independently localized by a
Git bisect over a seven-package subset containing all 19 programs: last old
`fb5706a22`, first changed `e5306e023`. The former and new per-program refusal
texts, subset manifest, probe and bisect log are retained privately. The source
change is inactive-controller selection, not a newly implemented Float codec.

### Mask precedence correction in this package

The 135 parse failures exposed an existing coverage invariant violation: every
program with a mask outside `MV-0701` / `MV-0705` must be refused for its mask,
not look like an otherwise near-supported product. The stricter source parser
is correct and is **not** weakened.

When code loading fails, `stored_mask_refusal` checks the already stored mask.
If that mask cannot load through memory, coverage and device preparation return
the existing `not-memory-mapped` refusal. A missing mask, missing program,
failed lookup or memory-mapped mask keeps the original code error. Both outcomes
still refuse preparation; no plan, transport call, retry or new device support
is added. Synthetic tests cover two non-memory masks and a memory-mask negative
control; removing the correction reproduces the category mismatch.

## Reproduction and limits

Run the ignored test explicitly, in a loopback-only network namespace, with
scratch `XDG_DATA_HOME` / `XDG_CONFIG_HOME` and `KNXBENCH_PRODUCT_CORPUS` pointing
to the private input set:

```sh
cargo test --offline --locked --release -p knx-app --test it \
  download_coverage_corpus:: -- --ignored --nocapture
```

The hash and category expectations are fixed literals, never inferred at run
time from the result being asserted. The always-on inventory helper test varies
content, length, multiplicity and order. Raw logs, manifests, per-program
dumps and bisect evidence are retained under
`<profile>/evidence/2026-11-08-kl006-baseline/`.

This is a characterisation of product-default offline planning, **not an
independent semantic oracle**, ETS-equivalence claim, project-configuration
census or hardware acceptance. The refusal-precedence correction changes the
diagnostic, not acceptance of unreadable products. PRG-01 and KL-006's actual
vendor plug-in limitation remain open.
