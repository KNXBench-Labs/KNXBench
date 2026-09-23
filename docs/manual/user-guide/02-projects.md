← Previous: [The user interface](01-user-interface.md) · [Manual index](../README.md)

# Projects: create, open, import, save, export

KNXBench works with two project files, and it is worth being clear about them before
anything else:

- **`.knxdb`** is KNXBench's own project file. It is a SQLite database, it is the
  working format, and it is the only format that keeps everything KNXBench knows about
  your project.
- **`.knxproj`** is the ETS project archive. KNXBench reads it and never writes one.
  It is the way a project gets *in*; once imported, your work lives in `.knxdb`.

Everything in this chapter is reachable from the **File** menu, and most of it also
from the command palette (`Ctrl+Shift+P`).

## Starting a new project

**New project…** opens a small dialog with four fields:

| Field | What it does |
| --- | --- |
| Project name | The name of the project. Pre-filled, and selected when you focus it, so you can just type over it. |
| Installation name | The name of the installation inside the project. |
| Language | A language tag such as `de-DE`. Checked for well-formedness only — no list of "supported" languages is enforced. |
| Group address style | `ThreeLevel` (`1/2/3`), `TwoLevel` (`1/2`) or `Free` (a plain number). |

![The New project dialog over the welcome screen, with Project name "Untitled project"
selected, Installation name "Installation 1", Project language "en", Group address style
"Three level (main/middle/sub)", and Cancel and Create project
buttons](../../assets/screenshots/porcelain-new-project.png)

The group-address style is a project-wide decision and it is made here. See
[Working with group addresses](04-group-addresses.md) for what that choice means in
daily use.

> **Warning**
>
> The dialog's own hint says the project properties can restyle it later. They cannot —
> no such control exists in the interface today. Treat the style you pick here as fixed
> for the life of the project, and see [Known issues](../known-issues.md) for the
> details.

> **Warning**
>
> If a project is already open and has unsaved edits, creating a new project asks you
> first, and offers to discard those edits. Discarded edits are gone — there is no
> second undo stack behind that prompt. Save first if you are unsure.

## Opening a KNXBench project

**Open (.knxdb)…** opens a project KNXBench saved earlier. This is the lossless path:
what you saved is what you get back, including the parts of an imported ETS file that
KNXBench preserves but does not model (see below).

![The file picker of the web build, showing the path /, one file named
kv-demo.knxdb, an Upload row with a file-choosing control, and a Cancel
button](../../assets/screenshots/porcelain-open-project-dialog.png)

The picker looks different depending on how you run KNXBench. The desktop build opens
your desktop's own file dialog. The web build — the one in the screenshot — browses
the directory the server is allowed to see, and offers an **Upload…** row for files
that are on your machine rather than on the server's. See
[Web and Docker deployment](11-web-and-docker.md) for what the server can and cannot
reach.

The web picker's upload area also accepts dropped files or a multi-file choice.
It uploads them one at a time, stops at the first failure and reports which file
failed; an upload does not automatically select or open a project. After an
upload, select the file from the server-side list to open it.

## Importing an ETS project

**Open project…** imports a `.knxproj` file. The file is read; it is never written
back to. Whatever you do afterwards happens in memory and, once you save, in a
`.knxdb` of your choosing.

While the import runs, a banner shows the file name, the phase the importer is in, and
a count where the server can give one.

![The import banner over the welcome screen, reading "Importing KV v2.5 -
demo.knxproj…", phase "Starting…", with a progress bar and the line "Aligning the
floors with gravity"](../../assets/screenshots/porcelain-loading-progress.png)

The banner walks through seventeen named phases — opening the archive, detecting the
schema version, parsing the topology, validating references, inferring datapoint
types, ingesting manufacturer data, and so on. The bar only fills proportionally while
the server is reporting a "phase N of M" pair; before that it moves without claiming
to know how far along it is. The cheerful line underneath is decoration. The phase
line above it is the one that tells you something.

### What the import actually understands

KNXBench detects the schema version of the file and imports accordingly. What has been
verified against real files, and what has not, is a distinction this project takes
seriously:

| Schema | ETS generation | Status |
| --- | --- | --- |
| 11 | ETS 4 | Reading verified against a real project, with counts pinned by tests |
| 21 | ETS 5.7 | Reading and writing verified, including a round-trip back to an equal model |
| 23 | ETS 6.3 | Reading verified against one real project; no round-trip claim |
| 12, 13, 14, 20, 22 | ETS 4 to ETS 6 | Accepted by the importer, but no real sample has been imported and reconciled |

"Accepted" in the last row means the importer will try, and will report what it did not
understand. It does not mean anyone has checked the result against a real file of that
schema. The full evidence table, test names included, is in
[Compatibility](../../COMPATIBILITY.md).

> **Note**
>
> Password-protected project files cannot be opened from the user interface or the
> command line today. The decryption for older (ETS 4/ETS 5) containers exists in the
> library and is tested, but nothing in the application asks you for a password yet,
> so there is no way to hand one over. Newer (ETS 6, AES) containers are refused by
> name, not attempted.

### The import report

KNXBench does not silently drop anything it fails to understand. Everything the
importer meets falls into one of these buckets:

- **Understood** — mapped into the project model.
- **Preserved opaque** — kept byte for byte, with its size and a checksum recorded,
  so it survives a save and an export even though KNXBench cannot interpret it.
- **Reported** — unknown elements and attributes, inferred values (a datapoint type
  derived from linked communication objects, for example), conflicts, unsupported
  features, and outright errors.

You see the result in three places:

1. **The Overview**, where import errors and warnings appear as counts alongside the
   project's other numbers.
2. **The Import: N errors · M warnings button** in the strip under the header, which
   takes you there.
3. **The Log**, which carries one entry per finding: one per import error, one per
   unknown construct (with how many times it was seen and where), one per opaque
   entry (with its size and checksum), one per datapoint-type conflict, and one per
   unsupported feature.

The structured report itself — counts read versus mapped, every unknown construct,
every opaque summary — is written as JSON by the command line:

```bash
knx import project.knxproj --store project.knxdb --report-json report.json
```

See [The command line](10-command-line.md) and, for the report's exact shape,
[Import and export](../../IMPORT_EXPORT.md).

> **Note**
>
> Log entries are the server's own text and stay in English, whatever language the
> interface is set to. They are written to be pasted into a bug report.

## Saving

**Save** writes the project to its file. If the project has no file yet — which is the
case for every newly created and every freshly imported project — Save asks where to
put it, exactly as Save As does. The suggested name is `project.knxdb`.

**Save As…** always asks.

In the browser, Save As writes to the server's permitted directory, not to your
computer's Downloads folder. To obtain a copy locally, choose **Download
project** from the File menu after opening a project. This downloads a freshly
serialized `.knxdb` from the current in-memory project; it is not a substitute
for saving changes on the server. The native desktop build uses its own file
dialog for Save As and does not show the browser-only Download project item.

There is no autosave. Nothing is written until you ask for it.

### What "unsaved changes" means right now

KNXBench compares the current project contents with a clean baseline established by
opening, importing, creating, or successfully saving a project. Undo history is
separate: saving keeps that history but clears the modified state; undoing edits back
to the baseline is also clean. A failed save does not clear the modified state.

In the desktop build, quitting with a modified project asks first, and offers
**Cancel** or **Quit without saving**. In the web build there is no Quit item — a
browser tab cannot close itself — so nothing intercepts a closed tab.

> **Warning**
>
> Closing the browser tab of the web build does not warn you about unsaved edits. Save
> before you close it.

## There is no `.knxproj` export

Until 2026-09-20 there was an **Export to .knxproj…** item in the File menu. It is
gone, together with the code behind it, and it is not coming back:
[ADR-0028](../../adr/0028-no-knxproj-export.md) records the decision. Import is
one-way. Once a project has been read into KNXBench, it stays in `.knxdb`.

If you have a screenshot, a script or a habit that expects that item, this is what
changed and what to use instead:

| You want to | Do this instead |
| --- | --- |
| Keep working on the project | Save to `.knxdb` — it keeps everything KNXBench knows |
| Hand group addresses to another tool | **Export group addresses (CSV)…** — [Working with group addresses](04-group-addresses.md) |
| Hand someone a readable account of the project | **Export documentation…** — [Documentation export and project comparison](08-reports-and-diff.md) |
| Give a colleague the original ETS file | Give them the `.knxproj` you imported; KNXBench never modified it |

The short reason: an exported archive was unsigned, no real ETS installation was ever
available here to test whether ETS would accept it, and maintaining a writer nobody
could verify cost more than it returned. A file KNXBench cannot promise ETS will open
is not a file KNXBench should write.

## The rest of the File menu

Three more items are covered elsewhere:

- **Export group addresses (CSV)…** and **Import group addresses (CSV)…** —
  [Working with group addresses](04-group-addresses.md).
- **Export documentation…** and **Compare with…** —
  [Documentation export and project comparison](08-reports-and-diff.md).
- **Debug report…** writes a log-based report for bug reports, with IP addresses
  removed. It still contains group addresses and the names of imported elements, so
  read it before you attach it to anything public.

[Manual index](../README.md) · Next: [Buildings and topology](03-buildings-and-topology.md) →
