# ADR 0025: Extension is data, not code — the plugin API stays unwritten

Date: 2026-09-20

Status: Accepted

## Context

The question put to this session was whether and how third parties could
extend KNXBench. The full survey is
[PLUGIN_FEASIBILITY.md](../PLUGIN_FEASIBILITY.md); this record holds the
decision and the evidence that forces it. No code was written for either
document, no dependency was added, and no extension point was created.

**What the repository actually exposes.** The whole 16-crate workspace
contains eight traits:

```sh
grep -rn "^pub trait \|^trait " crates/ apps/*/src apps/knx-desktop/src-tauri/src xtask/src
```

`LoadObserver`, `ImportObserver`, `ManagementTransport`, `BusConnection`,
`ScanTransport`, `GatewayConnector`, `BusTunnel`, and a private
`OptionalNotSaved<T>`. Six are statically dispatched; `BusConnection` uses
`async fn` in trait and so is not `dyn`-compatible at all, which its own
comment records as deliberate ("no `dyn BusConnection`, which this cycle
never does … single implementer"). Exactly two are dispatched dynamically —
`GatewayConnector` and `BusTunnel`, both in `apps/knx-server/src/bus.rs` —
and both say in their doc comments that their second implementer is a test
fake.

There is no importer trait, no exporter trait, no report template trait and
no registration function on the frontend's command registry.
`crates/knx-app/src/lib.rs` is 412 lines and re-exports
`import_ets_project` and `export_ets_project` **by name**;
`apps/knx-web/src/commandRegistry.ts`'s `COMMANDS` is a module-level array
literal with no `register`. A plugin API here would not be *exposed*. It
would be *invented*, by generalising from exactly one implementation of
each seam — `knx-etsproj` (12 310 lines, with an opaque passthrough store,
a three-schema detector and a roundtrip contract) and `knx-csv` (1 807
lines, with none of those) share no interface, and a trait fitted to both
would say almost nothing.

**Three mechanical facts close the in-tree-trait route.** All 16 Cargo
manifests are `publish = false`, so no third party can depend on
`knx-core`. `cargo run -p xtask -- check-layering` is a hand-written
allowlist of crate names in `xtask/src/main.rs` built from `cargo
metadata`, so a third-party crate is invisible to it and a runtime-loaded
plugin could never appear in it. And `knx-core` depends on `chrono` alone —
no `serde` — so `Command`, the 31-variant enum that is the only sanctioned
mutation path, has no serialisable form for any out-of-process or
WebAssembly boundary to carry.

**Data integrity, which decides it.** `Command::apply` returns its own
inverse, calls the validators in `validation.rs`, and feeds `CommandStack`
for undo and redo. But `crates/knx-core/src/project.rs:180` declares all
six of `Project`'s fields `pub`. "All mutations go through `Command`" is a
convention held by review, not an invariant held by the type system — fine
inside one workspace with one test suite, and a different proposition when
`&mut Project` is handed to a stranger. Worse, `Layer` (ADR-0004) has five
variants — `Program`, `ProgramRef`, `Instance`, `Inferred`, `UserEdit` —
and none of them means "a plugin did this". A plugin writing a value would
forge `UserEdit` provenance or force a sixth variant through the
persistence schema and its migration chain. A plugin that dropped the
opaque passthrough store (ADR-0006, ADR-0007) would break roundtrip
fidelity silently, because an absence trips no validator.

**What already works, and is not code.** Language packs
(`apps/knx-web/src/languagePack.ts`, documented for non-developers in
[LANGUAGE_PACKS.md](../LANGUAGE_PACKS.md)), product databases
(`crates/knx-productdb`, ADR-0005 and ADR-0011, ingested at runtime with
every file kept verbatim as a SHA-256-keyed blob as well as parsed), and
group-address CSV (`crates/knx-csv`, IMPORT_EXPORT.md §11) are all
installable by someone who has never compiled this repository. The `knx`
CLI — 3 595 lines, headless by design — is a fourth surface, for
automation. Four working extension points, zero lines of plugin host.

**The licence, stated so it is not misused.** The project is
`AGPL-3.0-or-later` (`LICENSE`, `Cargo.toml`, `README.md`). Under the FSF's
published interpretation — read for this study at `#GPLPlugins`,
`#MereAggregation`, `#IfInterpreterIsGPL`, and an interpretation rather
than case law — a dynamically linked in-process plugin forms one combined
program, so proprietary `cdylib` plugins are not viable; an out-of-process
helper over a genuinely arms-length protocol has a real separateness
argument; scripting loses that argument again through bindings; and
WebAssembly is uncharacterised either way. AGPL §13 reaches any of it that
is served over a network, and `apps/knx-server` is network-interactive by
design. **None of this is the reason for the decision below.** AGPL
sustains large plugin ecosystems elsewhere; the codebase, not the
copyright, is what is not ready. Recording the licence mechanics here is
not legal advice, and no lawyer reviewed it.

## Decision

**KNXBench does not build a plugin API. Third-party extension is
data-shaped, and the four surfaces that already exist are the supported
story: language packs, product databases, group-address CSV, and the `knx`
CLI.**

No plugin host is added — no `cdylib` loader, no WebAssembly runtime, no
embedded scripting engine, no plugin protocol on `knx-server`. No trait is
introduced "so it is ready later", because an interface derived from a
sample of one is the abstraction this project's own rules tell it to
refuse. `/api/*` is not promoted to a public contract: doing so would ship
KNOWN_LIMITATIONS.md §22 (no authentication) and §63 (one shared project,
one shared undo stack) as a contract instead of as limitations.

Anyone who needs a code-level extension today forks and rebuilds. That is
unambiguous under AGPL, it is the only mechanism that stays inside
`check-layering`'s reach, and it costs this project nothing per release.

**Reconsider when**, and not before: the `Command` layer is complete and
has a serialisable form (ROADMAP.md already names this as the blocker for
MCP capabilities and for the in-app LLM surface — this ADR reaches the same
blocker by a different road); §22 and §63 are answered; `Project`'s
invariants are held by the type system rather than by review, or it is
decided explicitly that a plugin never receives the model; and at least two
concrete third-party extensions exist that the four data surfaces genuinely
cannot express.

**If a code extension point is then required, it is an out-of-process
helper over a documented protocol** — not a `cdylib`, not a scripting
engine. Crash containment is total, the separateness argument is the most
defensible of the executable options, and `knx-server` already is the
process boundary. WebAssembly is the second alternative and is deliberately
not ranked second-best: better isolation and a better versioning story, for
a host that does not need to run untrusted computation, with the least
characterised licence position of the five.

**The claim this decision rests on is falsifiable, and the experiment is
named rather than performed.** Write a second implementation of one seam as
an ordinary workspace crate — the cheapest candidate is a second output
format on the pure, IO-free `knx-report` path — and see whether a shared
trait falls out. If it does, fitting both implementations without
contorting the first, a real seam exists and this ADR should be revised. If
the trait has to be bent, or ends up conveying nothing, the "sample of one"
finding is confirmed at the cost of one crate that is a feature rather than
scaffolding.

## Alternatives considered

**A dynamically loaded Rust `cdylib`.** Native speed, full access, the
obvious shape. Rejected on the ABI alone: Rust has no stable ABI, so host
and plugin must share a compiler version — `rust-toolchain.toml` pins
1.98.0, making every bump a flag day — and the interface would have to be
`extern "C"` with `#[repr(C)]` mirrors of `Command`'s 31 variants and
`CommandError`'s 22, maintained by hand. Crash containment is nil: a panic
across the boundary is undefined behaviour and a segfault takes the open
project's unsaved state with it. For an application whose priority order
begins Correctness → Data Integrity, that is close to disqualifying before
the licence question is even reached. (`libloading` is already in
`Cargo.lock`, but only transitively via `libappindicator-sys` in the
archived GTK3 tray stack of KNOWN_LIMITATIONS.md §16; `cargo tree -i
libloading --target all` reports nothing reachable. It is not a head
start.)

**WebAssembly with a host runtime.** The best isolation and, via the
component model and WIT, the best versioning story of any mechanism here.
Rejected *for now*, not on merit: it needs a serialisable form of the
domain model that does not exist, and building one creates a permanent,
versioned serialisation surface for precisely the data whose integrity this
project treats as paramount — plus a substantial runtime dependency and a
capability model to design and defend. Its licence position is also the
least characterised of the five, which is an awkward property for the
mechanism a third party would pick *because* it looks safe.

**An out-of-process helper over a documented protocol.** Kept as the named
alternative above rather than rejected. Not adopted now because it is
blocked on four things — a serialisable `Command`, a mature `Command`
layer, §22 and §63 — all of which are worth doing for other reasons, and
paying for the protocol speculatively before any of them is exactly the
speculative abstraction this project's rules forbid.

**An embedded scripting engine.** Rejected as the worst trade in the table
for this application: every useful script reaches `Project` and `Command`
through bindings, which is both the data-integrity hazard of §4 of the
study (all of `Project`'s fields are `pub`) and the case where the FSF's
interpreter analysis stops favouring the script author. It costs a
dependency and a binding layer re-checked against every domain change, and
buys an extension surface that erodes invisibly. Worth separating from it:
a *macro* facility that records and replays `Command` sequences is not a
scripting engine, needs no dependency, and would inherit validation and
undo for free — if "automate repetitive tasks" is the real request, that is
the shape to study, and it is not a plugin.

**A trait surface published to crates.io for compiled-in plugins.**
Rejected: it requires publishing at least `knx-core` and committing to a
public API and SemVer at version `0.1.0-alpha.1` with Session 7 not closed,
and the traits to publish do not exist. Fork-and-rebuild delivers the same
capability today at zero cost to this project.

**Promoting `knx-server`'s `/api/*` routes to a documented plugin API.**
Roughly 40 routes already exist, so this looks like the cheapest option on
the table. Rejected: `apps/knx-server/src/main.rs` binds `0.0.0.0` with no
authentication, no CORS layer and no token (§22), and the server holds one
shared project with one shared undo stack and no conflict detection (§63).
A plugin issuing commands concurrently with a user writes into the undo
history that user is about to press Ctrl+Z on. Documenting the routes would
freeze shapes that currently change whenever the UI needs them to, and
would convert two known limitations into two promises.

**Building a UI plugin surface.** Rejected on three independent walls, any
one of which suffices: `PaletteCommand.labelKey` is typed `MessageKey`
derived from `messages/en.ts`, so a third-party label is a compile error
(ADR-0024 decision 2, taken deliberately); `CommandContext` is a closed
interface of fifteen named callbacks with no generic capability to hand
out; and ADR-0022's theme token boundary means a third-party panel either
builds against an internal contract that moves or looks foreign in every
theme. There is also no native surface beneath it —
`apps/knx-desktop/src-tauri/src/` is 105 lines with zero `#[tauri::command]`
attributes, on the archived GTK3 stack of §16.

**Doing nothing and leaving the question open.** Rejected because the
question keeps being asked, and an unanswered "should we have plugins?"
tends to be resolved incrementally by whoever is nearest a seam. Writing
the answer down, with the falsifying experiment attached, is cheaper than
re-deriving it.

## Consequences

**Easier.** Nothing new to maintain per release: no ABI, no protocol, no
sandbox, no runtime, no second licence gate. `check-layering` stays a
closed allowlist over a closed workspace, which is what makes it a
guarantee rather than a heuristic. Domain types stay free to change —
`Command` can gain its 32nd variant, `Layer` can gain a sixth, `Project`
can be reshaped — without breaking a stranger's build. And the four
existing extension surfaces get named as a story instead of existing as
four unrelated features.

**Harder.** A third party who wants a new file format, a new protocol or a
new panel must fork and rebuild, which needs a Rust toolchain, a build, and
rebasing onto upstream. That is a real barrier and this ADR does not
pretend otherwise. There is also no answer for a commercial third party who
wants a proprietary addon — under §3 of the study that answer would have
been "no" or "unsettled" for every in-process mechanism anyway, so the
decision forecloses less than it appears to.

**Enforced by.** Nothing mechanical, and that is worth stating plainly
rather than discovering later: no gate in this repository can fail a pull
request for adding a plugin host. This ADR is enforced by review. The
signals a reviewer should treat as this decision being reopened without an
ADR: a new `dyn` trait introduced for a non-test second implementer that
does not exist yet; a `wasmtime`, `libloading`, `abi_stable` or scripting
dependency appearing in `Cargo.toml`; `serde` reaching `knx-core`; or
documentation that describes `/api/*` as a stable interface.

**Recorded as a limitation.** KNOWN_LIMITATIONS.md §107 states what a third
party cannot do as a result, so the absence is a published boundary rather
than an omission a user discovers by trying.

**Not claimed anywhere.** No ETS compatibility and no KNX certification.
The ETS plugin channels this application already preserves opaquely —
device `BinaryData`, described in schema 23 as "For use by plugins", and
`ExtraData/` (ADR-0019 §E3) — stay preserved and **never executed**.
Nothing here is a step toward running vendor plugin payloads out of a
`.knxproj`.
