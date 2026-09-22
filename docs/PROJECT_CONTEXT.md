# KNXBench Project Context

## Repository authority

When sources disagree, use this order:

1. Source code, tests, and the current Git state.
2. Architecture decision records and focused technical documentation.
3. `IMPLEMENTATION_STATUS.md`, `KNOWN_LIMITATIONS.md`, and `ROADMAP.md`.
4. The generated local memory index, as a discovery aid only.
5. Private Claude, Codex, and Hermes memories, as provenance sources but never
   authority by themselves.

The shared index does not merge or overwrite private agent stores. It provides
short topic summaries and links back to their owning notes. Generated files are
read-only and remain local under `.agent-memory/`.

## Promoting durable knowledge

Before treating a memory item as project truth, verify it against current code,
tests, Git history, or other direct evidence. Update the responsible repository
document, then correct or retire the private note that led to it. Do not edit the
generated index by hand.
