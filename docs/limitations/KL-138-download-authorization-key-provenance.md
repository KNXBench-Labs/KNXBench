# Download authorization key provenance

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-138)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §138 A device's access key comes from the project or a key file, and nothing checks it live

**Status (2026-09-29, K11).** A download authorises with a key when one is
given, and with none otherwise. Implemented and tested in the simulator,
the CLI and the HTTP routes. **Not tested on hardware**: the test device
(`1.1.67`) has no key set, and setting one would be a write the goal does
not need.

**Where the key comes from.**

- **The project**: `Installation/@BCUKey` (Project Schema 23 p. 38, *"The
  key used to lock devices supporting authentication"*), read from the
  retained attributes of the imported project. The default `4294967295`
  (`FFFFFFFFh`) means no key. All three corpus projects carry the default.
- **The operator**: `knx device download --key-file <path>` (decimal or
  `0x` hex). There is no `--key` flag: argv is readable by every user
  through `ps`. The HTTP API takes no key at all.
- **Neither**: the free access level. A device that refuses it gets a hint
  naming the missing key as one possible cause. **No key is ever guessed.**
  A key file wins over the project; two installations with different keys
  are refused before anything opens, because a guess between them would
  be a guess.

**What a key does on the bus.**

- Mask `070nh` (BIM M112) runs MP §3.5.2 `DM_Authorize2_RCo`: the free key
  `FFFFFFFFh` first, the project's key only if the free level is not the
  highest. The diagram on p. 76 nests the key request inside *"If the free
  access level is not the highest level"*. Before this, the code sent the
  key whenever the free level was 0 as well; the test that pinned that
  reading was rewritten against the diagram.
- Other masks run MP §3.5.1: one `A_Authorize_Request` with the key.
- The key level is checked against the profile's 16 levels (Profiles
  Table 4.2 p. 37).

**Why no live test.** A locked `0701h` device would be needed. Locking one
means `A_Key_Write` (MP §3.6 `DM_SetKey`), which KNXBench does not
implement and the goal does not ask for. The tests use the simulator with
the free level at 3, a key at level 1 and writes needing level 2.

**Web UI.** The server sends `accessKey` in the plan, an `authorised`
event and a `hint` on failure. `DeviceDownloadPanel` does not show them yet
(the web lock belonged to the parallel UI session). It ignores the unknown
event kind, so nothing breaks.

**Lifted when.** A download to a device locked with a known key succeeds
live, the trace shows two `A_Authorize_Request`s, and the key appears in
no log or output.
<!-- END retained catalogue record -->
