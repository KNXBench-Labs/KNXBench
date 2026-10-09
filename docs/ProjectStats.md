# Project Statistics

**Last update:** 2026-10-09 19:16:37 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **55.984.228 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **46.653 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **381.7 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **18.999 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **111.968 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **5.6 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **23007.2 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **2.239.369 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **245.8 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **402.8 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **358.364.648 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **7.46x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **559.8 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **223.936 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **22.393 kilometers** (about **0.56x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **8.96 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **83.42 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **319.909.879 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **1.119.684.579 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **59.323 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **2.239 kWh** and **739.0 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 2.392
- **Merges:** 276
- **Pushes/Sync:** 2.392
- **Lines Added (+):** 740.773
- **Lines Deleted (-):** 222.231

## Git Text Churn Distribution

| P50 changed text lines | P90 | P95 |
| ---: | ---: | ---: |
| 112 | 808 | 1.389 |

**2.067** measured commits;
**325** unavailable (for example, merge-only
history or binary/unsupported `--numstat` entries). A zero means Git explicitly
reported zero text-line changes; missing data is not turned into zero. This
counts historical additions + deletions per commit, **not AI-attributed** work
or surviving source lines. Branch lifetime cannot be inferred from this table.

## Coding Statistics (Current Working Tree)

**Git-tracked source files:** 991; **physical lines:** 360.151;
**code:** 294.182; **comments:** 41.958; **blank:** 24.011.
**Code/comment ratio:** 7.01:1; **comment share of nonblank lines:** 12.5%.
Tracked source paths unavailable in the working tree: **0**;
excluded generated/vendor/build source paths: **21**.

| Language | Files | Code lines | Comment lines | Blank lines |
| :--- | ---: | ---: | ---: | ---: |
| Rust | 494 | 214.096 | 32.879 | 16.400 |
| TSX | 175 | 37.254 | 3.884 | 3.577 |
| TypeScript | 260 | 31.643 | 4.587 | 2.736 |
| Python | 38 | 4.901 | 82 | 734 |
| CSS | 5 | 3.648 | 419 | 430 |
| JavaScript | 16 | 2.441 | 71 | 106 |
| Shell | 3 | 199 | 36 | 28 |

### Direct-source syntax structures

| Language | Parsed files | Functions/methods | Loops | Type declarations | Branch constructs |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Rust | 494/494 | 8.893 | 2.130 | 1.263 | 4.916 |
| TSX | 0/175 | N/A | N/A | N/A | N/A |
| TypeScript | 0/260 | N/A | N/A | N/A | N/A |
| Python | 38/38 | 310 | 202 | 37 | 378 |
| CSS | N/A (no parser) | N/A | N/A | N/A | N/A |
| JavaScript | 0/16 | N/A | N/A | N/A | N/A |
| Shell | N/A (no parser) | N/A | N/A | N/A | N/A |
| **Supported-source subtotal** | **532/983** | **N/A** | **N/A** | **N/A** | **N/A** |

Structure coverage is **532/983 supported source files**;
**8** source files have no structural parser (for example CSS/Shell).
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
| **Claude** | 153h 33m 53s | 176h 26m 28s | 3.845 |
| **Codex** | N/A | 113h 16m 55s | 708 |
| **Other** | N/A | N/A | 0 |
| **TOTAL** | **153h 33m 53s** | **289h 43m 24s** | **4.553** |

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
**90** unavailable (open sessions,
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
| **Claude** | 4.657.404 | 65.464.623 | 15.654.589.715 | 669.650.253 |
| **Codex** | 149.839.852 | 24.126.539 | 5.825.363.200 | 0 |
| **Other** | 0 | 0 | 0 | 0 |
| **TOTAL** | **154.497.256** | **89.591.162** | **21.479.952.915** | **669.650.253** |

### 🚀 GRAND TOTAL CONSUMPTION
**22.393.691.586 Total Tokens**

## Weekly Usage Trend

| Day (UTC) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| Monday | 4.088.699.726 | 14.966.495 | 490 |
| Tuesday | 948.723.549 | 3.354.454 | 387 |
| Wednesday | 2.376.045.878 | 9.274.042 | 243 |
| Thursday | 3.839.327.454 | 16.339.515 | 663 |
| Friday | 2.599.334.610 | 12.558.999 | 367 |
| Saturday | 4.313.946.675 | 13.977.164 | 1.114 |
| Sunday | 4.227.613.694 | 19.120.493 | 1.289 |

Dated: **22.393.691.586 tokens / 4.553 tasks**;
unallocated: **0 tokens / 0 tasks**.
Monday through Sunday sum **all observed UTC days** for the selected project;
these are weekday totals, not a single calendar week or daily averages. Zero
means no attributable dated usage in that weekday bucket, not proof of no activity.
Local Claude and Codex use event timestamps; Hermes session-scoped model usage
uses the session start timestamp, not an invented per-turn time. Cumulative cloud
model usage remains **unallocated across days**; only timestamped user tasks enter
the weekday totals. The existing collector ledger is not a billing audit.

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 12.483.795.956 | 291.478 | 50.037.163 | 11.965.507.607 | 176h 26m 28s | 3.790 | 69.612 |
| Hermes Agent via Headroom | 6.575.571.426 | 112.395.137 | 28.434.866 | 6.293.951.705 | 0s | 120 | 10.705 |
| Hermes Agent (direct) | 2.281.577.687 | 3.640.231 | 7.020.722 | 2.210.015.907 | 0s | 29 | 3.710 |
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
| Claude Opus 5.5 (`claude-opus-5-5`) | 3.771.199.124 | 2.618.125 | 14.368.925 | 3.570.338.218 | 0s | 51 | 6.286 |
| Claude Opus 5 (`claude-opus-5`) | 3.767.184.218 | 100.370 | 20.790.313 | 3.568.644.548 | 100h 14m 31s | 1.298 | 22.640 |
| GPT-6.1 Sol (`gpt-6.1-sol`) | 2.047.226.552 | 65.049.732 | 12.338.484 | 1.969.838.336 | 0s | 22 | 2.077 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 1.460.223.501 | 34.710.821 | 4.243.304 | 1.421.269.376 | 80h 10m 0s | 227 | 891 |
| gpt-6-sol-900k | 1.217.683.037 | 13.755.988 | 2.635.145 | 1.201.291.904 | 0s | 3 | 624 |
| gpt-6.1-sol-900k | 655.914.651 | 11.369.422 | 1.933.517 | 642.611.712 | 0s | 16 | 2.551 |
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

## Model & Cache Trend

| Relative week | Model | Tokens | Cache read | Cache-read share |
| :--- | :--- | ---: | ---: | ---: |
| 2 weeks earlier | Claude Sonnet 5 (`claude-sonnet-5`) | 64.808.814 | 58.343.740 | 90.4% |
| 3 weeks earlier | Claude Sonnet 5 (`claude-sonnet-5`) | 2.094.049.771 | 2.022.052.095 | 96.9% |
| 4 weeks earlier | Claude Sonnet 5 (`claude-sonnet-5`) | 4.588.883.015 | 4.376.501.810 | 95.7% |
| 5 weeks earlier | Claude Sonnet 5 (`claude-sonnet-5`) | 1.890.995.725 | 1.861.301.396 | 98.6% |
| Latest observed week | Claude Opus 5.5 (`claude-opus-5-5`) | 1.102.461.800 | 1.071.711.186 | 97.6% |
| 1 week earlier | Claude Opus 5.5 (`claude-opus-5-5`) | 2.117.292.797 | 2.025.763.198 | 96.0% |
| 2 weeks earlier | Claude Opus 5.5 (`claude-opus-5-5`) | 551.444.527 | 472.863.834 | 86.3% |
| 2 weeks earlier | Claude Opus 5 (`claude-opus-5`) | 303.582.253 | 276.558.994 | 91.4% |
| 3 weeks earlier | Claude Opus 5 (`claude-opus-5`) | 1.739.086.785 | 1.656.048.885 | 95.7% |
| 4 weeks earlier | Claude Opus 5 (`claude-opus-5`) | 1.692.215.588 | 1.605.147.744 | 95.4% |
| 5 weeks earlier | Claude Opus 5 (`claude-opus-5`) | 32.299.592 | 30.888.925 | 95.7% |
| Latest observed week | GPT-6.1 Sol (`gpt-6.1-sol`) | 455.976.423 | 439.556.096 | 96.9% |
| 1 week earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 1.591.250.129 | 1.530.282.240 | 96.8% |
| 2 weeks earlier | GPT-5.6 Sol (`gpt-5.6-sol`) | 1.233.389.499 | 1.202.323.712 | 97.8% |
| 3 weeks earlier | GPT-5.6 Sol (`gpt-5.6-sol`) | 226.834.002 | 218.945.664 | 96.9% |

Top five models by selected-project token volume, across the last eight
observed UTC weeks (Monday through Sunday). **Latest observed week** is the most
recent week with dated usage among these models, not necessarily the current week.
**N weeks earlier** counts calendar weeks back from that shared reference, including
weeks without recorded activity. **Unallocated** means that a canonical model ledger has
no reliable per-day timestamp (especially cumulative cloud usage); it is not
placed on the last result's day. Model-time mapping covers
**22.393.691.586 tokens**; **0 tokens**
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
| high | 20.460.609.758 | 122.400.783 | 81.141.832 | 19.640.807.593 | 189h 45m 52s | 2.613 | 74.984 |
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
| 2 | read_file | 36.777 |
| 3 | terminal | 23.401 |
| 4 | execute_code | 14.684 |
| 5 | skill_view | 10.225 |
| 6 | patch | 9.552 |
| 7 | Read | 9.203 |
| 8 | search_files | 7.281 |
| 9 | Edit | 5.290 |
| 10 | tool_call | 4.022 |
| 11 | write_file | 2.239 |
| 12 | Write | 941 |
| 13 | Agent | 880 |
| 14 | ToolSearch | 554 |
| 15 | tool_describe | 533 |
| 16 | vision_analyze | 430 |
| 17 | Monitor | 369 |
| 18 | wait_agent | 368 |
| 19 | skill_manage | 189 |
| 20 | SendMessage | 187 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | autonomous-goal-boundaries | 948 |
| 2 | test-driven-development | 942 |
| 3 | cross-layer-contract-verification | 860 |
| 4 | compressed-tool-output-recovery | 857 |
| 5 | repository-delivery | 726 |
| 6 | frontend-design | 725 |
| 7 | playwright | 699 |
| 8 | systematic-debugging | 541 |
| 9 | safety-confirmation-gates | 467 |
| 10 | requesting-code-review | 446 |
| 11 | private-corpus-regression-testing | 443 |
| 12 | data-integrity-feature-verification | 367 |
| 13 | project-completion-auditing | 363 |
| 14 | hermes-agent | 280 |
| 15 | grounded-citations | 252 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 30,230.2 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 3.33:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 93.0 | AI output ÷ added and deleted Git lines |
| Git commit density | 9,361,911.2 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $8,203.51 | 95.9% | $42,267.39 |
| Codex | $2,192.84 | 97.5% | $13,107.07 |
| Other | N/A | N/A | N/A |
| **PRICED SUBTOTAL** | **$10,396.35** | **96.3%** | **$55,374.46** |

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
- **Autonomy input/output ratio:** 1.72:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 18.63 logged tool calls per task (84.824 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 518 | 11.4% |
| Morning (06-11) | 1.372 | 30.1% |
| Afternoon (12-17) | 1.550 | 34.0% |
| Evening (18-23) | 1.113 | 24.4% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 536 | 11.8% |
| Tuesday | 396 | 8.7% |
| Wednesday | 230 | 5.1% |
| Thursday | 641 | 14.1% |
| Friday | 388 | 8.5% |
| Saturday | 1.079 | 23.7% |
| Sunday | 1.283 | 28.2% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 2,239.4 kWh
- **Estimated CO₂ equivalent:** 739.0 kg CO₂e
- **Estimated physical keystrokes avoided:** 358.364.648

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.


## Project Size & File Inventory

Scope: **partial — measured subset only**, no tracked index/stat changes detected; HEAD revision: <code>58e2852f731d7f9fc2ff3db8a9502e5fe71d395e</code>.

Git index/stat metadata only; conservative, without content comparisons. Secret/cache/report exclusions and submodule working contents are not assessed.

Logical bytes are file lengths, not disk allocation. Source suffixes are counted independently of categories; tests/docs take category precedence. Source lines are streaming physical UTF-8 text lines, including comments and blanks, not parser/tokei code counts.

Directories are unique parents of measured tracked files, including the root at depth zero. File depth means its parent directory depth. Name/path lengths count Unicode characters (not graphemes) separately from filesystem-encoded bytes, excluding the absolute root prefix.

| Metric | Value |
| --- | --- |
| Files (measured) | 1,459 |
| Logical bytes (measured) | 47,807,465 |
| Source files | 982 |
| Source logical bytes | 14,743,504 |
| Source physical lines | 359,234 |
| Tracked directories (including root) | 157 |
| Maximum directory depth | 6 |

| Category | Files | Logical bytes |
| --- | --- | --- |
| source | 593 | 10,119,743 |
| tests | 387 | 4,599,444 |
| docs | 339 | 23,151,911 |
| config | 63 | 1,010,711 |
| assets | 26 | 2,785,660 |
| other | 51 | 6,139,996 |

Readable UTF-8 source files: **982**; median file lines: **171.0**; p95 file lines: **1,225** (nearest-rank). Unreadable source files are excluded from distributions; their aggregate source lines are N/A.

### Extension distribution (top 10 by bytes)

| Extension | Files | Logical bytes |
| --- | --- | --- |
| <code>.gif</code> | 5 | 11,468,773 |
| <code>.rs</code> | 486 | 10,107,426 |
| <code>.png</code> | 56 | 6,360,656 |
| <code>.md</code> | 228 | 4,668,685 |
| <code>.knxdb</code> | 3 | 3,059,712 |
| <code>.mp4</code> | 3 | 2,114,631 |
| <code>.tsx</code> | 175 | 2,044,027 |
| <code>.ts</code> | 259 | 1,954,785 |
| <code>.zip</code> | 4 | 1,601,762 |
| <code>.json</code> | 68 | 1,409,020 |

### File and path extremes

| Extreme | Path/name | Measurement |
| --- | --- | --- |
| Longest filename | <code>docs/adr/0053-contributions-come-with-a-license-grant-for-dual-licensing.md</code> | 66 Unicode characters; 66 filesystem-encoded bytes |
| Longest relative path | <code>docs/evidence/community-demos/screenshots/multi-unit-residential-buildings.png</code> | 78 Unicode characters; 78 filesystem-encoded bytes |
| Deepest file | <code>crates/knx-productdb/fixtures/legacy/src-pr/MARVIN/ets.pr&#95;</code> | Parent depth 6 |
| Shortest nonempty readable source | <code>apps/knx-web/src/vite-env.d.ts</code> | 1 physical lines; 38 bytes |
| Longest readable source line | <code>crates/knx-app/src/contribution&#95;bundle.rs</code> | Line 197; 1,036 Unicode characters (without newline) |
| Most common basename (case-sensitive) | <code>Cargo.toml</code> | 19 files |

### Largest files by logical bytes (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>docs/assets/readme/hero-add-device.gif</code> | 7,883,659 | N/A |
| <code>docs/assets/readme/telegram-flow.gif</code> | 2,752,893 | N/A |
| <code>demos/1.0.0/office-building.knxdb</code> | 1,421,312 | N/A |
| <code>website/assets/project-work.mp4</code> | 1,256,389 | N/A |
| <code>demos/1.0.0/multi-unit-residential.knxdb</code> | 1,146,880 | N/A |
| <code>docs/history/IMPLEMENTATION&#95;STATUS&#95;2026-09.md</code> | 832,001 | N/A |
| <code>website/assets/bus-flow.mp4</code> | 761,738 | N/A |
| <code>demos/1.0.0/knxbench-community-demos-1.0.0.zip</code> | 725,144 | N/A |
| <code>demos/1.0.0/projects.json</code> | 593,778 | N/A |
| <code>docs/KNOWN&#95;LIMITATIONS.md</code> | 571,733 | N/A |

### Largest source files by physical lines (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>crates/knx-core/src/command.rs</code> | 352,915 | 9,306 |
| <code>apps/knx-server/src/domain.rs</code> | 310,715 | 7,715 |
| <code>crates/knx-net/src/commissioning.rs</code> | 266,441 | 6,221 |
| <code>crates/knx-core/src/dpt/codec.rs</code> | 238,140 | 5,961 |
| <code>apps/knx-cli/src/main.rs</code> | 184,379 | 4,852 |
| <code>crates/knx-productdb/tests/dynamic&#95;tree.rs</code> | 176,748 | 4,443 |
| <code>crates/knx-productdb/src/migration.rs</code> | 186,163 | 4,237 |
| <code>crates/knx-net/src/cemi.rs</code> | 177,343 | 4,132 |
| <code>crates/knx-productdb/src/query.rs</code> | 164,613 | 3,958 |
| <code>apps/knx-web/src/styles.css</code> | 142,820 | 3,950 |

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
- Excluded ignored: 265.
- Excluded report or state: 1.
- Excluded secret or credential: 11.
- Excluded symlink: 1.
- Unavailable tracked working files: 1; unreadable source files: 0.
- Excluded tracked files&#58; generated or cache=21, ignored=265, report or state=1, secret or credential=11, symlink=1.
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
| cargo | serde | 7 |
| cargo | zip | 7 |
| cargo | knx\-testsupport | 6 |
| cargo | sha2 | 6 |
| cargo | tokio | 6 |
| cargo | knx\-store | 5 |
Other 55 package names omitted; full records remain in JSON.

### Multiple locked versions (up to 5; observed subset if coverage is incomplete)

- cargo / windows\-sys: 0\.45\.0, 0\.52\.0, 0\.59\.0; other versions omitted
- cargo / base64: 0\.21\.7, 0\.22\.1, 0\.23\.1
- cargo / getrandom: 0\.2\.17, 0\.3\.4, 0\.4\.3
- cargo / hashbrown: 0\.12\.3, 0\.16\.1, 0\.17\.1
- cargo / proc\-macro\-crate: 1\.3\.1, 2\.0\.2, 3\.5\.0

### Compact resolved tree

At most 3 roots, 10 direct branches, display depth 3 and 30 tree lines; repeated nodes are not expanded.

- Cargo\.lock:package:knx\-cli@0\.1\.0\-alpha\.6\#194
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

Previous run (UTC): 2026-10-09T16:49:52.138225+00:00

| Metric | Previous | Current | Change |
| :--- | ---: | ---: | ---: |
| Tracked files | N/A | N/A | N/A |
| Logical project bytes | N/A | N/A | N/A |
| Source files | N/A | N/A | N/A |
| Source bytes | N/A | N/A | N/A |
| Physical source lines | N/A | N/A | N/A |
| Tracked-file directories | N/A | N/A | N/A |
| Maximum directory depth | N/A | N/A | N/A |
| Unique declared dependencies | 65 | 65 | +0 |
| Resolved packages | N/A | N/A | N/A |
| Packages with multiple versions | N/A | N/A | N/A |
| Git commits | 2385 | 2392 | +7 |
| Logged tasks | 4552 | 4553 | +1 |
| Logged tool calls | 85034 | 84824 | -210 |
| Input tokens | 154076152 | 154497256 | +421104 |
| Output tokens | 89513495 | 89591162 | +77667 |
| Cache-read tokens | 21459169299 | 21479952915 | +20783616 |
| Cache-write tokens | 669650253 | 669650253 | +0 |

### Dependency Changes

- **Added declarations:** 0
- **Removed declarations:** 0
- **Changed requirements:** 0
- **Changed locked versions:** N/A — incomplete source coverage in either run


These are inventory differences between successful local runs, not interval billing, code authorship or an agent-efficiency score. Removed/moved session logs can reduce counts. The generated report and private state are excluded from file-size measurements. Missing measurements remain N/A; snapshots stay in Git-ignored private state.
