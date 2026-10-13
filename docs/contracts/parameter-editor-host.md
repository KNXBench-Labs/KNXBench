# Parameter editor host handoff (AP3 bounded subset)

The shared ParameterPanel/workspace/navigation mount belongs to Session1.
This owner provides `features/parameter-editors/ParameterEditor.tsx` and an
optional read-only source endpoint. The existing `/api/device/{id}/parameters`
read/write handler remains authoritative for values, write admission, dependent
sections, history and native recovery. Nothing in the source endpoint grants
editability or creates a new parameter write policy.

## Source producer contract

`GET /api/device/{id}/parameter-editor-source?etsId=<encoded exact ParameterRef>`
uses the existing authentication middleware. It does not accept `programId`,
source bytes, module aliases or another client-selected catalogue identity.

Response (camelCase):

- `device`, `parameterRefId`: exact requested device/reference;
- `serverIncarnation`, `projectIncarnation`, `snapshotRevision`: captured project
  context rechecked after the ProductDB read;
- `evidence`: either `{status:"declared",source:...}` or
  `{status:"unavailable",reason:...}`.

Declared `source` includes `sourceSha256`, `programId`, `parameterTypeId`,
`schema`, `child`, `attributes`, `parentAttributes`. Attribute keys preserve
qualification. This is source evidence, not a proof of a colour/image/time codec.
Unavailable reason is one of `missingProductDatabase`, `missingProgram`,
`missingReference`, `missingSource`, `sourceBudget`, `sourceIdentity`,
`unsupportedXml`, `ambiguousDefinition`, `modelMismatch`, `missingDeclaration`.
Database failures stay HTTP500; project drift stays409. A missing project/device
or empty/over1024-byte reference is400. The response is inspection-only and
source interpretation is bounded to direct Static definitions/namespaces20–23.

Read per explicitly selected candidate field, not once per row of a large
project on every render. Admission is bounded but parses a retained source; this
package does not introduce an unmeasured cache or bulk-query abstraction.

## Component binding

1. Fetch the existing authoritative parameter panel and optional source response
   under the existing authenticated request mechanism. Keep exact evaluated
   scope, device, ref, source/program and current project identity; a late source
   response must not be merged into another selection/revision. Unknown
   unavailable reasons are visible generic fallback, never successful evidence.
2. Project the real field into `ParameterEditorField` without changing `value`,
   `min`, `max`, `access`, `editable` or `writeEtsId`. Do not manufacture authority
   from a source hint. Preserve null versus explicitly empty strings.
3. Project declared source to `ParameterEditorSource`: copy the source fields,
   add the **response** `parameterRefId` (there is no invented source-inner ref
   field), and keep `parentAttributes` distinct from child attributes. Verify the
   response device/ref/context against the current request and panel program.
4. Pass `context.programId/sourceSha256/moduleScoped`, stable exact
   `selectionKey` including project/device/module, and `revision` encoding the
   authoritative server/project/snapshot identity. Do not derive a revision from
   browser time, local counters or a human label. Module scopes use explicit
   unsupported fallback until separately audited mapping exists.
5. `onApply(writeEtsId,raw)` invokes only existing `setParameterValue` using the
   exact device/write target and original raw draft. `onApplied(result)` publishes
   the **actual** returned panel and project tree through the current owner.
   Do not locally predict dependency changes or refresh only the edited number.
6. Host must keep an outstanding mutation tracked even if selection unmounts the
   editor, and serialize writes appropriately. Suppressing a late UI callback is
   not cancellation/rollback of an already sent legacy HTTP mutation. This
   package does not claim revision-conditional multi-client parameter writes.
   **Amended 2026-10-13 (Goal A2):** the write route now accepts the binding
   ([revision-bound writes](revision-bound-writes.md)); the host's `revision`
   is exactly its three context fields and `expectedValue` is the field's
   received `value`. Adopting it in the host is Goal B1.
   **Amended 2026-10-13 (Goal B1a):** adopted. The host binds to the shown
   panel's own snapshot (`writeBinding`), a conflict reloads tree and panel
   and leaves the leaf's draft stale, a failed reload is named
   ([web consumer](revision-bound-writes.md#web-consumer-goal-b1a-13-october-2026)).

## Supported presentation and refusals

- Time: decimal integer count, not a date/clock/duration conversion. Declared
  `Unit` is visibly uninterpreted. The exact lexical value is not rewritten by
  selection/render/no-op. Integer draft comparison is exact and bounded to64
  digits; server validation may still reject a value outside its own i64/type
  contract, with its original refusal displayed and draft retained.
- Number Slider: explicit unqualified child `UIHint="Slider"`, `Type` is
  `unsignedInt`/`signedInt`, declared bounds and current value are valid safe
  integers, unit step only. No Increment, unknown transformations, rounding or
  automatic clamping. Editing the slider changes a draft, not a project value.
- Narrow child attribute admission: `SizeInBit`, `Type`, `minInclusive`,
  `maxInclusive`, `Unit`, `UIHint`; parent `Id`, `Name`, `UIHint` and XML namespace
  declarations. Other attributes cause a visible unsupported fallback.
  Qualified vendor hints are not promoted to unqualified KNX hints.
- Colour/Picture: visible unsupported-codec original-value view. No HTML/image
  execution, URLs/resource fetches, colour conversion or file upload. The host
  may keep its existing generic editor; it must not label this fallback a colour
  picker or image editor. Full PDB-04 remains open for verified value contracts.
- Read/None access, missing write authority, invalid drafts and no-ops cannot
  invoke Apply. Revision changes preserve the draft but block Apply until an
  explicit discard/reload when the field changed; with an identical field (Save
  bumps the revision) the draft moves onto the new revision, a draft equal to the
  new authoritative value settles, and an editor without a draft follows the
  project. A revision that arrives during the editor's own write is never adopted
  (amended 2026-10-12). Selection/source changes reset the isolated editor;
  late outcomes are not republished into the new context.

## Evidence scopes

Synthetic tests exercise exact-source admission and actual ProductDB
close/read-only-reopen; real HTTP tests exercise source/authentication/refusals
and existing parameter validation, one undo step, undo/redo, native Save/Open,
retained source bytes and unchanged files on refusal/read. The separately built
Chromium fixture exercises EN1440 Graphite/DE400 Porcelain, explicit callbacks,
keyboard slider drafts, stale state/refusals and no external requests. Its
callbacks are synthetic, so it is **not** full browser-to-native integration,
production shell, physical device, ETS reimport, Orca or WebKitGTK certification.
Final exact-source receipt is recorded separately; intermediate counts do not
certify a changed assembled candidate.

## Workbench host (12 October 2026)

The device editor's **Parameters** tab offers a **Checked editor** button under
Time/Number/Color/Picture rows when the tree carries a complete identity
(`snapshotContextFromTree`) and the panel has a program; not in
**Manufacturer fields** or **Diagnostics**. `ParameterSourceEditor.tsx` replaces the
row (never nesting controls in its `<label>`), reads the source per opened field,
and `parameterSourceHost.ts` admits only a well-formed answer for the same device,
reference, panel program and snapshot; other answers are "malformed"/"mismatch" and
the leaf gets no source. The source is re-read on each new revision while the
previous answer stays on screen, so the leaf (keyed on selection, reference,
program, source digest and scope) keeps its draft. `selectionKey` is
server/project/device/scope; `revision` is server/project/snapshot.

Writes use only `setParameterValue`. The host publishes the returned panel and tree
inside `onApply`, so a write that finishes after the editor closed or after another
revision arrived is still published (item 6); the leaf's `onApplied` is then a
notification only. `useDeviceParameters` no longer blanks the panel on a pure
revision refresh, only on another device or product language, so an open editor
survives Save/Apply/Undo. Evidence: unit tests with named mutants, 11 intercepted
production-build Chromium cases and a loopback-only real-server smoke importing a
synthetic product with retained source; see
[IMPLEMENTATION_STATUS](../IMPLEMENTATION_STATUS.md). Self-review only. Module
mapping, verified Color/Picture codecs and PDB-04 remain open.
