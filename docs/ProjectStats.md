# Project Statistics

**Last update:** 2026-10-08 21:16:35 CEST (UTC+02:00)

**Source revision:** `16163ef7aff503d3a2601ed33b364252101cb96f` — freshly fetched `origin/main`; collection precedes the report's publication commit.

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **54.058.501 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **45.048 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **368.6 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **18.345 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **108.117 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **5.4 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **22215.8 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **2.162.340 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **237.3 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **388.9 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **348.873.372 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **7.21x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **540.6 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **216.234 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **21.623 kilometers** (about **0.54x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **8.65 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **80.55 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **308.905.723 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **1.081.170.032 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **57.282 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **2.162 kWh** and **713.6 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 2.322
- **Merges:** 261
- **Pushes/Sync:** 2.322
- **Lines Added (+):** 691.363
- **Lines Deleted (-):** 220.383

## Git Text Churn Distribution

| P50 changed text lines | P90 | P95 |
| ---: | ---: | ---: |
| 112 | 813 | 1.389 |

**2.020** measured commits;
**302** unavailable (for example, merge-only
history or binary/unsupported `--numstat` entries). A zero means Git explicitly
reported zero text-line changes; missing data is not turned into zero. This
counts historical additions + deletions per commit, **not AI-attributed** work
or surviving source lines. Branch lifetime cannot be inferred from this table.

## Coding Statistics (Current Working Tree)

**Git-tracked source files:** 926; **physical lines:** 345.863;
**code:** 280.886; **comments:** 41.450; **blank:** 23.527.
**Code/comment ratio:** 6.78:1; **comment share of nonblank lines:** 12.9%.
Tracked source paths unavailable in the working tree: **0**;
excluded generated/vendor/build source paths: **21**.

| Language | Files | Code lines | Comment lines | Blank lines |
| :--- | ---: | ---: | ---: | ---: |
| Rust | 474 | 205.539 | 32.509 | 16.088 |
| TSX | 158 | 35.502 | 3.831 | 3.525 |
| TypeScript | 239 | 29.875 | 4.550 | 2.696 |
| Python | 34 | 4.104 | 59 | 659 |
| CSS | 4 | 3.514 | 415 | 427 |
| JavaScript | 14 | 2.176 | 61 | 106 |
| Shell | 3 | 176 | 25 | 26 |

### Direct-source syntax structures

| Language | Parsed files | Functions/methods | Loops | Type declarations | Branch constructs |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Rust | 474/474 | 8.603 | 2.032 | 1.218 | 4.646 |
| TSX | 0/158 | N/A | N/A | N/A | N/A |
| TypeScript | 0/239 | N/A | N/A | N/A | N/A |
| Python | 34/34 | 279 | 157 | 36 | 333 |
| CSS | N/A (no parser) | N/A | N/A | N/A | N/A |
| JavaScript | 0/14 | N/A | N/A | N/A | N/A |
| Shell | N/A (no parser) | N/A | N/A | N/A | N/A |
| **Supported-source subtotal** | **508/919** | **N/A** | **N/A** | **N/A** | **N/A** |

Structure coverage is **508/919 supported source files**;
**7** source files have no structural parser (for example CSS/Shell).
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
comments/blanks; **10** nested
code-fence example lines are also classified as comments, not executable
source. Python docstrings remain code lines. Tests are included. Untracked
source, non-programming files, known generated/vendor/build directories and
binary files are excluded.
This is a **current checkout** inventory, not Git history, AI attribution or
proof that all lines execute. Line classification follows tokei's syntax rules.

## Session Time & Execution Analysis
| Provider | Reasoning-adjacent gap estimate | Execution interval proxy | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 153h 33m 53s | 176h 26m 28s | 3.843 |
| **Codex** | N/A | 113h 16m 55s | 694 |
| **Other** | N/A | N/A | 0 |
| **TOTAL** | **153h 33m 53s** | **289h 43m 24s** | **4.537** |

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
| 59 | 4m 41s | 22m 24s | 25m 32s | 1h 22m 8s |

**59** measured Hermes sessions;
**74** unavailable (open sessions,
invalid timestamps or untrusted end reasons). Percentiles use the nearest-rank
method. Peak concurrency: **3**
trusted closed Hermes sessions, counting touching endpoints as non-overlapping.
Open/orphaned sessions are excluded, so this is not total provider concurrency.
These elapsed start-to-close intervals can include human pauses and tool waits:
they are **not API generation time**. Other providers' session durations are
`N/A` until an equally reliable end marker is verified.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 4.478.054 | 64.932.511 | 15.480.779.476 | 665.995.115 |
| **Codex** | 135.778.420 | 22.285.832 | 5.249.151.232 | 0 |
| **Other** | 0 | 0 | 0 | 0 |
| **TOTAL** | **140.256.474** | **87.218.343** | **20.729.930.708** | **665.995.115** |

### 🚀 GRAND TOTAL CONSUMPTION
**21.623.400.640 Total Tokens**

## Weekly Usage Trend

| Week starting (UTC Monday) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| 2026-10-05 | 1.444.061.928 | 5.856.676 | 36 |
| 2026-09-28 | 4.809.439.684 | 20.582.120 | 32 |
| 2026-09-21 | 2.699.241.852 | 10.225.718 | 384 |
| 2026-09-14 | 4.119.946.629 | 17.575.916 | 1.222 |
| 2026-09-07 | 6.555.024.646 | 28.737.531 | 2.106 |
| 2026-08-31 | 1.995.685.901 | 4.240.382 | 757 |

Dated: **21.623.400.640 tokens / 4.537 tasks**;
unallocated: **0 tokens / 0 tasks**.
The last eight observed weeks are shown in UTC. Dates for local Claude and Codex
are event dates; Hermes session-scoped model usage is attributed to the session
start date, not to an invented per-turn time. Cumulative cloud model usage is
**unallocated across days**; only its timestamped user tasks appear by date.
Trend totals follow the existing collector ledger and are not a billing audit.

## Cloud Attribution Coverage

| Repository-matched cloud sessions | Counted | Local duplicates | No usable ledger | Events unavailable |
| ---: | ---: | ---: | ---: | ---: |
Cloud collection is disabled; cloud sessions were not queried.

This diagnostic counts only Claude Code cloud sessions with one matching local
repository/project. It excludes other repositories and sessions without usable
repository identity; it is **not** an account-wide completeness percentage.
Duplicates and missing ledgers contribute no cloud tokens. If a page fails, a
session is skipped rather than counted from a partial transcript.

### API-visible account inventory (metadata only)

| Observation | Sessions |
| :--- | ---: |
| API-visible cloud sessions | N/A (not queried) |

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
| Selected-project event reads | N/A | N/A | N/A |
| Unassigned session listing | N/A | N/A | N/A |

First/last observed (UTC): **N/A — no persisted observations for this selection.**. Failed or incomplete
attempts in these rows: **N/A**. Session-listing traffic is
unassigned, not silently attributed to the selected project. Measurement is
opt-in (`--measure-api`) and began only when enabled; no
historical byte counters are inferred from tokens or log sizes.

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 12.483.795.956 | 291.478 | 50.037.163 | 11.965.507.607 | 176h 26m 28s | 3.790 | 69.612 |
| Hermes Agent via Headroom | 5.983.457.319 | 98.333.705 | 26.594.159 | 5.717.739.737 | 0s | 106 | 8.756 |
| Hermes Agent (direct) | 2.103.400.848 | 3.460.881 | 6.488.610 | 2.036.205.668 | 0s | 27 | 3.268 |
| Codex CLI | 1.052.746.517 | 38.170.410 | 4.098.411 | 1.010.477.696 | 113h 16m 55s | 614 | 797 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Sonnet 5 (`claude-sonnet-5`) | 8.638.737.325 | 162.531 | 28.854.876 | 8.318.199.041 | 72h 28m 43s | 1.317 | 44.974 |
| Claude Opus 5 (`claude-opus-5`) | 3.767.184.218 | 100.370 | 20.790.313 | 3.568.644.548 | 100h 14m 31s | 1.298 | 22.640 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 3.593.022.285 | 2.438.775 | 13.836.813 | 3.396.527.979 | 0s | 49 | 5.844 |
| GPT-6.1 Sol (`gpt-6.1-sol`) | 1.846.669.227 | 58.330.213 | 11.509.830 | 1.776.829.184 | 0s | 16 | 1.338 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 1.460.223.501 | 34.710.821 | 4.243.304 | 1.421.269.376 | 80h 10m 0s | 227 | 891 |
| gpt-6-sol-900k | 1.217.683.037 | 13.755.988 | 2.635.145 | 1.201.291.904 | 0s | 3 | 624 |
| gpt-6.1-sol-900k | 264.357.869 | 4.027.509 | 921.464 | 259.408.896 | 0s | 8 | 1.341 |
| GPT-6 Sol (`gpt-6-sol`) | 190.171.069 | 2.082.754 | 303.739 | 187.784.576 | 0s | 3 | 124 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 183.022.732 | 13.937.809 | 2.164.987 | 166.919.936 | 3h 0m 5s | 55 | 2.293 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 153.728.019 | 38.190 | 330.177 | 140.766.004 | 2h 37m 49s | 11 | 1.781 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 136.567.795 | 6.447.704 | 685.851 | 129.434.240 | 20h 9m 9s | 174 | 87 |
| GPT-6 Astra (`gpt-6-astra`) | 110.929.072 | 4.207.023 | 508.929 | 106.213.120 | 9h 53m 35s | 21 | 182 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| <synthetic> | 0 | 0 | 0 | 0 | 33m 35s | 113 | 0 |
| Claude Sonnet 4.6 (`claude-sonnet-4-6`) | 0 | 0 | 0 | 0 | 0s | 1 | 0 |
| unknown | 0 | 0 | 0 | 0 | 4m 4s | 1.235 | 0 |

Model IDs are discovered directly from Claude, Codex and Hermes usage records;
display names are loaded dynamically from every available Hermes model cache.
There is no fixed model allowlist, so future persisted model IDs appear automatically
and unknown IDs remain visible verbatim instead of being discarded.

### Latest Models Discovered in Hermes

| Model | Model ID | Released | Project usage status |
| :--- | :--- | :--- | :--- |
| Claude Haiku 5.5 | `claude-haiku-5.5` | 2026-10-07 | available; no selected-project usage |
| Claude Haiku 5.5 (EU) | `claude-haiku-5-5@eu` | 2026-10-07 | available; no selected-project usage |
| Claude Haiku 5.5 | `claude-haiku-5-5@default` | 2026-10-07 | available; no selected-project usage |
| Claude Haiku 5.5 | `claude-haiku-5-5` | 2026-10-07 | available; no selected-project usage |
| GPT-6.1 Sol (EU) | `gpt-6.1-sol@eu` | 2026-09-29 | available; no selected-project usage |
| GPT-6.1 Sol | `gpt-6.1-sol` | 2026-09-29 | used in selected projects |
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
| 2026-08-31 | Claude Sonnet 5 (`claude-sonnet-5`) | 1.890.995.725 | 1.861.301.396 | 98.6% |
| 2026-09-21 | Claude Opus 5 (`claude-opus-5`) | 303.582.253 | 276.558.994 | 91.4% |
| 2026-09-14 | Claude Opus 5 (`claude-opus-5`) | 1.739.086.785 | 1.656.048.885 | 95.7% |
| 2026-09-07 | Claude Opus 5 (`claude-opus-5`) | 1.692.215.588 | 1.605.147.744 | 95.4% |
| 2026-08-31 | Claude Opus 5 (`claude-opus-5`) | 32.299.592 | 30.888.925 | 95.7% |
| 2026-10-05 | Claude Opus 5.5 (`claude-opus-5-5`) | 924.284.961 | 897.900.947 | 97.5% |
| 2026-09-28 | Claude Opus 5.5 (`claude-opus-5-5`) | 2.117.292.797 | 2.025.763.198 | 96.0% |
| 2026-09-21 | Claude Opus 5.5 (`claude-opus-5-5`) | 551.444.527 | 472.863.834 | 86.3% |
| 2026-10-05 | GPT-6.1 Sol (`gpt-6.1-sol`) | 255.419.098 | 246.546.944 | 97.1% |
| 2026-09-28 | GPT-6.1 Sol (`gpt-6.1-sol`) | 1.591.250.129 | 1.530.282.240 | 96.8% |
| 2026-09-21 | GPT-5.6 Sol (`gpt-5.6-sol`) | 1.233.389.499 | 1.202.323.712 | 97.8% |
| 2026-09-14 | GPT-5.6 Sol (`gpt-5.6-sol`) | 226.834.002 | 218.945.664 | 96.9% |

Top five models by selected-project token volume, across the last eight
observed UTC weeks. **Unallocated** means that a canonical model ledger has
no reliable per-day timestamp (especially cumulative cloud usage); it is not
placed on the last result's day. Model-time mapping covers
**21.623.400.640 tokens**; **0 tokens**
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
| high | 19.690.318.812 | 108.160.001 | 78.769.013 | 18.890.785.386 | 189h 45m 52s | 2.597 | 72.593 |
| medium | 1.058.713.712 | 29.712.689 | 3.862.468 | 1.014.352.332 | 73h 24m 19s | 406 | 4.195 |
| xhigh | 701.478.238 | 1.429.132 | 4.171.672 | 665.866.602 | 22h 4m 42s | 273 | 3.654 |
| unknown | 171.753.871 | 879.425 | 402.602 | 157.878.196 | 4h 8m 17s | 1.261 | 1.989 |
| max | 1.136.007 | 75.227 | 12.588 | 1.048.192 | 19m 17s | 0 | 2 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | Bash | 51.657 |
| 2 | read_file | 34.270 |
| 3 | terminal | 22.187 |
| 4 | execute_code | 13.457 |
| 5 | skill_view | 9.955 |
| 6 | patch | 9.226 |
| 7 | Read | 9.203 |
| 8 | search_files | 6.923 |
| 9 | Edit | 5.290 |
| 10 | tool_call | 3.692 |
| 11 | write_file | 2.039 |
| 12 | Write | 941 |
| 13 | Agent | 880 |
| 14 | ToolSearch | 554 |
| 15 | tool_describe | 495 |
| 16 | Monitor | 369 |
| 17 | wait_agent | 368 |
| 18 | vision_analyze | 353 |
| 19 | SendMessage | 187 |
| 20 | skill_manage | 168 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | autonomous-goal-boundaries | 926 |
| 2 | test-driven-development | 919 |
| 3 | cross-layer-contract-verification | 852 |
| 4 | compressed-tool-output-recovery | 843 |
| 5 | frontend-design | 720 |
| 6 | playwright | 685 |
| 7 | repository-delivery | 675 |
| 8 | systematic-debugging | 532 |
| 9 | safety-confirmation-gates | 467 |
| 10 | private-corpus-regression-testing | 434 |
| 11 | requesting-code-review | 426 |
| 12 | data-integrity-feature-verification | 367 |
| 13 | project-completion-auditing | 357 |
| 14 | hermes-agent | 280 |
| 15 | grounded-citations | 250 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

### Claude Code Local Tool Results

| Tool | Explicit success | Explicit error | Status unavailable | Error share of known |
| :--- | ---: | ---: | ---: | ---: |
| Bash | 50.275 | 1.382 | 0 | 2.7% |
| Read | 0 | 130 | 9.073 | 100.0% |
| Edit | 0 | 74 | 5.216 | 100.0% |
| Write | 0 | 3 | 938 | 100.0% |
| Agent | 0 | 6 | 874 | 100.0% |
| ToolSearch | 0 | 0 | 554 | N/A |
| Monitor | 0 | 1 | 368 | 100.0% |
| SendMessage | 0 | 3 | 184 | 100.0% |
| TaskStop | 0 | 4 | 160 | 100.0% |
| Skill | 0 | 1 | 130 | 100.0% |
| ListAgents | 0 | 0 | 95 | N/A |
| mcp__headroom__headroom_retrieve | 0 | 0 | 29 | N/A |
| TaskUpdate | 0 | 1 | 26 | 100.0% |
| TaskCreate | 0 | 0 | 25 | N/A |
| TaskOutput | 0 | 0 | 13 | N/A |
| **Other tools (10)** | 0 | 1 | 46 | 100.0% |

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
| Tokens per committed added line | 31,276.5 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 3.14:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 95.7 | AI output ÷ added and deleted Git lines |
| Git commit density | 9,312,403.4 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $8,129.14 | 95.8% | $41,798.10 |
| Codex | $1,986.02 | 97.5% | $11,810.59 |
| Other | N/A | N/A | N/A |
| **PRICED SUBTOTAL** | **$10,115.16** | **96.3%** | **$53,608.69** |

`Other` preserves unknown Hermes providers/models in usage totals. Their cost is
not assigned Claude/OpenAI prices; the priced subtotal excludes them. Cache share
is the overall observed share, not a measured billing discount.

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

- **Prime hour:** 16:00–17:00 local time (337 logged activity events)
- **Prime weekday:** Sunday (1.283 logged activity events)
- **Generation throughput:** N/A — output tokens and measured generation seconds are not joined per event across all sources
- **Autonomy input/output ratio:** 1.61:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 18.17 logged tool calls per task (82.433 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 518 | 11.4% |
| Morning (06-11) | 1.365 | 30.1% |
| Afternoon (12-17) | 1.545 | 34.1% |
| Evening (18-23) | 1.109 | 24.4% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 536 | 11.8% |
| Tuesday | 396 | 8.7% |
| Wednesday | 230 | 5.1% |
| Thursday | 640 | 14.1% |
| Friday | 373 | 8.2% |
| Saturday | 1.079 | 23.8% |
| Sunday | 1.283 | 28.3% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 2,162.3 kWh
- **Estimated CO₂ equivalent:** 713.6 kg CO₂e
- **Estimated physical keystrokes avoided:** 348.873.372

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.


## Session Attribution Scope

Sessions are matched to the current project plus explicitly configured additional roots. Historical names and separately selected worktrees are not inferred from bare basenames.

Preserved original manual selection labels: **17**. Labels are provenance, not fabricated session data; actual usage still requires local metadata or explicitly enabled cloud collection.

- Additional session root: <code>/mnt/daten-i/Sourcecode/KNX</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees/pdb1-safe-member-names</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees/pdb3-install-reports</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees/pdb5-schemes12-14</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees/pdb6-scheme21</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees/pdb7-catalog-metadata</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees/t14-report-residue</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees/t16-group-address-csv</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/knx-spec-kb</code>
- Additional session root: <code>/mnt/daten-i/Sourcecode/KNXBench.worktrees</code>
- Hermes profile without recorded working directory: <code>knxbench</code> (43 sessions)

Listed Hermes profiles are an explicit project choice: their sessions that recorded neither a working directory nor a Git root are attributed here. Sessions with a recorded location are still matched by path.

Cloud sessions are not queried when the saved cloud setting is disabled.

## Project Size & File Inventory

Scope: **partial — measured subset only**, no tracked index/stat changes detected; HEAD revision: <code>16163ef7aff503d3a2601ed33b364252101cb96f</code>.

Git index/stat metadata only; conservative, without content comparisons. Secret/cache/report exclusions and submodule working contents are not assessed.

Logical bytes are file lengths, not disk allocation. Source suffixes are counted independently of categories; tests/docs take category precedence. Source lines are streaming physical UTF-8 text lines, including comments and blanks, not parser/tokei code counts.

Directories are unique parents of measured tracked files, including the root at depth zero. File depth means its parent directory depth. Name/path lengths count Unicode characters (not graphemes) separately from filesystem-encoded bytes, excluding the absolute root prefix.

| Metric | Value |
| --- | --- |
| Files (measured) | 1,316 |
| Logical bytes (measured) | 38,927,543 |
| Source files | 917 |
| Source logical bytes | 14,056,334 |
| Source physical lines | 344,938 |
| Tracked directories (including root) | 147 |
| Maximum directory depth | 6 |

| Category | Files | Logical bytes |
| --- | --- | --- |
| source | 551 | 9,680,666 |
| tests | 364 | 4,351,351 |
| docs | 281 | 20,352,353 |
| config | 55 | 400,528 |
| assets | 25 | 2,687,558 |
| other | 40 | 1,455,087 |

Readable UTF-8 source files: **917**; median file lines: **181**; p95 file lines: **1,282** (nearest-rank). Unreadable source files are excluded from distributions; their aggregate source lines are N/A.

### Extension distribution (top 10 by bytes)

| Extension | Files | Logical bytes |
| --- | --- | --- |
| <code>.gif</code> | 5 | 11,468,773 |
| <code>.rs</code> | 466 | 9,758,528 |
| <code>.md</code> | 206 | 4,415,573 |
| <code>.png</code> | 38 | 3,922,431 |
| <code>.mp4</code> | 3 | 2,114,631 |
| <code>.tsx</code> | 158 | 1,921,659 |
| <code>.ts</code> | 238 | 1,824,490 |
| <code>.sqlite</code> | 7 | 860,160 |
| <code>.json</code> | 43 | 603,492 |
| <code>.html</code> | 19 | 434,376 |

### File and path extremes

| Extreme | Path/name | Measurement |
| --- | --- | --- |
| Longest filename | <code>docs/adr/0053-contributions-come-with-a-license-grant-for-dual-licensing.md</code> | 66 Unicode characters; 66 filesystem-encoded bytes |
| Longest relative path | <code>docs/adr/0053-contributions-come-with-a-license-grant-for-dual-licensing.md</code> | 75 Unicode characters; 75 filesystem-encoded bytes |
| Deepest file | <code>crates/knx-productdb/fixtures/legacy/src-pr/MARVIN/ets.pr&#95;</code> | Parent depth 6 |
| Shortest nonempty readable source | <code>apps/knx-web/src/vite-env.d.ts</code> | 1 physical lines; 38 bytes |
| Longest readable source line | <code>crates/knx-app/src/contribution&#95;bundle.rs</code> | Line 197; 1,036 Unicode characters (without newline) |
| Most common basename (case-sensitive) | <code>Cargo.toml</code> | 19 files |

### Largest files by logical bytes (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>docs/assets/readme/hero-add-device.gif</code> | 7,883,659 | N/A |
| <code>docs/assets/readme/telegram-flow.gif</code> | 2,752,893 | N/A |
| <code>website/assets/project-work.mp4</code> | 1,256,389 | N/A |
| <code>docs/history/IMPLEMENTATION&#95;STATUS&#95;2026-09.md</code> | 832,001 | N/A |
| <code>website/assets/bus-flow.mp4</code> | 761,738 | N/A |
| <code>docs/KNOWN&#95;LIMITATIONS.md</code> | 559,508 | N/A |
| <code>story/previews/2026-10-08.4.html</code> | 354,505 | N/A |
| <code>docs/IMPLEMENTATION&#95;STATUS.md</code> | 350,751 | N/A |
| <code>crates/knx-core/src/command.rs</code> | 340,261 | 8,964 |
| <code>docs/assets/workflows/new-project.gif</code> | 321,760 | N/A |

### Largest source files by physical lines (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>crates/knx-core/src/command.rs</code> | 340,261 | 8,964 |
| <code>apps/knx-server/src/domain.rs</code> | 302,160 | 7,468 |
| <code>crates/knx-net/src/commissioning.rs</code> | 266,441 | 6,221 |
| <code>crates/knx-core/src/dpt/codec.rs</code> | 238,140 | 5,961 |
| <code>apps/knx-cli/src/main.rs</code> | 184,379 | 4,852 |
| <code>crates/knx-productdb/tests/dynamic&#95;tree.rs</code> | 176,748 | 4,443 |
| <code>crates/knx-productdb/src/migration.rs</code> | 186,163 | 4,237 |
| <code>crates/knx-net/src/cemi.rs</code> | 177,343 | 4,132 |
| <code>crates/knx-productdb/src/query.rs</code> | 164,613 | 3,958 |
| <code>apps/knx-web/src/styles.css</code> | 137,928 | 3,889 |

### Empty files (up to 10)

Total empty files: **0**.

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |

### Case-insensitive path collisions (Unicode casefold; up to 10 groups)

Total collision groups: **0**.

| Casefolded path | Files | Paths (up to 10) |
| --- | --- | --- |

### Exclusions and coverage

- Only current Git-tracked working files; untracked files are excluded.
- Git-ignored files, caches, generated files, third-party and build directories are excluded.
- Excluded directory components&#58; .cache, .git, .mypy&#95;cache, .nox, .pytest&#95;cache, .ruff&#95;cache, .tox, .venv, &#95;&#95;pycache&#95;&#95;, bindings, build, coverage, dist, generated, node&#95;modules, target, vendor, venv.
- docs/ProjectStats.md and .projectstats state are excluded to prevent report self-growth.
- Secret/credential-looking paths are excluded without opening files or exposing names; heuristic, not a secret scan.
- Symlink files/ancestors, submodules, nonregular files and unsafe paths are excluded.
- Excluded generated or cache: 21.
- Excluded ignored: 257.
- Excluded report or state: 1.
- Excluded secret or credential: 11.
- Excluded symlink: 1.
- Unavailable tracked working files: 1; unreadable source files: 0.
- Excluded tracked files&#58; generated or cache=21, ignored=257, report or state=1, secret or credential=11, symlink=1.
- Partial inventory&#58; unavailable working files are omitted; unreadable source lines are N/A.


## Package Dependencies

Offline, Git-tracked declarations and lockfiles only; no package code is executed.

| Metric | Value |
|---|---:|
| Unique declared dependencies | 65 |
| Unique locked package versions | N/A |
| Packages with multiple locked versions | N/A |
| Internal/local package names | 19 |
| Maximum resolved depth (edges) | N/A |
| Unresolved lock edges | 26 |
| Cyclic components | 0 |

Declaration coverage: complete.
Lock inventory coverage: **incomplete / unavailable**; resolved records are a measured subset.

### Most frequently declared packages (up to 10)

| Ecosystem | Package | Declarations |
|---|---|---:|
| cargo | knx\-core | 13 |
| cargo | tempfile | 12 |
| cargo | chrono | 8 |
| cargo | serde\_json | 8 |
| cargo | zip | 7 |
| cargo | knx\-testsupport | 6 |
| cargo | serde | 6 |
| cargo | tokio | 6 |
| cargo | knx\-store | 5 |
| cargo | sha2 | 5 |
Other 55 package names omitted; full records remain in JSON.

### Multiple locked versions (up to 5; observed subset if coverage is incomplete)

- cargo / windows\-sys: 0\.45\.0, 0\.52\.0, 0\.59\.0; other versions omitted
- cargo / base64: 0\.21\.7, 0\.22\.1, 0\.23\.1
- cargo / getrandom: 0\.2\.17, 0\.3\.4, 0\.4\.3
- cargo / hashbrown: 0\.12\.3, 0\.16\.1, 0\.17\.1
- cargo / proc\-macro\-crate: 1\.3\.1, 2\.0\.2, 3\.5\.0

### Compact resolved tree

At most 3 roots, 10 direct branches, display depth 3 and 30 tree lines; repeated nodes are not expanded.

- Cargo\.lock:package:knx\-cli@0\.1\.0\-alpha\.5\#194
  - Cargo\.lock:package:chrono@0\.4\.45\#45
    - Cargo\.lock:package:iana\-time\-zone@0\.1\.65\#159
      - Cargo\.lock:package:android\_system\_properties@0\.1\.6\#4
      - Cargo\.lock:package:core\-foundation\-sys@0\.8\.7\#49
      - Cargo\.lock:package:iana\-time\-zone\-haiku@0\.1\.2\#160
    - Cargo\.lock:package:num\-traits@0\.2\.19\#241
      - Cargo\.lock:package:autocfg@1\.5\.1\#13
    - Cargo\.lock:package:serde@1\.0\.229\#332
      - Cargo\.lock:package:serde\_core@1\.0\.229\#334
      - Cargo\.lock:package:serde\_derive@1\.0\.229\#335
  - Cargo\.lock:package:knx\-app@0\.1\.0\-alpha\.3\#192
    - Cargo\.lock:package:base64@0\.22\.1\#17
    - Cargo\.lock:package:chrono@0\.4\.45\#45 (repeated/cyclic)
    - Cargo\.lock:package:getrandom@0\.4\.3\#131
      - Cargo\.lock:package:cfg\-if@1\.0\.4\#44
      - Cargo\.lock:package:libc@0\.2\.189\#213
      - Cargo\.lock:package:r\-efi@6\.0\.0\#297
  - Cargo\.lock:package:knx\-build\-stamp@0\.1\.0\-alpha\.1\#193
    - Cargo\.lock:package:tempfile@3\.27\.0\#388
      - Cargo\.lock:package:fastrand@2\.5\.0\#101
      - Cargo\.lock:package:getrandom@0\.4\.3\#131 (repeated/cyclic)
      - Cargo\.lock:package:once\_cell@1\.21\.4\#262
  - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#195
    - Cargo\.lock:package:chrono@0\.4\.45\#45 (repeated/cyclic)
  - Cargo\.lock:package:knx\-csv@0\.1\.0\-alpha\.1\#196
    - Cargo\.lock:package:csv@1\.4\.0\#59
      - Cargo\.lock:package:csv\-core@0\.1\.13\#60
      - Cargo\.lock:package:itoa@1\.0\.18\#176
      - Cargo\.lock:package:ryu@1\.0\.23\#322

Display is truncated where limits apply; full measured graph remains in JSON.

### Availability notes

- Resolved records are a measured subset; lock inventory incomplete\.
- apps/knx\-web/package\-lock\.json: lock coverage unavailable; missing/stale records or unsupported static requirements\.
- apps/knx\-web/package\-lock\.json: missing npm dependency edge; depth unavailable\.


## Changes Since the Previous Run

Baseline recorded; no previous compatible snapshot exists. Changes are N/A, not zero.

These are inventory differences between successful local runs, not interval billing, code authorship or an agent-efficiency score. Removed/moved session logs can reduce counts. The generated report and private state are excluded from file-size measurements. Missing measurements remain N/A; snapshots stay in Git-ignored private state.
