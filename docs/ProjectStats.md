# Project Statistics

**Last update:** 2026-10-07 00:06:38 CEST (UTC+02:00)

**Source revision:** `62d25a2452a3695038c31dd5b4f085a0d956a1c6` — freshly fetched `origin/main`; collection precedes the report's publication commit.

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **50.642.178 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **42.201 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **345.3 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **17.186 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **101.284 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **5.1 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **20811.9 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **2.025.687 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **222.3 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **364.4 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **327.364.688 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.75x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **506.4 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **202.568 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **20.256 kilometers** (about **0.51x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **8.10 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **75.46 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **289.383.878 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **1.012.843.576 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **53.662 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **2.025 kWh** and **668.5 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 2.202
- **Merges:** 250
- **Pushes/Sync:** 2.202
- **Lines Added (+):** 643.073
- **Lines Deleted (-):** 165.094

## Git Text Churn Distribution

| P50 changed text lines | P90 | P95 |
| ---: | ---: | ---: |
| 112 | 781 | 1.270 |

**1.920** measured commits;
**282** unavailable (for example, merge-only
history or binary/unsupported `--numstat` entries). A zero means Git explicitly
reported zero text-line changes; missing data is not turned into zero. This
counts historical additions + deletions per commit, **not AI-attributed** work
or surviving source lines. Branch lifetime cannot be inferred from this table.

## Coding Statistics (Current Working Tree)

**Git-tracked source files:** 750; **physical lines:** 307.580;
**code:** 247.527; **comments:** 38.912; **blank:** 21.141.
**Code/comment ratio:** 6.36:1; **comment share of nonblank lines:** 13.6%.
Tracked source paths unavailable in the working tree: **0**;
excluded generated/vendor/build source paths: **21**.

| Language | Files | Code lines | Comment lines | Blank lines |
| :--- | ---: | ---: | ---: | ---: |
| Rust | 403 | 185.538 | 30.820 | 14.666 |
| TSX | 129 | 31.347 | 3.636 | 3.206 |
| TypeScript | 183 | 23.123 | 3.980 | 2.222 |
| Python | 26 | 2.968 | 32 | 501 |
| CSS | 2 | 2.861 | 377 | 417 |
| JavaScript | 5 | 1.528 | 48 | 104 |
| Shell | 2 | 162 | 19 | 25 |

### Direct-source syntax structures

| Language | Parsed files | Functions/methods | Loops | Type declarations | Branch constructs |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Rust | 403/403 | 7.775 | 1.745 | 1.049 | 4.098 |
| TSX | 129/129 | 4.945 | 72 | 50 | 1.180 |
| TypeScript | 183/183 | 2.876 | 353 | 296 | 1.050 |
| Python | 26/26 | 189 | 111 | 22 | 230 |
| CSS | N/A (no parser) | N/A | N/A | N/A | N/A |
| JavaScript | 5/5 | 308 | 21 | 0 | 137 |
| Shell | N/A (no parser) | N/A | N/A | N/A | N/A |
| **Supported-source subtotal** | **746/746** | **16.093** | **2.302** | **1.417** | **6.695** |

Structure coverage is **746/746 supported source files**;
**4** source files have no structural parser (for example CSS/Shell).
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
| **Claude** | 153h 33m 53s | 176h 26m 28s | 3.830 |
| **Codex** | N/A | 113h 16m 55s | 679 |
| **Other** | N/A | N/A | 0 |
| **TOTAL** | **153h 33m 53s** | **289h 43m 24s** | **4.509** |

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
**46** unavailable (open sessions,
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
| **Claude** | 3.924.634 | 61.996.555 | 14.658.971.322 | 644.510.168 |
| **Codex** | 124.424.865 | 19.844.617 | 4.743.199.360 | 0 |
| **Other** | 0 | 0 | 0 | 0 |
| **TOTAL** | **128.349.499** | **81.841.172** | **19.402.170.682** | **644.510.168** |

### 🚀 GRAND TOTAL CONSUMPTION
**20.256.871.521 Total Tokens**

## Weekly Usage Trend

| Week starting (UTC Monday) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| 2026-10-05 | 158.789.361 | 668.310 | 8 |
| 2026-09-28 | 4.728.183.132 | 20.393.315 | 32 |
| 2026-09-21 | 2.699.241.852 | 10.225.718 | 384 |
| 2026-09-14 | 4.119.946.629 | 17.575.916 | 1.222 |
| 2026-09-07 | 6.555.024.646 | 28.737.531 | 2.106 |
| 2026-08-31 | 1.995.685.901 | 4.240.382 | 757 |

Dated: **20.256.871.521 tokens / 4.509 tasks**;
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
| Hermes Agent via Headroom | 5.463.710.677 | 86.980.150 | 24.152.944 | 5.211.787.865 | 0s | 91 | 6.666 |
| Hermes Agent (direct) | 1.256.618.371 | 2.907.461 | 3.552.654 | 1.214.397.514 | 0s | 14 | 1.335 |
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
| Claude Opus 5.5 (`claude-opus-5-5`) | 2.746.239.808 | 1.885.355 | 10.900.857 | 2.574.719.825 | 0s | 36 | 3.736 |
| GPT-6.1 Sol (`gpt-6.1-sol`) | 1.591.280.454 | 51.004.167 | 9.990.079 | 1.530.286.208 | 0s | 9 | 764 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 1.460.223.501 | 34.710.821 | 4.243.304 | 1.421.269.376 | 80h 10m 0s | 227 | 891 |
| gpt-6-sol-900k | 1.217.683.037 | 13.755.988 | 2.635.145 | 1.201.291.904 | 0s | 3 | 624 |
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
| 2026-08-31 | Claude Sonnet 5 (`claude-sonnet-5`) | 1.890.995.725 | 1.861.301.396 | 98.6% |
| 2026-09-21 | Claude Opus 5 (`claude-opus-5`) | 303.582.253 | 276.558.994 | 91.4% |
| 2026-09-14 | Claude Opus 5 (`claude-opus-5`) | 1.739.086.785 | 1.656.048.885 | 95.7% |
| 2026-09-07 | Claude Opus 5 (`claude-opus-5`) | 1.692.215.588 | 1.605.147.744 | 95.4% |
| 2026-08-31 | Claude Opus 5 (`claude-opus-5`) | 32.299.592 | 30.888.925 | 95.7% |
| 2026-10-05 | Claude Opus 5.5 (`claude-opus-5-5`) | 158.759.036 | 153.068.401 | 96.8% |
| 2026-09-28 | Claude Opus 5.5 (`claude-opus-5-5`) | 2.036.036.245 | 1.948.787.590 | 96.0% |
| 2026-09-21 | Claude Opus 5.5 (`claude-opus-5-5`) | 551.444.527 | 472.863.834 | 86.3% |
| 2026-10-05 | GPT-6.1 Sol (`gpt-6.1-sol`) | 30.325 | 3.968 | 13.3% |
| 2026-09-28 | GPT-6.1 Sol (`gpt-6.1-sol`) | 1.591.250.129 | 1.530.282.240 | 96.8% |
| 2026-09-21 | GPT-5.6 Sol (`gpt-5.6-sol`) | 1.233.389.499 | 1.202.323.712 | 97.8% |
| 2026-09-14 | GPT-5.6 Sol (`gpt-5.6-sol`) | 226.834.002 | 218.945.664 | 96.9% |

Top five models by selected-project token volume, across the last eight
observed UTC weeks. **Unallocated** means that a canonical model ledger has
no reliable per-day timestamp (especially cumulative cloud usage); it is not
placed on the last result's day. Model-time mapping covers
**20.256.871.521 tokens**; **0 tokens**
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
| high | 18.385.971.991 | 97.584.700 | 73.700.276 | 17.621.866.819 | 189h 45m 52s | 2.571 | 68.859 |
| medium | 1.032.641.238 | 29.712.445 | 3.775.116 | 990.068.185 | 73h 24m 19s | 406 | 4.096 |
| xhigh | 665.368.414 | 97.702 | 3.950.590 | 631.309.290 | 22h 4m 42s | 271 | 3.464 |
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
| 2 | read_file | 31.613 |
| 3 | terminal | 18.792 |
| 4 | execute_code | 12.314 |
| 5 | skill_view | 9.593 |
| 6 | Read | 9.203 |
| 7 | patch | 8.721 |
| 8 | search_files | 6.428 |
| 9 | Edit | 5.290 |
| 10 | tool_call | 3.353 |
| 11 | write_file | 1.679 |
| 12 | Write | 941 |
| 13 | Agent | 880 |
| 14 | ToolSearch | 554 |
| 15 | tool_describe | 438 |
| 16 | Monitor | 369 |
| 17 | wait_agent | 368 |
| 18 | vision_analyze | 208 |
| 19 | SendMessage | 187 |
| 20 | TaskStop | 164 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | autonomous-goal-boundaries | 898 |
| 2 | test-driven-development | 888 |
| 3 | cross-layer-contract-verification | 831 |
| 4 | compressed-tool-output-recovery | 813 |
| 5 | frontend-design | 706 |
| 6 | playwright | 661 |
| 7 | repository-delivery | 618 |
| 8 | systematic-debugging | 521 |
| 9 | safety-confirmation-gates | 466 |
| 10 | private-corpus-regression-testing | 431 |
| 11 | requesting-code-review | 400 |
| 12 | data-integrity-feature-verification | 367 |
| 13 | project-completion-auditing | 344 |
| 14 | hermes-agent | 280 |
| 15 | codex | 244 |

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
| Tokens per committed added line | 31,500.1 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 3.90:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 101.3 | AI output ÷ added and deleted Git lines |
| Git commit density | 9,199,305.9 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $7,756.33 | 95.8% | $39,579.22 |
| Codex | $1,794.53 | 97.4% | $10,672.20 |
| Other | N/A | N/A | N/A |
| **PRICED SUBTOTAL** | **$9,550.86** | **96.2%** | **$50,251.42** |

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

- **Prime hour:** 16:00–17:00 local time (336 logged activity events)
- **Prime weekday:** Sunday (1.283 logged activity events)
- **Generation throughput:** N/A — output tokens and measured generation seconds are not joined per event across all sources
- **Autonomy input/output ratio:** 1.57:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 17.39 logged tool calls per task (78.410 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 518 | 11.5% |
| Morning (06-11) | 1.353 | 30.0% |
| Afternoon (12-17) | 1.538 | 34.1% |
| Evening (18-23) | 1.100 | 24.4% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 536 | 11.9% |
| Tuesday | 396 | 8.8% |
| Wednesday | 215 | 4.8% |
| Thursday | 627 | 13.9% |
| Friday | 373 | 8.3% |
| Saturday | 1.079 | 23.9% |
| Sunday | 1.283 | 28.5% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 2,025.7 kWh
- **Estimated CO₂ equivalent:** 668.5 kg CO₂e
- **Estimated physical keystrokes avoided:** 327.364.688

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
- Hermes profile without recorded working directory: <code>knxbench</code> (21 sessions)

Listed Hermes profiles are an explicit project choice: their sessions that recorded neither a working directory nor a Git root are attributed here. Sessions with a recorded location are still matched by path.

Cloud sessions are not queried when the saved cloud setting is disabled.

## Project Size & File Inventory

Scope: **partial — measured subset only**, no tracked index/stat changes detected; HEAD revision: <code>62d25a2452a3695038c31dd5b4f085a0d956a1c6</code>.

Git index/stat metadata only; conservative, without content comparisons. Secret/cache/report exclusions and submodule working contents are not assessed.

Logical bytes are file lengths, not disk allocation. Source suffixes are counted independently of categories; tests/docs take category precedence. Source lines are streaming physical UTF-8 text lines, including comments and blanks, not parser/tokei code counts.

Directories are unique parents of measured tracked files, including the root at depth zero. File depth means its parent directory depth. Name/path lengths count Unicode characters (not graphemes) separately from filesystem-encoded bytes, excluding the absolute root prefix.

| Metric | Value |
| --- | --- |
| Files (measured) | 1,196 |
| Logical bytes (measured) | 37,478,102 |
| Source files | 744 |
| Source logical bytes | 12,438,562 |
| Source physical lines | 306,870 |
| Tracked directories (including root) | 128 |
| Maximum directory depth | 5 |

| Category | Files | Logical bytes |
| --- | --- | --- |
| source | 449 | 8,615,388 |
| tests | 335 | 4,625,586 |
| docs | 324 | 21,237,149 |
| config | 48 | 572,251 |
| assets | 6 | 317,816 |
| other | 34 | 2,109,912 |

Readable UTF-8 source files: **744**; median file lines: **192.5**; p95 file lines: **1,453** (nearest-rank). Unreadable source files are excluded from distributions; their aggregate source lines are N/A.

### Extension distribution (top 10 by bytes)

| Extension | Files | Logical bytes |
| --- | --- | --- |
| <code>.png</code> | 46 | 11,985,652 |
| <code>.rs</code> | 398 | 8,926,341 |
| <code>.md</code> | 300 | 7,309,940 |
| <code>.gif</code> | 1 | 2,752,893 |
| <code>.tsx</code> | 129 | 1,693,302 |
| <code>.ts</code> | 182 | 1,419,290 |
| <code>.html</code> | 22 | 1,132,692 |
| <code>.sqlite</code> | 7 | 860,160 |
| <code>.json</code> | 39 | 752,141 |
| <code>.py</code> | 26 | 167,365 |

### File and path extremes

| Extreme | Path/name | Measurement |
| --- | --- | --- |
| Longest filename | <code>docs/adr/0053-contributions-come-with-a-license-grant-for-dual-licensing.md</code> | 66 Unicode characters; 66 filesystem-encoded bytes |
| Longest relative path | <code>docs/superpowers/specs/2026-09-09-standalone-product-database-install-design.md</code> | 79 Unicode characters; 79 filesystem-encoded bytes |
| Deepest file | <code>apps/knx-desktop/.claude/skills/run-knx-desktop/SKILL.md</code> | Parent depth 5 |
| Shortest nonempty readable source | <code>apps/knx-web/src/vite-env.d.ts</code> | 1 physical lines; 38 bytes |
| Longest readable source line | <code>crates/knx-productdb/tests/scheme12&#95;14.rs</code> | Line 471; 838 Unicode characters (without newline) |
| Most common basename (case-sensitive) | <code>Cargo.toml</code> | 18 files |

### Largest files by logical bytes (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>docs/assets/readme/telegram-flow.gif</code> | 2,752,893 | N/A |
| <code>docs/design/2026-09-13-codex-ui-concept/03-busmonitor.png</code> | 1,322,979 | N/A |
| <code>docs/design/2026-09-13-codex-ui-concept/02-graphite.png</code> | 1,250,734 | N/A |
| <code>docs/design/2026-09-13-codex-ui-concept/01-porcelain.png</code> | 1,067,455 | N/A |
| <code>docs/history/IMPLEMENTATION&#95;STATUS&#95;2026-09.md</code> | 828,249 | N/A |
| <code>docs/KNOWN&#95;LIMITATIONS.md</code> | 518,022 | N/A |
| <code>docs/assets/screenshots/porcelain-log.png</code> | 465,414 | N/A |
| <code>docs/assets/screenshots/porcelain-settings.png</code> | 446,214 | N/A |
| <code>docs/assets/screenshots/porcelain-help-panel.png</code> | 346,214 | N/A |
| <code>docs/assets/screenshots/porcelain-group-addresses.png</code> | 346,166 | N/A |

### Largest source files by physical lines (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>crates/knx-core/src/command.rs</code> | 340,261 | 8,964 |
| <code>apps/knx-server/src/domain.rs</code> | 287,216 | 6,991 |
| <code>crates/knx-net/src/commissioning.rs</code> | 266,441 | 6,221 |
| <code>crates/knx-core/src/dpt/codec.rs</code> | 235,902 | 5,916 |
| <code>apps/knx-cli/src/main.rs</code> | 181,981 | 4,811 |
| <code>crates/knx-productdb/tests/dynamic&#95;tree.rs</code> | 176,745 | 4,443 |
| <code>crates/knx-net/src/cemi.rs</code> | 177,343 | 4,132 |
| <code>crates/knx-productdb/src/migration.rs</code> | 181,475 | 4,128 |
| <code>crates/knx-productdb/src/query.rs</code> | 164,613 | 3,958 |
| <code>apps/knx-web/src/App.test.tsx</code> | 153,049 | 3,490 |

### Empty files (up to 10)

Total empty files: **1**.

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>docs/Issues.md</code> | 0 | N/A |

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
- Excluded ignored: 1.
- Excluded report or state: 1.
- Excluded secret or credential: 6.
- Excluded symlink: 1.
- Unavailable tracked working files: 1; unreadable source files: 0.
- Excluded tracked files&#58; generated or cache=21, ignored=1, report or state=1, secret or credential=6, symlink=1.
- Partial inventory&#58; unavailable working files are omitted; unreadable source lines are N/A.


## Package Dependencies

Offline, Git-tracked declarations and lockfiles only; no package code is executed.

| Metric | Value |
|---|---:|
| Unique declared dependencies | 59 |
| Unique locked package versions | N/A |
| Packages with multiple locked versions | N/A |
| Internal/local package names | 18 |
| Maximum resolved depth (edges) | N/A |
| Unresolved lock edges | 26 |
| Cyclic components | 0 |

Declaration coverage: complete.
Lock inventory coverage: **incomplete / unavailable**; resolved records are a measured subset.

### Most frequently declared packages (up to 10)

| Ecosystem | Package | Declarations |
|---|---|---:|
| cargo | knx\-core | 12 |
| cargo | tempfile | 10 |
| cargo | chrono | 7 |
| cargo | serde\_json | 7 |
| cargo | knx\-testsupport | 6 |
| cargo | serde | 5 |
| cargo | sha2 | 5 |
| cargo | zip | 5 |
| cargo | base64 | 4 |
| cargo | knx\-etsproj | 4 |
Other 49 package names omitted; full records remain in JSON.

### Multiple locked versions (up to 5; observed subset if coverage is incomplete)

- cargo / windows\-sys: 0\.45\.0, 0\.59\.0, 0\.60\.2; other versions omitted
- cargo / getrandom: 0\.2\.17, 0\.3\.4, 0\.4\.3
- cargo / hashbrown: 0\.12\.3, 0\.16\.1, 0\.17\.1
- cargo / proc\-macro\-crate: 1\.3\.1, 2\.0\.2, 3\.5\.0
- cargo / schemars: 0\.8\.22, 0\.9\.0, 1\.2\.2

### Compact resolved tree

At most 3 roots, 10 direct branches, display depth 3 and 30 tree lines; repeated nodes are not expanded.

- Cargo\.lock:package:knx\-cli@0\.1\.0\-alpha\.4\#183
  - Cargo\.lock:package:chrono@0\.4\.45\#40
    - Cargo\.lock:package:iana\-time\-zone@0\.1\.65\#148
      - Cargo\.lock:package:android\_system\_properties@0\.1\.6\#4
      - Cargo\.lock:package:core\-foundation\-sys@0\.8\.7\#44
      - Cargo\.lock:package:iana\-time\-zone\-haiku@0\.1\.2\#149
    - Cargo\.lock:package:num\-traits@0\.2\.19\#224
      - Cargo\.lock:package:autocfg@1\.5\.1\#10
    - Cargo\.lock:package:serde@1\.0\.229\#303
      - Cargo\.lock:package:serde\_core@1\.0\.229\#305
      - Cargo\.lock:package:serde\_derive@1\.0\.229\#306
  - Cargo\.lock:package:knx\-app@0\.1\.0\-alpha\.3\#181
    - Cargo\.lock:package:base64@0\.22\.1\#14
    - Cargo\.lock:package:chrono@0\.4\.45\#40 (repeated/cyclic)
    - Cargo\.lock:package:getrandom@0\.4\.3\#120
      - Cargo\.lock:package:cfg\-if@1\.0\.4\#39
      - Cargo\.lock:package:libc@0\.2\.189\#200
      - Cargo\.lock:package:r\-efi@6\.0\.0\#277
  - Cargo\.lock:package:knx\-build\-stamp@0\.1\.0\-alpha\.1\#182
    - Cargo\.lock:package:tempfile@3\.27\.0\#358
      - Cargo\.lock:package:fastrand@2\.5\.0\#91
      - Cargo\.lock:package:getrandom@0\.4\.3\#120 (repeated/cyclic)
      - Cargo\.lock:package:once\_cell@1\.21\.4\#244
  - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#184
    - Cargo\.lock:package:chrono@0\.4\.45\#40 (repeated/cyclic)
  - Cargo\.lock:package:knx\-csv@0\.1\.0\-alpha\.1\#185
    - Cargo\.lock:package:csv@1\.4\.0\#54
      - Cargo\.lock:package:csv\-core@0\.1\.13\#55
      - Cargo\.lock:package:itoa@1\.0\.18\#165
      - Cargo\.lock:package:ryu@1\.0\.23\#294

Display is truncated where limits apply; full measured graph remains in JSON.

### Availability notes

- Resolved records are a measured subset; lock inventory incomplete\.
- apps/knx\-web/package\-lock\.json: lock coverage unavailable; missing/stale records or unsupported static requirements\.
- apps/knx\-web/package\-lock\.json: missing npm dependency edge; depth unavailable\.


## Changes Since the Previous Run

Baseline recorded; no previous compatible snapshot exists. Changes are N/A, not zero.

These are inventory differences between successful local runs, not interval billing, code authorship or an agent-efficiency score. Removed/moved session logs can reduce counts. The generated report and private state are excluded from file-size measurements. Missing measurements remain N/A; snapshots stay in Git-ignored private state.
