# Project Statistics

**Last update:** 2026-10-03 13:25:17 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **42.531.077 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **35.442 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **290.0 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **14.433 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **85.062 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **4.3 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **17478.5 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.701.243 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **186.7 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **306.0 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **270.668.996 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **5.67x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **425.3 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **170.124 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **17.012 kilometers** (about **0.42x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **6.80 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **63.38 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **243.034.727 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **850.621.546 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **45.067 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.701 kWh** and **561.4 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 1.797
- **Merges:** 181
- **Pushes/Sync:** 1.874
- **Lines Added (+):** 489.934
- **Lines Deleted (-):** 99.716

## Git Text Churn Distribution

| P50 changed text lines | P90 | P95 |
| ---: | ---: | ---: |
| 112 | 708 | 1.185 |

**1.594** measured commits;
**203** unavailable (for example, merge-only
history or binary/unsupported `--numstat` entries). A zero means Git explicitly
reported zero text-line changes; missing data is not turned into zero. This
counts historical additions + deletions per commit, **not AI-attributed** work
or surviving source lines. Branch lifetime cannot be inferred from this table.

## Coding Statistics (Current Working Tree)

**Git-tracked source files:** 539; **physical lines:** 251.911;
**code:** 198.102; **comments:** 35.850; **blank:** 17.959.
**Code/comment ratio:** 5.53:1; **comment share of nonblank lines:** 15.3%.
Tracked source paths unavailable in the working tree: **0**;
excluded generated/vendor/build source paths: **17**.

| Language | Files | Code lines | Comment lines | Blank lines |
| :--- | ---: | ---: | ---: | ---: |
| Rust | 329 | 155.481 | 28.804 | 13.024 |
| TSX | 101 | 25.431 | 3.325 | 2.808 |
| TypeScript | 98 | 12.724 | 3.279 | 1.448 |
| CSS | 1 | 2.330 | 356 | 368 |
| Python | 5 | 1.696 | 1 | 269 |
| Shell | 4 | 301 | 74 | 42 |
| JavaScript | 1 | 139 | 11 | 0 |

### Direct-source syntax structures

| Language | Parsed files | Functions/methods | Loops | Type declarations | Branch constructs |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Rust | 329/329 | 6.654 | 1.371 | 947 | 3.485 |
| TSX | 101/101 | 3.832 | 49 | 44 | 883 |
| TypeScript | 98/98 | 1.399 | 113 | 189 | 370 |
| CSS | N/A (no parser) | N/A | N/A | N/A | N/A |
| Python | 5/5 | 93 | 30 | 15 | 104 |
| Shell | N/A (no parser) | N/A | N/A | N/A | N/A |
| JavaScript | 1/1 | 23 | 3 | 0 | 12 |
| **Supported-source subtotal** | **534/534** | **12.001** | **1.566** | **1.195** | **4.854** |

Structure coverage is **534/534 supported source files**;
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
| **Claude** | 150h 0m 46s | 167h 7m 48s | 3.586 |
| **Codex** | N/A | 83h 55m 7s | 380 |
| **Other** | N/A | N/A | 0 |
| **TOTAL** | **150h 0m 46s** | **251h 2m 56s** | **3.966** |

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
**21** unavailable (open sessions,
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
| **Claude** | 549.455 | 53.835.546 | 12.430.867.263 | 571.581.662 |
| **Codex** | 94.464.308 | 13.831.703 | 3.847.300.992 | 0 |
| **Other** | 0 | 0 | 0 | 0 |
| **TOTAL** | **95.013.763** | **67.667.249** | **16.278.168.255** | **571.581.662** |

### 🚀 GRAND TOTAL CONSUMPTION
**17.012.430.929 Total Tokens**

## Weekly Usage Trend

| Week starting (UTC Monday) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| 2026-09-28 | 2.119.486.999 | 9.436.629 | 17 |
| 2026-09-21 | 2.548.798.802 | 9.303.234 | 257 |
| 2026-09-14 | 4.119.946.629 | 17.575.916 | 1.222 |
| 2026-09-07 | 6.361.524.728 | 27.658.556 | 1.816 |
| 2026-08-31 | 1.862.673.771 | 3.692.914 | 654 |

Dated: **17.012.430.929 tokens / 3.966 tasks**;
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
| Claude Code | 12.235.336.962 | 283.228 | 48.714.117 | 11.728.710.564 | 167h 7m 48s | 3.566 | 68.032 |
| Hermes Agent via Headroom | 3.891.994.513 | 64.872.007 | 15.874.766 | 3.697.295.131 | 0s | 74 | 4.877 |
| Codex CLI | 825.843.693 | 28.042.654 | 2.880.975 | 794.920.064 | 83h 55m 7s | 320 | 711 |
| Hermes Agent (direct) | 59.255.761 | 1.815.874 | 197.391 | 57.242.496 | 0s | 6 | 301 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Sonnet 5 (`claude-sonnet-5`) | 8.509.322.803 | 159.365 | 28.198.954 | 8.193.987.534 | 68h 12m 23s | 1.201 | 44.164 |
| Claude Opus 5 (`claude-opus-5`) | 3.656.626.992 | 98.248 | 20.145.339 | 3.463.501.101 | 95h 26m 57s | 1.243 | 22.001 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 1.312.190.109 | 27.829.221 | 3.336.376 | 1.281.024.512 | 64h 43m 32s | 166 | 859 |
| gpt-6-sol-900k | 1.217.683.037 | 13.755.988 | 2.635.145 | 1.201.291.904 | 0s | 3 | 624 |
| GPT-6.1 Sol (`gpt-6.1-sol`) | 885.573.667 | 29.641.911 | 4.526.444 | 851.405.312 | 0s | 6 | 573 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 684.538.867 | 239.827 | 4.750.311 | 583.412.809 | 0s | 16 | 1.141 |
| GPT-6 Sol (`gpt-6-sol`) | 188.857.362 | 2.004.010 | 298.216 | 186.555.136 | 0s | 2 | 102 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 179.927.531 | 13.624.777 | 2.149.922 | 164.152.832 | 2h 43m 52s | 46 | 2.293 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 145.240.773 | 35.228 | 308.027 | 133.323.915 | 2h 24m 43s | 10 | 1.650 |
| GPT-6 Astra (`gpt-6-astra`) | 103.285.407 | 3.903.596 | 464.947 | 98.916.864 | 8h 24m 5s | 14 | 167 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 68.079.890 | 3.704.805 | 420.653 | 63.954.432 | 8h 0m 58s | 32 | 33 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| <synthetic> | 0 | 0 | 0 | 0 | 31m 55s | 111 | 0 |
| Claude Sonnet 4.6 (`claude-sonnet-4-6`) | 0 | 0 | 0 | 0 | 0s | 1 | 0 |
| unknown | 0 | 0 | 0 | 0 | 2m 38s | 1.109 | 0 |

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
| 2026-09-07 | Claude Sonnet 5 (`claude-sonnet-5`) | 4.573.859.940 | 4.362.502.639 | 95.7% |
| 2026-08-31 | Claude Sonnet 5 (`claude-sonnet-5`) | 1.776.604.278 | 1.751.089.060 | 98.8% |
| 2026-09-21 | Claude Opus 5 (`claude-opus-5`) | 303.582.253 | 276.558.994 | 91.4% |
| 2026-09-14 | Claude Opus 5 (`claude-opus-5`) | 1.739.086.785 | 1.656.048.885 | 95.7% |
| 2026-09-07 | Claude Opus 5 (`claude-opus-5`) | 1.591.791.799 | 1.509.577.245 | 95.4% |
| 2026-08-31 | Claude Opus 5 (`claude-opus-5`) | 22.166.155 | 21.315.977 | 96.3% |
| 2026-09-21 | GPT-5.6 Sol (`gpt-5.6-sol`) | 1.085.356.107 | 1.062.078.848 | 98.1% |
| 2026-09-14 | GPT-5.6 Sol (`gpt-5.6-sol`) | 226.834.002 | 218.945.664 | 96.9% |
| 2026-09-28 | `gpt-6-sol-900k` | 1.090.808.664 | 1.076.377.472 | 98.9% |
| 2026-09-21 | `gpt-6-sol-900k` | 126.874.373 | 124.914.432 | 98.7% |
| 2026-09-28 | GPT-6.1 Sol (`gpt-6.1-sol`) | 885.573.667 | 851.405.312 | 96.6% |

Top five models by selected-project token volume, across the last eight
observed UTC weeks. **Unallocated** means that a canonical model ledger has
no reliable per-day timestamp (especially cumulative cloud usage); it is not
placed on the last result's day. Model-time mapping covers
**17.012.430.929 tokens**; **0 tokens**
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
| high | 15.489.743.954 | 68.775.238 | 60.981.537 | 14.832.233.045 | 169h 22m 21s | 2.363 | 65.703 |
| medium | 810.527.254 | 25.441.838 | 2.739.963 | 776.237.510 | 58h 4m 25s | 231 | 3.407 |
| xhigh | 554.768.659 | 95.741 | 3.593.663 | 524.933.273 | 20h 48m 36s | 244 | 2.964 |
| unknown | 156.255.055 | 625.719 | 339.498 | 143.716.235 | 2h 27m 21s | 1.128 | 1.845 |
| max | 1.136.007 | 75.227 | 12.588 | 1.048.192 | 19m 17s | 0 | 2 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | Bash | 50.518 |
| 2 | read_file | 27.649 |
| 3 | terminal | 12.472 |
| 4 | Read | 9.012 |
| 5 | skill_view | 8.675 |
| 6 | execute_code | 8.212 |
| 7 | patch | 7.971 |
| 8 | search_files | 5.614 |
| 9 | Edit | 5.145 |
| 10 | tool_call | 2.549 |
| 11 | write_file | 950 |
| 12 | Write | 912 |
| 13 | Agent | 850 |
| 14 | ToolSearch | 534 |
| 15 | Monitor | 367 |
| 16 | wait_agent | 363 |
| 17 | tool_describe | 348 |
| 18 | SendMessage | 182 |
| 19 | TaskStop | 162 |
| 20 | delegate_task | 141 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | test-driven-development | 818 |
| 2 | autonomous-goal-boundaries | 812 |
| 3 | cross-layer-contract-verification | 776 |
| 4 | compressed-tool-output-recovery | 739 |
| 5 | frontend-design | 680 |
| 6 | playwright | 626 |
| 7 | repository-delivery | 482 |
| 8 | systematic-debugging | 465 |
| 9 | safety-confirmation-gates | 436 |
| 10 | private-corpus-regression-testing | 372 |
| 11 | requesting-code-review | 346 |
| 12 | data-integrity-feature-verification | 339 |
| 13 | project-completion-auditing | 295 |
| 14 | hermes-agent | 261 |
| 15 | codex | 244 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

### Claude Code Local Tool Results

| Tool | Explicit success | Explicit error | Status unavailable | Error share of known |
| :--- | ---: | ---: | ---: | ---: |
| Bash | 49.257 | 1.261 | 0 | 2.5% |
| Read | 0 | 127 | 8.885 | 100.0% |
| Edit | 0 | 73 | 5.072 | 100.0% |
| Write | 0 | 3 | 909 | 100.0% |
| Agent | 0 | 6 | 844 | 100.0% |
| ToolSearch | 0 | 0 | 534 | N/A |
| Monitor | 0 | 1 | 366 | 100.0% |
| SendMessage | 0 | 3 | 179 | 100.0% |
| TaskStop | 0 | 4 | 158 | 100.0% |
| Skill | 0 | 1 | 118 | 100.0% |
| ListAgents | 0 | 0 | 95 | N/A |
| mcp__headroom__headroom_retrieve | 0 | 0 | 29 | N/A |
| TaskUpdate | 0 | 1 | 26 | 100.0% |
| TaskCreate | 0 | 0 | 25 | N/A |
| TaskOutput | 0 | 0 | 13 | N/A |
| **Other tools (9)** | 0 | 1 | 41 | 100.0% |

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
| Tokens per committed added line | 34,723.9 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.91:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 114.8 | AI output ÷ added and deleted Git lines |
| Git commit density | 9,467,129.1 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $6,681.87 | 95.6% | $33,563.34 |
| Codex | $1,405.46 | 97.6% | $8,656.43 |
| Other | N/A | N/A | N/A |
| **PRICED SUBTOTAL** | **$8,087.33** | **96.1%** | **$42,219.77** |

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

- **Prime hour:** 16:00–17:00 local time (298 logged activity events)
- **Prime weekday:** Sunday (1.275 logged activity events)
- **Generation throughput:** N/A — output tokens and measured generation seconds are not joined per event across all sources
- **Autonomy input/output ratio:** 1.40:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 18.64 logged tool calls per task (73.921 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 495 | 12.5% |
| Morning (06-11) | 1.145 | 28.9% |
| Afternoon (12-17) | 1.353 | 34.1% |
| Evening (18-23) | 973 | 24.5% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 520 | 13.1% |
| Tuesday | 300 | 7.6% |
| Wednesday | 115 | 2.9% |
| Thursday | 469 | 11.8% |
| Friday | 315 | 7.9% |
| Saturday | 972 | 24.5% |
| Sunday | 1.275 | 32.1% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 1,701.2 kWh
- **Estimated CO₂ equivalent:** 561.4 kg CO₂e
- **Estimated physical keystrokes avoided:** 270.668.996

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.


## Project Size & File Inventory

Scope: **partial — measured subset only**, dirty current working checkout; HEAD revision: <code>307a5970ad5147437dffce4a86047e85e3319186</code>.

Git index/stat metadata only; conservative, without content comparisons. Secret/cache/report exclusions and submodule working contents are not assessed.

Logical bytes are file lengths, not disk allocation. Source suffixes are counted independently of categories; tests/docs take category precedence. Source lines are streaming physical UTF-8 text lines, including comments and blanks, not parser/tokei code counts.

Directories are unique parents of measured tracked files, including the root at depth zero. File depth means its parent directory depth. Name/path lengths count Unicode characters (not graphemes) separately from filesystem-encoded bytes, excluding the absolute root prefix.

| Metric | Value |
| --- | --- |
| Files (measured) | 856 |
| Logical bytes (measured) | 25,751,989 |
| Source files | 538 |
| Source logical bytes | 10,002,534 |
| Source physical lines | 251,760 |
| Tracked directories (including root) | 93 |
| Maximum directory depth | 5 |

| Category | Files | Logical bytes |
| --- | --- | --- |
| source | 342 | 7,368,555 |
| tests | 238 | 3,460,703 |
| docs | 221 | 13,839,252 |
| config | 32 | 84,713 |
| assets | 4 | 2,119 |
| other | 19 | 996,647 |

Readable UTF-8 source files: **538**; median file lines: **220.0**; p95 file lines: **1,737** (nearest-rank). Unreadable source files are excluded from distributions; their aggregate source lines are N/A.

### Extension distribution (top 10 by bytes)

| Extension | Files | Logical bytes |
| --- | --- | --- |
| <code>.png</code> | 35 | 8,426,773 |
| <code>.rs</code> | 328 | 7,648,772 |
| <code>.md</code> | 228 | 6,164,251 |
| <code>.tsx</code> | 101 | 1,346,253 |
| <code>.sqlite</code> | 7 | 860,160 |
| <code>.ts</code> | 98 | 804,582 |
| <code>.lock</code> | 1 | 127,791 |
| <code>.css</code> | 1 | 105,695 |
| <code>.py</code> | 5 | 69,480 |
| <code>.json</code> | 8 | 63,532 |

### File and path extremes

| Extreme | Path/name | Measurement |
| --- | --- | --- |
| Longest filename | <code>docs/adr/0053-contributions-come-with-a-license-grant-for-dual-licensing.md</code> | 66 Unicode characters; 66 filesystem-encoded bytes |
| Longest relative path | <code>docs/superpowers/specs/2026-09-09-standalone-product-database-install-design.md</code> | 79 Unicode characters; 79 filesystem-encoded bytes |
| Deepest file | <code>apps/knx-desktop/.claude/skills/run-knx-desktop/SKILL.md</code> | Parent depth 5 |
| Shortest nonempty readable source | <code>apps/knx-web/src/vite-env.d.ts</code> | 1 physical lines; 38 bytes |
| Longest readable source line | <code>crates/knx-productdb/tests/scheme12&#95;14.rs</code> | Line 471; 838 Unicode characters (without newline) |
| Most common basename (case-sensitive) | <code>Cargo.toml</code> | 17 files |

### Largest files by logical bytes (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>docs/design/2026-09-13-codex-ui-concept/03-busmonitor.png</code> | 1,322,979 | N/A |
| <code>docs/design/2026-09-13-codex-ui-concept/02-graphite.png</code> | 1,250,734 | N/A |
| <code>docs/design/2026-09-13-codex-ui-concept/01-porcelain.png</code> | 1,067,455 | N/A |
| <code>docs/IMPLEMENTATION&#95;STATUS.md</code> | 864,072 | N/A |
| <code>docs/RESEARCH.md</code> | 451,340 | N/A |
| <code>docs/KNOWN&#95;LIMITATIONS.md</code> | 422,811 | N/A |
| <code>docs/assets/screenshots/porcelain-group-addresses.png</code> | 324,849 | N/A |
| <code>crates/knx-core/src/command.rs</code> | 308,008 | 8,146 |
| <code>crates/knx-net/src/commissioning.rs</code> | 265,528 | 6,199 |
| <code>docs/design/2026-09-13-codex-ui-proof/04-large-list-1280.png</code> | 258,286 | N/A |

### Largest source files by physical lines (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>crates/knx-core/src/command.rs</code> | 308,008 | 8,146 |
| <code>crates/knx-net/src/commissioning.rs</code> | 265,528 | 6,199 |
| <code>apps/knx-server/src/domain.rs</code> | 248,652 | 6,013 |
| <code>crates/knx-core/src/dpt/codec.rs</code> | 232,392 | 5,832 |
| <code>crates/knx-productdb/tests/dynamic&#95;tree.rs</code> | 169,832 | 4,273 |
| <code>apps/knx-cli/src/main.rs</code> | 157,276 | 4,212 |
| <code>crates/knx-net/src/cemi.rs</code> | 177,343 | 4,132 |
| <code>crates/knx-productdb/src/migration.rs</code> | 168,606 | 3,832 |
| <code>crates/knx-productdb/src/query.rs</code> | 148,931 | 3,596 |
| <code>crates/knx-net/src/commissioning/download.rs</code> | 147,166 | 3,433 |

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
- Excluded generated or cache: 17.
- Excluded ignored: 95.
- Excluded secret or credential: 1.
- Excluded symlink: 1.
- Unavailable tracked working files: 1; unreadable source files: 0.
- Dirty current checkout&#58; sizes and lines reflect working files, not HEAD; Git index/stat metadata detection is conservative.
- Excluded tracked files&#58; generated or cache=17, ignored=95, secret or credential=1, symlink=1.
- Partial inventory&#58; unavailable working files are omitted; unreadable source lines are N/A.


## Package Dependencies

Offline, Git-tracked declarations and lockfiles only; no package code is executed.

| Metric | Value |
|---|---:|
| Unique declared dependencies | 57 |
| Unique locked package versions | 611 |
| Packages with multiple locked versions | 36 |
| Internal/local package names | 17 |
| Maximum resolved depth (edges) | N/A |
| Unresolved lock edges | 26 |
| Cyclic components | 0 |

Declaration coverage: complete.
Lock inventory coverage: complete.

### Most frequently declared packages (up to 10)

| Ecosystem | Package | Declarations |
|---|---|---:|
| cargo | knx\-core | 12 |
| cargo | tempfile | 9 |
| cargo | chrono | 7 |
| cargo | serde\_json | 7 |
| cargo | knx\-testsupport | 6 |
| cargo | serde | 5 |
| cargo | sha2 | 5 |
| cargo | zip | 5 |
| cargo | base64 | 4 |
| cargo | knx\-etsproj | 4 |
Other 47 package names omitted; full records remain in JSON.

### Multiple locked versions (up to 5; observed subset if coverage is incomplete)

- cargo / windows\-sys: 0\.45\.0, 0\.59\.0, 0\.60\.2; other versions omitted
- cargo / getrandom: 0\.2\.17, 0\.3\.4, 0\.4\.3
- cargo / hashbrown: 0\.12\.3, 0\.16\.1, 0\.17\.1
- cargo / proc\-macro\-crate: 1\.3\.1, 2\.0\.2, 3\.5\.0
- cargo / schemars: 0\.8\.22, 0\.9\.0, 1\.2\.2

### Compact resolved tree

At most 3 roots, 10 direct branches, display depth 3 and 30 tree lines; repeated nodes are not expanded.

- Cargo\.lock:package:knx\-cli@0\.1\.0\-alpha\.1\#182
  - Cargo\.lock:package:chrono@0\.4\.45\#40
    - Cargo\.lock:package:iana\-time\-zone@0\.1\.65\#148
      - Cargo\.lock:package:android\_system\_properties@0\.1\.6\#4
      - Cargo\.lock:package:core\-foundation\-sys@0\.8\.7\#44
      - Cargo\.lock:package:iana\-time\-zone\-haiku@0\.1\.2\#149
    - Cargo\.lock:package:num\-traits@0\.2\.19\#223
      - Cargo\.lock:package:autocfg@1\.5\.1\#10
    - Cargo\.lock:package:serde@1\.0\.229\#302
      - Cargo\.lock:package:serde\_core@1\.0\.229\#304
      - Cargo\.lock:package:serde\_derive@1\.0\.229\#305
  - Cargo\.lock:package:knx\-app@0\.1\.0\-alpha\.1\#181
    - Cargo\.lock:package:base64@0\.22\.1\#14
    - Cargo\.lock:package:chrono@0\.4\.45\#40 (repeated/cyclic)
    - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#183
      - Cargo\.lock:package:chrono@0\.4\.45\#40 (repeated/cyclic)
  - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#183 (repeated/cyclic)
  - Cargo\.lock:package:knx\-csv@0\.1\.0\-alpha\.1\#184
    - Cargo\.lock:package:csv@1\.4\.0\#54
      - Cargo\.lock:package:csv\-core@0\.1\.13\#55
      - Cargo\.lock:package:itoa@1\.0\.18\#165
      - Cargo\.lock:package:ryu@1\.0\.23\#293
    - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#183 (repeated/cyclic)
    - Cargo\.lock:package:serde@1\.0\.229\#302 (repeated/cyclic)
  - Cargo\.lock:package:knx\-diff@0\.1\.0\-alpha\.1\#186
    - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#183 (repeated/cyclic)
  - Cargo\.lock:package:knx\-etsproj@0\.1\.0\-alpha\.1\#187
    - Cargo\.lock:package:chrono@0\.4\.45\#40 (repeated/cyclic)
    - Cargo\.lock:package:flate2@1\.1\.10\#95
      - Cargo\.lock:package:crc32fast@1\.5\.1\#48

Display is truncated where limits apply; full measured graph remains in JSON.

### Availability notes

- apps/knx\-web/package\-lock\.json: missing npm dependency edge; depth unavailable\.


## Changes Since the Previous Run

Previous run (UTC): 2026-10-03T11:21:16.077889+00:00

| Metric | Previous | Current | Change |
| :--- | ---: | ---: | ---: |
| Tracked files | N/A | N/A | N/A |
| Logical project bytes | N/A | N/A | N/A |
| Source files | N/A | N/A | N/A |
| Source bytes | N/A | N/A | N/A |
| Physical source lines | N/A | N/A | N/A |
| Tracked-file directories | N/A | N/A | N/A |
| Maximum directory depth | N/A | N/A | N/A |
| Unique declared dependencies | 57 | 57 | +0 |
| Resolved packages | 611 | 611 | +0 |
| Packages with multiple versions | 36 | 36 | +0 |
| Git commits | 1797 | 1797 | +0 |
| Logged tasks | 3966 | 3966 | +0 |
| Logged tool calls | 73905 | 73921 | +16 |
| Input tokens | 94992143 | 95013763 | +21620 |
| Output tokens | 67655481 | 67667249 | +11768 |
| Cache-read tokens | 16276524607 | 16278168255 | +1643648 |
| Cache-write tokens | 571581662 | 571581662 | +0 |

### Dependency Changes

- **Added declarations:** 0
- **Removed declarations:** 0
- **Changed requirements:** 0
- **Changed locked versions:** 0

These are inventory differences between successful local runs, not interval billing, code authorship or an agent-efficiency score. Removed/moved session logs can reduce counts. The generated report and private state are excluded from file-size measurements. Missing measurements remain N/A; snapshots stay in Git-ignored private state.

