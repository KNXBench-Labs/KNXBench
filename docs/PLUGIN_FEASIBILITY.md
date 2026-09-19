# Plugin and extension feasibility

Whether a third party could extend KNXBench, through which seam, by which mechanism, and
at what cost — a study, with the evidence against.

No code was written for this document, no dependency was added, and no extension point
was created. It exists so that a decision can be made on evidence rather than on
enthusiasm. The decision it argues for is recorded in
[ADR-0025](adr/0025-extension-is-data-not-code.md).

Evidence markers follow [RESEARCH.md](RESEARCH.md):

* **[V]** — verified in this repository, on the branch this document lands on.
  Reproducible with the command or the file path given.
* **[D]** — documented by a citable external source, read for this study but not
  independently tested here.
* **[A]** — assumption or inference. Must be validated before it drives anything
  irreversible.

---

## Summary

**Recommendation: do not build a plugin API. Extension stays data-shaped.**

Three findings carry that, and a reader can stop after them.

1. **There is almost nothing to implement.** [V] The whole workspace exposes eight
   traits. Six are single-implementer or test seams, one is a private helper, and
   exactly one is dispatched dynamically — and that one lives in `apps/knx-server`, is
   shaped around a KNXnet/IP tunnel, and says in its own doc comment that its other
   implementer is a test fake. There is **no** format adapter trait, **no** product
   database provider trait, **no** report template trait, and **no** registration
   function on the frontend's command registry. A plugin API here would not be exposed;
   it would be invented, from a single implementation of each thing, which is the
   textbook way to get an abstraction wrong.

2. **The project already has a third-party extension story, and it is files.** [V]
   Language packs (`apps/knx-web/src/languagePack.ts`, documented for non-developers in
   [LANGUAGE_PACKS.md](LANGUAGE_PACKS.md)), product databases (`crates/knx-productdb`,
   ingested at runtime per [ADR-0005](adr/0005-separate-product-database.md)) and
   group-address CSV (`crates/knx-csv`, [IMPORT_EXPORT.md §11](IMPORT_EXPORT.md)) are all
   installable by someone who has never compiled this repository. A fourth surface exists
   for automation: the `knx` CLI, 3 595 lines, headless by design. Four working extension
   points, zero lines of plugin host.

3. **The licence is not the reason to say no, and it is important not to pretend it
   is.** AGPL-3.0-or-later is entirely compatible with a thriving free-software plugin
   ecosystem. What it does — under the FSF's published reading, which is an
   interpretation and not case law — is make *proprietary in-process* plugins
   untenable, with the answer getting progressively less certain as the boundary moves
   from a `dylib` to WebAssembly to a separate process. §3 works this through. The
   reason to say no is the codebase, not the copyright.

**The cheapest honest next step** is not a prototype. It is one page of documentation
that names the four surfaces above as *the* extension story, so that the question stops
being asked in the abstract — plus, if a code seam is genuinely wanted later, writing a
*second* implementation of one seam **inside the workspace** before designing any trait
for it. §5.3 gives that experiment and states what result would falsify this
recommendation.

**Reconsider when** the `Command` layer is complete and has a serialisable form, and
when [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §22 (no authentication) and §63 (one
shared project, one shared undo stack) are answered. [ROADMAP.md](ROADMAP.md) already
names the first of those as the blocker for MCP capabilities and for the in-app LLM
surface; this study reaches the same blocker by a different road, which is mild
evidence that the blocker is real.

---

## 1. What a plugin would extend: the real seams

The question is not "where could an extension point go" — anywhere could. It is "where
does an interface already exist that a third party could implement". The answer is
gathered by listing every trait in the workspace:

```sh
grep -rn "^pub trait \|^trait " crates/ apps/*/src apps/knx-desktop/src-tauri/src xtask/src
```

[V] Eight results, on the branch this document lands on:

| Trait | File | Dispatch | Implementers |
| --- | --- | --- | --- |
| `LoadObserver` | `crates/knx-app/src/progress.rs:55` | static | progress reporting ([ADR-0023](adr/0023-load-progress-operation.md)) |
| `ImportObserver` | `crates/knx-etsproj/src/progress.rs:69` | static | same, one layer down |
| `ManagementTransport` | `crates/knx-net/src/management.rs:44` | static | blanket impl over `ScanTransport`; its doc comment says "so a fake written for one serves the other" |
| `BusConnection` | `crates/knx-net/src/client.rs:113` | static | its own comment: "no `dyn BusConnection`, which this cycle never does … single implementer" |
| `ScanTransport` | `crates/knx-net/src/scan.rs:203` | static | test seam, see `ManagementTransport` |
| `GatewayConnector` | `apps/knx-server/src/bus.rs:52` | **`dyn`** | `RealConnector` and `fake::FakeConnector` |
| `BusTunnel` | `apps/knx-server/src/bus.rs:64` | **`dyn`** | `RealTunnel` and the test fake |
| `OptionalNotSaved<T>` | `crates/knx-store/src/project.rs:474` | private | not `pub` |

That table is the whole extensibility surface of a 16-crate workspace. Taking the
brief's candidate seams one at a time against it:

### 1.1 Import and export format adapters — would have to be invented

[V] `crates/knx-app/src/lib.rs` is 412 lines across four files and re-exports two
functions by name:

```rust
pub use export::export_ets_project;
pub use import::{import_ets_project, import_ets_project_observed, import_ets_project_with, …};
```

There is no `Importer` trait, no `Exporter` trait, no format enum and no registry. The
`.knxproj` reader (`crates/knx-etsproj`, 12 310 lines), the group-address CSV
reader/writer (`crates/knx-csv`, 1 807 lines) and the HTML documentation renderer
(`crates/knx-report`, 2 325 lines) share no interface whatsoever — they are three
crates called by name from the application layer and from `apps/knx-cli/src/main.rs`.

This is not an oversight; it is what [IMPORT_EXPORT.md §1](IMPORT_EXPORT.md)'s pipeline
describes, and it is correct for a project that has exactly one project-file format. But
it means the honest answer to "could a third party add a format?" today is: **no, not
without editing `knx-app` and the CLI.** [A] An interface derived from a single
implementation would encode that implementation's accidents. `knx-etsproj` carries an
opaque passthrough store ([ADR-0006](adr/0006-opaque-passthrough-store.md)), a schema
detector for three ETS schema versions, a provenance-layered import report and a
roundtrip fidelity contract ([ADR-0007](adr/0007-roundtrip-fidelity.md)); `knx-csv`
carries none of those and never will. A trait that fits both is a trait that says almost
nothing.

### 1.2 Product database providers — already solved, and not by code

[V] This one is genuinely extensible today, and has been since Session 4. [ADR-0005]
(adr/0005-separate-product-database.md) puts manufacturer data in a separate SQLite file
shared across projects; [ADR-0011](adr/0011-product-database-storage.md) gives it its own
storage layer; `crates/knx-productdb/src/ingest.rs` reads product files at runtime; and
`xtask check-layering` enforces that `knx-productdb` reaches neither `knx-etsproj` nor
`knx-store`, so product data cannot travel through the project importer
(`xtask/src/main.rs`, the `knx-productdb` block).

The crate's own module header states the integrity property that makes this safe:

> Every ingested file is kept twice: verbatim as a blob keyed by its SHA-256, and as
> parsed rows. The blob is the integrity guarantee — the parser may not understand a
> construct, but nothing is ever lost.

[V] `crates/knx-productdb/src/dynamic/` holds a parser and an evaluator for the
manufacturer-authored dynamic parameter logic — that is, the product file already carries
*behaviour*, which this application interprets rather than executes.

**Finding.** A "manufacturer plugin" in this application is a file you import. It needs no
plugin API, no ABI, no sandbox and no licence analysis, because it is data. CLAUDE.md's
standing rule ("manufacturer data must remain separable from application code") is
already mechanically enforced. Anyone proposing a product-database *code* plugin should be
asked what it would do that ingesting a file does not.

### 1.3 Protocol adapters — one `dyn` seam, shaped like a tunnel

[V] `apps/knx-server/src/bus.rs:52` is the only dynamically dispatched seam in the
repository, and `apps/knx-server/src/domain.rs:74` holds the `Box<dyn GatewayConnector>`.
Its doc comment is explicit about why it is `dyn` at all:

> `knx_net::KnxNetIpClient` is the production implementer via [`RealConnector`]; tests
> use [`fake::FakeConnector`]. … `dyn` safety is exactly the point here, unlike in
> `knx-net`'s own `BusConnection`.

So: a test seam, deliberately. And its shape is KNXnet/IP's, not a protocol's in
general — `connect_tunnel(SocketAddrV4)` returning a `BusTunnel` whose vocabulary is
`TunnelEvent`, `Destination`, `ApplicationService`. [A] A KNX-RF, TP-UART or USB
interface adapter would either fit this by pretending to be a tunnel, or would need the
trait redesigned. Neither is a plugin story; the second is ordinary in-tree work.

[V] Meanwhile the library-layer trait that *looks* like the protocol seam, `BusConnection`,
is not usable for dynamic dispatch at all: it uses `async fn` in trait under
`#[allow(async_fn_in_trait)]`, which is not `dyn`-compatible, and its own comment records
that this is fine precisely because nothing needs `dyn`.

### 1.4 Report and documentation templates — the strongest candidate, and still weak

[V] `crates/knx-report` is the cleanest crate in the workspace for this purpose. Its
header states the properties a plugin host would want:

> `knx-report` is pure: a `&knx_core::Project` in, a `String` and typed warnings out. It
> knows nothing of the filesystem, HTTP, SQLite or the system clock — the generation
> timestamp travels in through [`ReportOptions`] so the same project renders to
> byte-identical HTML on every call.

A pure, deterministic, IO-free function from project to string is exactly the shape that
sandboxes well. [V] `xtask check-layering` already holds it to that: `knx-report` may
reach none of `knx-store`, `knx-etsproj`, `knx-productdb`, nor any of `CORE_FORBIDDEN`
(`serde_json`, `quick-xml`, `rusqlite`, `tokio`, `axum`, `tower`).

But: [V] the crate has one public entry point, `render_html`, and one output format. There
is no `Template` trait and no template language — the HTML is produced by
`render.rs`/`html.rs` walking the model in Rust. [A] Making this pluggable means either
introducing a template language (a new dependency, a new injection surface, and prose
authored by strangers rendered into a document users trust) or exposing `knx_core::Project`
across a plugin boundary, which §4 argues against on data-integrity grounds. The cheaper
version of "custom reports" is a structured export a third party post-processes with their
own tools — which the CSV path already half-provides.

### 1.5 UI panels and commands — the type system says no, deliberately

[V] `apps/knx-web/src/commandRegistry.ts` is 174 lines. `COMMANDS` is a module-level array
literal (`export const COMMANDS: PaletteCommand[]`, line 58) and there is no `register`
function — `filterCommands` at line 167 is the only other export. Two further walls stand
behind it:

* **The label.** `PaletteCommand.labelKey` is typed `MessageKey`, and `MessageKey` is
  `keyof typeof messages` derived from `messages/en.ts`. A third-party command with a
  third-party label is a **compile error**. This is not incidental — it is
  [ADR-0024](adr/0024-in-application-help.md) decision 2, taken so that a missing German
  string fails the build rather than falling back silently.
* **The context.** `CommandContext` (line 5) is a closed interface of fifteen named
  callbacks. A command can call `openBusMonitor` or `saveProjectAs`; there is no generic
  capability to hand a stranger.

[V] Styling is walled too: [ADR-0022](adr/0022-theme-token-boundary.md) makes a theme
responsible for a declared set of `--knx-*` palette tokens. A third-party panel either
consumes those tokens (and is therefore built against an internal contract that moves) or
ignores them and looks foreign in every theme.

[V] And there is no native surface underneath: `apps/knx-desktop/src-tauri/src/` is 105
lines total with **zero** `#[tauri::command]` attributes — its header calls it a "thin
native wrapper" that spawns `knx-server`'s router and points a WebView at it. [V] That
shell also sits on the archived GTK3 stack recorded as
[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §16; building a plugin surface onto it means
building onto a dependency the project is already waiting to migrate off.

### 1.6 The surfaces that are already extensible

Set against the four "would have to be invented" answers above, four things already work.

**Language packs** [V] — `apps/knx-web/src/languagePack.ts` (576 lines) plus
[LANGUAGE_PACKS.md](LANGUAGE_PACKS.md) (303 lines, addressed to a non-developer). A JSON
file with four required fields, validated for shape, installed into `localStorage`,
exportable again. The format document's own forward-compatibility rule is the part worth
copying:

> Any field in the pack's JSON that is not in the list above — whether because it belongs
> to a future format version, or because you added your own bookkeeping field — is
> preserved unread rather than stripped out.

That is [ADR-0006](adr/0006-opaque-passthrough-store.md)'s opaque-passthrough principle
applied to a third-party file, and it is why this extension point can accept files from
people who have never read the source.

**Product databases** [V] — §1.2.

**Group-address CSV** [V] — `crates/knx-csv`, documented as an exchange format in
[IMPORT_EXPORT.md §11](IMPORT_EXPORT.md). Text in, typed rows out, no SQLite and no ZIP
along for the ride (enforced by `check-layering`).

**The CLI** [V] — `apps/knx-cli`, 3 595 lines, whose own header states the reason it
exists:

> Keeping a real CLI alongside the desktop application is what forces the core to stay
> free of user-interface dependencies and makes import, roundtrip and regression tests
> runnable in CI without a display.

A headless binary is an integration surface. It is the *only* extension mechanism in this
study whose licence position is not arguable (§3.5), whose crash containment is total, and
whose per-release maintenance cost is already being paid.

### 1.7 What `check-layering` has to say

[V] `cargo run -p xtask -- check-layering` is not a generic rule engine. `xtask/src/main.rs`
lists forbidden reachability **by crate name**, one hand-written block per crate:
`knx-core`, `knx-etsproj`, `knx-productdb`, `knx-projection`, `knx-csv`, `knx-report`,
`knx-diff`, `knx-secure`. Each block carries a comment explaining the rule it enforces.

Two consequences for any plugin proposal:

* **A third-party crate is invisible to this gate.** It is not in the list, so nothing
  checks that a plugin refrains from reaching `rusqlite`, `tokio` or `knx-store`. The one
  mechanical guarantee this architecture rests on does not extend past the workspace
  boundary, and making it extend would mean rewriting the check from an allowlist of known
  names into a rule about categories — work that is not small and that nothing currently
  needs.
* **`cargo metadata` is where the graph comes from** (`xtask/src/layering.rs`,
  `workspace_graph`). A plugin loaded at runtime is not in `cargo metadata` at all, so
  even a rewritten check could not see it. Only the compiled-in mechanism of §2.5 stays
  inside the gate's reach.

[V] A third fact closes the loop: **all 16 manifests are `publish = false`.** A third
party cannot `cargo add knx-core`. Any mechanism that requires implementing an in-tree
trait requires publishing at least `knx-core`, and publishing means committing to a
public API and to SemVer — at version `0.1.0-alpha.1`, with Session 7 not closed.

---

## 2. Mechanisms, and what each costs per release

The per-release column is the one that matters. A plugin system's cost is not its
construction; it is that every subsequent change to the host has to be checked against it,
for as long as the plugin system exists.

### 2.1 In-process Rust `cdylib`

**Isolation:** none. Same address space, same allocator, same stack.

**Crash containment:** none. A panic unwinding across an `extern "C"` boundary is undefined
behaviour; a segfault in the plugin is a segfault in KNXBench, taking the open project's
unsaved state with it. [A] For an application whose priority order starts Correctness →
Data Integrity, "a third party can corrupt our process" is close to disqualifying on its
own.

**Versioning:** this is the whole story, not a detail. [D] Rust has no stable ABI: `repr(Rust)`
layout is explicitly unspecified and may differ between compiler versions, between
compilations, and with different generic instantiations. Two consequences:

* The host and every plugin must be built by the **same compiler version**. [V]
  `rust-toolchain.toml` pins `channel = "1.98.0"` today; every bump becomes a
  flag-day for every plugin in existence.
* The interface must be `extern "C"` with `#[repr(C)]` types only — which means
  `knx_core::Project`, `Command`, `Resolved<T>` and every other domain type would need a
  hand-written, hand-maintained C-shaped mirror. [V] `Command` alone has 31 variants,
  several carrying `Option<…>`, `Vec<…>` and `Text`; `CommandError` has 22.

[D] The `abi_stable` and `stabby` crates exist to attack exactly this, at the cost of a
heavy dependency, a macro layer over every shared type, and a constrained subset of Rust.
Not evaluated here beyond noting they exist; neither is in `Cargo.lock`. [V]

**What breaks on a host update:** every plugin, on every toolchain bump, silently — the
failure mode is memory corruption, not a link error, unless the interface carries its own
version handshake.

**Linux packaging:** `.so` files in a search path. Straightforward, and the only easy
part. [V] Note that `libloading` 0.7.4 is already present in `Cargo.lock` — but only as a
transitive entry of `libappindicator-sys`, part of the GTK3 tray stack of §1.5's
limitation §16. `cargo tree -i libloading --target all` reports nothing reachable, so it is
not linked into anything this workspace builds. It is not a head start.

**Per-release cost:** high and permanent. Every domain type that crosses the boundary is
frozen or versioned by hand; every toolchain bump is a compatibility event; every crash
report from a user with plugins installed is ambiguous about whose fault it is.

### 2.2 WebAssembly with a host runtime

**Isolation:** real, and the best of any in-process option. Separate linear memory, no
host pointers, capability-based access to anything outside (WASI, or an explicit import
list).

**Crash containment:** good. A trap is catchable by the host; the host survives.

**Versioning:** far better than §2.1. [D] The WebAssembly Component Model and WIT exist
precisely to give a language-neutral, versioned interface description, so the host/plugin
contract is a declared IDL rather than an accident of compiler layout. [A] This is the
mechanism that would age best.

**The cost, which is the model crossing:** [V] `knx-core`'s only dependency is `chrono`
(`crates/knx-core/Cargo.toml`) — no `serde`, and `Command` derives only `Debug, Clone,
PartialEq`. There is no serialisable form of the domain model or of a mutation, anywhere.
The serde boundary is `knx-projection` (`serde` + `ts-rs`), which is a *display* shape for
the frontend, not the model. So a WASM plugin that wanted to read a project would
either be handed the projection (lossy for anything the UI does not show) or require a new
wire format for the domain model — a new, permanent, versioned serialisation surface for
the exact data whose integrity this project treats as paramount.

**What breaks on a host update:** a changed WIT interface, visibly and at load time, which
is the right failure mode.

**Linux packaging:** `.wasm` files. Genuinely easy, architecture-independent, and
inspectable.

**Per-release cost:** a runtime dependency of substantial size (`wasmtime` or equivalent;
[V] not in `Cargo.lock` today), a host-side capability model to design and defend, a WIT
interface to version, and a translation layer between the domain model and the interface
types that has to be kept faithful — the last of which is a data-integrity surface, not
just work.

### 2.3 Out-of-process helper over a documented protocol

**Isolation:** total. Separate process, separate address space, OS-enforced.

**Crash containment:** total. The helper dies, the host notices and reports.

**Versioning:** a documented protocol with a negotiated version — the same discipline any
network protocol needs, and the one this project already has experience with.

**What exists:** [V] `apps/knx-server` already speaks HTTP over ~40 routes
(`routes.rs`, `bus_routes.rs`, `fs_routes.rs`, `debug_report_routes.rs`). That is the
closest thing in the repository to an out-of-process extension surface. Two verified
reasons it is not one yet:

* [V] **No authentication.** `apps/knx-server/src/main.rs:32` binds `0.0.0.0`; there is no
  `CorsLayer`, no token and no auth middleware. This is recorded as
  [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §22.
* [V] **One shared project, one shared undo stack, no conflict detection** —
  [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §63. A plugin issuing commands concurrently
  with a user does not merely risk a race; it writes into the same undo history the user is
  about to press Ctrl+Z on.

Promoting `/api/*` to a public plugin contract would ship both of those as a contract
rather than as a limitation, and would freeze route shapes that currently change whenever
the UI needs them to.

**Linux packaging:** an ordinary executable. No ABI, no runtime, no toolchain coupling.

**Per-release cost:** the protocol document, its version negotiation, and the discipline
of not breaking it — plus, unavoidably, answering §22 and §63 first.

### 2.4 Embedded scripting engine

**Isolation:** depends entirely on the engine and on what is exposed; typically weak by
default, because the value of scripting is reaching host objects.

**Crash containment:** usually good for script errors, poor for resource exhaustion unless
the engine offers fuel/instruction limits.

**Versioning:** the exposed API is the contract, and scripting APIs erode slowly and
invisibly — a field rename is a runtime failure in someone's script months later.

**The specific problem here:** [V] every interesting script would reach `Project` and
`Command`. `Project`'s six fields are **all `pub`** (`crates/knx-core/src/project.rs:180`),
so a binding that hands a script a mutable project hands it the ability to bypass the
command layer entirely — see §4. And per §3.4, bindings are precisely the case where the
licence analysis stops favouring the script author.

**Per-release cost:** a dependency, a binding layer that has to be re-checked against every
domain change, and a support burden from users whose scripts broke.

**[A]** Worth separating from this: a *macro* facility that records and replays `Command`
sequences is not a scripting engine and does not need one. It would live entirely inside
the existing command layer, carry no new dependency, and inherit undo/redo and validation
for free. If "automate repetitive tasks" is the actual request —
[ROADMAP.md](ROADMAP.md) lists it as a deferred item — that is the shape to study, and it
is not a plugin.

### 2.5 Compiled-in crates: fork and rebuild

**Isolation:** none needed, because there is no boundary. The plugin is a workspace member.

**Crash containment:** identical to any in-tree crate.

**Versioning:** none required. Everything is compiled together against one source tree.

**What it costs the extender:** a Rust toolchain, a build, and the discipline of rebasing
onto upstream. Not nothing — but [V] it is the *only* mechanism that stays inside
`xtask check-layering`'s reach (§1.7), and the only one where the existing test suite
covers the combination.

**Licence:** unambiguous (§3.5). The fork is a modified version; AGPL applies; this is the
outcome the licence was chosen to produce.

**Per-release cost to this project: zero.** No host, no ABI, no protocol, no sandbox, no
new gate. The cost lands entirely on the extender, who is also the party that chose to
extend.

**[A]** This is the option that looks like a non-answer and is not one. It is how GNU/Linux
distributions carry patches, and — given that all 16 crates are `publish = false` and the
licence is AGPL — it is what a serious third-party extender would end up doing anyway.

### 2.6 Side by side

| | `cdylib` | WASM | Out-of-process | Scripting | Compiled-in |
| --- | --- | --- | --- | --- | --- |
| Isolation | none | strong | total | weak | n/a |
| Crash containment | none | good | total | partial | n/a |
| Versioning story | worst (no stable ABI) | best (WIT) | good (protocol) | poor (implicit API) | none needed |
| Breaks on host update | silently | at load | at handshake | at runtime, later | at compile time |
| Linux packaging | `.so`, toolchain-locked | `.wasm`, portable | executable | text file | source tree |
| New dependency | possibly `abi_stable` | a WASM runtime | none | an engine | none |
| Proprietary plugin viable? | no (§3.1) | unsettled (§3.2) | arguable (§3.3) | mostly no (§3.4) | no (§3.5) |
| Per-release cost to us | high, permanent | high, permanent | moderate, permanent | moderate, permanent | **zero** |
| Blocked on | nothing technical | serialisable model | §22, §63, serialisable `Command` | nothing technical | nothing |

---

## 3. The licence question

**This section describes licence mechanics. It is not legal advice, no lawyer has
reviewed it, and the central question it turns on has no controlling precedent that this
study can cite.** Where the FSF's published interpretation is quoted, it is quoted as the
interpretation of the licence's author — influential, widely relied upon, and not binding
on a court.

[V] The project is `AGPL-3.0-or-later`: `LICENSE` carries the AGPLv3 text,
`Cargo.toml`'s `[workspace.package]` sets `license = "AGPL-3.0-or-later"`, and `README.md`
states it, including the network clause:

> Modified versions made available to users over a network must also offer those users the
> corresponding source code as required by the AGPL.

Two features of AGPL matter more here than they would for GPL:

* **§13, Remote Network Interaction.** If a modified version is made available to users
  over a network, those users must be offered the Corresponding Source. [V] KNXBench has
  a network-interactive deployment target — `apps/knx-server` with a `0.0.0.0` bind and a
  documented web/Docker target in [ROADMAP.md](ROADMAP.md). So the usual proprietary-SaaS
  escape hatch from GPL is closed by design. Any third party running host-plus-plugin as a
  service owes source for whatever counts as the modified version — which is exactly the
  question below.
* **§5's aggregate carve-out.** A compilation of the Program with other *separate* works
  on a storage volume is an "aggregate", and the licence does not extend to the other
  works. Everything therefore hinges on separate-versus-combined, which the licence does
  not define and which turns on derivative-work analysis under copyright law.

[D] The FSF's GPL FAQ addresses this directly. At `#MereAggregation` it concedes the
difficulty first — "Where's line between two separate programs, and one program with two
parts? This is legal question" — then gives two criteria: "If modules included in same
executable file, they definitely combined in one program", and, for communicating
processes, "But if semantics of communication intimate enough, exchanging complex internal
data structures, too could be basis" for treating them as one. Mechanism *and* semantics,
not mechanism alone.

### 3.1 In-process `cdylib` — proprietary plugins are not viable

[D] `#GPLPlugins` is squarely on point: "It depends on how the main program invokes its
plug-ins", with dynamic linking plus mutual function calls and shared data structures
tending toward one combined program, and "Using shared memory to communicate with complex
data structures is pretty much equivalent dynamic linking". `#GPLAndPlugins` states the
consequence: "A main program separate from its plug-ins makes no requirements for
plug-ins" — and, where they are not separate, the plug-in must be under a
GPL-compatible free licence.

[A] A KNXBench `cdylib` plugin is the combined case by any reading: it would be loaded into
the process, called through function pointers, and handed domain structures. **Conclusion:
under the FSF reading, a proprietary in-process plugin is not viable, and AGPL §13 reaches
the combination whenever it is served over a network.** This is the *least* uncertain of
the five answers.

### 3.2 WebAssembly — genuinely unsettled

[A] This is the honest gap in the study, stated as one. A WASM module has no host
pointers, no shared allocator and no Rust types in common with the host; those are the
facts the "separate work" argument would rest on. Against that: it is loaded into the host
process, it exists only to run as part of the host, and it communicates through an
interface defined by the host in the host's own domain vocabulary — which is the
"intimate semantics" limb of `#MereAggregation`, and that limb does not care about memory
safety.

The FSF FAQ, read for this study, contains no statement about WebAssembly. [A] I am aware
of no case law and no authoritative interpretation either way. **A third party planning a
proprietary WASM plugin on the strength of the sandbox would be relying on an argument
nobody has tested.** That is a finding, not a gap to paper over: the mechanism with the
best technical isolation story has the worst-characterised licence story.

### 3.3 Out-of-process — the strongest separateness argument, and still not free

[D] `#MereAggregation` treats pipes, sockets and command-line arguments as the mechanisms
normally used between separate programs, and `#GPLInProprietarySystem` adds the substance
test: the two must "communicate arms length, not combined in way would make them
effectively single program", with "compiler kernel" and "editor shell" given as examples
of genuine separation.

[A] So a proprietary out-of-process helper has a real argument — but only if the protocol
is genuinely arms-length. A protocol that exists solely to drive KNXBench, carrying
KNXBench's internal domain structures, designed so that nothing else could sensibly
implement either end, is the "intimate semantics" case with a socket in the middle. The
separateness is bought by the protocol's generality, not by the process boundary alone,
and that is a design constraint on the protocol, not a formality.

### 3.4 Scripting — bindings undo the advantage

[D] `#IfInterpreterIsGPL` is clear in both directions. Plain interpretation imposes
nothing — "When interpreter just interprets language, answer is no", because "The
interpreted program, to the interpreter, is just data". But where bindings exist, "interpreted
program effectively linked facilities it uses through bindings", and the FAQ states that
using GPLed interpreted libraries obliges you to release your program compatibly
*irrespective of the interpreter's own licence*.

[A] Every useful KNXBench script reaches `Project` and `Command` through bindings. So
scripting inherits close to the `cdylib` answer, while giving up the `cdylib`'s
performance and gaining a dependency. It is the worst trade in the table for this
particular application.

### 3.5 Compiled-in / fork — unambiguous

[A] A workspace member compiled into the binary is part of the Program. There is no
argument to have. AGPL applies, source must be offered, and §13 applies when it is served
over a network. For an AGPL project this is the *intended* outcome rather than a problem,
and it is the only row of the table where a third party can know where they stand without
a lawyer.

### 3.6 What would need a lawyer, and one asymmetry worth naming

A lawyer, not a developer, is required to answer: whether a given plugin is a derivative
work in the relevant jurisdiction; whether the FSF's interpretation would be followed by a
court; whether a WASM boundary is materially different from a `dylib` boundary; and what
"Corresponding Source" covers for a host-plus-plugin combination served under §13.

[V] One asymmetry the repository already exhibits: `deny.toml` is a mechanical licence gate
for code coming **in** ("The hard constraint is RESEARCH R6: no GPL-licensed crate may
enter the runtime dependency graph"), enforced by `cargo deny check` on an allowlist of
eleven permissive licences. There is no corresponding mechanism for code going **out**
through a plugin boundary, and there could not easily be one — a host cannot inspect the
licence of a `.so` it is handed. [A] Any plugin system would therefore ship an
unenforceable licence rule, which is a maintenance and community-management cost that
belongs in the decision rather than being discovered later.

### 3.7 The conclusion that should not be overstated

AGPL does not prevent a plugin ecosystem. GIMP, Blender and Inkscape are the existence
proof that copyleft hosts sustain large third-party ecosystems. What AGPL does is make the
*proprietary* plugin case unattractive-to-untenable, with certainty decreasing from §3.1
to §3.3. **If this study recommends against a plugin system, the licence is not the
reason** — and saying otherwise would be borrowing an argument the licence does not make.

---

## 4. Data integrity: what a plugin could be allowed to touch

[V] The intended mutation path is narrow and well built. `crates/knx-core/src/command.rs`
defines `Command`, a closed enum of 31 variants; `Command::apply(&self, project: &mut
Project) -> Result<Command, CommandError>` (line 529) performs the mutation **and returns
its own inverse**; it calls the validators in `validation.rs` (duplicate individual
addresses, duplicate group addresses, group-range ordering and nesting, overlapping
ranges, still-linked-on-delete); and `CommandStack` (line 1327) records the inverse for
undo and redo. `CommandError` has 22 variants, so failures are typed rather than
best-effort.

Around that sit two more integrity mechanisms:

* [V] **Provenance** ([ADR-0004](adr/0004-provenance-model.md)). Every overridable value
  carries the layer it was resolved from. `Layer` has exactly five variants —
  `Program`, `ProgramRef`, `Instance`, `Inferred`, `UserEdit` — and
  `Layer::is_exported()` returns true for only `Instance` and `UserEdit`. **There is no
  `Layer::Plugin` and no external-authorship layer.** A plugin writing a value would
  either have to forge `UserEdit` — telling the user they made a change they did not — or
  the enum, the persistence schema and its migration chain would all have to grow a sixth
  variant.
* [V] **Opaque passthrough** ([ADR-0006](adr/0006-opaque-passthrough-store.md),
  [ADR-0007](adr/0007-roundtrip-fidelity.md)). Unparsed constructs are preserved verbatim
  so that export can put them back. A plugin that rewrote a project without carrying that
  store would break roundtrip fidelity **silently** — no validator catches it, because it
  is an absence.

### The hole in the wall

[V] `crates/knx-core/src/project.rs:180` — `Project`'s six fields (`schema_version`,
`strings`, `info`, `installations`, `devices`, `ids`) are **all `pub`**. So is
`ProjectInfo`'s entire field set.

"All mutations go through `Command`" is therefore a **convention held by review, not an
invariant held by the type system**. Inside one workspace, with one test suite and one
`check-layering` gate, that is a reasonable trade. Handing `&mut Project` to a stranger is
a different proposition entirely: a plugin could set `schema_version`, drop an
installation, or rewrite a resolved value's layer, and nothing — not `apply`, not
`validation.rs`, not the undo stack — would observe it happening.

### The rule any plugin design would have to adopt

[A] Stated here so that a future design does not have to rediscover it:

1. **No plugin ever holds `&mut Project`.** Not through a `dylib`, not through a
   binding, not through a projection with interior mutability.
2. **A plugin proposes `Command`s; the core validates and applies them.** That preserves
   validation, undo/redo and the typed error surface at no design cost — the machinery
   already exists.
3. **Reads go through a projection, not the model.** `knx-projection` is already that
   shape (serde + ts-rs, no IO, `check-layering`-constrained), though it is currently
   sized for what the UI displays rather than for what a plugin might want.
4. **External authorship is visible.** Either a new `Layer` variant with its migration, or
   a provenance record naming the plugin. A change a user cannot attribute is a data
   integrity failure even when the value is correct.
5. **The opaque store is not a plugin's to touch.** It is the roundtrip guarantee; there
   is no safe partial access to it.

### Is there a sandbox in the recommended mechanism?

Yes, and it is total: the recommendation (§5.1) is that plugins run as *files this
application parses* or as *separate processes calling the CLI*. In the first case the
extension has no execution at all — it is data, validated on the way in, exactly like a
language pack or a product database. In the second, the OS is the sandbox and
`Command::apply` is still the only writer. **Neither requires the rule list above to be
built, which is the strongest practical argument for both.**

---

## 5. Recommendation

### 5.1 Recommended: no plugin API. Extension is data, plus the CLI.

Do not build a plugin host of any kind. Instead, name and document what already works:

* **Language packs** — translate the UI. Already documented for non-developers.
* **Product databases** — add manufacturers, hardware, application programs and their
  parameters. Already a runtime import, already separable by architectural rule.
* **Group-address CSV** — exchange group addresses with other tooling. Already a
  documented format.
* **The `knx` CLI** — script import, export, reporting and diffing from anything that can
  run a process. Already headless, already tested in CI.

The reasoning, in the project's own priority order:

* **Correctness and Data Integrity.** The mechanisms with real power (§2.1, §2.4) have no
  sandbox and would be handed a `Project` whose fields are all public (§4). The
  provenance model has no way to say "a plugin did this" (§4). Nothing in the four
  surfaces above can corrupt a project, because none of them executes.
* **Compatibility.** The value this application must protect is fidelity to imported
  project data ([ADR-0007](adr/0007-roundtrip-fidelity.md)). A plugin that touches the
  model is a fidelity risk with no test coverage, because the tests live in this
  repository and the plugin does not.
* **Maintainability.** CLAUDE.md's "avoid speculative abstractions" applies with unusual
  force: every candidate seam has exactly one implementation (§1), so every candidate trait
  would be designed by generalising from a sample of one. [V] `check-layering` cannot see
  past the workspace (§1.7) and `publish = false` blocks the trait route outright.
* **The licence is not doing this work.** §3.7 — AGPL is compatible with a plugin
  ecosystem. The codebase is what is not ready.

**Cost of the recommendation: one documentation page, and the discipline of keeping the
four formats stable.** It creates nothing to maintain per release.

### 5.2 The alternative, if a code extension point is later required

**An out-of-process helper speaking a documented protocol (§2.3)** — not a `cdylib`, not a
scripting engine, and not WASM-because-it-sounds-modern.

It wins on the only two axes that are non-negotiable here: crash containment is total, and
its licence position is the most defensible of the executable options (§3.3), which is what
a commercial third party would actually be asking about. It is also the only one that
composes with the existing architecture rather than fighting it, since `knx-server` already
is the process boundary.

It is blocked on four things, all of which are worth doing anyway:

1. A serialisable form of `Command` — [V] none exists; `knx-core` has no `serde` and
   `Command` derives only `Debug, Clone, PartialEq`.
2. A mature `Command` layer — [V] [ROADMAP.md](ROADMAP.md) already names this as the
   prerequisite for MCP capabilities and for the in-app LLM surface. Same blocker, third
   consumer.
3. **§22** — authentication on `knx-server`.
4. **§63** — one shared project and one shared undo stack, with no conflict detection.

[A] If all four are resolved for other reasons, the marginal cost of the plugin protocol
becomes small — which is itself an argument for not paying any of it speculatively now.

**WASM (§2.2) is the second alternative and is explicitly not ranked second-best.** It has
the better versioning story and the better in-process isolation, and it would be the right
answer for a host that must run untrusted computation. This host does not, and its licence
position is the least characterised of the five (§3.2).

### 5.3 The smallest experiment that would falsify this recommendation

The recommendation rests on one empirical claim: **there is no interface here to expose,
because every seam has exactly one implementation.** That claim is falsifiable, cheaply,
without a plugin host, without a dependency and without any public commitment:

> **Write a second implementation of one seam as an ordinary workspace crate, and see
> whether a shared trait falls out of it.**

The cheapest candidate is a second output format on the pure, IO-free report path
(`crates/knx-report`, §1.4) — the crate whose inputs and outputs are already the simplest
in the workspace. A second project-file reader would be a stronger test and a far more
expensive one.

* **If a trait falls out naturally**, fitting both implementations without contorting the
  first — then a real seam exists, it has been found by the rule of three rather than by
  guessing, and a plugin API has an honest shape to take. **This study is wrong and should
  be revised.**
* **If the trait has to be bent around the existing implementation**, or ends up so thin
  it conveys nothing — then §1's claim is confirmed at the cost of one crate, and the
  crate is still a useful feature rather than scaffolding.

[A] Either outcome is worth more than a plugin prototype, because either outcome is a
shipped capability rather than a host with nothing to host.

### 5.4 What must be true before this is reconsidered

* [V] Session 7 closed and the `Command` layer complete — [ROADMAP.md](ROADMAP.md)'s
  existing condition, reached here independently.
* [V] §22 answered: `knx-server` authenticates.
* [V] §63 answered: concurrent edits have a defined outcome.
* [V] `Project`'s invariants held by the type system, not by review (§4) — or an
  explicit decision that a plugin never receives the model.
* [A] A demonstrated demand: at least two concrete third-party extensions that the four
  existing surfaces genuinely cannot express. Until such a case exists, the honest reading
  of CLAUDE.md's rule against speculative abstractions is that a plugin system is the
  example it was written for.

---

## What this document does not claim

No ETS compatibility and no KNX certification are claimed anywhere here. The observations
about ETS's own plugin channels — device `BinaryData` described in schema 23 as "For use
by plugins", and the `ExtraData/` directory
([ADR-0019](adr/0019-building-model-stays-topological.md) §E3) — describe data this
application preserves opaquely and **never executes**. Nothing in this study proposes
executing vendor plugin payloads from a `.knxproj`, and nothing in it should be read as a
step toward doing so.

The external-technology claims in §2 are marked [D] or [A] and were not benchmarked or
prototyped here; §3's licence reading is the FSF's published interpretation plus this
study's application of it, and is not legal advice.
