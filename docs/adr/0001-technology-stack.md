# ADR 0001: Technology stack — Rust core, Tauri, React, SQLite

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

A project carries about 22 MB of unpacked application program XML, with single
files up to 5.7 MB (RESEARCH §4.1). Opening a project therefore means streaming
and indexing, not loading a document tree into memory and hoping.

The provenance model (ADR-0004) is a type-level idea: every resolved value
carries the layer it came from, and the compiler is the cheapest place to check
that the layer travels with the value.

`xknxproject`, the Python library used for Session 0 research, is GPL-2.0-only
(RESEARCH §10). It must not enter the runtime dependency graph of an
application we intend to license separately.

## Decision

The core is written in Rust. The desktop application is a Tauri shell with a
React and TypeScript user interface. Project and product data are stored in
SQLite. Code, comments and documentation are written in English.

The Rust side is split into a pure domain crate (`knx-core`), service and
infrastructure crates around it, and thin application entry points. The choice
of Rust for everything below the UI is what makes the two mechanical gates in
this session possible at all: a dependency-graph check that keeps `knx-core`
free of IO, and a licence check over the whole runtime graph.

## Alternatives considered

**Python with PySide6.** The fastest possible start, and it would continue
directly from the Session 0 research scripts. Rejected because the product
database work is the heaviest part of the project and Python's story for a
large, indexed, concurrent local database is weaker; and because packaging a
Python desktop application for Linux distribution drags in problems that have
nothing to do with KNX.

**TypeScript with Electron.** One language across the whole stack. Rejected
because the parts that hurt — 5.7 MB XML files, a product database, binary
protocol handling — are exactly what the runtime is worst at, and because there
is no usable KNX stack in the ecosystem: KNXnet/IP would have to be written
from scratch regardless, which removes the only reason to prefer it.

## Consequences

Two languages meet at the UI boundary, so that boundary needs generated
bindings rather than hand-written ones (ADR-0009).

The GPL exposure disappears at the root: no Python KNX library is in the
runtime graph, and `cargo-deny` proves it on every CI run (ADR-0002).

A Rust toolchain is a prerequisite for building anything, and the toolchain
version is pinned in `rust-toolchain.toml` so that CI and developer machines
agree.
