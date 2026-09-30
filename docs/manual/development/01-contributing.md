← Previous: [Ideas and roadmap](../ideas-and-roadmap.md) · [Manual index](../README.md)

# Contributing

KNXBench is a small project with a single maintainer. There is no team, no triage
rotation, and no service level. That is worth saying up front, because it changes what
"contributing" looks like here: a good bug report is often worth more than a patch, and a
patch that arrives with tests and a documentation update is worth more than three that do
not.

Everything happens in one place:
[github.com/KNXBench-Labs/KNXBench](https://github.com/KNXBench-Labs/KNXBench).

## Reporting a bug

A useful report answers four questions:

1. **What did you do?** The exact steps, including which build you used — AppImage,
   Docker, or a local `cargo run`.
2. **What did you expect?**
3. **What happened instead?** Error text verbatim, not paraphrased.
4. **What is your environment?** Distribution, KNXBench version (the footer shows it,
   and `knx --version` / `knx-server --version` print it with the commit they were built
   from), and — for import problems — the ETS schema version the import reported.

If a `.knxproj` file is involved, say whether you can share it. Most cannot: a real
project file is a map of somebody's building, complete with device addresses. Do not
attach one you are not free to publish. A synthetic file that reproduces the same failure
is better for everyone.

> **Warning**
>
> Before you attach anything, read it. A project file, a log, or a bus capture can contain
> individual addresses, IP addresses of your gateways, room names, and customer names.
> Once it is in a public issue, it stays there.

### The debug report

KNXBench can assemble most of that for you. Open the **File** menu and choose **Debug
report**. The dialog asks for a short description, then lets you decide what goes in:

| File | Contents | Optional? |
| --- | --- | --- |
| `report.md` | The human-readable summary | always included |
| `environment.json` | Version, shell (browser or desktop), interface language, theme | always included |
| `log.json` | The session log | opt-out, on by default |
| `project-summary.json` | Counts and structure of the open project, not its contents | opt-in |
| `bus-telegrams.json` | Recently captured telegrams | opt-in |

The dialog lists exactly which of those it will write, in your own language, and says
which one the redaction pass deliberately leaves alone — the bus telegrams. Read that
line before you tick the box.

There are two exits, and you watch both happen:

- **Save** writes a `.zip` to a path you pick. Local, always.
- **Open a GitHub issue** builds the same bundle in memory, writes nothing to disk, and
  opens a prefilled `issues/new` page in your browser with the report already in the body.

The second one is worth being precise about, because "the app files an issue" sounds
alarming and is not what happens. `apps/knx-web/src/githubIssue.ts` builds a URL. There is
no token, no credential, no `POST`, and no API client anywhere in that path. KNXBench
cannot submit anything on your behalf; it opens a page with the fields filled in, and you
read it and decide whether to press GitHub's button. Nothing leaves your machine before
you do.

One detail you may notice: if the report is long, the body is shortened and ends with a
note saying so. Browsers and desktop shells disagree about how long a URL may be, so the
builder caps the finished URL at 8000 characters. When that happens, save the zip as well
and attach it to the issue — the complete text is in there.

## Where discussion happens

In the [issue tracker](https://github.com/KNXBench-Labs/KNXBench/issues). That is the whole
list. There is no chat room, no forum, and no mailing list, and pretending otherwise would
just send you looking for a door that is not there.

Questions are welcome as issues. If the answer turns out to be "the manual should have
said that", the fix is usually a documentation change, which is a perfectly respectable
outcome.

## The license, and what contributing under it means

KNXBench is licensed under the **GNU Affero General Public License, version 3 or later**
(`AGPL-3.0-or-later`). The full text is in [`LICENSE`](../../../LICENSE) at the repository
root, and every crate in the workspace inherits that license from `Cargo.toml`.

For a contributor, the practical consequences are:

- Your contribution is licensed under the same terms. There is no separate contributor
  license agreement and no copyright assignment.
- Anyone may use, modify, and redistribute KNXBench, commercially included, under the
  AGPL's terms.
- The "Affero" part is the one people miss: if you modify KNXBench and let other people
  use your modified version **over a network** — which the `knx-server` web deployment
  makes easy — those users must be offered the corresponding source code.

Third-party dependencies are held to a narrower rule than the project's own license.
`deny.toml` carries an explicit allowlist, and any license not on it is rejected; GPL is
not on it, so no GPL-licensed crate may enter the runtime dependency graph. If a
dependency you want to add fails `cargo deny check`, that is the gate doing its job, not a
misconfiguration.

## The gates a change has to pass

These run in CI on every push and pull request
([`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml)), and every one of them
runs locally with the same command. A check that only exists on CI gets ignored, so there
is deliberately nothing in CI you cannot reproduce on your own machine.

Rust:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The repository's own architecture gates, which live in the `xtask` crate:

```bash
cargo run -p xtask -- check-layering
cargo run -p xtask -- check-headers
cargo run -p xtask -- check-anchors
```

- **`check-layering`** walks the resolved dependency graph from `cargo metadata` and fails
  if a crate reaches something it is not allowed to reach — `knx-core` must not reach
  `serde_json`, `quick-xml`, `rusqlite`, `tokio`, `axum` or `tower`; `knx-etsproj` must not
  reach `knx-store`; `knx-productdb` must reach neither; `knx-secure` must reach neither
  `knx-core` nor `serde`; and `knx-projection`, `knx-csv`, `knx-report` and `knx-diff` have
  their own roots. When it fails it prints the shortest path to the forbidden package,
  which is usually enough to see which `use` line did it. See
  [Architecture tour](03-architecture-tour.md) for what the rule is protecting.
- **`check-headers`** enforces the one-sentence file header convention (ADR-0018): a source
  file's first line says what the file is for — `//! …` in Rust, `/** … */` in TypeScript —
  and is at most 100 columns wide. It is a ratchet, not a sweep: the number of files
  *without* a header may only go down, so new files need one and old ones are left alone.
- **`check-anchors`** walks `docs/` and the repository's root Markdown files, slugs every
  heading the way GitHub's renderer would, and fails with a `file:line` and a nearest-match
  suggestion for any in-repo link whose target heading does not exist. This manual is
  covered by it, which is why every cross-reference in it points at a heading that was
  actually read.

The license and advisory gate, which needs `cargo install cargo-deny` once:

```bash
cargo deny check
```

The frontend, from `apps/knx-web` after `npm ci`:

```bash
npx tsc --noEmit
npx vitest run
```

Those two are what the package's own scripts wrap: `npm run build` runs `tsc` and then the
Vite build, and `npm test` is `vitest run`. There is also `npm run test:e2e`, a Playwright
suite that builds the frontend and drives it against a running server; it is not part of
the main CI job, and it needs Playwright's browsers installed.

Finally, the Docker image has a smoke test that builds it, boots it, checks health, and
does a native save-and-reopen cycle:

```bash
apps/knx-server/scripts/smoke-test.sh
```

> **Tip**
>
> Run `cargo fmt --all --check` and the three `xtask` gates first. They take seconds and
> catch the kind of failure that is annoying to discover after a twelve-minute test run.

## Commits

The repository follows a conventional-commit shape: a type, an optional scope in
parentheses, a colon, and a lower-case summary.

```text
feat(core): the sixth communication flag is allowed to exist
fix(server): the delay was a tax, not a limit, and said so in three places
docs(privacy): the LAN stops living in the history as well as in the tree
test(knx-net): the skip arm stops excusing a send that never happened
```

The types that actually appear in the history, most common first, are `docs`, `feat`,
`fix`, `test`, `merge` and `chore`, with an occasional `refactor`, `perf`, `ci` or `style`.
Scopes are crate or area names.

The summaries read the way they do on purpose. Dry humor is welcome; inaccuracy is not. A
commit message may be funny about a bug, but it must be right about what the commit
changed.

Two rules matter more than the wording:

- **Focused commits.** One change per commit. Do not mix an unrelated refactor into a
  feature — it makes the diff unreviewable and the revert impossible.
- **Keep the repository buildable.** Every commit should pass the gates above on its own.

## Documentation is part of the change

This is not a nice-to-have in this repository; it is part of the definition of done. A
change that alters behavior and leaves the documents describing the old behavior has not
been finished, it has been abandoned halfway.

Where things belong:

- **User-facing behavior** → the relevant chapter of this manual, under `docs/manual/`.
- **What works and what does not** → [`IMPLEMENTATION_STATUS.md`](../../IMPLEMENTATION_STATUS.md),
  [`KNOWN_LIMITATIONS.md`](../../KNOWN_LIMITATIONS.md) and the manual's
  [Known issues](../known-issues.md) chapter.
- **Format or protocol facts you discovered** → [`IMPORT_EXPORT.md`](../../IMPORT_EXPORT.md),
  [`COMPATIBILITY.md`](../../COMPATIBILITY.md) or [`RESEARCH.md`](../../RESEARCH.md),
  with the evidence.
- **A durable architectural decision** → a new record in [`docs/adr/`](../../adr/README.md),
  following [`template.md`](../../adr/template.md) and taking the next free number. An ADR
  is never edited to reverse itself; a later ADR supersedes it, and the old one is marked
  `Superseded by ADR-NNNN`.

Three habits keep the documents trustworthy, and they are the same three the code is held
to:

- **Do not invent facts.** If something has not been verified against a real file, a real
  gateway, or a test, say so.
- **Never claim ETS compatibility.** The wording is "KNX-compatible". Never
  "KNX-certified", never "full ETS compatibility". Compatibility claims are made only for
  schema versions with real test material behind them.
- **Say what is missing.** An honest gap in a document is useful. A confident wrong
  sentence costs somebody an afternoon.

## What to expect

One maintainer, evenings and weekends, a project that is `0.1.0-alpha.1` with no release
cut yet. Issues may sit for a while. Pull requests that arrive with a test and a
documentation update are far more likely to be merged quickly than ones that need a
conversation first.

If you are unsure whether an idea fits the project's direction, open an issue and ask
before you write the code. It is a cheaper conversation than a rejected branch, and the
[Ideas and roadmap](../ideas-and-roadmap.md) chapter may already have an opinion.

[Manual index](../README.md) · Next: [Building from source](02-building-from-source.md) →
