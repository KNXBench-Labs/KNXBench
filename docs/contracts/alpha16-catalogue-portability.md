# Alpha 16 catalogue-authored native source ownership

## Acquisition unit and authority

Save and Save As acquire source dependencies for resolvable device product and
Hardware2Program references that are not already covered by the saved source
manifest. Authority is the installed **winning source hashes**, not an order
number, filename, application version family or current catalogue selection.
Product, hardware, Hardware2Program and application must agree on their hardware
relationship and, for new acquisition, have measured, unique winning declarations.
Explicitly programless hardware needs product/hardware sources, without inventing
an application mapping. Divergent or unmeasured new declarations are refused
rather than picked by ordering.

A new acquisition requires exactly one recorded installed package containing all
these winning sources. Package membership is read from the retained package
index; missing historical membership is not reconstructed. Several candidate
packages are ambiguous even when they share source bytes. Existing manifest
coverage is not rebound to another package on later saves.

The native file carries that package's **exact original bytes**, SHA-256 and a
`CataloguePackage` manifest entry at `catalogue-packages/<sha256>.knxprod` (a
native-owned storage key, not a claimed historical filename). It also records
and carries the exact direct source members and the package's master and
matching catalogue source members. Whole original bytes are the conservative
closure for baggage, signatures, unknown content and other package-internal
requirements: they are not rewritten, selectively rebuilt or claimed to be
independently understood. Unrelated installed packages/the global database are
never embedded. An original may inherently contain additional products; those
bytes remain part of the original, not a newly manufactured export.

Every acquired payload is length-bounded before materialization and SHA-256
checked. Required member paths/lengths/bytes are also checked against the retained
original ZIP, not merely trusted because an index row names them. The existing
retained-value and total expanded-archive ceilings apply;
no format/schema version changes. A missing/corrupt original or required member,
unmeasured identity, inconsistent hardware relationship or ambiguous package
refuses acquisition **before destination publication or clean-state updates**.

## Save, history and reopen

Acquisition joins the project-led save snapshot. Newly acquired manifest entries
are part of the history source context; retained payloads remain outside history
images. Native history/root changes and newly acquired payload insertion share
one SQLite transaction, including stale-generation admission. Save As constructs
its new candidate with the payloads before publishing it. In-memory manifest,
clean baseline, save location and revision advance only after save succeeds.
Undo/redo changes the model, not the retained payloads. Imported opaque entries
and existing manifest entries are never replaced or trimmed by acquisition.

On server/desktop reopen, carried `CataloguePackage` originals install through
the existing package installer **before** individual source replay. That restores
master data, package membership and opaque baggage through the normal validated
installer. Failure is reported, not converted to a successful reconstruction.
Individual legacy manifest entries retain their existing recovery behavior.
The source hashes identify bytes, not manufacturer authenticity or signatures.

## Explicit compatibility boundary

Already manifest-backed imported projects retain their previous save/open
behavior, including named missing-file warnings. Unresolved historical devices
can also have an **empty** manifest: the importer retains their declared product
and program references even when the archive contains no manufacturer data.
Save preserves those references and all retained opaque entries, and reports
unavailable ownership / unestablished portability; it does not invent a package
or treat an empty manifest as complete coverage. The same per-device rule applies
when another device has acquired a `CataloguePackage` in a mixed project.

Native catalogue creation records the paired source path and ETS ID
`KB-DEV-<canonical nonzero u32>`. That positive authored identity survives local
ID remapping; an imported ETS ID alone is not that identity (the import mapper
keeps the project XML member as its source path). Authored devices still require
their mapping even when an unrelated legacy manifest exists. For **resolvable**
dependencies, either the manifest already covers their winning hashes or all new
acquisition checks above apply, regardless of source identity. Parameter fixtures
that only ingested loose XML therefore need real installed synthetic originals,
not an exception to package ownership validation.

Already-covered historical hashes need not acquire new identity evidence. New
acquisitions support measured modern XML `.knxprod` sources; legacy or incomplete
ownership is not inferred from filenames. With no
product database, Save/Save As preserve the existing manifest and use carried
fallbacks; they cannot acquire a new catalogue source, and explicitly report that
ownership was not checked. Native-authored dependencies can use this offline
fallback only when the same native dependency was in the explicit saved baseline
and its manifest contains a previously acquired `CataloguePackage`. A recovered
unsaved working device is not part of that baseline; another saved package is
not authority for its new dependency. An unbacked authored
project without resolvable sources is refused. Old files with no original or
missing source bytes cannot be made portable by this feature alone; restore or
reinstall matching originals first. CLI read-only consumers do not auto-install.

The contract does not promise independent sources for every retained historical
version, arbitrary project export, or authentic manufacturer package provenance.
Focused implementation regressions exercise real HTTP catalogue creation,
Save As, acquisition on plain Save, empty-database reopen, exact original bytes,
master/baggage replay and offline Save As. The same HTTP module refuses missing
originals/membership/mappings, corrupt originals/sources/member paths, ambiguous
packages and divergent declarations on both Save and Save As, checking unchanged
model, destination, manifest, revision, generation and clean/save metadata. It
also pins per-file/closure limits and read-only acquisition. Existing imported
project recovery cases remain in that module.

- [HTTP regressions](../../apps/knx-server/tests/it/http_knxdb_product_data.rs)
- [Native generation/partial-source rollback and opaque-baggage regression](../../crates/knx-store/tests/it/history_compare_and_save.rs)
- [Programless source-acquisition regression](../../crates/knx-productdb/src/catalogue_sources.rs)

These focused results are not complete release, large-database, CLI replay or
independent-review acceptance.
