# Saved schema-20 node text: bounded display admission

## Scope

This admits only a read-only display of the literal `Node/@Text` retained in an
imported schema-20 project's saved `GroupObjectTree`. It does not establish the
manufacturer's current folder text, translation, activation, template semantics,
or folder placement after edits. This is a WP-06 increment, not full WP-06 or
ETS compatibility acceptance. Implementation and runtime acceptance are pending.

## Primary observation

Read-only acquisition on 5 November 2026 from the producer's public test fixture:

- Repository: <https://github.com/XKNX/xknxproject>.
- Commit: `f97db068884eca2b460abb1fb4db2262345f7f47`.
- Path: `test/resources/module-definition-test.knxproj`.
- Original package: 10,376,392 bytes, SHA-256
  `8966d0fcde22a0ca8e133deddb9873e82f7ee1fee93195963787085143e7675b`.
- Topology member: `P-0810/0.xml`, explicit namespace
  `http://knx.org/xml/project/20`.
- Independent XML walk: 274 `Node` elements, 246 `Folder` and 28 `Channel`;
  all 274 declare an unqualified `Text`. Module and non-module references both
  occur. Some texts contain unresolved `{{ChNo}}` markers.

The archive and topology were read without extraction or manufacturer-code
execution. Original input bytes remain outside the repository; no upstream
implementation or vendor XML is copied into tests. The original public source
is inspected as format evidence, not redistributed. The separate public fixture
`xknx_test_project_no_password.knxproj` at the same commit has no `Node` and is
not a positive text witness. These observations agree with the earlier
[schema-20 inventory](project-format.md#36-schema-20-ets-57-and-schema-22-ets-61-public-samples-2026-10-12-v),
but are a fresh, independently counted source observation, not a defining
schema specification or an ETS rendering comparison.

## Admitted contract

- The owning project's recorded ETS schema must be exactly 20. An explicitly
  namespaced retained tree must also be exactly project/20; a wholly unbound
  retained fragment uses that owning project's schema. Mixed namespaces, foreign
  ownership, corrupt retained hashes and all existing structural refusals stay
  refused. Explicit other-schema `Text` does not acquire display authority.
- Only the exact unqualified `Text` attribute is display evidence. Preserve its
  XML-normalized string, including explicit empty and whitespace values. Missing
  and qualified lookalikes remain absent. At more than 4,096 UTF-8 bytes, withhold
  only the display text and fall back to the declared ID; the original remains
  retained unchanged. Never truncate it or infer a replacement caption.
- Carry this as an additive optional saved-text field. Missing legacy JSON is
  accepted. This changes no domain model, project schema, source blob, migration,
  command or protocol behavior.
- Identity remains the complete `Type`/`RefId` ancestry. Duplicate-path detection
  and opaque grouping keys must ignore text, including when duplicate paths have
  different captions. No caption matching, module resolution or ID fabrication.
- Existing module, inactive, ambiguous and unidentified refusals expose no text.
  A label cannot turn a root/channel-only placement into a folder.
- Show the saved folder text alongside its ID; keep the complete saved ID path
  visible and the existing import-time-state note. Use plain React text, never
  HTML or template substitution. Empty or whitespace-only text falls back to the
  ID without modifying its retained/projection value. Search may include admitted
  folder text but must retain ID matching and refusal boundaries.

## Required verification

Registered app/server/projection tests must cover exact source/version/namespace
admission, qualified/missing/empty/entity/oversized text, duplicate identity with
unequal labels, all existing refusals and unchanged keys. Real handler tests must
import an independently authored synthetic schema-20 project using schema-20's
direct Line/DeviceInstance structure, save/reopen and edit/undo it, checking the
same text, identity, retained bytes/hash and relevant history at each boundary.
Legacy schema-23 behavior remains an independent unchanged control.

Web tests must cover literal and HTML-looking/template-looking text, ID-visible
fallbacks, search and grouping-key stability. Intercepted real-browser cases
must exercise the existing table in both supported interface languages, narrow
and wide geometry, and reject unexpected requests. Source text is not translated.
Independent whole-feature review and the complete maintained selected package
gates, main-first integration, publication/readback and owned cleanup remain
required. No hardware, bus, native GUI, VM, release or deployment is authorised.
