# WP-04.3 consolidated design and readiness boundary

Status: **architecture approved for staged implementation** (20 October 2026),
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md). Both final review
halves return DESIGN READY after scoped correction rereviews. Their original
REVISE verdicts remain retained. This parent approval composes the two reviewed
halves; no new service, route, parser control, artifact manager, validator or
desktop hook is implemented by the documents. All proposed production controls
remain unexecuted. The private A1 primitive and construction-disabled A2 kernel
are delivered with their own focused and maintained gate evidence. The private
B1 schema-admission helper is now delivered with independent I1 closure and
accepted branch/integrated gates plus publication readback; see the
[definitive delivery record](../../IMPLEMENTATION_STATUS.md#wp-043-b1-schema-admission--source-delivered-21-october-2026).
This is
not production job, binding, installer or full Status acceptance. See the
[A2 delivery record](../../IMPLEMENTATION_STATUS.md#wp-043-a2-admission-kernel--delivered-21-october-2026).
ProductDB schema and ordinary APIs are unchanged.

## Normative document set

- [Job/consent/UI contract](../catalogue-update-jobs.md): R1 Ready/apply/expiry,
  R2 App-owned job lifecycle, R3 bounded replay and M1 growth-cliff policy.
- [Internal A2 admission decisions](control-admission.md): independently reviewed
  construction-disabled kernel scope, exact replay/clock/counter choices and
  future producer/full-Status boundaries; no callable production Ready factory.
- [B1 schema-admission decision](schema-admission.md): fresh-chain runtime oracle,
  private bounded descriptor admission and explicit unimplemented boundaries;
  independently accepted for the private incremental-TDD slice only.
- [Managed binding/transaction supplement](managed-binding.md): concrete owner,
  health/generation/incarnation, every existing server mutator, complete typed
  streamed witness, support refusals and explicit same-W transaction settlement.
- [Protocol/resource/artifact/lifetime supplement](protocol-artifacts.md): exact
  transport/state semantics, pre-allocation admission, safe complete report
  projection, filesystem ownership/lease/publication, stop and desktop drain.
- [Wire definitions](wp043-protocol.schema.json),
  [record definitions](wp043-artifact-records.schema.json),
  [internal artifact definitions](wp043-artifact-internal.schema.json) and
  [cross-field policy](wp043-protocol-policy.json): static exact design artifacts,
  not generated production bindings or validator execution.
- [Schema-28 source inventory](schema28-source-inventory.json): static fixture
  seed, not an inspected runtime database or an instruction to create a schema.
- [Primary research and scoped mechanics evidence](../../research/2026-10-20-catalogue-job-transactions.md).

These documents are one contract. JSON Schema fixes field/null/closed-enum
structure; policy/prose impose the cross-field, byte, duplicate-key, numerical,
state, ownership and publication rules not expressible by schema alone. A
conflict is a design refusal requiring correction, not permission to choose the
weaker interpretation. Delegated metadata such as `scope-ready-design-only`,
`externalDependency` or `acceptedAdr:false` describes that artifact's original
scope and is not a runtime field. The managed-binding dependency now has a
concrete companion here; whole-contract design approval is recorded above.
Runtime tests and delivery acceptance remain separate.

## Cross-scope integration decisions

1. **One managed owner, not new parallel SQL authority.** The existing server
   connection slot becomes a narrowly wrapped managed state. All actual server
   mutators, ordinary restore/reopen and health/quarantine cooperate through it.
   Read adapters cannot retain the connection. Existing quiet public/CLI APIs
   and synchronous wire shapes stay unchanged; their path-based independent
   writers remain external SQLite writers, not secretly same-W transactions.
2. **Full state and continuity, not package-set equality.** Ready retains the
   original source/name/numeric profile/policy/raw preview identity and the
   original owner stamp. Apply compares it under IMMEDIATE on persistent W,
   before incrementing generation for its own admitted mutation. Same-W
   generation and same-connection data_version are complementary. Full typed
   row/schema witness includes every admitted table/column/hidden rowid and
   actual source bytes, not merely their claimed hashes. Conservative stale is
   acceptable; resetting a baseline to make it equal is not. Unsupported schema
   and native observation uncertainty refuse explicitly.
3. **Initial support is deliberate and narrow.** Existing managed schema 28,
   UTF-8 STRICT rowid-table shapes, local Linux default VFS/private cache/normal
   locking, DELETE journal, W synchronous FULL/EXTRA before BEGIN, no attached
   databases/application TEMP/recovery uncertainty. No migration, CREATE-live,
   journal conversion, alternate profile or raw-file replacement to obtain job
   admission. Managed restore invalidates before touching the live handle;
   arbitrary external open-file rename/alias/recovery is unsupported. This does
   not claim protection against a full finite native counter cycle or hostile
   same-user filesystem mutation.
4. **Identity primitive is explicit infrastructure.** A tiny private audited
   ProductDB adapter may call the pinned `Connection::handle` and
   `sqlite3_file_control(main, SQLITE_FCNTL_HAS_MOVED)` with exclusive live
   connection ownership and a valid out-parameter. This is an explicit narrow
   unsafe-FFI boundary, not guessed SQLite internals or a new connection API.
   It must document pointer lifetime/callback rules, refuse unsupported VFS/error
   and have focused checks before use. No broad unsafe allowance, external FD
   extraction or new dependency is authorized. std/rustix handle filesystem
   ownership. Successful HAS_MOVED alone does not replace path/stamp/continuity.
5. **W/S/D roles never merge.** W owns the sole live reservation/commit. S is a
   distinct READ_ONLY/private-cache source pinned after reservation by an actual
   read. Fresh equal-page D and recovery B are copied from S's pre-DML snapshot,
   require observed Done and no destination SQL before dropping Backup. Close S
   before W writes/COMMIT. Live Ready/apply keeps exact schema_version in
   CatalogueWitness/1. Completed D/B uses the same complete payload under
   CopyContentWitness/1, excluding only that cookie, with independently recorded
   fresh destination cookie 0 and observed completed cookie 1. Do not rewrite
   cookies or substitute a copy witness for live stale checking. Rehearsal may
   commit only its private D. Whole raw
   source/member/native copies remain disclosed; disk rehearsal is not a claim
   that the parser became streaming or RSS-bounded.
6. **Caps apply to actual allocators.** Pinned page-count admission bounds the
   fresh equal-page copy; max_page_count alone does not. Reassert/read back D
   growth cap after copy and W's cap before live DML. No source/member budget
   widening, truncation or quota fiction. The same per-job result budget covers
   preview/result raw/public/index/manifests, with one logical record per raw/
   public pair and actual 32-byte indexes. Rehearsal repeats into a comparator,
   not a second retained preview. Live result must match fully admitted evidence
   before commit claim. Every new String/map/vector/report clone is admitted
   first; native SQL and existing parser allocations remain a separate scope.
7. **Commit evidence outlives cleanup.** Transaction uses explicit settlement
   and DropBehavior::Ignore with an armed quarantine-on-unfinished owner; no
   hidden retry or discarded Drop result establishes rollback. Only COMMIT OK
   witnesses committed. The demonstrated Busy/active/explicit-rollback path can
   witness notCommitted; other ambiguous commit failures remain unknown until
   specifically proved. Scoped settings restoration and artifact settlement
   cannot erase committed evidence. Unknown retires/quarantines all catalogue
   writers and retains recovery data; no automatic reopen/retry.
8. **Artifact visibility is owned and acknowledged.** Existing rustix fs gives
   stable flock lease and NOREPLACE rename, not a new generic framework. Private
   root/read/write/static containment is enforced before routes become reachable.
   Application-created files/dirs are exclusive and identity-bound; manifests
   are published last. SQLite-created rollback journals are a narrow native
   exception: exact private basename, absence/owned-state admission before use,
   no interference while native handles live, then no-follow post-close adoption
   of actual surviving identity or quarantine. Unknown crash survivors are not
   adopted by suffix; live/source journals are never cleanup-owned.
   Completed recovery copy is validated, no-replace published in existing backup
   grammar and file/directory sync acknowledged before live install. Incomplete
   files stay hidden. Only exact owned transient files can be reclaimed; final
   committed/unknown backups remain user-managed. Sync acknowledgements are not
   universal power-loss guarantees. Quarantine remains charged and stops fresh
   admission even if its worker has ended.
9. **Resource and binding refusals map explicitly.** ProductDB typed
   `WitnessSchemaAdmissionLimit`/`UnsupportedSchemaShape`, unsupported settings,
   native observation support or recovery state map to public
   `unsupportedCatalogue`. Input page excess maps to `catalogueAdmissionLimit`;
   growth to `catalogueGrowthLimit`; complete-result/record admission to
   `resultLimit`/`recordLimit`; unsafe required projection to
   `publicProjectionRefused`. A differing known Ready stamp is
   `catalogueUpdateStale`; unresolved integrity/I/O/commit is not mislabeled
   stale or clean cancellation. Original internal cause/settlement remains
   retained separately from fixed public error kinds.
10. **One lifecycle owner across layers.** Atomic Ready/apply/expiry/stop and
    cancel/commit gates, exact bounded serial/revision policy, retained worker/
    receive/detail-reader ownership and explicit drains are shared by standalone
    and embedded lifetimes. App-level controller is above actual wizard/catalogue
    unmounts, with selection-local generation separate from commit invalidation.
    Strict DTOs must admit every intermediate and terminal combination; casts,
    status-only polling or a toy gate do not establish the production contract.
    Ordinary Destroyed/ExitRequested notifications all coalesce into one owned
    begin-close; no native event count or exit code authorizes force. Only a
    separate explicit force action bound to the active close generation (or
    explicit headless second stop) can signal escape. Remove Destroyed's old
    pre-drain AppHandle.exit enqueue; set exitAuthorized before final exit.
    Existing device-write shutdown safety is preserved, not replaced by this
    catalogue cancellation design.

## Accounting conventions

Static schema source gives 46 application tables, 24 explicit indexes and 50
inferred automatic indexes: **120 stored sqlite_schema entries**. The complete
accounting universe is **121 including the explicitly scanned sqlite_schema
relation**, not 121 stored entries. There are 47 scanned table relations when
including that relation. Runtime schema introspection and accepted migration SQL
variants must be verified before use; count equality is never schema validation.
The 14 protocol state branches and 17 flat report variants are declarations,
not product tests or proofs that clients enforce semantic annotations.

## Execution evidence already available — limited scope

A standalone, checksum-bound rusqlite 0.40.2/libsqlite3-sys 0.38.2 synthetic
DELETE-mode harness observed SQLite 3.53.2 and its actual source identity/options.
Five mechanics cases were independently reconciled: same-W backup returned Busy;
distinct pinned S copied D through 91 More + one Done; another process failed
IMMEDIATE while W held reservation and acquired after release; aborted partial
D was disposed with exact live logical contents unchanged; held S made COMMIT
Busy/active and explicit ROLLBACK restored the before-state. Fifteen synthetic
row-byte witnesses, native binary identity, eleven lockfile-bound archives and
1,098 members were authenticated. This did not run ProductDB schema 28, its
installer, ownership/witness, cancellation gate, quotas or production lifetime.

Original harness compile failure and first cross-process controller failure
remain retained; the first owner did not record/reap its writer's OS exit.
Later PID absence is not retroactive correct ownership. Only the fresh successful
cross-process controller records both children exited zero and reaped. This
qualification remains in evidence; no failed attempt is relabeled a product pass.

## Implementation order after independent readiness approval

- **A — gate/outcome primitives:** tests before implementation for both actual
  gate winners, Ready boundary/delayed reaper/stop and exact settlement enums.
  No route, installer success stub or cancellation capability is exposed.
- **B — managed snapshot/binding:** schema-28 fixture oracle, streamed codec,
  owner generation/health and actual mutator adapters, W/S/D copy, source/policy
  binding and artifact ownership. B alone grants no Ready/install consent.
- **C — one transitive installer:** typed Stop/context through current wrapper,
  both ZIP views, every read/hash/decode/parser skip/identity/classification/
  history/report seam; quiet wrappers preserve exact old behavior. Resource
  admission and complete artifact comparator must be wired, not added afterward.
- **D — owned job/lifetime API:** strict DTO/admission, upload/spool, status/cancel
  outside database mutex, lease/reclamation/fencing and real standalone/desktop
  close/drain. Do not enable production routes until all their earlier invariants
  are exercised.
- **E — shared App-root UI:** actual catalogue/wizard mounting, EN/DE error and
  withheld-evidence disclosure, progress/consent/background/refresh, legacy path
  after only eligible known refusal and settlement. Browser tests cover both real
  parents, not two copies of a leaf. No fallback for unknown or resource refusal.
- **F — complete reviewed delivery:** fault/barrier and compatibility controls,
  compiled critical mutants, maintained scoped branch/main-first integrated
  gates, publication/readback and owned evidence archival/retirement.

The design may be approved before these tests execute; that approval authorizes
only implementing the stated tested stages, not claiming their runtime behavior.
A discovered source/runtime contradiction blocks its dependent capability until
corrected and reviewed. No production acceptance, completed WP-04.3, WP-19,
release, hardware admission or expanded ETS compatibility is implied.
