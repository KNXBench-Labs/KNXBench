# Domain docs

How the engineering skills consume this repository's domain documentation.
KNXBench is a single-context repository with its glossary under `docs/`.

## Before exploring, read these

- **[`docs/GLOSSARY.md`](../GLOSSARY.md)**: terms that mean one thing in KNX and
  another on the web, each fixed to exactly one meaning in KNXBench.
- **[`docs/adr/`](../adr/)**: read the ADRs that touch the area you are about to
  work in. Later amendments are appended to the ADR they change.
- **[`docs/ARCHITECTURE.md`](../ARCHITECTURE.md)** and
  **[`docs/DATA_MODEL.md`](../DATA_MODEL.md)** for layer boundaries and the
  domain model.

There is no root `GLOSSARY.md` or `GLOSSARY-MAP.md`; do not create one. When
`/domain-modeling` resolves a new term, add it to `docs/GLOSSARY.md`. A new
decision becomes the next numbered ADR in `docs/adr/`, in the existing format.

## Use the glossary's vocabulary

When your output names a domain concept (an issue title, a refactor proposal,
a hypothesis, a test name), use the term as defined in `docs/GLOSSARY.md`. Do
not drift to synonyms the glossary rules out; for example, "download" always
means writing to the device, never saving a file.

If the concept you need is not in the glossary yet, either you are inventing
language the project does not use (reconsider) or there is a real gap (note it
for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, say so explicitly rather than
silently overriding it:

> _Contradicts ADR-0039 (project mutation goes through commands), but worth reopening because…_
