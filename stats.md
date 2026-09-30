# Project Statistics

**Last update:** 2026-09-30 07:28:41 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **48.622.432 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **40.518 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **331.5 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **16.500 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **97.244 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **4.9 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **19981.8 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.944.897 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **213.5 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **349.8 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **292.633.548 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.48x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **486.2 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **194.489 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **19.448 kilometers** (about **0.49x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **7.78 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **72.45 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **277.842.470 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **972.448.645 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **51.522 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.944 kWh** and **641.8 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 1.684
- **Merges:** 181
- **Pushes/Sync:** 1.684
- **Lines Added (+):** 464.783
- **Lines Deleted (-):** 92.385

## Git Text Churn Distribution

| P50 changed text lines | P90 | P95 |
| ---: | ---: | ---: |
| 118 | 710 | 1.185 |

**1.481** measured commits;
**203** unavailable (for example, merge-only
history or binary/unsupported `--numstat` entries). A zero means Git explicitly
reported zero text-line changes; missing data is not turned into zero. This
counts historical additions + deletions per commit, **not AI-attributed** work
or surviving source lines. Branch lifetime cannot be inferred from this table.

## Coding Statistics (Current Working Tree)

**Git-tracked source files:** 493; **physical lines:** 235.486;
**code:** 183.452; **comments:** 34.909; **blank:** 17.125.
**Code/comment ratio:** 5.26:1; **comment share of nonblank lines:** 16.0%.
Tracked source paths unavailable in the working tree: **0**;
excluded generated/vendor/build source paths: **15**.

| Language | Files | Code lines | Comment lines | Blank lines |
| :--- | ---: | ---: | ---: | ---: |
| Rust | 306 | 144.480 | 27.929 | 12.417 |
| TSX | 89 | 22.894 | 3.294 | 2.650 |
| TypeScript | 87 | 11.780 | 3.248 | 1.402 |
| CSS | 1 | 2.162 | 352 | 345 |
| Python | 5 | 1.696 | 1 | 269 |
| Shell | 4 | 301 | 74 | 42 |
| JavaScript | 1 | 139 | 11 | 0 |

### Direct-source syntax structures

| Language | Parsed files | Functions/methods | Loops | Type declarations | Branch constructs |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Rust | 306/306 | 6.280 | 1.299 | 901 | 3.256 |
| TSX | 0/89 | N/A | N/A | N/A | N/A |
| TypeScript | 0/87 | N/A | N/A | N/A | N/A |
| CSS | N/A (no parser) | N/A | N/A | N/A | N/A |
| Python | 5/5 | 93 | 30 | 15 | 104 |
| Shell | N/A (no parser) | N/A | N/A | N/A | N/A |
| JavaScript | 0/1 | N/A | N/A | N/A | N/A |
| **Supported-source subtotal** | **311/488** | **N/A** | **N/A** | **N/A** | **N/A** |

Structure coverage is **311/488 supported source files**;
**5** source files have no structural parser (for example CSS/Shell).
`N/A` is not zero: if any file of a supported language cannot be parsed, its
language counts and the subtotal are unavailable, not partial values. Counts
are direct syntax nodes (Rust `fn`/`for`/`while`/`loop`, Python definitions and
loops, TypeScript/JavaScript functions including arrows and methods). Type
nodes include Rust structs/enums/traits/unions, Python classes, and JS/TS
classes/interfaces/type aliases/enums. Branches include `if`/`match` in Rust
and Python, or `if`/`switch`/conditional expressions in JS/TS; these are
constructs, **not** individual paths/arms. The supported-source subtotal
combines language-specific construct definitions and is not a measure of
runtime behavior. Macros are not expanded, Python comprehensions are not loops
here, and TS interface method signatures are not executables. Rust's recovery
parser may accept some malformed but balanced source without an `ERROR` node;
these counts are not compiler validation. Tooling: tokei line classification,
rust-analyzer syntax JSON, Python `ast`, and the repository's optional
TypeScript parser. Tokei attributes Rust documentation Markdown to Rust
comments/blanks; **7** nested
code-fence example lines are also classified as comments, not executable
source. Python docstrings remain code lines. Tests are included. Untracked
source, non-programming files, known generated/vendor/build directories and
binary files are excluded.
This is a **current checkout** inventory, not Git history, AI attribution or
proof that all lines execute. Line classification follows tokei's syntax rules.

## Session Time & Execution Analysis
| Provider | Reasoning-adjacent gap estimate | Execution interval proxy | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 191h 30m 33s | 4.328 |
| **Codex** | N/A | 99h 32m 10s | 496 |
| **TOTAL** | **164h 39m 19s** | **291h 2m 44s** | **4.824** |

`N/A` means there is no attributable interval in the available logs; it does
not mean the model achieved enlightenment instantaneously. Claude/Codex local
event gaps shorter than ten minutes are heuristic spans and can include tool
waits and human pauses; they are mixed here with separately source-reported
`duration_ms`/cloud `duration_api_ms`. This proxy is **not measured model generation time**,
and the reasoning-adjacent gap is not a measured thinking duration. The rows
must not be used for provider latency or throughput comparisons.

## Session Wall-Clock Durations

| Measured Hermes sessions | P50 | P90 | P95 | Longest |
| ---: | ---: | ---: | ---: | ---: |
| 62 | 4m 40s | 22m 24s | 25m 32s | 28h 57m 36s |

**62** measured Hermes sessions;
**13** unavailable (open sessions,
invalid timestamps or untrusted end reasons). Percentiles use the nearest-rank
method. Peak concurrency: **4**
trusted closed Hermes sessions, counting touching endpoints as non-overlapping.
Open/orphaned sessions are excluded, so this is not total provider concurrency.
These elapsed start-to-close intervals can include human pauses and tool waits:
they are **not API generation time**. Other providers' session durations are
`N/A` until an equally reliable end marker is verified.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 1.040.074 | 64.662.553 | 14.968.712.268 | 704.340.086 |
| **Codex** | 1.017.712.875 | 8.495.834 | 2.684.009.216 | 0 |
| **TOTAL** | **1.018.752.949** | **73.158.387** | **17.652.721.484** | **704.340.086** |

### 🚀 GRAND TOTAL CONSUMPTION
**19.448.972.906 Total Tokens**

## Weekly Usage Trend

| Week starting (UTC Monday) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| 2026-09-28 | 711.911.974 | 2.511.491 | 15 |
| 2026-09-21 | 4.080.061.141 | 15.099.778 | 381 |
| 2026-09-14 | 4.367.001.624 | 17.583.909 | 1.222 |
| 2026-09-07 | 6.570.282.534 | 28.442.776 | 1.935 |
| 2026-08-31 | 3.695.416.238 | 9.392.251 | 1.271 |

Dated: **19.424.673.511 tokens / 4.824 tasks**;
unallocated: **24.299.395 tokens / 0 tasks**.
The last eight observed weeks are shown in UTC. Dates for local Claude and Codex
are event dates; Hermes session-scoped model usage is attributed to the session
start date, not to an invented per-turn time. Cumulative cloud model usage is
**unallocated across days**; only its timestamped user tasks appear by date.
Trend totals follow the existing collector ledger and are not a billing audit.

## Cloud Attribution Coverage

| Repository-matched cloud sessions | Counted | Local duplicates | No usable ledger | Events unavailable |
| ---: | ---: | ---: | ---: | ---: |
| 3 | 3 | 0 | 0 | 0 |

This diagnostic counts only Claude Code cloud sessions with one matching local
repository/project. It excludes other repositories and sessions without usable
repository identity; it is **not** an account-wide completeness percentage.
Duplicates and missing ledgers contribute no cloud tokens. If a page fails, a
session is skipped rather than counted from a partial transcript.

### API-visible account inventory (metadata only)

| Observation | Sessions |
| :--- | ---: |
| API-visible cloud sessions with unique ID | 3 |
| Uniquely matched local repository | 3 |
| No verifiable repository identity | 0 |
| Conflicting repository identities | 0 |
| No matching local repository | 0 |
| Multiple matching local repositories | 0 |
| Duplicate session-list entries | 0 |
| Invalid cloud session metadata | 0 |

These counts span the cloud sessions returned by the account API, not just the
selected project. They contain no repository names, session IDs or transcript
data. **No verifiable repository** includes identity hints without a trustworthy
host/owner/repo; such sessions are not guessed into a project. A missing API
response is `N/A`, not an observed zero or a claim of account completeness.

## Measured Collector API Payload

**Boundary:** response-body bytes returned to `ai-stats.py` by `urllib` before
JSON decoding. These are **not on-wire bytes**: TLS, HTTP headers, chunk framing
and remote cloud job/model traffic are not measured. GET request headers and
their outbound wire bytes are **N/A**, not zero. Both complete and failed/partial
request attempts are counted as bytes actually read by this process; a failed
request does not prove that the remote side sent zero bytes.

| Attribution | Observed response-body bytes | Attempts | Endpoint |
| :--- | ---: | ---: | :--- |
| Selected-project event reads | 3.628.273 | 11 | event GET |
| Unassigned session listing | 136.177 | 2 | list GET |

First/last observed (UTC): **2026-09-29T16:13:01.800018+00:00 through 2026-09-29T16:13:06.721620+00:00**. Failed or incomplete
attempts in these rows: **0**. Session-listing traffic is
unassigned, not silently attributed to the selected project. Measurement is
opt-in (`AI_STATS_MEASURE_COLLECTOR_API=1`) and began only when enabled; no
historical byte counters are inferred from tokens or log sizes.

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Hermes Agent via Headroom | 3.236.852.230 | 28.148.021 | 13.786.966 | 2.999.427.588 | 0s | 69 | 4.408 |
| Codex CLI | 1.945.039.227 | 988.269.387 | 3.856.816 | 952.913.024 | 99h 32m 10s | 443 | 730 |
| Hermes Agent (direct) | 59.255.761 | 1.815.874 | 197.391 | 57.242.496 | 0s | 6 | 301 |
| Claude Code Cloud | 24.299.395 | 7.787 | 128.182 | 23.746.860 | 47m 53s | 4 | 177 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Sonnet 5 (`claude-sonnet-5`) | 10.199.499.647 | 356.049 | 33.161.909 | 9.842.273.676 | 84h 20m 12s | 1.592 | 49.371 |
| Claude Opus 5 (`claude-opus-5`) | 3.932.069.670 | 136.736 | 21.806.057 | 3.719.638.821 | 101h 51m 24s | 1.338 | 23.265 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 2.260.892.486 | 822.049.580 | 4.288.410 | 1.434.554.496 | 80h 10m 0s | 226 | 876 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 1.364.890.391 | 484.074 | 8.903.282 | 1.185.148.258 | 47m 53s | 22 | 1.373 |
| gpt-6-sol-900k | 749.052.955 | 8.561.112 | 1.610.179 | 738.881.664 | 0s | 3 | 736 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 220.630.773 | 53.996.626 | 1.405.603 | 165.228.544 | 2h 43m 52s | 46 | 2.293 |
| GPT-6 Sol (`gpt-6-sol`) | 181.395.561 | 2.025.718 | 283.891 | 179.085.952 | 0s | 1 | 39 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| GPT-6 Astra (`gpt-6-astra`) | 163.031.658 | 61.760.130 | 474.216 | 100.797.312 | 8h 28m 13s | 14 | 169 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 135.214.492 | 69.319.709 | 433.535 | 65.461.248 | 8h 6m 6s | 32 | 33 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| <synthetic> | 0 | 0 | 0 | 0 | 57m 51s | 138 | 0 |
| Claude Sonnet 4.6 (`claude-sonnet-4-6`) | 0 | 0 | 0 | 0 | 0s | 1 | 0 |
| unknown | 0 | 0 | 0 | 0 | 3m 57s | 1.393 | 0 |

Model IDs are discovered directly from Claude, Codex and Hermes usage records;
display names are loaded dynamically from every available Hermes model cache.
There is no fixed model allowlist, so future persisted model IDs appear automatically
and unknown IDs remain visible verbatim instead of being discarded.

### Latest Models Discovered in Hermes

| Model | Model ID | Released | Project usage status |
| :--- | :--- | :--- | :--- |
| GPT-6.1 Sol (EU) | `gpt-6.1-sol@eu` | 2026-09-29 | available; no selected-project usage |
| GPT-6.1 Sol | `gpt-6.1-sol` | 2026-09-29 | available; no selected-project usage |
| Claude Sonnet 5.5 | `claude-sonnet-5.5` | 2026-09-28 | available; no selected-project usage |
| Claude Sonnet 5.5 (EU) | `claude-sonnet-5-5@eu` | 2026-09-28 | available; no selected-project usage |
| Claude Sonnet 5.5 | `claude-sonnet-5-5@default` | 2026-09-28 | available; no selected-project usage |
| Claude Sonnet 5.5 | `claude-sonnet-5-5` | 2026-09-28 | available; no selected-project usage |
| Claude Opus 5.5 Fast | `claude-opus-5-5-fast` | 2026-09-26 | available; no selected-project usage |
| GPT-6 Sol (EU) | `gpt-6-sol@eu` | 2026-09-22 | available; no selected-project usage |
| GPT-6 Sol | `gpt-6-sol` | 2026-09-22 | used in selected projects |
| GPT-6 Luna (EU) | `gpt-6-luna@eu` | 2026-09-22 | available; no selected-project usage |
| GPT-6 Luna | `gpt-6-luna` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 | `claude-opus-5.5` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 (EU) | `claude-opus-5-5@eu` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 | `claude-opus-5-5@default` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 | `claude-opus-5-5` | 2026-09-22 | used in selected projects |
| GPT Image 2.5 Sunburst | `gpt-image-2.5-sunburst` | 2026-09-08 | available; no selected-project usage |
| GPT Image 2.5 Flare | `gpt-image-2.5-flare` | 2026-09-08 | available; no selected-project usage |
| GPT-6 Astra | `gpt-6-astra` | 2026-09-04 | used in selected projects |
| Claude Fable 5.1 (EU) | `claude-fable-5.1@eu` | 2026-09-01 | available; no selected-project usage |
| Claude Fable 5.1 | `claude-fable-5.1` | 2026-09-01 | available; no selected-project usage |

This catalogue is generated from the newest 20 Claude/GPT entries in the local
Hermes model caches. **Available** is not reported as **used**: a cached model with
no attributable activity remains labelled `available; no selected-project usage`.
That makes newly published models visible without inventing token consumption.

## Model & Cache Trend

| UTC week starting Monday | Model | Tokens | Cache read | Cache-read share |
| :--- | :--- | ---: | ---: | ---: |
| 2026-09-21 | Claude Sonnet 5 (`claude-sonnet-5`) | 64.808.814 | 58.343.740 | 90.4% |
| 2026-09-14 | Claude Sonnet 5 (`claude-sonnet-5`) | 2.094.049.771 | 2.022.052.095 | 96.9% |
| 2026-09-07 | Claude Sonnet 5 (`claude-sonnet-5`) | 4.588.883.015 | 4.376.501.810 | 95.7% |
| 2026-08-31 | Claude Sonnet 5 (`claude-sonnet-5`) | 3.451.758.047 | 3.385.376.031 | 98.3% |
| 2026-09-21 | Claude Opus 5 (`claude-opus-5`) | 356.962.453 | 321.987.598 | 90.5% |
| 2026-09-14 | Claude Opus 5 (`claude-opus-5`) | 1.739.086.785 | 1.656.048.885 | 95.7% |
| 2026-09-07 | Claude Opus 5 (`claude-opus-5`) | 1.692.215.588 | 1.605.147.744 | 95.4% |
| 2026-08-31 | Claude Opus 5 (`claude-opus-5`) | 143.804.844 | 136.454.594 | 95.5% |
| 2026-09-21 | GPT-5.6 Sol (`gpt-5.6-sol`) | 1.808.312.470 | 1.212.236.032 | 67.2% |
| 2026-09-14 | GPT-5.6 Sol (`gpt-5.6-sol`) | 452.580.016 | 222.318.464 | 49.2% |
| 2026-09-28 | Claude Opus 5.5 (`claude-opus-5-5`) | 88.638.028 | 72.098.025 | 82.3% |
| 2026-09-21 | Claude Opus 5.5 (`claude-opus-5-5`) | 1.251.952.968 | 1.089.303.373 | 87.5% |
| unallocated | Claude Opus 5.5 (`claude-opus-5-5`) | 24.299.395 | 23.746.860 | 98.2% |
| 2026-09-28 | `gpt-6-sol-900k` | 622.178.582 | 613.967.232 | 98.9% |
| 2026-09-21 | `gpt-6-sol-900k` | 126.874.373 | 124.914.432 | 98.7% |

Top five models by selected-project token volume, across the last eight
observed UTC weeks. **Unallocated** means that a canonical model ledger has
no reliable per-day timestamp (especially cumulative cloud usage); it is not
placed on the last result's day. Model-time mapping covers
**19.448.972.906 tokens**; **0 tokens**
have no model-time evidence. Cache-read share uses cache-read / (input +
cache-read + cache-creation) tokens, excluding output; a zero denominator is
`N/A`. It is not an invoice, a measured cost saving, or proof of cache hit
quality. Raw model IDs remain visible even without catalogue metadata.

## External AI Integrations

| Integration | Detection status | Attributable tool calls |
| :--- | :--- | ---: |
| Laya | configured; no attributable tool calls | 0 |

Laya is detected from Hermes configuration and executed Laya MCP tool names. Its
router-selected model work remains accounted under the concrete model recorded by
the resulting Claude/Codex/Hermes session; only explicit Laya MCP calls are counted
in the integration row, avoiding double-counted tokens.

## Effort Breakdown

| Effort | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| high | 15.624.616.985 | 377.696.447 | 58.648.208 | 14.640.835.700 | 200h 23m 58s | 2.835 | 69.765 |
| medium | 2.288.078.606 | 638.147.620 | 8.079.778 | 1.541.698.025 | 61h 4m 27s | 249 | 3.639 |
| xhigh | 1.303.655.495 | 1.137.024 | 5.622.789 | 1.256.961.469 | 25h 6m 59s | 318 | 4.582 |
| unknown | 216.504.459 | 644.706 | 518.043 | 199.148.789 | 3h 54m 51s | 1.418 | 2.506 |
| max | 16.117.361 | 1.127.152 | 289.569 | 14.077.501 | 31m 33s | 4 | 111 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | Bash | 55.117 |
| 2 | terminal | 13.049 |
| 3 | read_file | 10.503 |
| 4 | Read | 10.084 |
| 5 | Edit | 6.008 |
| 6 | patch | 3.375 |
| 7 | execute_code | 3.216 |
| 8 | search_files | 2.841 |
| 9 | tool_call | 1.273 |
| 10 | Write | 1.169 |
| 11 | Agent | 1.001 |
| 12 | skill_view | 601 |
| 13 | write_file | 594 |
| 14 | ToolSearch | 582 |
| 15 | Monitor | 372 |
| 16 | wait_agent | 368 |
| 17 | SendMessage | 194 |
| 18 | Skill | 189 |
| 19 | TaskStop | 167 |
| 20 | delegate_task | 147 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | compressed-tool-output-recovery | 54 |
| 2 | test-driven-development | 49 |
| 3 | knx-live-bus-operations | 47 |
| 4 | hermes-agent | 38 |
| 5 | repository-delivery | 38 |
| 6 | superpowers:subagent-driven-development | 35 |
| 7 | superpowers:finishing-a-development-branch | 31 |
| 8 | autonomous-goal-boundaries | 30 |
| 9 | requesting-code-review | 29 |
| 10 | superpowers:brainstorming | 28 |
| 11 | superpowers:writing-plans | 24 |
| 12 | local-telemetry-collectors | 20 |
| 13 | superpowers:using-git-worktrees | 19 |
| 14 | codex | 18 |
| 15 | playwright | 18 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

### Claude Code Local Tool Results

| Tool | Explicit success | Explicit error | Status unavailable | Error share of known |
| :--- | ---: | ---: | ---: | ---: |
| Bash | 53.346 | 1.614 | 0 | 2.9% |
| Read | 0 | 135 | 9.946 | 100.0% |
| Edit | 0 | 82 | 5.922 | 100.0% |
| Write | 0 | 3 | 1.162 | 100.0% |
| Agent | 0 | 8 | 993 | 100.0% |
| ToolSearch | 0 | 0 | 578 | N/A |
| Monitor | 0 | 2 | 369 | 100.0% |
| SendMessage | 0 | 3 | 191 | 100.0% |
| Skill | 0 | 3 | 186 | 100.0% |
| TaskStop | 0 | 5 | 162 | 100.0% |
| ListAgents | 0 | 0 | 98 | N/A |
| mcp__headroom__headroom_retrieve | 0 | 0 | 29 | N/A |
| TaskUpdate | 0 | 1 | 26 | 100.0% |
| TaskCreate | 0 | 0 | 25 | N/A |
| AskUserQuestion | 0 | 0 | 17 | N/A |
| **Other tools (13)** | 0 | 2 | 79 | 100.0% |

**0** unmatched result blocks/IDs remain
unattributed. Results are joined to tool names by `tool_use_id` inside each
local Claude log and deduplicated by that ID. Success is **not inferred from missing**
`is_error`: absent or conflicting flags are status unavailable, and error share
excludes those unknowns. Claude cloud, Codex and Hermes outcomes are `N/A`:
their call counts here must not be mistaken for successful completions. No
arguments, result contents, paths or credentials enter this table.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 41,845.3 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 5.03:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 131.3 | AI output ÷ added and deleted Git lines |
| Git commit density | 11,549,271.3 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $8,104.95 | 95.5% | $40,415.52 |
| Codex | $3,342.72 | 72.5% | $6,039.02 |
| **TOTAL** | **$11,447.67** | **91.1%** | **$46,454.54** |

Price scenario (retrieved 2026-09-24): Claude uses public Sonnet 4 rates
($3 input / $15 output / $0.30 cache read / $3.75 cache write per MTok) from
[Anthropic pricing](https://www.anthropic.com/pricing). Codex uses public GPT-5.4
rates ($2.50 input / $15 output / $0.25 cached input per MTok) from
[OpenAI API pricing](https://openai.com/api/pricing/); cache creation is conservatively
priced as ordinary input. These are **API-equivalent estimates, not invoices**:
OAuth/subscription usage can have zero marginal API cost, model routing varies, and
tool/search/container charges are excluded. Cache savings compare cached reads with
the corresponding ordinary-input list rate.

## Time & Intensity Patterns

- **Prime hour:** 16:00–17:00 local time (339 logged activity events)
- **Prime weekday:** Sunday (1.275 logged activity events)
- **Generation throughput:** N/A — output tokens and measured generation seconds are not joined per event across all sources
- **Autonomy input/output ratio:** 13.93:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 16.71 logged tool calls per task (80.603 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 545 | 11.3% |
| Morning (06-11) | 1.468 | 30.4% |
| Afternoon (12-17) | 1.629 | 33.8% |
| Evening (18-23) | 1.182 | 24.5% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 526 | 10.9% |
| Tuesday | 300 | 6.2% |
| Wednesday | 242 | 5.0% |
| Thursday | 825 | 17.1% |
| Friday | 581 | 12.0% |
| Saturday | 1.075 | 22.3% |
| Sunday | 1.275 | 26.4% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 1,944.9 kWh
- **Estimated CO₂ equivalent:** 641.8 kg CO₂e
- **Estimated physical keystrokes avoided:** 292.633.548

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.
