# Declared product-folder captions: source and identity admission

## State and boundary

WP-06 research on 6 November 2026, baseline `e53e75417`. Independent raw-source
recount agrees with the parent's identity and literal-candidate populations;
the parent separately checked the complete shown ancestry and source policy.
The [admitted bounded contract](../contracts/declared-product-folder-captions.md)
adds only literal declared product text to an already assigned saved folder.
This source study does not itself establish implementation, original-tool
rendering, broader compatibility or hardware acceptance. The separate
[implementation/verification boundary](../status/2026-11-06-declared-product-folder-captions.md)
keeps runtime evidence and delivery requirements explicit.

## Public primary sources

1. [Project Schema23 v01.00.00](https://support.knx.org/hc/article_attachments/17389755651474),
   dated 1 March 2024; retrieved from the actual attachment list of the official
   [Project schema description](https://support.knx.org/hc/en-us/articles/4408207190674-Project-schema-description)
   article. The original PDF was fetched, hashed and its full text extracted.
   Section 1.1.3.3, printed pp. 26–27, defines RELIDREF relative to a known
   application program. Section 1.2.5.21–23, pp. 51–52, relates saved Folder
   nodes to ParameterBlocks with ShowInComObjectTree and uses a shortened
   Channel/ParameterBlock reference. This is not a label-matching rule.
2. [How to modernize your App](https://support.knx.org/hc/en-us/articles/360015340940-How-to-modernize-your-App),
   publisher's complete public article body, retrieved through its public Help
   Center API. Its GroupObjectTree SDK description connects a displayed Folder
   with an active ParameterBlock whose ShowInComObjectTree property is set.
   This is SDK documentation, not an observed original-tool run in this study.
   Labelling an existing saved folder with a declaration does not establish that
   the folder is currently active or reconstruct its current placement.
3. [Pinned producer fixture](https://github.com/XKNX/xknxproject/blob/f97db068884eca2b460abb1fb4db2262345f7f47/test/resources/module-definition-test.knxproj),
   XKNX/xknxproject commit `f97db068884eca2b460abb1fb4db2262345f7f47`,
   `test/resources/module-definition-test.knxproj`. Previously retained exact
   original bytes were reauthenticated against that revision's public GitHub
   content metadata by full Git-blob identity and size on 6 November 2026.
   Package: 10,376,392 bytes; SHA-256
   `8966d0fcde22a0ca8e133deddb9873e82f7ee1fee93195963787085143e7675b`.

No vendor implementation or XML is copied into regression fixtures. Original
PDF/article/package payloads and retrieval receipts stay outside the repository.
Public documentation records the sources, method and finite counts only.

## Bounded witness method and counts

Read ZIP members without general extraction, with entry/size/DTD bounds. Use the
exact canonical ApplicationProgram selected through each device's
Hardware2Program/ApplicationProgramRef, then that program's own direct Dynamic.
Reconstruct relative references by removing only the known full application ID
and delimiter. Compare full declared Channel/visible-Folder ancestry, not the
last ID alone. No captions participate in identity. Module definitions and
module instances are not root declarations.

| Population in this one package | Parent count |
| --- | ---: |
| ApplicationProgram declarations | 3 |
| Devices with a saved GroupObjectTree | 4 |
| Saved Folder occurrences | 246 |
| Module-path Folder occurrences, excluded from this increment | 241 |
| Non-module Folder occurrences | 5 |
| Distinct application/full saved Folder paths among those occurrences | 5 |
| Unique exact full-ancestry matches to root product declarations | 5 |
| Literal nonblank, no text-parameter/template, eligible distinct paths | 4 |
| Application programs contributing those four eligible paths | 2 |

The independent reader authenticates canonical ownership and the same five
relative/full identities and four literal candidates. All three applications
place module-definition Dynamic nodes before the direct application Dynamic;
selecting the first descendant would therefore read the wrong tree in all three.
Non-module describes the selected folder scope, not non-modular devices.
All five saved texts differ from their product declarations, so equality is
neither a matching key nor a reason to overwrite saved text. None witnesses
missing-saved-text fallback; that policy requires separately authored tests.

All five matched blocks explicitly declare unqualified ShowInComObjectTree=true.
Their own Access attribute is absent; none is an alias, below Repeat/Module or
below a denied/unknown Access ancestor. The applications do contain Repeat
elsewhere, which is not evidence that an unrelated root block is repeated.
One remaining matched folder has parameter-dependent text and is outside the
literal subset. Counts are package/node/path populations, not independent
projects, rendered UI cases or accepted device families.

The real witness is project/product schema 20. The public schema-23 definition
supports that schema's reference contract; it is not a measured schema-23
export in this study. The source contract keeps the supported combinations and
synthetic-versus-real evidence explicit.

## Engineering consequences, not external format claims

- Preserve saved membership, ID ancestry, keys and schema-20 saved Text exactly.
- Keep declared installed-product caption provenance separate from saved text,
  evaluated labels, current placement and the original import-time product bytes.
- Read the selected measured winning source with bounds/hash/owner checks;
  normalized root-node identity must agree. Do not reparse audit `extra` fields.
- Initial display supports only a literal declaration: no translations, dynamic
  text substitution, renames, aliases, modules, Repeat descendants or priority.
  Conservative true/1 flag and access admission are KNXBench policy, not an
  exhaustive lexical or original-tool compatibility claim from this witness.
- No domain/storage/import schema change or product-DB migration is needed for
  this optional read-only overlay. Missing or refused caption evidence leaves
  the old saved-folder presentation intact.

## Separate implementation verification

Raw-source admission is not runtime acceptance. The separate implementation
requires real-reader calibration on original member bytes, independently authored
registered malformed/ambiguous/unsupported/resource regressions, the actual
installed-product HTTP/save/reopen/undo path and every selecting web configuration.
The original-member reader probe is not original-package importer proof. Full
selected branch/main-first acceptance, source-bound server build/version and
publication/readback remain separate requirements.
