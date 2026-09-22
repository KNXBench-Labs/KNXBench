# UI Residue Batch B Design

**Date:** 2026-09-22

**Status:** Approved by the user's standing instruction to approve comparable
designs and plans automatically.

## 1. Purpose and scope

T13 closes four independent, user-visible correctness gaps without moving their
invariants into the UI:

1. §§81/103 receive an honest project-modified signal instead of using undo
   availability as a proxy.
2. §91 keeps an active bus session's group-address context current when the
   open project's address style changes.
3. §89 represents all five additional `Space/@Type` tokens documented by KNX
   Project Schema23 instead of reporting and coarsening them.
4. §120 adds a deterministic contrast gate for the theme roles whose pairing is
   already defined by ADR-0022.

The web half of §60 is excluded. Its required T15 crate/API report does not
exist, so this batch must not guess a diff shape or extend `knx-diff`.

The four slices share one integration branch and final review, but each has its
own RED/GREEN cycle and focused commit so it can be reviewed or reverted
independently.

## 2. Project modification state (§§81/103)

### 2.1 Invariant

`is_modified` means that the normalized user-visible project differs from the
last successfully established clean baseline. Native open, ETS import, and new
project creation establish a clean baseline. Successful Save and Save As replace
the baseline with the exact project state that was saved. Failed saves do not.

The comparison ignores only `IdAllocators` high-water marks. Those counters are
internal collision-prevention bookkeeping: allocating an ID before applying a
command and then undoing the command must return the user-visible project to a
clean state even though the allocator remains advanced. Every other current and
future `Project` field participates by default.

### 2.2 Ownership and API

- `knx-core` makes `Project` and `StringTable` cloneable and exposes a narrowly
  named content comparison that normalizes allocator state before using the
  existing structural equality implementation.
- `knx-server::AppState` owns `clean_project: Mutex<Option<Project>>`. It is
  transient session state like undo history, not serialized into `.knxdb`, so
  the native store schema remains version 9.
- Project replacement publishes project, clean baseline, command stack, import
  counts, and store-path metadata under one documented lock order. A tree read
  observes one complete state, never a mixed replacement.
- `ProjectTree` adds `is_modified: bool`; pure projection defaults it to
  `false`, and the server overlay supplies the live value beside `can_undo` and
  `can_redo`.
- The server's new-project guard and the desktop quit prompt consume
  `is_modified`. Undo and redo controls continue to consume history availability.

No client-maintained dirty flag, command-stack cursor, or manual `mark_dirty()`
calls are introduced. Snapshot comparison catches direct mutations outside the
command stack and returns to clean after undoing to the baseline.

### 2.3 Required proof

- Edit then undo to the baseline: `can_redo == true`, `is_modified == false`.
- Direct project mutation with an empty undo stack: `can_undo == false`,
  `is_modified == true`, and new-project replacement is refused unless discard
  is explicit.
- Successful Save and Save As clear modified state; failed saves retain it.
- Native open, import, and new project establish clean state.
- Quit confirmation follows `is_modified`, including disagreement with
  `can_undo` in both directions.

## 3. Live group-address session context (§91)

Slash notation remains fixed and has no setting. This slice concerns the KNX
project's `Free`, `TwoLevel`, or `ThreeLevel` address style used to interpret and
format values, not slash-versus-dot presentation.

`BusSession` replaces its immutable `GroupAddressContext` with a shared,
short-held synchronization cell used by both the drain task and write path.
Accessors read the current context; a new update method atomically replaces the
whole context. Updating the whole value keeps style, group-address names, and
DPT resolution coherent and avoids a second partial-refresh mechanism.

After `SetGroupAddressStyle` succeeds, the HTTP application layer builds a fresh
`GroupAddressContext` from the current project, releases the project mutex, and
then updates an active session under the bus-session mutex. Project and async bus
locks are never held together. The operation performs no KNX traffic and does
not restart or reconnect the tunnel.

The acceptance regression starts a fake session, changes project style through
the public route, then proves subsequent monitor rows and write parsing use the
new style. Existing no-project/raw behavior and session-start behavior remain.

Rejected alternatives:

- Reformatting only in React leaves server write parsing stale.
- Restarting the tunnel adds avoidable network behavior and loses diagnostics.
- Updating only the copied style creates a second coherence rule for names and
  DPTs; whole-context replacement is simpler.

## 4. Complete documented building-space vocabulary (§89)

### 4.1 Evidence boundary

The local KNX Standard v3.0.0 PDF `Project Schema23 v01.00.00.pdf` is the
authority:

- §1.1.2.3 `SpaceType_t` lists `Building`, `BuildingPart`, `Floor`,
  `Stairway`, `Room`, `Corridor`, `DistributionBoard`, `Area`, `Ground`, and
  `Segment`.
- §§1.2.6.3–1.2.6.4 describe `Space` and name `BuildingPart`, `RoomPart`, and
  `DistributionBoard`; this prose names `RoomPart` but omits `Segment`.

The document is internally inconsistent. KNXBench therefore supports both
literal documented tokens and states the limitation honestly. It does not claim
that `RoomPart` is present in the enumerated XSD type, nor that the three local
fixtures prove either disputed token.

The five additions are `Stairway`, `RoomPart`, `Area`, `Ground`, and `Segment`.
Unknown future strings remain explicit map errors and retain the existing
reported fallback; no open-ended string variant weakens exhaustiveness.

### 4.2 End-to-end mapping

- Add the five variants to `BuildingPartType` with exact external spelling.
- Extend ETS value parsing, store text codecs, project-tree projection, server
  request parsing, Inspector labels, and the building-part creation selector.
- Add English and German catalogue entries. User-facing raw enum debug text is
  not a substitute for localization.
- The store already persists `kind` as unconstrained text; no schema migration
  or version bump is needed.
- Unknown stored `kind` values become a typed load error rather than silently
  becoming `Building`. Corrupt or future data must not be silently rewritten.
- There is no ETS project exporter after ADR-0028. Native `.knxdb` save/load is
  the writable round-trip proof; documentation must not revive removed export
  claims.

### 4.3 Required proof

- Table-driven parser and mapper coverage for all five additions plus a truly
  unknown token.
- Synthetic schema-23 import maps each token without a type-related problem.
- Native store save/load/re-save retains every variant exactly.
- Project-tree/API and web creation/Inspector tests expose localized values.
- Diff or report coverage proves a new type remains distinguishable rather than
  rendering as legacy `BuildingPart`.

## 5. Theme contrast gate (§120)

The gate extends the existing `themeTokens.ts` parser; it adds no runtime code or
browser dependency. For every registered palette it evaluates these role pairs:

- `--knx-foreground` on `--knx-bg`;
- `--knx-foreground` on `--knx-surface`;
- `--knx-on-accent` on `--knx-accent`.

Every registered accent variation is evaluated after overlaying its accent pair
on the base theme. Each pair must meet WCAG AA normal-text contrast ratio 4.5:1.
This is a role invariant, not a claim that every possible component composition
or assistive technology has been audited.

The parser supports the concrete color forms currently used by these roles and
recursive `var(--knx-...)` references. An unsupported notation, unresolved
reference, alpha value that cannot be evaluated against the paired opaque
background, or reference cycle fails with a named diagnostic. It must never skip
an unparsed theme and report a green run. Extending role colors to `oklch()`,
`color-mix()`, or system colors therefore requires extending the evaluator in
the same change.

Tests cover luminance/ratio boundary examples, all shipped themes and accent
variations, unsupported syntax, unresolved/cyclic references, and a deliberately
illegible palette that fails for the expected pair. Status and muted role pairs
remain outside this narrow closure because §120's lifted-when contract names the
foreground/background and on-accent/accent relationships.

## 6. Cross-cutting constraints

- `docs/LIMITATION_TRIAGE.md` is not edited.
- No KNX, LAN, multicast, gateway, or hardware traffic is generated.
- No private-LAN literals or the prohibited T12 individual-address fixture are
  introduced.
- Group-address display remains slash-only with no notation selector.
- New user-visible strings use both English and German catalogues.
- UI styling remains inside `--knx-*` tokens and existing reduced-motion guards.
- Every new Rust/TypeScript source file has the required purpose header.
- Each slice updates its stable `KNOWN_LIMITATIONS.md` section without changing
  the numbered heading; final reconciliation updates `IMPLEMENTATION_STATUS.md`,
  `DATA_MODEL.md`, `IMPORT_EXPORT.md`, `COMPATIBILITY.md`, and ADR-0022 where
  their assertions changed.

## 7. Completion

Each slice receives focused tests and a focused commit. The branch then runs
TypeScript, the complete frontend suite, Rust formatting, warning-denied
workspace Clippy, workspace tests, layering, header, anchor, and dependency
gates. A fresh whole-branch review must report no unresolved Critical or
Important findings before non-fast-forward integration into `main` and repeated
merged-result verification.
