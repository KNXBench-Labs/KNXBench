# Cross-Agent Project Memory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a deterministic, privacy-preserving local memory index that lets
Claude, Codex, and Hermes discover the same KNXBench memory topics without
merging or overwriting their private stores.

**Architecture:** A standard-library Python command scans allowlisted curated
Markdown roots, extracts bounded provenance-rich topic summaries, reconciles
exact duplicates and topic collisions, then publishes immutable snapshots
behind one atomic `current` symlink. Separate reversible installers copy the
tool to a stable user location, install a systemd timer, and add bounded local
instruction blocks.

**Tech Stack:** Python 3.11 standard library (`argparse`, `dataclasses`,
`hashlib`, `json`, `fcntl`, `pathlib`, `tempfile`, `unittest`), Markdown,
systemd user units, Git.

**Spec:**
`docs/superpowers/specs/2026-09-22-cross-agent-project-memory-design.md`

## Global Constraints

- Private Claude, Codex, and Hermes stores are read-only inputs.
- Never read session JSONL, Hermes `state.db`, credentials, raw tool output, or
  `.ai/` handover files.
- `docs/` remains authoritative; generated output never promotes facts into
  committed documentation.
- Use Python's standard library only; add no runtime dependency.
- Accept only `.md` files resolving beneath configured allowlisted roots.
- Skip secret-like content without copying the suspected value into output or
  logs.
- Cap each source file at 256 KiB.
- Publish a complete old or new snapshot through one atomic `current` symlink
  swap; never expose a mixed generation.
- Keep `.agent-memory/` ignored and local.
- No KNX network, gateway, multicast, LAN scan, or hardware access.
- Commits use KNXBench `<github@knxbench.com>` and no co-author trailer.

## Review Focus

- A symlink inside an allowlisted root that resolves outside it must be skipped
  without reading the target; Task 1 pins this with
  `test_discovery_rejects_escaping_symlink`.
- Two different notes with the same normalized topic must remain separate and
  visible as a conflict; Task 2 pins this with
  `test_reconcile_preserves_same_topic_conflict`.
- A failure before publication must leave the previous `current` snapshot
  active; Task 3 pins this with
  `test_publish_failure_preserves_current_snapshot`.
- Installer reruns must not duplicate marker blocks or overwrite unrelated
  instructions; Task 4 pins this with
  `test_agent_link_install_is_idempotent_and_preserves_surroundings`.
- The systemd service must execute a stable installed copy rather than a
  disposable worktree path; Task 4 pins this with
  `test_timer_service_targets_stable_copy`.

---

### Task 1: Safe source discovery and bounded topic extraction

**Files:**

- Create: `tools/agent_memory_sync.py`
- Create: `tools/tests/__init__.py`
- Create: `tools/tests/test_agent_memory_sync.py`

**Interfaces:**

- Produces `SourceRoot(owner: str, path: Path, required: bool = False)`; CLI
  defaults are optional and explicit `--source` roots are required.
- Produces `MemoryNote(owner, source_path, relative_path, digest, mtime_ns,
  topic, title, summary, normalized_text)`.
- Produces `ScanResult(notes, warnings, skipped, errors)`.
- Produces `scan_sources(source_roots, *, max_file_bytes=262_144) -> ScanResult`.
- Later tasks consume `MemoryNote` without reopening source files.

- [ ] **Step 1: Create the test package and write failing extraction tests**

```python
class ScanSourcesTests(unittest.TestCase):
    def test_extracts_bounded_markdown_topic_with_provenance(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp) / "claude"
            root.mkdir()
            note = root / "KNX Safety.md"
            note.write_text(
                "# KNX safety\n\nNever write to live hardware without approval.\n",
                encoding="utf-8",
            )

            result = scan_sources([SourceRoot("claude", root)])

            self.assertEqual(len(result.notes), 1)
            self.assertEqual(result.notes[0].topic, "knx-safety")
            self.assertEqual(result.notes[0].title, "KNX safety")
            self.assertEqual(
                result.notes[0].summary,
                "Never write to live hardware without approval.",
            )
            self.assertEqual(result.notes[0].owner, "claude")
            self.assertEqual(result.notes[0].relative_path, Path("KNX Safety.md"))
            self.assertEqual(len(result.notes[0].digest), 64)

    def test_discovery_rejects_escaping_symlink(self):
        with TemporaryDirectory() as tmp:
            base = Path(tmp)
            root = base / "codex"
            root.mkdir()
            outside = base / "outside.md"
            outside.write_text("# Secret\n\noutside\n", encoding="utf-8")
            (root / "escape.md").symlink_to(outside)

            result = scan_sources([SourceRoot("codex", root)])

            self.assertEqual(result.notes, ())
            self.assertEqual(result.skipped[0].reason, "path escapes source root")

    def test_discovery_skips_oversized_file(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "large.md").write_text("x" * 33, encoding="utf-8")

            result = scan_sources(
                [SourceRoot("hermes", root)], max_file_bytes=32
            )

            self.assertEqual(result.notes, ())
            self.assertEqual(result.skipped[0].reason, "file exceeds 32 bytes")
```

- [ ] **Step 2: Run the focused tests and verify the expected import failure**

Run:

```bash
python3 -m unittest \
  tools.tests.test_agent_memory_sync.ScanSourcesTests -v
```

Expected: FAIL because `tools.agent_memory_sync` does not exist.

- [ ] **Step 3: Implement immutable records, safe traversal, and extraction**

Implement these exact public shapes:

```python
MAX_FILE_BYTES = 262_144

@dataclass(frozen=True)
class SourceRoot:
    owner: str
    path: Path
    required: bool = False

@dataclass(frozen=True)
class SkippedSource:
    owner: str
    path: Path
    reason: str

@dataclass(frozen=True)
class MemoryNote:
    owner: str
    source_path: Path
    relative_path: Path
    digest: str
    mtime_ns: int
    topic: str
    title: str
    summary: str
    normalized_text: str

@dataclass(frozen=True)
class ScanResult:
    notes: tuple[MemoryNote, ...]
    warnings: tuple[str, ...]
    skipped: tuple[SkippedSource, ...]
    errors: tuple[str, ...]

def scan_sources(
    source_roots: Sequence[SourceRoot],
    *,
    max_file_bytes: int = MAX_FILE_BYTES,
) -> ScanResult:
    notes: list[MemoryNote] = []
    warnings: list[str] = []
    skipped: list[SkippedSource] = []
    errors: list[str] = []
    for source in sorted(source_roots, key=lambda item: (item.owner, str(item.path))):
        scan_one_root(source, notes, warnings, skipped, errors, max_file_bytes)
    return ScanResult(tuple(notes), tuple(warnings), tuple(skipped), tuple(errors))
```

Also define
`scan_one_root(source, notes, warnings, skipped, errors, max_file_bytes) ->
None` and
`extract_note(owner, root, candidate, raw, stat_result) -> MemoryNote`; neither
function may reopen an accepted file.

Traversal requirements:

- Sort roots by `(owner, str(path))` and files by relative POSIX path.
- A missing optional root adds a warning; a missing or unreadable required root
  adds an error. Never downgrade a required-root read failure to partial output.
- Resolve each root once. For each candidate, resolve it before opening and
  require `candidate_resolved.is_relative_to(root_resolved)`.
- Read only regular `.md` files.
- Use `lstat().st_size` before reading and `read_bytes()` once accepted.
- Decode UTF-8 with `errors="replace"`; append a warning when U+FFFD occurs.
- Derive topic from normalized filename stem; fall back to the first H1 only
  when the stem normalizes to an empty string.
- Bound title to 120 characters and summary to 400 characters.
- Summary is the first non-heading, non-empty paragraph or bullet, with
  whitespace collapsed.
- Add focused tests named `test_missing_optional_root_is_warning`,
  `test_unreadable_configured_root_is_error`,
  `test_malformed_utf8_is_replaced_and_reported`, and
  `test_scan_order_is_stable_across_creation_order`.

- [ ] **Step 4: Run Task 1 tests and verify they pass**

Run the command from Step 2.

Expected: all `ScanSourcesTests` pass.

- [ ] **Step 5: Commit Task 1**

```bash
git add tools/agent_memory_sync.py tools/tests
git commit -m "feat(memory): catalog the agents' private notebooks"
```

### Task 2: Secret filtering, duplicate collapse, conflicts, and rendering

**Files:**

- Modify: `tools/agent_memory_sync.py`
- Modify: `tools/tests/test_agent_memory_sync.py`

**Interfaces:**

- Consumes `MemoryNote` and `ScanResult` from Task 1.
- Produces `TopicEntry(topic: str, notes: tuple[MemoryNote, ...], conflict:
  bool)` and
  `Reconciliation(entries, duplicate_groups, conflicts, warnings, skipped,
  errors)`.
- Produces `reconcile(scan: ScanResult) -> Reconciliation`.
- Produces `render_index(result) -> str`, `render_report(result) -> str`, and
  `build_manifest(result, source_roots, generated_at) -> dict[str, object]`.
- Test helpers are `make_note(owner: str, filename: str, text: str) ->
  MemoryNote` and `make_scan(*notes: MemoryNote) -> ScanResult`; they construct
  complete immutable records and do not bypass production normalization.

- [ ] **Step 1: Write failing reconciliation and privacy tests**

```python
class ReconciliationTests(unittest.TestCase):
    def test_reconcile_collapses_normalized_exact_duplicates(self):
        first = make_note("claude", "safety.md", "# Safety\n\nNo bus writes.")
        second = make_note("codex", "safety-copy.md", "# SAFETY\n\nNo   bus writes.")

        result = reconcile(make_scan(first, second))

        self.assertEqual(len(result.entries), 1)
        self.assertEqual(len(result.entries[0].notes), 2)
        self.assertFalse(result.entries[0].conflict)
        self.assertEqual(result.duplicate_groups, 1)

    def test_reconcile_preserves_same_topic_conflict(self):
        first = make_note("claude", "gateway.md", "# Gateway\n\nUse address A.")
        second = make_note("codex", "gateway.md", "# Gateway\n\nUse address B.")

        result = reconcile(make_scan(first, second))

        self.assertEqual(len(result.entries[0].notes), 2)
        self.assertTrue(result.entries[0].conflict)
        self.assertEqual(result.conflicts, ("gateway",))
        self.assertIn("CONFLICT", render_index(result))

    def test_secret_like_summary_is_skipped_without_value_leak(self):
        secret = make_note(
            "hermes", "credentials.md", "# Login\n\nOPENAI_API_KEY=sk-example-value"
        )

        result = reconcile(make_scan(secret))
        report = render_report(result)

        self.assertEqual(result.entries, ())
        self.assertIn("secret-like content", report)
        self.assertNotIn("sk-example-value", report)
```

- [ ] **Step 2: Run the new test class and verify missing-symbol failures**

```bash
python3 -m unittest \
  tools.tests.test_agent_memory_sync.ReconciliationTests -v
```

Expected: FAIL because reconciliation functions do not exist.

- [ ] **Step 3: Implement deterministic reconciliation and renderers**

Use exact-text normalization only:

```python
def normalize_text(value: str) -> str:
    return " ".join(value.casefold().split())

def reconcile(scan: ScanResult) -> Reconciliation:
    safe = tuple(note for note in scan.notes if not is_secret_like(note.summary))
    entries: list[TopicEntry] = []
    for topic, topic_notes in itertools.groupby(safe, key=lambda note: note.topic):
        notes = tuple(topic_notes)
        variants = {note.normalized_text for note in notes}
        entries.append(TopicEntry(topic, notes, conflict=len(variants) > 1))
    conflicts = tuple(entry.topic for entry in entries if entry.conflict)
    return Reconciliation.from_scan(scan, tuple(entries), conflicts)
```

Sort `safe` by `(topic, normalized_text, owner, relative_path.as_posix())`
before `groupby`. `Reconciliation.from_scan` removes secret-like notes from the
entry set, adds generic skipped records, and counts topics containing one
normalized variant with multiple provenance records as duplicate groups.

Secret detection must cover private-key markers and assignment-like keys ending
in `TOKEN`, `SECRET`, `PASSWORD`, `PASSWD`, `API_KEY`, `PRIVATE_KEY`, or
`ACCESS_KEY`. Reports name only owner and relative path plus the generic reason.

Render requirements:

- Begin generated files with a do-not-edit warning.
- Sort topics and provenance deterministically.
- Link absolute source paths as plain Markdown links.
- Show title, bounded summary, owner, relative path, digest prefix, and mtime.
- Put a visible `CONFLICT` label before conflicting variants.
- Report counts for scanned, indexed, duplicate, conflict, skipped, and warning
  items.
- Manifest keys are `schema_version`, `generated_at`, `sources`, `entries`,
  `conflicts`, `warnings`, `skipped`, and `output_sha256`.
- `output_sha256` is the SHA-256 of `index + "\0" + report`; it does not hash
  the manifest containing itself.

- [ ] **Step 4: Run Task 1 and Task 2 tests**

```bash
python3 -m unittest tools.tests.test_agent_memory_sync -v
```

Expected: all tests pass.

- [ ] **Step 5: Commit Task 2**

```bash
git add tools/agent_memory_sync.py tools/tests/test_agent_memory_sync.py
git commit -m "feat(memory): flag disagreements without inventing consensus"
```

### Task 3: Snapshot publication, locking, and CLI behavior

**Files:**

- Modify: `tools/agent_memory_sync.py`
- Modify: `tools/tests/test_agent_memory_sync.py`

**Interfaces:**

- Consumes renderers from Task 2.
- Produces `SyncPaths(output_root, snapshots, current, index_view,
  report_view, manifest_view, lock_file)` and
  `SyncPaths.for_project(project_root: Path) -> SyncPaths`.
- Produces `Generation(index: str, report: str, manifest_json: str,
  output_sha256: str, conflicts: tuple[str, ...], errors: tuple[str, ...])` and
  `generate(project_root, source_roots, generated_at) -> Generation`.
- Produces `publish(paths, generation, *, fail_before_swap=False) -> Path`.
- Produces CLI subcommands `preview`, `apply`, and `check`.
- Test helper `MemoryFixture` owns temporary `project`, `source`, `note`,
  `output`, and `paths` attributes. Its `cli_args(command)` returns the command,
  `--project-root`, and an explicit `--source codex=<temporary-source>` so tests
  never inspect real home directories.

- [ ] **Step 1: Write failing publication and CLI tests**

```python
class PublicationTests(unittest.TestCase):
    def test_apply_publishes_three_views_from_one_snapshot(self):
        with temporary_project_with_note() as fixture:
            exit_code = main(fixture.cli_args("apply"))

            self.assertEqual(exit_code, 0)
            output = fixture.project / ".agent-memory"
            current = (output / "current").resolve()
            self.assertEqual((output / "PROJECT_MEMORY.md").resolve().parent, current)
            self.assertEqual((output / "REPORT.md").resolve().parent, current)
            self.assertEqual((output / "manifest.json").resolve().parent, current)

    def test_publish_failure_preserves_current_snapshot(self):
        with temporary_project_with_note() as fixture:
            first = generate(
                fixture.project,
                (SourceRoot("codex", fixture.source),),
                "2026-09-22T00:00:00Z",
            )
            publish(fixture.paths, first)
            current_before = os.readlink(fixture.output / "current")
            generation = generation_with_title("new title")

            with self.assertRaises(PublishError):
                publish(fixture.paths, generation, fail_before_swap=True)

            self.assertEqual(os.readlink(fixture.output / "current"), current_before)
            self.assertEqual(
                (fixture.output / "PROJECT_MEMORY.md").read_text(), first.index
            )

    def test_check_reports_stale_and_conflicted_states(self):
        with temporary_project_with_note() as fixture:
            self.assertEqual(main(fixture.cli_args("apply")), EXIT_OK)
            fixture.note.write_text("# Changed\n\nnew value\n", encoding="utf-8")
            self.assertEqual(main(fixture.cli_args("check")), EXIT_STALE)
            add_conflicting_note(fixture)
            self.assertEqual(main(fixture.cli_args("check")), EXIT_CONFLICT)
```

Add `test_exclusive_lock_blocks_second_writer_until_release` using two
processes and a bounded event/queue handshake, plus
`test_unknown_manifest_schema_fails_closed_without_publication`.

- [ ] **Step 2: Run publication tests and verify failures**

```bash
python3 -m unittest \
  tools.tests.test_agent_memory_sync.PublicationTests -v
```

Expected: FAIL because publication and CLI functions do not exist.

- [ ] **Step 3: Implement locking and snapshot publication**

Define exit codes explicitly:

```python
EXIT_OK = 0
EXIT_ERROR = 1
EXIT_STALE = 2
EXIT_CONFLICT = 3

@contextmanager
def exclusive_lock(lock_path: Path) -> Iterator[None]:
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+", encoding="utf-8") as handle:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        yield
```

Publication sequence:

1. Create `.agent-memory/snapshots/<output-sha256>/` with mode `0700`.
2. Write all files with mode `0600`, flush, and `os.fsync()` each file.
3. `fsync()` the snapshot directory.
4. Create `current.next` symlink to the relative snapshot path.
5. Replace `current` with `os.replace(current.next, current)` and fsync the
   `.agent-memory` directory.
6. Create stable relative symlinks for the three top-level views once, replacing
   only an absent or tool-owned link.
7. Remove incomplete unreferenced snapshot directories on the next successful
   apply, but never follow symlinks and never remove `current`'s target.

- [ ] **Step 4: Implement CLI defaults and stale comparison**

Default source roots for a project at `/mnt/daten-i/Sourcecode/KNXBench`:

```python
def default_source_roots(project_root: Path, home: Path) -> tuple[SourceRoot, ...]:
    claude_slug = "-" + str(project_root.resolve()).lstrip("/").replace("/", "-")
    return (
        SourceRoot("claude", home / ".claude" / "projects" / claude_slug / "memory"),
        SourceRoot("codex", project_root / "ai" / "codex" / "memory"),
        SourceRoot(
            "hermes",
            home / ".hermes" / "profiles" / "knxbench" / "memories",
        ),
    )
```

Allow repeated `--source OWNER=PATH` to replace defaults in tests and advanced
use. `preview` writes index then report to stdout and returns conflict status
when conflicts exist. `check` regenerates in memory, compares all output hashes
to the active manifest, and uses the exit-code precedence error > conflict >
stale > success.

- [ ] **Step 5: Run all Python tests**

```bash
python3 -m unittest tools.tests.test_agent_memory_sync -v
```

Expected: all tests pass.

- [ ] **Step 6: Commit Task 3**

```bash
git add tools/agent_memory_sync.py tools/tests/test_agent_memory_sync.py
git commit -m "feat(memory): swap complete snapshots without torn pages"
```

### Task 4: Reversible timer and local agent-link installation

**Files:**

- Modify: `tools/agent_memory_sync.py`
- Modify: `tools/tests/test_agent_memory_sync.py`

**Interfaces:**

- Produces CLI subcommands `install-timer`, `uninstall-timer`,
  `install-agent-links`, and `uninstall-agent-links`.
- Produces `upsert_marked_block`, `remove_marked_block`, `install_timer`, and
  `uninstall_timer` functions with injectable filesystem roots and command
  runner for tests.

- [ ] **Step 1: Write failing installer tests**

```python
class InstallerTests(unittest.TestCase):
    def test_timer_service_targets_stable_copy(self):
        with TemporaryDirectory() as tmp:
            home = Path(tmp)
            calls: list[list[str]] = []
            install_timer(
                project_root=Path("/project"),
                home=home,
                script_path=Path("/worktree/tools/agent_memory_sync.py"),
                runner=lambda argv: calls.append(argv),
            )

            service = (
                home / ".config/systemd/user/knxbench-memory-sync.service"
            ).read_text(encoding="utf-8")
            stable = home / ".local/lib/knxbench-memory-sync/agent_memory_sync.py"
            self.assertTrue(stable.is_file())
            self.assertIn(str(stable), service)
            self.assertNotIn("/worktree/", service)
            self.assertIn("--project-root /project", service)
            self.assertEqual(calls[-1], ["systemctl", "--user", "enable", "--now", "knxbench-memory-sync.timer"])

    def test_agent_link_install_is_idempotent_and_preserves_surroundings(self):
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / "AGENTS.md"
            path.write_text("before\nafter\n", encoding="utf-8")

            install_agent_links(project_root=Path(tmp), hermes_home=Path(tmp) / "h")
            first = path.read_text(encoding="utf-8")
            install_agent_links(project_root=Path(tmp), hermes_home=Path(tmp) / "h")

            self.assertEqual(path.read_text(encoding="utf-8"), first)
            self.assertIn("before", first)
            self.assertIn("after", first)
            self.assertEqual(first.count(MARKER_START), 1)
```

Add `test_timer_install_and_uninstall_are_idempotent`,
`test_agent_link_install_creates_backup_before_first_change`, and
`test_agent_link_uninstall_removes_only_marked_block`.

- [ ] **Step 2: Run installer tests and verify failures**

```bash
python3 -m unittest \
  tools.tests.test_agent_memory_sync.InstallerTests -v
```

Expected: FAIL because installer functions do not exist.

- [ ] **Step 3: Implement stable-copy systemd installation**

Use exact installed locations:

```text
~/.local/lib/knxbench-memory-sync/agent_memory_sync.py
~/.config/systemd/user/knxbench-memory-sync.service
~/.config/systemd/user/knxbench-memory-sync.timer
```

The service is `Type=oneshot` and runs the stable copy with `apply`, explicit
`--project-root`, and explicit `--home`. The timer uses:

```ini
[Timer]
OnBootSec=2min
OnUnitActiveSec=5min
Persistent=true
Unit=knxbench-memory-sync.service
```

Installation writes via temporary file plus `os.replace`, then calls:

```text
systemctl --user daemon-reload
systemctl --user enable --now knxbench-memory-sync.timer
```

Uninstall first calls `disable --now`, removes only the three exact known files,
prunes only the now-empty private install directory, and calls `daemon-reload`.
It does not remove `.agent-memory/`.

- [ ] **Step 4: Implement bounded local instruction blocks**

Use markers:

```text
<!-- BEGIN KNXBENCH SHARED MEMORY -->
<!-- END KNXBENCH SHARED MEMORY -->
```

The block tells agents to read `docs/PROJECT_CONTEXT.md`, then
`.agent-memory/PROJECT_MEMORY.md`, and open only relevant linked source notes.
It states that generated files are read-only and repository docs win conflicts.

Targets are the ignored project `AGENTS.md`, ignored project `MEMORY.md`, and
`~/.hermes/profiles/knxbench/SOUL.md`. Before changing an existing target,
create a sibling `.bak.<unix-nanoseconds>` with `shutil.copy2`. Reinstalling an
identical block makes no backup and no write. Uninstall removes only the marked
block and leaves the rest byte-for-byte except the single joining newline.

- [ ] **Step 5: Run all Python tests**

```bash
python3 -m unittest tools.tests.test_agent_memory_sync -v
```

Expected: all tests pass.

- [ ] **Step 6: Commit Task 4**

```bash
git add tools/agent_memory_sync.py tools/tests/test_agent_memory_sync.py
git commit -m "feat(memory): keep the shared notebook quietly refreshed"
```

### Task 5: Versioned project guidance and operator documentation

**Files:**

- Create: `docs/PROJECT_CONTEXT.md`
- Modify: `.gitignore:28`
- Modify: `CLAUDE.md:55`
- Modify: `README.md` in the development/tooling section
- Modify: `tools/tests/test_agent_memory_sync.py`

**Interfaces:**

- Documents the authority order consumed by all agents.
- Ensures Git never stages `.agent-memory/` output.
- Gives Claude the same shared-index read rule installed locally for Codex and
  Hermes.

- [ ] **Step 1: Write a failing repository-contract test**

```python
class RepositoryContractTests(unittest.TestCase):
    def test_project_files_define_shared_memory_contract(self):
        root = Path(__file__).resolve().parents[2]
        context = (root / "docs/PROJECT_CONTEXT.md").read_text(encoding="utf-8")
        claude = (root / "CLAUDE.md").read_text(encoding="utf-8")
        gitignore = (root / ".gitignore").read_text(encoding="utf-8")

        self.assertIn("Repository documentation wins", context)
        self.assertIn(".agent-memory/PROJECT_MEMORY.md", claude)
        self.assertIn(".agent-memory/", gitignore.splitlines())
```

- [ ] **Step 2: Run the contract test and verify it fails on missing context**

```bash
python3 -m unittest \
  tools.tests.test_agent_memory_sync.RepositoryContractTests -v
```

Expected: FAIL because `docs/PROJECT_CONTEXT.md` does not exist.

- [ ] **Step 3: Add the concise authority guide**

`docs/PROJECT_CONTEXT.md` must define this order:

1. Source code, tests, and current Git state.
2. ADRs and focused technical documentation.
3. `IMPLEMENTATION_STATUS.md`, `KNOWN_LIMITATIONS.md`, and `ROADMAP.md`.
4. Generated local memory index as a discovery aid only.
5. Private agent memories as provenance sources, never authority by themselves.

It must explain promotion: verify a candidate against current code/evidence,
update the responsible document, then correct or retire the private note.

- [ ] **Step 4: Wire tracked Claude guidance, ignore output, and document commands**

Add one short section to `CLAUDE.md` instructing main sessions to read
`docs/PROJECT_CONTEXT.md` and the generated index, without loading every linked
note. Add `.agent-memory/` to `.gitignore`. Add README commands for preview,
apply, check, timer installation, link installation, and their uninstall forms.

- [ ] **Step 5: Run the contract test and full Python suite**

```bash
python3 -m unittest tools.tests.test_agent_memory_sync -v
git check-ignore -q .agent-memory/probe
git diff --check
```

Expected: all Python tests pass; ignore check and diff check exit 0.

- [ ] **Step 6: Commit Task 5**

```bash
git add .gitignore CLAUDE.md README.md docs/PROJECT_CONTEXT.md \
  tools/tests/test_agent_memory_sync.py
git commit -m "docs(memory): give every agent the same map"
```

### Task 6: Live dry run, local installation, and full verification

**Files:**

- Runtime-generated, ignored: `.agent-memory/**`
- Local installation: `~/.local/lib/knxbench-memory-sync/**`
- Local systemd units: `~/.config/systemd/user/knxbench-memory-sync.*`
- Local instruction files: ignored `AGENTS.md`, ignored `MEMORY.md`, and
  `~/.hermes/profiles/knxbench/SOUL.md`

**Interfaces:**

- Exercises every public command against the real curated KNXBench stores.
- Produces an enabled user timer and current shared memory snapshot.

- [ ] **Step 1: Run the complete automated suite from a clean process**

```bash
python3 -m unittest tools.tests.test_agent_memory_sync -v
```

Expected: all tests pass with zero failures and errors.

- [ ] **Step 2: Preview real sources without writes and inspect the report**

```bash
python3 tools/agent_memory_sync.py preview \
  --project-root /mnt/daten-i/Sourcecode/KNXBench
```

Expected: Claude and Codex source counts are non-zero; missing Hermes Markdown
is reported as optional; no secret value appears. Record any topic conflicts
verbatim before deciding whether they are valid separate notes or stale facts.

- [ ] **Step 3: Apply and prove the active snapshot is coherent**

```bash
python3 tools/agent_memory_sync.py apply \
  --project-root /mnt/daten-i/Sourcecode/KNXBench
python3 tools/agent_memory_sync.py check \
  --project-root /mnt/daten-i/Sourcecode/KNXBench
```

Expected: apply succeeds. Check succeeds only when the real source set has no
topic conflict; otherwise it returns `EXIT_CONFLICT` while the complete snapshot
remains readable and the report names every conflicting topic.

- [ ] **Step 4: Install local links and timer, then verify external state**

```bash
python3 tools/agent_memory_sync.py install-agent-links \
  --project-root /mnt/daten-i/Sourcecode/KNXBench \
  --hermes-home /home/knxbench/.hermes/profiles/knxbench
python3 tools/agent_memory_sync.py install-timer \
  --project-root /mnt/daten-i/Sourcecode/KNXBench \
  --home /home/knxbench
systemctl --user start knxbench-memory-sync.service
systemctl --user status knxbench-memory-sync.timer --no-pager
```

Expected: marked blocks occur exactly once; the service exits successfully; the
timer is active; `ExecStart` references the stable `~/.local/lib` copy.

- [ ] **Step 5: Run repository and privacy verification**

```bash
git status --short
git diff --check
git check-ignore -v .agent-memory/PROJECT_MEMORY.md
python3 -m unittest tools.tests.test_agent_memory_sync -v
rg -n '(BEGIN (RSA|OPENSSH|EC) PRIVATE KEY|[A-Z_]*(TOKEN|SECRET|PASSWORD|API_KEY)=)' \
  /mnt/daten-i/Sourcecode/KNXBench/.agent-memory
```

Expected: only planned tracked changes exist before their commits; diff check
and tests pass; Git identifies `.agent-memory/` as ignored; the privacy scan has
no match and therefore exits 1.

- [ ] **Step 6: Commit any final test or documentation corrections**

Stage only tracked plan-related files. Do not stage `.agent-memory/`, ignored
local instruction files, systemd units, backups, or private source memories.

```bash
git add .gitignore CLAUDE.md README.md docs/PROJECT_CONTEXT.md \
  tools/agent_memory_sync.py tools/tests/test_agent_memory_sync.py
git diff --cached --check
git commit -m "test(memory): prove the notebooks stay private and coherent"
```

Skip this commit when verification required no tracked correction.

- [ ] **Step 7: Record rollback commands in the handoff**

```bash
python3 tools/agent_memory_sync.py uninstall-timer --home /home/knxbench
python3 tools/agent_memory_sync.py uninstall-agent-links \
  --project-root /mnt/daten-i/Sourcecode/KNXBench \
  --hermes-home /home/knxbench/.hermes/profiles/knxbench
```

State explicitly that these commands preserve all private Claude, Codex, and
Hermes memories and leave `.agent-memory/` available for manual removal.
