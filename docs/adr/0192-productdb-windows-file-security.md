# ADR 0192: Windows file-security requirements for ProductDB

Date: 2026-11-26
Status: Proposed
Session: WP-50 Slice 3a (docs only)

## Context

The owner chose a docs-only Slice 3a on 26 November 2026. This record proposes
Windows counterparts to the ProductDB filesystem mechanisms; it neither accepts
an implementation nor changes the Linux contract. Source inspection is against
`origin/main` at `fe6b30dc3`. No Windows build, execution, adversarial test or
native proof was performed for this slice. The historical cross typechecks in
[the research record](../research/2026-11-23-windows-cross-build.md) are not native
acceptance or Windows support.

[ADR-0011](0011-product-database-storage.md) retains manufacturer bytes;
[ADR-0117](0117-previewed-recoverable-catalogue-updates.md) requires preview-bound
updates and recovery; [ADR-0118](0118-catalogue-updates-through-the-web-server.md)
places catalogue access under the server owner. [ADR-0150](0150-cancellable-catalogue-jobs.md)
and its [managed-binding](../contracts/catalogue-jobs/managed-binding.md) and
[artifact contract](../contracts/catalogue-jobs/protocol-artifacts.md) require
owned copies, exclusion, publication and explicit uncertain settlement. This
proposal does not supersede those decisions or enlarge their support boundary.

### Current Unix mechanisms (source facts, not Windows equivalents)

Paths below are relative to the repository; symbols are the navigation anchors.
W is the live SQLite writer, S its separate source reader, D the rehearsal copy,
and B the before-state recovery copy. These are roles, not arbitrary file paths.

| Concern | Current implementation and its limit |
| --- | --- |
| File and parent identity | `crates/knx-productdb/src/managed_catalogue.rs::ManagedFileIdentity::{from_path,is_current}` records canonical pathname, parent device/inode and main device/inode/link count; later observations reject a non-regular main, link count other than one, symlinked main/parent or identity mismatch. `FileStamp::from_path` samples device/inode/link count, length, mtime and ctime with nanoseconds. These are pathname observations, not a retained directory capability or proof against arbitrary same-user swaps. Canonicalization can resolve links during initial admission. |
| Retained file anchor | `crates/knx-productdb/src/catalogue_copy/destination.rs::ArtifactInventory::create_new_observed` exclusively creates D/B with `create_new(true)` and mode `0600`, registers ownership immediately and retains the actual `File` before SQLite opens by path. `Destination::{check_identity,check_frozen}` compare pathname metadata with anchor device/inode, require one link and the original immediate parent identity, and compare frozen length/mtime/ctime. Checks surround close/reopen transitions; SQLite still reopens by pathname. |
| Private directory and ownership | `crates/knx-productdb/src/managed_apply.rs::ApplyWorkspace::create` creates a new directory with mode `0700`; `in_owned_directory` checks directory type, no final symlink and exact `0700`, but does **not** itself check UID or retain a directory FD. Caller ownership is a precondition. The server adapter below performs the root UID/lease checks. Neither Unix mode is a Windows ACL implementation. |
| Directory pinning and no traversal | `crates/knx-productdb/src/catalogue_copy/destination/publication.rs::ValidatedRecovery::publish_to` opens source/target directories using `openat(CWD, ..., DIRECTORY | NOFOLLOW | CLOEXEC)`, checks source-parent device/inode, and uses those FDs for `renameat_with`. This binds the rename to opened directory objects. `NOFOLLOW` here rejects a final symlink; it is not a component-by-component ancestor traversal guarantee. The earlier D/B allocation and several observations remain path based. |
| Lease exclusion and lifetime (caller boundary) | `apps/knx-server/src/catalogue_jobs/artifacts.rs::ArtifactRoot::acquire`, `acquire_lease`, `verify_lease` check effective UID/private `0700` root, create/open the stable lease with `NOFOLLOW`, check regular file, one link, `0600`, owner and path/FD identity, then acquire nonblocking exclusive `flock`. `ArtifactRoot::_lease` retains the file for its lifetime. This is server artifact exclusion, **not** a ProductDB-internal flock or a replacement for SQLite's locks. The root itself is retained as a path. |
| SQLite reservation and sync policy | `managed_catalogue.rs::ManagedCatalogue::{preview_current_controlled,apply_current_controlled_with_claim}` retain W's IMMEDIATE transaction while opening a separate read-only S by path, pin S with a real read transaction, compare full witnesses and header facts, and recheck W via `check_reserved_writer`. `crates/knx-productdb/src/catalogue_growth.rs::require_synchronous` refuses live apply unless `PRAGMA main.synchronous` is 2 or 3. SQLite snapshot pinning is not directory pinning, and these checks alone do not prove the Windows VFS reopen binds the intended file. |
| Publication, not live DB replacement | `ValidatedRecovery::publish_to` requires a closed validated B, rejects SQLite sidecars, syncs the file, then uses `RenameFlags::NOREPLACE` with no copy fallback. It updates retained ownership to the moved path and `PublicationUncertain` before further fallible work. `PublishedRecovery` is minted only after identity checks and file plus both directory sync acknowledgements. A collision never replaces another backup. Live catalogue changes use SQLite transactions, not file replacement. Server `catalogue_jobs/persistence.rs` separately publishes inventories/artifacts via rename; that caller contract also needs review before any Windows integration. |
| Failure and cleanup | `ArtifactInventory::{copy_and_close,validate,close_rehearsal}` retain failed native-close handles and primary plus cleanup errors. The inventory has no unlinking destructor. `ApplyWorkspace::release_closed_ownership` refuses open handles or failed rehearsal cleanup. `managed_catalogue.rs::MutationGuard::drop` quarantines uncertain/panicking mutations; `ManagedCatalogue::apply_current_controlled_with_claim` retains actual commit evidence. No failed close, rollback, rename or sync may be relabelled a clean cancellation. |

These facts do not assert blanket Unix race freedom or power-loss proof. The
managed-binding contract already excludes hostile same-user raw-file replacement,
broken/network locking and discontinuous observations. Rechecking a pathname
cannot close a time-of-check/time-of-use (TOCTOU) interval. A Windows counterpart
must preserve the documented protection and state its own admitted threat model;
it must not inherit a stronger claim from an API name.

## Decision

Propose a narrowly scoped ProductDB platform boundary for owned D/B files,
identity observations and recovery publication, with explicit caller contracts
for the server root/lease. Keep the existing Linux protection and SQLite
transaction/witness/health rules. Do not build a universal filesystem abstraction
or change other crates' backup, project-store or application path policies in
this ADR. Windows code and dependencies are deferred.

Admission is fail-closed: an unavailable primitive, unproved identity/ACL/root
binding, unsupported filesystem or uncertain publication refuses the controlled
operation before live mutation. No `cfg` branch may silently omit a check, accept
a pathname precheck as handle binding, return a dummy publication receipt, or
replace a failed durable operation with best effort. The following are proposed
requirements and research candidates, **not implemented equivalents**.

### 1. Bind handles, volume and the namespace actually used

Investigate local NTFS on Windows 11 x64 first; this is a candidate test scope,
not a support declaration. Network shares, FAT/exFAT, ReFS, cloud placeholders and
other redirectors remain unadmitted until their required behavior is independently
proved. Refuse unknown filesystem/API results rather than substitute length/time.

`CreateFileW` supports exclusive `CREATE_NEW`, directory handles using
`FILE_FLAG_BACKUP_SEMANTICS`, sharing policy and `FILE_FLAG_OPEN_REPARSE_POINT`
[M1]. The latter opens a reparse point itself; success does not mean that the
object is safe. Inspect the opened object and reject reparse points/tags outside
the admitted policy, including junctions and mount points. The docs do not make
this flag a proof that intermediate components were never traversed. A final-path
string, canonicalized prefix comparison or pre-open attribute check is not that
proof either.

Investigate `NtCreateFile` with `OBJECT_ATTRIBUTES.RootDirectory` for opens
relative to retained directory handles [M7, M8]. `OBJ_DONT_REPARSE` is documented
to fail on reparses encountered during parsing [M8]; its suitability through the
chosen user-mode API, Windows version and filesystem must be verified. These are
research leads, not permission to guess native ABI details. Prove secure initial
root acquisition, component handling, and binding through every child open,
SQLite reopen, publication and cleanup. Define which ancestor handles must stay
open and whether denying delete sharing prevents their rename/replacement for
the entire operation. A pinned child handle alone does not establish that the
configured path still names it. Ancestor replacement, intermediate-component
reparse traversal and SQLite/VFS reopening remain unresolved blockers.

Compare `FILE_ID_INFO`'s 128-bit file ID **with** its volume serial number [M2]
on actual opened objects; inspect `FILE_STANDARD_INFO.NumberOfLinks` [M3] and
require a regular, non-reparse, single-link file where Unix requires it. Define
the matching directory identity receipts as well. Hold handles for the whole
identity interval; file IDs are not a permanent global identifier or a new
catalogue incarnation. Windows timestamps are not a mechanical replacement for
Unix ctime; define conservative observation/ABA detection separately, retaining
same-W data_version, generation, full witness and refusal on lost continuity.
An ID check followed by a fresh path open is still a race.

### 2. Create private objects under an explicit SID/DACL policy

Propose a security descriptor at creation with owner SID bound to the intended
user token, an explicit DACL and protected inheritance. Read back owner, DACL and
control flags from the opened object, not a separate pathname. `GetTokenInformation`
and `GetSecurityInfo` are candidates [M4, M5]; the latter explicitly does not
handle concurrent security-descriptor changes. Default DACLs and inherited ACEs
are not equivalents to `0600`/`0700`: inheritable ACEs merge unless protected,
and absence of a DACL can allow everyone full access [M6]. Reject an unreadable,
null, broad or unexpectedly inherited descriptor; do not repair an existing
foreign root by chmod-like ACL changes.

The exact allowed rights, ACE forms and principal set still need design and
native proof: owner SID versus effective/impersonated token, directory traversal,
child deletion, WRITE_DAC/WRITE_OWNER, owner rights and group grants. Do not assume
that SYSTEM/Administrators are either automatically safe grants or containable
adversaries. Microsoft documents backup, restore and take-ownership privileges
that can bypass ordinary ACL restrictions [M9]; `BACKUP_SEMANTICS` also interacts
with backup/restore privileges [M1]. The future threat model must name privileged
principals and same-user limitations, and reject deployments needing guarantees
it cannot provide. A protected DACL alone does not settle this boundary.

### 3. Keep lease exclusion distinct from SQLite transactions

At the server adapter boundary, investigate `LockFileEx` exclusive plus
`LOCKFILE_FAIL_IMMEDIATELY` on a fixed range of a stable lease file [M10]. Retain
the verified handle for the complete root lifetime; no unlink/recreate lease
protocol, inheritable escape or reopening of a different object. Specify range,
sharing flags, synchronous/asynchronous completion and release behavior before
implementation. A pending lock is not an acquired lease. Prove contention from a
second process and second handle, handle closure and process-death recovery.
Microsoft says mapped views bypass these locks and OS release after termination
can be delayed [M10]. This mechanism excludes cooperating artifact owners; it
does not police all filesystem access or replace SQLite writer/read locks.

### 4. Publish the actual B; distinguish visibility from durability

Require same-volume, handle-bound **no-replace** publication of the closed,
validated, anchored B, with collision refusal and no copy/delete fallback.
Investigate `SetFileInformationByHandle` with `FILE_RENAME_INFO`: the documented
`ReplaceIfExists=FALSE` collision rule and relative `RootDirectory` are useful
candidates [M11, M12]. Prove their supported combination and namespace binding,
required access/sharing, observer visibility and failure states on the admitted
filesystem. Documentation of a rename entry point is not proof of atomic
visibility under every interleaving, nor power-loss durability.

If the server adapter later needs atomic **replacement** of its inventory or
manifest, specify that separately from backup publication and SQLite COMMIT.
`ReplaceFileW` uses pathnames, preserves/merges attributes including DACLs, requires
one volume and documents partial-failure states; its
`REPLACEFILE_WRITE_THROUGH` flag is unsupported [M13]. Never use ignore-ACL-error
flags to make publication succeed. `MoveFileExW` can copy/delete across volumes
and its write-through wording does not establish the required directory-entry
commit protocol [M14]. Neither API is automatically equivalent to the Unix
sequence.

`FlushFileBuffers` acknowledges flushing the specified file's buffered data;
Microsoft also describes volume flushing as privileged [M15]. The inspected docs
do not establish an unprivileged equivalent of syncing the file and both rename
directories with the durability required to mint `PublishedRecovery`. Identify
that mechanism and its storage/cache assumptions before admitting Windows live
apply. Do not silently reduce durable recovery to visible-after-rename. Keep
atomic visibility, OS flush acknowledgement, process-crash recovery and actual
power-loss durability as separate evidence classes. This slice proves none of
them on Windows.

### 5. Preserve ownership and uncertainty on every failure

Retain actual handles, object IDs, moved destination and native error state across
publication, SQLite close, cancellation and unwind. A rename followed by a failed
flush is publication-uncertain, not absent or rolled back. Inventory settlement
must classify what exists before cleanup; cleanup operates only on owned exact
identities, never broad pathname deletion. Preserve primary and cleanup failures,
quarantine unresolved ownership, and prohibit live mutation until the required
recovery receipt exists. Existing Linux receipts/health must not be weakened to
fit a Windows API.

## Alternatives considered

- Add `cfg(windows)` stubs, mode no-ops or existence checks to get a typecheck:
  rejected; removes the security property rather than porting it.
- Treat a list of Win32 APIs as accepted equivalence: rejected; root binding,
  ACL policy, SQLite/VFS behavior and durability are still unproved.
- Build a universal cross-platform filesystem framework now: rejected; widens
  scope beyond the ProductDB failure frontier without evidence.
- Resume cargo-xwin/SDK or use llvm-mingw as a substitute for native acceptance:
  rejected for this slice. The first route is closed; the second stopped at its
  unresolved Cephes notice. Docs-only is not an exception to that stop. See
  [the chronology and primary sources](../research/2026-11-23-windows-cross-build.md).

## Consequences

Porting waits for an owner-provided Windows 11 x64 machine or **new** VM with
MSVC. The existing analysis VM is excluded. Environment selection neither accepts
unread toolchain terms nor authorizes VM creation or installation; applicable
terms require separate inspection/acceptance. This proposal is not ADR acceptance,
Windows support, a completed port or authority for bus/hardware/release/deploy.

Future native acceptance must record the exact source/toolchain/OS/filesystem
scope and ordinary positive controls plus deterministic barriers/faults at each
open, reopen, rename, flush, close and cleanup transition:

| Required acceptance case | Adversarial or mutant control that must expose loss of protection |
| --- | --- |
| Private root/file creation and admission | Broad/null/inherited ACL, foreign SID, impersonation/elevated-token variants; remove the DACL/protected-inheritance/owner check and demonstrate the named failure. |
| Root/ancestor/leaf binding | Leaf symlink, intermediate junction/mount point, root/ancestor rename and replacement between observation and use, including SQLite reopen and cleanup; remove relative binding/reparse refusal and catch escape or refuse the platform. |
| File identity/continuity | Hardlink creation, equal-size/time replacement, volume/ID mismatch and lost-handle/reopen ABA; remove each relevant identity/link check and catch substitution. |
| Lease lifetime | Competing process and second handle, contention, pending acquisition, early drop, process death and attempted lease replacement; remove the lock or shorten its lifetime and catch simultaneous ownership. |
| Recovery publication | Target collision leaves existing bytes unchanged, cross-volume refusal, concurrent observers never accept a partial backup; remove no-replace or substitute a copy fallback and catch overwrite/partial visibility. |
| Flush and failure settlement | Faults before/after rename, each required flush, SQLite close and cleanup, cancellation/panic at barriers; omit a required acknowledgement and catch a falsely issued receipt or lost retained artifact. |
| End-to-end preservation | Controlled preview/apply/recovery and existing Linux security regressions remain valid; test Windows SQLite journal/sidecar sharing and restart behavior. Treat process-crash tests separately from any future authorized power-loss evidence. |

No tests or mutants in this table were run now. Source tests/mutants, Windows
checks and the package gate are outside this docs-only execution. Independent
review and the parent's package gate remain pending.

## Primary Microsoft documentation

Read via public documentation HTTP on 26 November 2026. These constrain candidate
mechanisms; they are not native test evidence.

- [M1 CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew): dispositions, sharing, reparse and directory flags.
- [M2 FILE_ID_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info): volume serial and 128-bit ID.
- [M3 FILE_STANDARD_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_standard_info): link count/type/delete-pending facts.
- [M4 GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation): token information and access requirements.
- [M5 GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo): handle-based owner/DACL reading and race caveat.
- [M6 DACL for a New Object](https://learn.microsoft.com/en-us/windows/win32/secauthz/dacl-for-a-new-object) and [Security Descriptor Control](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-control): inheritance, defaults and protection.
- [M7 NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile): relative opens and reparse behavior.
- [M8 OBJECT_ATTRIBUTES](https://learn.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-_object_attributes): RootDirectory and OBJ_DONT_REPARSE.
- [M9 Privilege Constants](https://learn.microsoft.com/en-us/windows/win32/secauthz/privilege-constants): backup/restore/take-ownership privileges.
- [M10 LockFileEx](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex): range exclusion, completion, mapped views and release.
- [M11 FILE_RENAME_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info): collision and relative-target fields.
- [M12 SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle): information classes and restrictions.
- [M13 ReplaceFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew): ACL merge, unsupported write-through and partial failure.
- [M14 MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw): copy fallback and write-through scope.
- [M15 FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers): file flushing and privileged volume flushing.
