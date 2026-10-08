← Previous: [Web and Docker deployment](11-web-and-docker.md) · [Manual index](../README.md)

# AI agents over MCP

KNXBench can answer an AI agent's questions about your saved projects. The
`knx-mcp` program speaks the [Model Context Protocol](https://modelcontextprotocol.io)
over stdio, so an agent you already use (Claude Code, Hermes, Cursor and
similar) can look things up in a project instead of guessing:

- "Which group address switches the kitchen light, and who listens on it?"
- "Which devices have no application program, and which addresses are
  used twice?"
- "What changed between last week's save and today's?"
- "What does parameter X of 1.1.5 mean, and what is it set to?"
- "Prepare twenty group addresses for the first floor following this scheme."

> **Note**
>
> This is **experimental** and **read-only**. `knx-mcp` cannot change a
> project, cannot write any file and cannot reach the KNX bus: the program
> is not even linked against the bus code. The tool names and answers may
> still change during the alpha.

## What it can and cannot do

| It can | It cannot |
|---|---|
| Summarise a project and list its aliases | Edit, save or import anything |
| Search devices, group addresses, ranges, rooms, objects | Program, download or reset devices |
| Show a device with objects, links, location, parameters | Read or write the bus |
| Tell which parameters the saved settings show or hide | See changes you have not saved yet |
| Show who sends to and listens on a group address | Apply a parameter's access rules |
| List structural problems (duplicates, conflicts, gaps) | Show a parameter's program default |
| Compare two saved projects | Compare against ETS |
| Check a proposed group-address CSV | Apply that CSV |

When you ask for new group addresses, the agent writes a group-address CSV,
checks it with `knx-mcp`, and hands it to you. **You** apply it, either in the
KNXBench UI or with `knx ga-import file.knxdb file.csv --dry-run` and then
without `--dry-run`. Deletions and re-addressing need the confirmation token
the dry run prints. See [Working with group addresses](04-group-addresses.md)
and [The command line](10-command-line.md).

## Your data and the model

KNXBench sends nothing anywhere. But your agent passes what `knx-mcp` answers
to **its** language model, and that model may run at a provider's data centre.
Project names, room names, device names, addresses and parameter values can
therefore leave your machine through the agent. Choose the agent and model
accordingly.

`knx-mcp` never returns KNX Secure keys, passwords, keyring contents or the
unparsed archive members a project keeps for completeness.

## Getting `knx-mcp`

Releases attach a `knx-mcp-x86_64-linux` binary beside the AppImage, with a
`SHA256SUMS` file covering both. Release `v0.1.0-alpha.5` and earlier do not
include it. Otherwise build it from source:

```bash
cargo build --release -p knx-mcp
# the binary is target/release/knx-mcp
```

Check it with `knx-mcp --version`.

## Connecting an agent

`knx-mcp` is configured entirely on its command line. Each `--project`
names a saved `.knxdb` file and the short alias the agent uses for it:

```bash
knx-mcp --project home=/path/to/home.knxdb --project old=/path/to/home-before.knxdb
```

The installed product database is used automatically, so parameter values
can be decoded. `--product-db <path>` picks another one, `--no-product-db`
uses none.

For agents that read the common `mcpServers` JSON (Claude Code's `.mcp.json`,
Claude Desktop, Cursor):

```json
{
  "mcpServers": {
    "knxbench": {
      "command": "/path/to/knx-mcp",
      "args": ["--project", "home=/path/to/home.knxdb"]
    }
  }
}
```

For Hermes, in `config.yaml`:

```yaml
mcp_servers:
  knxbench:
    command: "/path/to/knx-mcp"
    args: ["--project", "home=/path/to/home.knxdb"]
```

`hermes mcp test knxbench` checks the connection. To give a Hermes run only
these tools, pass the server name as the toolset: `-t knxbench` (the toolset
`mcp-knxbench` exists only after the server has connected).

Both clients were tried on 2026-10-08 against copies of real projects:
Claude Code 2.1.289 (`--mcp-config` with `--strict-mcp-config`) and Hermes
(`hermes chat -t knxbench`). Each found all eight tools, answered visibility
questions from the saved data and refused a request to change a parameter.

Large devices come in pages: `get_device` returns 50 communication objects
and 50 parameter values at a time, with totals over all of them. An agent can
narrow it with `linkedOnly` or `parameterVisibility` (for example only the
`inactive` values), so even a device with hundreds of objects fits within a
client's limit for one tool answer.

The repository also contains an agent skill,
`integrations/agent-skill/knxbench/SKILL.md`. It teaches an agent how to use
these tools and which CLI commands it must never run. Copy it into your
agent's skill directory if it supports skills.

## Good to know

- **Save first.** The agent sees the last saved state. After you save in
  KNXBench, its next question picks the new state up automatically.
- **Older project files stay as they are.** A project saved by an older
  KNXBench is upgraded in memory only; the file is never rewritten.
- **Imported text is data.** A device named "ignore all previous
  instructions" is reported as a device name. Every answer reminds the agent
  of that.
- **Shown or hidden parameters.** For each stored value the agent gets a
  `visibility`: `active` if the device's settings show that parameter,
  `inactive` if they hide it (the value stays stored; whether a hidden value
  still reaches the device on download depends on the device's program), or
  `stale` if the program has no such parameter. KNXBench's parameter panel
  decides this with the same code. When it cannot decide, for example
  without a product database, the answer says `notEvaluated` and why. A
  parameter's access setting is not applied, so an `active` parameter may
  still be one the panel never displays.
- **The agent may still be wrong.** It can pick the wrong device or
  misunderstand a request. Answers name addresses and ids, so you can check
  them in KNXBench.

The design and its limits are recorded in
[ADR-0090](../../adr/0090-read-only-mcp-adapter.md) and
[known limitation §165](../../KNOWN_LIMITATIONS.md#165-the-mcp-adapter-reads-saved-files-only-and-its-visibility-ignores-access).

[Manual index](../README.md) · Next: [Keyboard shortcuts](../reference/01-keyboard-shortcuts.md) →
