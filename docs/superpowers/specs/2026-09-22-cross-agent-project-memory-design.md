# Cross-Agent Project Memory Design

## Purpose

KNXBench is developed alternately with Claude Code, Codex, and Hermes. Each
agent currently has a separate private memory store. Project facts are shared
only when an agent deliberately copies them into repository documentation or
the local handover files. The goal is to give all three agents one compact,
local project-memory index without merging their private stores, exposing
credentials, or turning stale chat history into project truth.

Success means:

- Claude, Codex, and Hermes can discover the same current memory topics.
- Every shared entry identifies its original file and owning agent.
- Exact duplicates are collapsed and possible topic conflicts are visible.
- Synchronization is previewable, deterministic, atomic, and reversible.
- Repository documentation remains authoritative for product and architecture
  facts.

## Existing State and Constraints

- Claude's curated KNXBench memory lives under
  `~/.claude/projects/-mnt-daten-i-Sourcecode-KNXBench/memory/` and currently
  contains about 132 KiB of Markdown.
- Codex's curated project memory lives under `ai/codex/memory/` and currently
  contains about 32 KiB of Markdown. The repository-root `MEMORY.md` is a local
  pointer to that store.
- Hermes has a profile-local built-in memory store at
  `~/.hermes/profiles/knxbench/memories/`. It is enabled, but no project memory
  file exists there yet.
- Hermes already provides `hermes import-agent`, including preview, backups,
  exact-entry de-duplication, and secret exclusion. It is a one-time setup
  importer, however: it does not watch project-specific Claude memory and does
  not create a shared cross-agent project index.
- `.ai/` handover maintenance is suspended by the active `goal.md`; the new
  system must not write `.ai/CURRENT_STATE.md` or `.ai/logs/`.
- `docs/` is the source of truth. Generated memory must never silently replace
  an ADR, compatibility statement, implementation status, or verified KNX
  evidence.

## Chosen Architecture

### 1. Separate private stores from shared project memory

Private stores remain owned by their agents and are read-only inputs:

```text
Claude curated memory ----\
Codex curated memory ------+--> memory-sync --> local shared index
Hermes curated memory -----/                     + conflict report
```

The synchronizer never rewrites a source file. It does not read Claude/Codex
session JSONL, Hermes `state.db`, credentials, tool transcripts, or raw chat
history.

### 2. Two shared layers

The implementation introduces two distinct shared layers:

1. `docs/PROJECT_CONTEXT.md` is a small, versioned guide to the authoritative
   KNXBench documents and the rules for promoting durable facts. It is edited
   deliberately, never generated from private memory.
2. `.agent-memory/PROJECT_MEMORY.md` is a generated, local, Git-ignored index
   of curated private memory topics. `.agent-memory/REPORT.md` records skipped
   sources, exact duplicates, and conflicts from the latest run.

The generated index contains topic summaries and source paths, not a full
concatenation of all memory files. Agents load an original note only when its
topic is relevant. This prevents the current roughly 164 KiB input set from
being injected into every prompt.

### 3. Topic extraction and provenance

`tools/agent_memory_sync.py` uses only the Python standard library. For each
Markdown source it records:

- owner (`claude`, `codex`, or `hermes`);
- absolute source path;
- source modification time and SHA-256 digest;
- topic key derived from a normalized filename, falling back to the first
  level-one heading;
- title and the first substantive paragraph or bullet as a bounded summary.

Generated entries link to the original file and retain their source digest.
No model-generated interpretation is used during synchronization.

### 4. De-duplication and conflict policy

- Byte-identical or normalized-text-identical notes are one index entry with
  multiple provenance records.
- Different notes with the same normalized topic key are not merged. They are
  listed together and marked `CONFLICT` in both the index and report.
- Different topic keys are independent even when their text appears related;
  semantic merging would be guesswork and is outside this tool.
- A conflict never blocks generation of unrelated entries, but the command
  exits with a distinct non-zero status in `--check` mode so automation and
  agents cannot mistake the index for conflict-free.
- Resolving a conflict means updating the owning private notes or promoting the
  verified fact into repository documentation, then running synchronization
  again. The generated file is never edited by hand.

### 5. Privacy and trust boundary

The tool accepts only configured `.md` files below the three allowlisted source
roots. It rejects symlinks escaping those roots and skips files matching common
credential names. Before output, it scans candidate summaries for private-key
blocks and secret-like assignments (`*_TOKEN`, `*_SECRET`, `*_PASSWORD`,
`*_API_KEY`). A skipped item is reported without copying its value.

The source files themselves may contain private project context, so
`.agent-memory/` remains local and ignored. Nothing from it is committed or
sent to an external service by the synchronizer.

### 6. Preview, apply, and atomicity

The command surface is intentionally small:

```text
python3 tools/agent_memory_sync.py preview
python3 tools/agent_memory_sync.py apply
python3 tools/agent_memory_sync.py check
```

- `preview` prints the proposed index/report and writes nothing.
- `apply` writes temporary files beside their destinations, flushes them, then
  replaces `PROJECT_MEMORY.md`, `REPORT.md`, and `manifest.json` atomically.
- `check` compares generated content with disk and returns non-zero for stale
  output, unsafe content, unreadable configured sources, or topic conflicts.
- If generation fails, the prior complete output remains untouched.

`manifest.json` stores schema version, source roots, source hashes, generation
time, and output hash. It contains no memory text.

### 7. Automatic refresh

A companion command installs a reversible user-level systemd timer for this
Linux-first workstation:

```text
python3 tools/agent_memory_sync.py install-timer
python3 tools/agent_memory_sync.py uninstall-timer
```

The timer runs `apply` every five minutes and after login. Installation writes
only two units under `~/.config/systemd/user/`; uninstall removes only those
known unit files. The service is single-shot and uses a file lock under
`.agent-memory/` to prevent overlapping writers. Timer failure leaves the last
complete index intact and is visible through `systemctl --user status` and the
tool's report.

The timer does not invoke an LLM and therefore adds no model cost. Manual
`preview`, `apply`, and `check` remain fully usable when systemd is unavailable.

### 8. Agent integration

Local agent instruction files will point to the same generated index:

- Codex's root `MEMORY.md` points to `.agent-memory/PROJECT_MEMORY.md` and
  `docs/PROJECT_CONTEXT.md`.
- Claude's local `CLAUDE.md` instructs main sessions to read the shared index
  after repository documentation.
- The Hermes `knxbench` profile instruction file gives the same rule.

These local edits are part of installation state, not repository product
history. The tool exposes `install-agent-links` and `uninstall-agent-links`
commands that update only bounded, marker-delimited blocks and preserve the
surrounding user-authored files. Installation creates timestamped backups;
uninstall removes the marked blocks and restores no unrelated content.

Hermes continues to own its `memories/MEMORY.md`; it is an input, not a symlink
to generated output. This avoids a feedback loop and prevents Hermes' memory
writer from overwriting the shared index.

## Error Handling

- Missing optional source roots are reported and otherwise harmless.
- An unreadable existing source is an error; synchronization does not publish a
  partial replacement while a configured source is unreadable.
- Malformed UTF-8 is decoded with replacement and reported as a warning.
- Oversized files are skipped with a visible reason; the initial per-file cap is
  256 KiB.
- Unknown generated schema versions fail closed rather than being overwritten.
- Concurrent `apply` operations serialize through an advisory lock.

## Testing and Verification

Tests use temporary directories and real files; no agent home directory is
modified during test runs. Coverage includes:

- extraction from representative Claude, Codex, and Hermes Markdown;
- stable ordering and deterministic output;
- exact and normalized duplicate collapse;
- same-topic conflict reporting;
- secret and escaping-symlink rejection;
- missing and unreadable source behavior;
- stale `check` detection;
- atomic replacement preservation after injected failure;
- lock behavior for concurrent writers;
- idempotent install/uninstall of timer units and instruction blocks.

Repository verification will run the focused Python tests, a real `preview`
against the current KNXBench stores, `apply`, `check`, and validation that the
generated directory remains ignored by Git. No KNX network or hardware access
is involved.

## Rollback

Rollback is explicit and local:

1. Run `uninstall-timer` and `uninstall-agent-links`.
2. Remove the generated `.agent-memory/` directory if its local history is no
   longer wanted.
3. Revert the tracked script, tests, `.gitignore` entry, and
   `docs/PROJECT_CONTEXT.md` normally through Git.

Private Claude, Codex, and Hermes source memories are never modified, so
rollback cannot lose their original information.

## Non-Goals

- No automatic semantic reconciliation by an LLM.
- No ingestion of raw conversations or tool transcripts.
- No synchronization between machines or cloud storage.
- No replacement for `docs/`, ADRs, Git history, or test evidence.
- No automatic promotion of private memory into committed documentation.
