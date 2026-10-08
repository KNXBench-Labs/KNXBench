← Previous: [Bus monitor and KNXnet/IP](07-bus-and-interfaces.md) · [Manual index](../README.md)

# Documentation export and project comparison

**Goal:** produce a readable project report or compare two saved snapshots.
**Prerequisites:** an open project for a report; a second compatible `.knxdb` for comparison.
**Expected result:** a self-contained HTML file or a list of differences.
**Watch out:** review names and addresses before sharing a report. A diff does not
merge changes, and browser Print to PDF is not a native PDF exporter.

Two things you will eventually want from any engineering project: a document that
describes it to someone who cannot open it, and an answer to "what changed?". KNXBench
has one feature for each.

## Documentation export

### What it produces

One self-contained HTML file. Not a PDF, not a Word document, not a folder of assets —
a single `.html` file with its stylesheet inlined, which opens in any browser and can
be emailed, printed to PDF from the browser, or committed next to the project.

The document always includes its header, contents, and limits/warnings. Five
content sections can be selected, in the application's export dialog (below) or
through the server API. Every included section is linked from the table of contents:

| Section | What it holds |
| --- | --- |
| Header | Project number, group address style, completion state, last modified and project start dates, ETS schema version, KNXBench schema version, generation time |
| Contents | Links to every section below |
| Summary | Counts: installations, areas, lines, devices, communication objects, group ranges, group addresses, building parts, parameter values |
| Topology | Each installation's areas, lines and devices, plus a separate list of devices in no line |
| Buildings | The building structure, plus a list of building parts whose parent could not be resolved |
| Group addresses | The group range tree with each address under it, plus a list of addresses in no range |
| Devices | One block per device: individual address, description, commissioning state, raw references, hardware-consistent resolved product data when installed, stored parameter values, module-argument bindings, and a table of its communication objects |
| Limits and warnings | The document's own honest list of its limits and every anomaly found while generating it |

A device's communication object table carries the number, name, description, DPT, an
active flag, the five KNX flags (**R**ead, **W**rite, **T**ransmit, **U**pdate,
**C**ommunication) and the group addresses each object is linked to. Communication
objects with no owning device get their own table, because losing them silently would
be worse than admitting they are odd.

### What it deliberately leaves out

The last section of every generated document says so in the document itself, so the
caveats travel with the file:

- Product data is composed before the pure renderer runs. When the installed
  product database proves a hardware-consistent product/program pair, the report
  adds manufacturer, product and application-program names. Otherwise it retains
  both raw references and warns; it never borrows a program from other hardware.
- Stored parameter values are listed. Declared restriction values gain their
  display labels; unknown restrictions and parameter kinds without a formatter
  remain raw and warn instead of becoming blank cells.
- Module-instance argument bindings are listed with resolved names when available.
  `AllocatorRef`, unknown argument kinds, allocation metadata and repeat semantics
  remain uninterpreted, visible and warned.
- Binary data attached to devices is referenced by name and id only.
- The API can request English or German report chrome and product data. Some detail
  tables, communication-object text, and diagnostics remain in their existing
  language, so the report is only partially localized.
- This is not an ETS report and has not been compared to one.

Below that list, the document prints **Anomalies found while generating this
document** — every structurally odd thing the generator ran into, such as a link to a
group address that does not exist. If there were none, it says "None." The same
warnings are also reported back to whichever surface asked for the export, so nothing
is only in one place.

> **Note**
>
> The document is deterministic: the same project and the same generation timestamp
> produce byte-identical HTML. If you keep exports in version control, a `diff` between
> two of them shows what changed in the project and nothing else.

### Exporting from the application

Open the **File** menu and choose **Export documentation…**. A dialog opens with
a preview of the document:

- **Sections** — one checkbox each for Summary, Topology, Buildings, Group
  addresses and Devices, all ticked at first. The header, the contents and the
  limits section are always included. Changing a tick refreshes the preview.
- **Preview** — the document exactly as it will be written, shown in a sandboxed
  frame in which nothing can run. If the preview fails, the reason is shown in
  its place.
- **Warnings** — listed next to the preview, so you can see them before
  exporting.
- **Print…** — opens the browser's print dialog for the preview document, not
  for the application window. Use it to print to paper or to PDF.
- **Export…** — asks where to save (the file dialog filters on *HTML document*
  and suggests `project-documentation.html`) and writes the same sections you
  previewed.

The document follows the application language: German when the interface is
German, English otherwise.

When it is done you get one of two messages: *"Project documentation exported, no
warnings."* or *"Project documentation exported, 3 warnings — see Log."* The warnings
themselves land in the session log, described in
[Bus monitor and KNXnet/IP](07-bus-and-interfaces.md).

### Exporting from the command line

```bash
knx doc-export project.knxdb project-documentation.html
```

It prints the output path, the number of warnings, and then each warning on its own
line:

```text
exported documentation to project-documentation.html
  1 warning(s)
  warning: device 42: link to a group address that does not exist
```

A document that carries warnings is still a complete document, so this is a success.
See [The command line](10-command-line.md) for the exit-code rules.

## Project comparison

### What it compares

A comparison takes two KNXBench projects and reports what differs between them. It
works on the typed domain model, not on file bytes, so reordering or re-saving a
project does not show up as a change.

It walks these entity kinds:

| Entity | Fields it compares |
| --- | --- |
| Project info | Everything except the project id itself |
| Installation info | Per installation |
| Areas | Name, completion |
| Lines | Name, medium, domain address (and whether it is checked), IP routing multicast address, multicast TTL, completion, owning area |
| Devices | Name, description, individual address, product reference, program reference, commissioning state, line, building part |
| Group ranges | Name, start, end, parent |
| Group addresses | Name, central flag, unfiltered flag, range |
| Building parts | Name, number, kind, completion, default line |
| Communication objects | Text, description, DPT, the read/write/transmit/update/communication flags, its group links and its module instance, nested under their device |
| Parameters | The raw stored value, nested under their device |

The project id is excluded on purpose: two copies of the same project are still the
same project, and comparing their identifiers would report a difference in every pair
that matters.

### The shape of a result

For each entity kind you get four buckets:

- **added** — present on the right, not on the left
- **removed** — present on the left, not on the right
- **changed** — present in both, with a list of exactly which fields differ
- **ambiguous** — could not be matched one-to-one

Matching happens by ETS identifier where both sides have one, and otherwise by a
natural key — the address, the name, the position in the structure. The result records
which of the two was used, so a match you would not have made yourself is visible
rather than assumed. When neither produces a unique pairing, the entity lands in
**ambiguous** with a count of the candidates on each side, instead of being guessed
into the wrong bucket.

Device changes nest: a device whose own fields are identical but whose communication
objects differ is reported as changed, with the nested detail underneath and an
explicit note that its own fields are unchanged.

### Comparing in the application

Open the **File** menu and choose **Compare with…**. Pick a KNXBench project (`.knxdb`)
or a raw ETS project export (`.knxproj`); in the browser you can also upload one from
your computer in the same picker. KNXBench compares the project you currently have open
(the left side) against the file you picked (the right side) and opens a **Comparison
result** panel.

A `.knxproj` is imported for the comparison through the regular importer. The panel
then shows a collapsed **ETS import report** line above the result, for example
*"ETS import report: 3 diagnostics (1 warning)"*. Open it to read every diagnostic:
its severity, message, and where in the archive it was found. If the import reports
an error, KNXBench does not compare at all: the panel says *"Comparison refused"* and
shows the diagnostics instead, because a partly misread file would produce a
misleading comparison. Warnings do not stop the comparison.

The panel lists, per installation, one line per entity kind with the counts in each
bucket — *Areas*, *Lines*, *Devices*, *Group ranges*, *Group addresses*, *Buildings* —
plus separate lines for changed project and installation info fields. If nothing
differs it says *"No differences found."* and you close it again.

Below those lines, each non-empty table has a collapsed entry such as *Devices (3)*.
Open it with a click, Enter or Space to list every entity in it: its status in words
(*added*, *removed*, *changed*, *ambiguous*), its key (a device's address, a group
address, a building path…) and its name. A changed entity shows how it was matched
and a *Field / Before / After* table; a changed device lists its changed
communication objects and parameters underneath. A table with more than 20 entries
gets a search field (key or name), buttons to show only *added*, *removed*,
*changed* or *ambiguous* entries, and a count such as *11 of 3300 entries shown*. Its
entries scroll inside the table; Page Up/Down, Home and End work once the list
has focus.

Comparing does not change either project. The file you pick is read, compared and
released; it never becomes the open project.

### Comparing on the command line

```bash
knx diff old.knxdb new.knxdb
knx diff export.knxproj current.knxdb
```

Either side may be a native `.knxdb` or a raw `.knxproj`. Raw ETS archives are
normalized through the regular importer, and their complete import reports are written
to standard error so compatibility losses are not hidden. Opening an older `.knxdb` may
run the normal forward-only native-store migration.

The output is a plain text tree using four markers:

```text
+ device 1.1.7
- device 1.1.9
~ device 1.1.5: name: Hallway switch -> Hall switch
~ device 1.1.5: description: - -> Main entrance
? group address 1/2/3: 2 left candidate(s), 1 right candidate(s)
```

`+` is added, `-` is removed, `~` is one changed field with its before/after values,
and `?` is ambiguous with the candidate counts on each side. When the two projects are
the same, it prints `no differences found`.

For CI, add `--exit-code`:

```bash
knx diff --exit-code expected.knxdb actual.knxdb
```

In that mode, exit code `0` means equal, `1` means different (an ambiguity also counts
as different), and `2` means arguments, file access, native-store loading, a failed ETS
import, or an ETS import report containing error-level diagnostics. Without
`--exit-code`, a successfully produced non-empty diff remains exit `0`; an import report
with errors remains a failure so partially interpreted data cannot pass unnoticed.

### Reviewing project versions in Git

A `.knxdb` is a SQLite file, so plain `git diff` only says *Binary files differ*. Git
can call `knx diff` instead. Put a small script somewhere on your machine:

```sh
#!/bin/sh
# git external diff driver: $1 is the path, $2 the old file, $5 the new one
case "$2" in /dev/null) echo "new project file: $1"; exit 0 ;; esac
case "$5" in /dev/null) echo "project file removed: $1"; exit 0 ;; esac
exec knx diff "$2" "$5"
```

make it executable, and register it for project files in your repository:

```bash
echo '*.knxdb diff=knxbench' >> .gitattributes
git config diff.knxbench.command /path/to/knx-git-diff.sh
```

`git diff` then prints the same `+`/`-`/`~`/`?` lines as above, for example
`~ group address 0/0/1: name: GA -> Flur Licht` after a rename. `git log -p` and
`git show` use the driver only with `--ext-diff`. The two `case` lines matter: for an
added or deleted file Git passes `/dev/null`, which `knx diff` refuses, and Git then
stops the whole log. This was checked on 2026-10-06 with a small repository holding two
commits of one project, including an added and a removed file.
It shows what `knx diff` compares, not every stored byte. Two cautions: the new side of
`git diff` (without a commit range) is your real working file, and if that file comes
from an older KNXBench version, `knx diff` upgrades it in place
([known issue](../known-issues.md#opening-an-older-project-upgrades-the-file)); and the
driver needs `knx` on the `PATH` of every machine that runs the diff.

### What comparison is good for, and where it stops

It answers the questions you actually ask about a project over time: which devices
appeared, which group addresses were renamed, whether a colleague's copy still matches
yours, and what a re-import of an updated ETS file changed against the project you
already had.

The limits are worth knowing before you rely on it:

- A raw `.knxproj` is always compared together with its import report, in the panel
  and on the command line; an import with errors is refused, not compared.
- It compares the typed domain model. Anything KNXBench stores as opaque preserved data
  rather than as typed fields is not compared field by field.
- Parameters are compared by their raw stored value, because KNXBench does not
  interpret parameter values — see
  [Devices and products](05-devices-and-products.md).
- It is a KNXBench comparison, not an ETS one. No ETS-produced comparison has been used
  as a reference, and no equivalence with one is claimed.

[Manual index](../README.md) · Next: [Settings, themes and languages](09-settings-and-appearance.md) →
