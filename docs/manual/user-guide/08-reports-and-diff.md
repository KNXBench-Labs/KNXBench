← Previous: [Bus monitor and KNXnet/IP](07-bus-and-interfaces.md) · [Manual index](../README.md)

# Documentation export and project comparison

Two things you will eventually want from any engineering project: a document that
describes it to someone who cannot open it, and an answer to "what changed?". KNXBench
has one feature for each.

## Documentation export

### What it produces

One self-contained HTML file. Not a PDF, not a Word document, not a folder of assets —
a single `.html` file with its stylesheet inlined, which opens in any browser and can
be emailed, printed to PDF from the browser, or committed next to the project.

The document has a fixed structure, and every section is linked from a table of
contents at the top:

| Section | What it holds |
| --- | --- |
| Header | Project number, group address style, completion state, last modified and project start dates, ETS schema version, KNXBench schema version, generation time |
| Contents | Links to every section below |
| Summary | Counts: installations, areas, lines, devices, communication objects, group ranges, group addresses, building parts, parameter values |
| Topology | Each installation's areas, lines and devices, plus a separate list of devices in no line |
| Buildings | The building structure, plus a list of building parts whose parent could not be resolved |
| Group addresses | The group range tree with each address under it, plus a list of addresses in no range |
| Devices | One block per device: individual address, description, commissioning state, unresolved product and program references, and a table of its communication objects |
| What this report does not contain | The document's own honest list of its limits |

A device's communication object table carries the number, name, description, DPT, an
active flag, the five KNX flags (**R**ead, **W**rite, **T**ransmit, **U**pdate,
**C**ommunication) and the group addresses each object is linked to. Communication
objects with no owning device get their own table, because losing them silently would
be worse than admitting they are odd.

### What it deliberately leaves out

The last section of every generated document says so in the document itself, so the
caveats travel with the file:

- Manufacturer, product and application-program names are not resolved. The product
  database is a separate store that the generator does not read, so the raw reference
  identifiers are printed instead.
- Parameter values are stored uninterpreted and are not listed.
- Module instance arguments on newer-schema devices are retained but not decoded.
- Binary data attached to devices is referenced by name and id only.
- Text is rendered in the project's default language only.
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

Open the **File** menu and choose **Export documentation…**. KNXBench asks where to
save — the file dialog filters on *HTML document* and suggests
`project-documentation.html` — and writes the file.

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

Open the **File** menu and choose **Compare with…**. Pick a `.knxdb` file; KNXBench
compares the project you currently have open (the left side) against the file you
picked (the right side) and opens a **Comparison result** panel.

The panel lists, per installation, one line per entity kind with the counts in each
bucket — *Areas*, *Lines*, *Devices*, *Group ranges*, *Group addresses*, *Buildings* —
plus separate lines for changed project and installation info fields. If nothing
differs it says *"No differences found."* and you close it again.

Comparing does not change either project. The file you pick is read, compared and
released; it never becomes the open project.

### Comparing on the command line

```bash
knx diff old.knxdb new.knxdb
```

The output is a plain text tree using four markers:

```text
+ device 1.1.7 Presence detector
- device 1.1.9 Old dimmer
~ device 1.1.5 Hallway switch: name, description
? group address 1/2/3: 2 left candidate(s), 1 right candidate(s)
```

`+` is added, `-` is removed, `~` is changed with the differing fields named after the
colon, and `?` is ambiguous with the candidate counts on each side. Field-level changes
are printed as `~ name: Hallway switch: Hall switch -> Hallway switch`. When the two
projects are the same, it prints `no differences found`.

### What comparison is good for, and where it stops

It answers the questions you actually ask about a project over time: which devices
appeared, which group addresses were renamed, whether a colleague's copy still matches
yours, and whether an import-then-export round trip preserved what you expected.

The limits are worth knowing before you rely on it:

- Both sides must be KNXBench `.knxdb` projects. You cannot compare a `.knxproj` ETS
  archive directly; import it first.
- It compares the typed domain model. Anything KNXBench stores as opaque preserved data
  rather than as typed fields is not compared field by field.
- Parameters are compared by their raw stored value, because KNXBench does not
  interpret parameter values — see
  [Devices and products](05-devices-and-products.md).
- It is a KNXBench comparison, not an ETS one. No ETS-produced comparison has been used
  as a reference, and no equivalence with one is claimed.

[Manual index](../README.md) · Next: [Settings, themes and languages](09-settings-and-appearance.md) →
