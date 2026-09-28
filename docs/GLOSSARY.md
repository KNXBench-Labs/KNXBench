# Glossary: words that mean two things

Some words have one meaning in KNX and another on the web. In KNXBench each
of them means exactly one thing, fixed below. Code, documentation, CLI output
and the UI use them only in that sense. A new use that doesn't fit is either
renamed or added here first.

## Download, upload and direction

| Term | Direction | Meaning in KNXBench | Where it appears |
|---|---|---|---|
| **Download (to the device)** | KNXBench → **device**, over the bus | Writing configuration into a KNX device: application program, parameters, address and association tables, and the load state machines that go with them. The KNX sense of the word. | `knx_net::commissioning::memory_download`, `…::download`, `knx device download`, `/api/device-download/*`, the UI tab "Download to device" (German "In Gerät laden"), KNOWN_LIMITATIONS §7/§92/§136, `goal-commission.md` |
| **Programming (the individual address)** | KNXBench → **device**, over the bus | Writing a device's individual address (MP §2.3). Counted as a write to the device, not as a download. | `individual_address_write`, `knx device program-address`, RESEARCH §8.8.6 |
| **Read-back** | **device** → KNXBench, over the bus | Reading memory or properties back from a device, to check a write or to inspect it. Never called "upload". | `write_memory_region`'s verification, `live_memory_read` |
| **Save / export (to your computer)** | KNXBench server → **user's computer**, over HTTP | Handing a project or report file to the browser. The browser File menu says "Export project…" / "Projekt exportieren…" (`toolbar.exportProject`); Save As instead writes to the server's permitted directory. | `/api/project/download` (route name kept for compatibility), File menu |
| **Upload (to KNXBench)** | **user's computer** → KNXBench server, over HTTP | Sending a file (project, product database) to the server for import. Never used for anything on the bus. | file-system routes, import |

### Why the device direction owns "download"

`[D]` KNX's own documents use *download* for the tool → device direction and
nothing else:

- *KNX Standard, Architecture* (`03_01_01` v03.00.02), p. 19: address and association tables *"are
  constructed by the configuration master (like ETS), and downloaded into the
  device."*
- *KNX Standard, Glossary* (`03_01_02` v01.05.03), p. 9, *Differential
  Download*: *"only the data
  is downloaded that is assumed to differ between the current contents and the
  intended contents after download"*. The contents in question are the
  device's.
- *KNX Standard, Configuration Procedures* (`03_05_03` v02.01.01) §3.7.5.5.2
  "Download", p. 62: *"When downloading the configuration into the device
  …"*.

All three read directly from the PDFs of *The KNX Standard v3.0.0*, Volume 3.

The Standard defines no *upload* for the reverse direction in the sources
read (Management and Configuration Procedures); reading a device back is
done with named read services (`A_Memory_Read`, `A_PropertyValue_Read`).
KNXBench therefore says **read-back** rather than inventing an "upload" the
Standard doesn't have.

### Rules

1. On its own, **"download" always means to the device.** Anywhere a reader
   could take it the other way, say **"download to the device"** in full.
2. A file going from KNXBench to the user is **saved** or **exported**,
   never "downloaded". The `/api/project/download` route keeps its name
   because clients depend on it; the File menu calls it "Export project…".
3. Every download report and progress display names its target device and
   says which direction the data goes (KNXBench → device).
