# KNXBench Project Context

## Repository authority

When sources disagree, use this order:

1. Source code, tests, and the current Git state.
2. Architecture decision records and focused technical documentation.
3. `IMPLEMENTATION_STATUS.md`, `KNOWN_LIMITATIONS.md`, and `ROADMAP.md`.
4. Private Claude, Codex, and Hermes memories, as provenance sources but never
   authority by themselves.

The generated shared memory index (`.agent-memory/`, `tools/agent_memory_sync.py`)
was retired on 2026-10-08; there is no shared index any more.

## Promoting durable knowledge

Before treating a memory item as project truth, verify it against current code,
tests, Git history, or other direct evidence. Update the responsible repository
document, then correct or retire the private note that led to it.

## Evidence for device support

ADR-0086 (maintainer decision, 2026-10-07): the KNX specification, product
databases and project files are enough evidence. Where they are silent but a
working solution exists, implement it as a named inference and disclose it
in readiness and the download acknowledgement; a real device run is needed
only for **Verified**. Aim for as many working devices as possible, not only
those the maintainer owns.
