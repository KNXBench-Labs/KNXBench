← Previous: [Documentation hub](README.md)

# Maintaining the documentation

The manual is a product interface. Broken instructions are bugs, even if they
compile beautifully.

## Source, release and evidence are different things

1. Read the relevant implementation and tests before describing a feature.
2. Check release metadata before claiming a download exists. Main-branch features
   can be newer than the AppImage. Date snapshot statements; do not say "latest"
   without checking releases, including pre-releases.
3. Distinguish project edits from bus operations, preservation from editable
   semantics, and synthetic tests from real-file or hardware evidence.
4. Keep the manual readable; link to engineering detail instead of duplicating
   corpus counts, historical gate totals or every limitation heading.
5. Keep existing filenames and headings when possible. If a heading changes,
   update its references or preserve an explicit anchor.

The manual's [implementation status](manual/implementation-status.md) is a
reader-facing summary. [The ledger](status/LEDGER.md) remains the tracked status
record; documentation changes do not silently reopen or close work.

## Chapter and tutorial conventions

- Use English, short paragraphs, descriptive headings and literal UI labels.
- Keep the numbered chapter order in [the manual index](manual/README.md).
- Put **Previous** on the first line and **Next** on the last line. The first
  chapter comes from the index; the last returns to it. Do not append content
  after navigation. That is how readers end up in the documentation basement.
- A practical workflow needs a goal, prerequisites, numbered steps, an expected
  result and common mistakes/limitations. User-guide chapter introductions carry
  those boundaries; the first-project and complete-configuration tutorials make
  the full exercise explicit.
- Warn about hardware writes before the command, not after it.
- Explain planned/experimental features in those terms. A disabled button is not
  a completed workflow.
- Use relative links and descriptive image alt text. A humorous caption is not
  a substitute for explaining what the picture shows.

## Visuals

[The asset guide](assets/README.md) is the canonical capture recipe. Use real
application output with fictional data, preferably the Porcelain theme. Capture
on an isolated server; do not connect a documentation run to a real bus.
Record one concept per GIF and use screenshots for details that need careful reading.
Keep assets and their recording scripts together in the same work package.

## Local validation

From the repository root:

```bash
python3 -m unittest tools.tests.test_check_documentation -v
python3 tools/check_documentation.py
cargo run --locked -p xtask -- check-anchors
git diff --check
```

The Python checker validates local file/image targets, non-empty image
descriptions and the manual's complete chapter chain. It ignores fenced/inline
code examples, supports the inline/reference/HTML link syntax used here, and
reports its actual file and link counts. It does **not** test external URLs or
prove technical claims. `check-anchors` supplies the heading-anchor check.

For a screenshot/capture-script change, rebuild the frontend and server and run
the isolated capture recipe too. Review each changed screenshot and sample GIF
frames at full size. A successful encoder does not establish legible text.

## Test catalogue maintenance

Keep [the test catalogue](TEST_CATALOGUE.md) alongside test changes: add or revise
its suite group, source links and prerequisites when coverage changes. Descriptions
use 10–15 words; fictional examples must be labelled and never derived from private
project data. Refresh the inspected revision and timestamp after a complete inventory
review. Catalogue presence is not fresh execution evidence or a compatibility claim.

## Before calling it finished

- Follow installation as a beginner, including prerequisites and where files land.
- Walk the affected workflow against the actual application.
- Check README, manual status, known issues and roadmap for contradictions.
- Run link/navigation and anchor checks after the final edit.
- Record what was actually exercised, what was only inspected and what remains
  unverified. Keep capture gaps explicit instead of adding invented screenshots.

Next: [Visual assets and capture instructions](assets/README.md) →
