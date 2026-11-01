# Project Statistics

**Last update:** 2026-11-01 18:50:19 CET (UTC+01:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **109.979.701 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **91.649 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **749.9 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **37.323 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **219.959 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **11.0 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **45197.1 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **4.399.188 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **482.9 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **791.3 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **646.123.476 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **14.66x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **1099.8 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **439.918 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **43.991 kilometers** (about **1.10x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **17.60 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **163.88 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **628.455.435 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **2.199.594.025 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **116.538 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **4.399 kWh** and **1451.7 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 4.307
- **Merges:** 697
- **Pushes/Sync:** 4.307
- **Lines Added (+):** 1.330.273
- **Lines Deleted (-):** 296.758

## Git Text Churn Distribution

| P50 changed text lines | P90 | P95 |
| ---: | ---: | ---: |
| 103 | 875 | 1.520 |

**3.548** measured commits;
**759** unavailable (for example, merge-only
history or binary/unsupported `--numstat` entries). A zero means Git explicitly
reported zero text-line changes; missing data is not turned into zero. This
counts historical additions + deletions per commit, **not AI-attributed** work
or surviving source lines. Branch lifetime cannot be inferred from this table.

## Coding Statistics (Current Working Tree)

**Git-tracked source files:** 2.190; **physical lines:** 715.467;
**code:** 616.239; **comments:** 58.978; **blank:** 40.250.
**Code/comment ratio:** 10.45:1; **comment share of nonblank lines:** 8.7%.
Tracked source paths unavailable in the working tree: **0**;
excluded generated/vendor/build source paths: **31**.

| Language | Files | Code lines | Comment lines | Blank lines |
| :--- | ---: | ---: | ---: | ---: |
| Rust | 1.171 | 463.670 | 45.882 | 27.807 |
| TSX | 327 | 62.761 | 5.208 | 5.098 |
| TypeScript | 537 | 62.594 | 6.697 | 4.398 |
| Python | 89 | 18.196 | 345 | 2.314 |
| CSS | 34 | 5.387 | 685 | 485 |
| JavaScript | 27 | 3.408 | 109 | 120 |
| Shell | 5 | 223 | 52 | 28 |

### Direct-source syntax structures

| Language | Parsed files | Functions/methods | Loops | Type declarations | Branch constructs |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Rust | 1.171/1.171 | 18.436 | 5.006 | 2.736 | 11.208 |
| TSX | 327/327 | 12.151 | 266 | 136 | 3.150 |
| TypeScript | 537/537 | 8.049 | 1.283 | 814 | 4.094 |
| Python | 89/89 | 1.250 | 678 | 118 | 1.360 |
| CSS | N/A (no parser) | N/A | N/A | N/A | N/A |
| JavaScript | 27/27 | 822 | 85 | 0 | 356 |
| Shell | N/A (no parser) | N/A | N/A | N/A | N/A |
| **Supported-source subtotal** | **2.151/2.151** | **40.708** | **7.318** | **3.804** | **20.168** |

Structure coverage is **2.151/2.151 supported source files**;
**39** source files have no function/type/loop structural parser (for example CSS/Shell).
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
comments/blanks; **12** nested
code-fence example lines are also classified as comments, not executable
source. Python docstrings remain code lines. Tests are included. Untracked
source, non-programming files, known generated/vendor/build directories and
binary files are excluded.
This is a **current checkout** inventory, not Git history, AI attribution or
proof that all lines execute. Line classification follows tokei's syntax rules.

### CSS syntax structures

| Parsed CSS files | Rule nodes | At-rule nodes | Declaration nodes |
| :--- | ---: | ---: | ---: |
| 34/34 | 2.756 | 96 | 8.381 |

Optional PostCSS 8 parser: `8.5.27`.
Unavailable CSS files: **0**. Partial coverage
makes all CSS totals **N/A**, not zero or a successful-file subtotal. Rule nodes
include nested selectors and keyframe steps; declarations include custom
properties. Comments and string contents are not nodes of those types. These
are syntax counts, **not compiler/browser validation**, computed without
plugins or source-map/resource loading. CSS counts are a separate metric family,
not functions, loops or type declarations. Shell has no admitted AST parser.

### Repository composition at the measured revision

The current source inventory is bound to Git revision `a93258e7ca5b02ac445aff9fd39c06ea77009f6e`.
It includes test files and excludes generated bindings, dependencies and build output.
Of the 2,190 measured source files, **1,065 separately
recognizable test files** contain **270,787 code lines**
(43.9% of the 616,239 measured code lines).
This path-based estimate does not include test modules embedded in other source
files and is neither executed-test count nor test coverage.

| Area (including its tests) | Code lines |
| :--- | ---: |
| Rust libraries (`crates/`) | 305,363 |
| Applications (`apps/`) | 286,045 |
| Web application (`apps/knx-web/`, subset of apps) | 130,434 |
| Server (`apps/knx-server/`, subset of apps) | 117,374 |
| CLI (`apps/knx-cli/`, subset of apps) | 31,744 |
| Project tools (`tools/` and `xtask/`) | 18,899 |

This revision has **4,068 Git-tracked files**, **19
Rust workspace members** and **152 numbered architecture decision
records**. The **894 Markdown files under `docs/`** contain
**150,215 physical lines** (including blank lines).
These documentation files are not counted as source-code comments above.
The area totals are subsets and are not an exhaustive partition of the source
inventory; the three application subareas overlap with `apps/`.

Largest measured source files by code lines (including any inline tests):
- `crates/knx-core/src/command.rs`: 8,954
- `apps/knx-server/src/domain.rs`: 7,527
- `apps/knx-cli/src/main.rs`: 4,905
- `crates/knx-net/src/commissioning.rs`: 4,548
- `apps/knx-web/src/messages/de.ts`: 4,332

## Session Time & Execution Analysis
| Provider | Reasoning-adjacent gap estimate | Execution interval proxy | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 157h 43m 41s | 182h 25m 54s | 4.216 |
| **Codex** | N/A | 127h 26m 28s | 1.443 |
| **Other** | N/A | N/A | 20 |
| **TOTAL** | **157h 43m 41s** | **309h 52m 22s** | **5.679** |

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
| 652 | 10m 2s | 35m 49s | 58m 45s | 271h 5m 10s |

**652** measured Hermes sessions;
**364** unavailable (open sessions,
invalid timestamps or untrusted end reasons). Percentiles use the nearest-rank
method. Peak concurrency: **9**
trusted closed Hermes sessions, counting touching endpoints as non-overlapping.
Open/orphaned sessions are excluded, so this is not total provider concurrency.
These elapsed start-to-close intervals can include human pauses and tool waits:
they are **not API generation time**. Other providers' session durations are
`N/A` until an equally reliable end marker is verified.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 6.894.808 | 93.630.117 | 23.315.517.751 | 1.065.015.671 |
| **Codex** | 435.239.902 | 64.710.399 | 18.157.424.000 | 0 |
| **Other** | 9.663.585 | 3.190.353 | 840.593.920 | 0 |
| **TOTAL** | **451.798.295** | **161.530.869** | **42.313.535.671** | **1.065.015.671** |

### 🚀 GRAND TOTAL CONSUMPTION
**43.991.880.506 Total Tokens**

## Weekly Usage Trend

| Day (UTC) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| Monday | 7.159.808.780 | 24.282.830 | 642 |
| Tuesday | 4.605.972.187 | 13.949.279 | 563 |
| Wednesday | 3.913.839.598 | 14.222.624 | 299 |
| Thursday | 6.953.432.192 | 27.260.535 | 758 |
| Friday | 5.137.351.577 | 20.468.063 | 534 |
| Saturday | 7.872.063.209 | 26.888.002 | 1.278 |
| Sunday | 8.349.412.963 | 34.459.536 | 1.526 |

Dated: **43.991.880.506 tokens / 5.600 tasks**;
unallocated: **0 tokens / 79 tasks**.
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
| Hermes Agent (direct) | 20.688.362.708 | 236.673.588 | 63.971.740 | 19.946.977.676 | 0s | 848 | 89.150 |
| Claude Code | 13.010.773.739 | 300.028 | 53.700.724 | 12.473.286.738 | 182h 25m 54s | 3.948 | 73.471 |
| Hermes Agent via Headroom | 9.042.664.931 | 169.165.039 | 38.811.589 | 8.693.898.585 | 0s | 168 | 19.188 |
| Codex CLI | 1.250.079.128 | 45.659.640 | 5.046.816 | 1.199.372.672 | 127h 26m 28s | 715 | 840 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| GPT-6.1 Sol (`gpt-6.1-sol`) | 12.990.350.039 | 318.353.242 | 47.753.213 | 12.624.243.584 | 10h 31m 54s | 650 | 63.026 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 11.537.231.913 | 4.206.091 | 40.482.712 | 10.923.567.826 | 3h 18m 13s | 276 | 30.130 |
| Claude Sonnet 5 (`claude-sonnet-5`) | 8.638.737.325 | 162.531 | 28.854.876 | 8.318.199.041 | 72h 28m 43s | 1.317 | 44.974 |
| Claude Opus 5 (`claude-opus-5`) | 3.767.184.218 | 100.370 | 20.790.313 | 3.568.644.548 | 100h 14m 31s | 1.298 | 22.640 |
| gpt-6.1-sol-900k | 2.307.167.752 | 41.276.765 | 7.052.843 | 2.258.838.144 | 0s | 62 | 10.296 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 1.501.717.133 | 36.916.666 | 4.452.563 | 1.460.347.904 | 82h 46m 27s | 246 | 894 |
| gpt-6-sol-900k | 1.217.683.037 | 13.755.988 | 2.635.145 | 1.201.291.904 | 0s | 3 | 624 |
| DeepSeek V4 Pro (`deepseek-v4-pro`) | 853.447.858 | 9.663.585 | 3.190.353 | 840.593.920 | 0s | 20 | 3.338 |
| Claude Sonnet 5.5 (`claude-sonnet-5-5`) | 319.679.869 | 4.140 | 1.801.419 | 307.674.108 | 2h 40m 59s | 16 | 1.938 |
| GPT-6 Sol (`gpt-6-sol`) | 212.529.664 | 2.629.893 | 385.787 | 209.513.984 | 56m 37s | 5 | 125 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 183.022.732 | 13.937.809 | 2.164.987 | 166.919.936 | 3h 0m 5s | 55 | 2.293 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 153.728.019 | 38.190 | 330.177 | 140.766.004 | 2h 37m 49s | 11 | 1.781 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 136.567.795 | 6.447.704 | 685.851 | 129.434.240 | 20h 9m 9s | 174 | 87 |
| GPT-6 Astra (`gpt-6-astra`) | 111.635.050 | 4.288.522 | 512.224 | 106.834.304 | 9h 57m 13s | 21 | 183 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| Claude Haiku 5.5 (`claude-haiku-5-5`) | 93.611 | 12 | 5.491 | 24.320 | 12s | 0 | 6 |
| <synthetic> | 0 | 0 | 0 | 0 | 33m 35s | 113 | 0 |
| Claude Sonnet 4.6 (`claude-sonnet-4-6`) | 0 | 0 | 0 | 0 | 0s | 1 | 0 |
| GPT-5.5 (`gpt-5.5`) | 0 | 0 | 0 | 0 | 2s | 1 | 0 |
| unknown | 0 | 0 | 0 | 0 | 4m 58s | 1.404 | 0 |

Model IDs are discovered directly from Claude, Codex and Hermes usage records;
display names are loaded dynamically from every available Hermes model cache.
There is no fixed model allowlist, so future persisted model IDs appear automatically
and unknown IDs remain visible verbatim instead of being discarded.

## Model & Cache Trend

| Relative week | Model | Tokens | Cache read | Cache-read share |
| :--- | :--- | ---: | ---: | ---: |
| Latest observed week | GPT-6.1 Sol (`gpt-6.1-sol`) | 3.424.719.949 | 3.348.897.536 | 98.1% |
| 1 week earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 4.177.581.004 | 4.055.693.824 | 97.4% |
| 2 weeks earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 2.440.227.752 | 2.383.230.336 | 97.9% |
| 3 weeks earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 1.356.571.205 | 1.306.139.648 | 96.9% |
| 4 weeks earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 1.591.250.129 | 1.530.282.240 | 96.8% |
| Latest observed week | Claude Opus 5.5 (`claude-opus-5-5`) | 2.216.828.207 | 2.103.861.717 | 95.2% |
| 1 week earlier | Claude Opus 5.5 (`claude-opus-5-5`) | 1.316.385.507 | 1.225.104.623 | 93.4% |
| 2 weeks earlier | Claude Opus 5.5 (`claude-opus-5-5`) | 4.073.497.603 | 3.881.843.614 | 95.6% |
| 3 weeks earlier | Claude Opus 5.5 (`claude-opus-5-5`) | 1.261.783.272 | 1.214.130.840 | 96.6% |
| 4 weeks earlier | Claude Opus 5.5 (`claude-opus-5-5`) | 2.117.292.797 | 2.025.763.198 | 96.0% |
| 5 weeks earlier | Claude Opus 5.5 (`claude-opus-5-5`) | 551.444.527 | 472.863.834 | 86.3% |
| 5 weeks earlier | Claude Sonnet 5 (`claude-sonnet-5`) | 64.808.814 | 58.343.740 | 90.4% |
| 6 weeks earlier | Claude Sonnet 5 (`claude-sonnet-5`) | 2.094.049.771 | 2.022.052.095 | 96.9% |
| 7 weeks earlier | Claude Sonnet 5 (`claude-sonnet-5`) | 4.588.883.015 | 4.376.501.810 | 95.7% |
| 5 weeks earlier | Claude Opus 5 (`claude-opus-5`) | 303.582.253 | 276.558.994 | 91.4% |
| 6 weeks earlier | Claude Opus 5 (`claude-opus-5`) | 1.739.086.785 | 1.656.048.885 | 95.7% |
| 7 weeks earlier | Claude Opus 5 (`claude-opus-5`) | 1.692.215.588 | 1.605.147.744 | 95.4% |
| Latest observed week | `gpt-6.1-sol-900k` | 49.413.062 | 48.272.768 | 98.0% |
| 1 week earlier | `gpt-6.1-sol-900k` | 98.461.199 | 96.081.152 | 97.8% |
| 2 weeks earlier | `gpt-6.1-sol-900k` | 195.558.189 | 190.849.408 | 97.8% |
| 3 weeks earlier | `gpt-6.1-sol-900k` | 1.963.119.321 | 1.923.225.088 | 98.3% |
| 4 weeks earlier | `gpt-6.1-sol-900k` | 615.981 | 409.728 | 66.7% |

Top five models by selected-project token volume, across the last eight
observed UTC weeks (Monday through Sunday). **Latest observed week** is the most
recent week with dated usage among these models, not necessarily the current week.
**N weeks earlier** counts calendar weeks back from that shared reference, including
weeks without recorded activity. **Unallocated** means that a canonical model ledger has
no reliable per-day timestamp (especially cumulative cloud usage); it is not
placed on the last result's day. Model-time mapping covers
**43.991.880.506 tokens**; **0 tokens**
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
| high | 39.276.839.321 | 391.832.675 | 145.125.420 | 37.804.236.500 | 207h 2m 31s | 3.493 | 163.804 |
| medium | 3.828.081.914 | 56.700.426 | 11.741.098 | 3.672.873.925 | 75h 22m 51s | 475 | 13.119 |
| xhigh | 712.876.041 | 2.212.187 | 4.237.172 | 676.415.850 | 22h 57m 36s | 277 | 3.657 |
| unknown | 172.947.223 | 977.780 | 414.591 | 158.961.204 | 4h 9m 11s | 1.434 | 2.067 |
| max | 1.136.007 | 75.227 | 12.588 | 1.048.192 | 19m 17s | 0 | 2 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | read_file | 88.449 |
| 2 | terminal | 66.861 |
| 3 | Bash | 53.919 |
| 4 | execute_code | 32.031 |
| 5 | patch | 19.602 |
| 6 | skill_view | 16.859 |
| 7 | search_files | 14.082 |
| 8 | tool_call | 10.884 |
| 9 | Read | 9.987 |
| 10 | write_file | 8.275 |
| 11 | Edit | 5.320 |
| 12 | vision_analyze | 2.162 |
| 13 | tool_describe | 1.394 |
| 14 | delegate_task | 1.280 |
| 15 | Write | 959 |
| 16 | Agent | 886 |
| 17 | skill_manage | 669 |
| 18 | Grep | 619 |
| 19 | ToolSearch | 564 |
| 20 | Monitor | 410 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | repository-delivery | 1.663 |
| 2 | compressed-tool-output-recovery | 1.564 |
| 3 | autonomous-goal-boundaries | 1.417 |
| 4 | test-driven-development | 1.390 |
| 5 | cross-layer-contract-verification | 1.093 |
| 6 | playwright | 833 |
| 7 | requesting-code-review | 818 |
| 8 | systematic-debugging | 767 |
| 9 | frontend-design | 757 |
| 10 | private-corpus-regression-testing | 702 |
| 11 | delegated-code-verification | 546 |
| 12 | data-integrity-feature-verification | 535 |
| 13 | project-completion-auditing | 518 |
| 14 | safety-confirmation-gates | 482 |
| 15 | claude-code | 400 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 33,069.8 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.48:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 99.3 | AI output ÷ added and deleted Git lines |
| Git commit density | 10,214,042.4 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $12,413.60 | 95.6% | $62,951.90 |
| Codex | $6,598.11 | 97.7% | $40,854.20 |
| Other | N/A | N/A | N/A |
| **PRICED SUBTOTAL** | **$19,011.71** | **96.5%** | **$103,806.10** |

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

- **Prime hour:** 16:00–17:00 local time (405 logged activity events)
- **Prime weekday:** Sunday (1.526 logged activity events)
- **Generation throughput:** N/A — output tokens and measured generation seconds are not joined per event across all sources
- **Autonomy input/output ratio:** 2.80:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 32.16 logged tool calls per task (182.649 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 647 | 11.6% |
| Morning (06-11) | 1.647 | 29.4% |
| Afternoon (12-17) | 1.865 | 33.3% |
| Evening (18-23) | 1.441 | 25.7% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 691 | 12.3% |
| Tuesday | 574 | 10.2% |
| Wednesday | 294 | 5.2% |
| Thursday | 734 | 13.1% |
| Friday | 546 | 9.8% |
| Saturday | 1.235 | 22.1% |
| Sunday | 1.526 | 27.3% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 4,399.2 kWh
- **Estimated CO₂ equivalent:** 1,451.7 kg CO₂e
- **Estimated physical keystrokes avoided:** 646.123.476

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.


## Project Size & File Inventory

Scope: **partial — measured subset only**, no tracked index/stat changes detected; HEAD revision: <code>a93258e7ca5b02ac445aff9fd39c06ea77009f6e</code>.

Git index/stat metadata only; conservative, without content comparisons. Secret/cache/report exclusions and submodule working contents are not assessed.

Logical bytes are file lengths, not disk allocation. Source suffixes are counted independently of categories; tests/docs take category precedence. Source lines are streaming physical UTF-8 text lines, including comments and blanks, not parser/tokei code counts.

Directories are unique parents of measured tracked files, including the root at depth zero. File depth means its parent directory depth. Name/path lengths count Unicode characters (not graphemes) separately from filesystem-encoded bytes, excluding the absolute root prefix.

| Metric | Value |
| --- | --- |
| Files (measured) | 3,568 |
| Logical bytes (measured) | 86,047,457 |
| Source files | 2,160 |
| Source logical bytes | 30,039,256 |
| Source physical lines | 709,090 |
| Tracked directories (including root) | 306 |
| Maximum directory depth | 8 |

| Category | Files | Logical bytes |
| --- | --- | --- |
| source | 1,282 | 18,915,499 |
| tests | 898 | 14,345,594 |
| docs | 1,195 | 38,579,694 |
| config | 97 | 1,574,555 |
| assets | 33 | 3,420,354 |
| other | 63 | 9,211,761 |

Readable UTF-8 source files: **2,160**; median file lines: **170.0**; p95 file lines: **1,004** (nearest-rank). Unreadable source files are excluded from distributions; their aggregate source lines are N/A.

### Extension distribution (top 10 by bytes)

| Extension | Files | Logical bytes |
| --- | --- | --- |
| <code>.rs</code> | 1,142 | 20,245,217 |
| <code>.png</code> | 104 | 14,865,911 |
| <code>.gif</code> | 5 | 11,482,795 |
| <code>.md</code> | 913 | 9,933,633 |
| <code>.sqlite</code> | 11 | 6,905,856 |
| <code>.ts</code> | 536 | 4,300,354 |
| <code>.tsx</code> | 327 | 3,833,251 |
| <code>.json</code> | 225 | 3,463,910 |
| <code>.knxdb</code> | 3 | 3,059,712 |
| <code>.mp4</code> | 3 | 2,136,600 |

### File and path extremes

| Extreme | Path/name | Measurement |
| --- | --- | --- |
| Longest filename | <code>docs/research/commissioning/2026-09-29-download-coverage-re-measured-signed-and-text-values-honest-mask.md</code> | 78 Unicode characters; 78 filesystem-encoded bytes |
| Longest relative path | <code>crates/knx-app/tests/it/system2&#95;validation&#95;preparation/configured&#95;hardware/pid7&#95;policy/pid7&#95;failure&#95;diagnostic.rs</code> | 113 Unicode characters; 113 filesystem-encoded bytes |
| Deepest file | <code>crates/knx-net/src/commissioning/memory&#95;download/tests/region&#95;reads/budget/note.rs</code> | Parent depth 8 |
| Shortest nonempty readable source | <code>apps/knx-web/src/vite-env.d.ts</code> | 1 physical lines; 38 bytes |
| Longest readable source line | <code>tools/design-studies/group-addresses/style.css</code> | Line 4; 14,666 Unicode characters (without newline) |
| Most common basename (case-sensitive) | <code>tests.rs</code> | 22 files |

### Largest files by logical bytes (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>docs/assets/readme/hero-add-device.gif</code> | 7,883,659 | N/A |
| <code>docs/assets/readme/telegram-flow.gif</code> | 2,752,893 | N/A |
| <code>crates/knx-store/tests/fixtures/native13-before-notes-working.sqlite</code> | 1,806,336 | N/A |
| <code>website/assets/project-work.mp4</code> | 1,537,621 | N/A |
| <code>crates/knx-store/fixtures/project&#95;v12&#95;instruction&#95;history.sqlite</code> | 1,474,560 | N/A |
| <code>crates/knx-store/fixtures/project&#95;v12&#95;instruction&#95;history&#95;device.sqlite</code> | 1,474,560 | N/A |
| <code>demos/1.0.0/office-building.knxdb</code> | 1,421,312 | N/A |
| <code>crates/knx-store/tests/fixtures/native13-before-notes-saved.sqlite</code> | 1,290,240 | N/A |
| <code>demos/1.0.0/multi-unit-residential.knxdb</code> | 1,146,880 | N/A |
| <code>docs/history/IMPLEMENTATION&#95;STATUS&#95;2026-09.md</code> | 831,943 | N/A |

### Largest source files by physical lines (up to 10)

| Path | Logical bytes | Source physical lines |
| --- | --- | --- |
| <code>crates/knx-core/src/command.rs</code> | 375,748 | 9,833 |
| <code>apps/knx-server/src/domain.rs</code> | 375,859 | 9,291 |
| <code>crates/knx-net/src/commissioning.rs</code> | 278,750 | 6,496 |
| <code>crates/knx-core/src/dpt/codec.rs</code> | 238,140 | 5,961 |
| <code>apps/knx-cli/src/main.rs</code> | 215,666 | 5,552 |
| <code>crates/knx-productdb/src/migration.rs</code> | 223,823 | 5,134 |
| <code>apps/knx-web/src/styles.css</code> | 207,614 | 4,980 |
| <code>apps/knx-web/src/App.test.tsx</code> | 238,777 | 4,951 |
| <code>apps/knx-web/src/messages/en.ts</code> | 368,376 | 4,662 |
| <code>apps/knx-web/src/messages/de.ts</code> | 389,239 | 4,480 |

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
- Excluded generated or cache: 31.
- Excluded ignored: 434.
- Excluded report or state: 1.
- Excluded secret or credential: 33.
- Excluded symlink: 1.
- Unavailable tracked working files: 1; unreadable source files: 0.
- Excluded tracked files&#58; generated or cache=31, ignored=434, report or state=1, secret or credential=33, symlink=1.
- Partial inventory&#58; unavailable working files are omitted; unreadable source lines are N/A.


## Package Dependencies

Offline, Git-tracked declarations and lockfiles only; no package code is executed.

| Metric | Value |
|---|---:|
| Unique declared dependencies | 68 |
| Unique locked package versions | N/A |
| Packages with multiple locked versions | N/A |
| Internal/local package names | 20 |
| Maximum resolved depth (edges) | N/A |
| Unresolved lock edges | 26 |
| Cyclic components | 0 |

Declaration coverage: complete.
Lock inventory coverage: **incomplete / unavailable**; resolved records are a measured subset.

### Most frequently declared packages (up to 10)

| Ecosystem | Package | Declarations |
|---|---|---:|
| cargo | knx\-core | 14 |
| cargo | tempfile | 12 |
| cargo | serde\_json | 9 |
| cargo | tokio | 9 |
| cargo | chrono | 8 |
| cargo | serde | 8 |
| cargo | knx\-testsupport | 7 |
| cargo | sha2 | 7 |
| cargo | zip | 7 |
| cargo | knx\-store | 6 |
Other 58 package names omitted; full records remain in JSON.

### Multiple locked versions (up to 5; observed subset if coverage is incomplete)

- cargo / windows\-sys: 0\.45\.0, 0\.52\.0, 0\.59\.0; other versions omitted
- cargo / base64: 0\.21\.7, 0\.22\.1, 0\.23\.1
- cargo / getrandom: 0\.2\.17, 0\.3\.4, 0\.4\.3
- cargo / hashbrown: 0\.12\.3, 0\.16\.1, 0\.17\.1
- cargo / proc\-macro\-crate: 1\.3\.1, 2\.0\.2, 3\.5\.0

### Compact resolved tree

At most 3 roots, 10 direct branches, display depth 3 and 30 tree lines; repeated nodes are not expanded.

- Cargo\.lock:package:knx\-cli@0\.1\.0\-alpha\.19\#195
  - Cargo\.lock:package:chrono@0\.4\.45\#46
    - Cargo\.lock:package:iana\-time\-zone@0\.1\.65\#160
      - Cargo\.lock:package:android\_system\_properties@0\.1\.6\#4
      - Cargo\.lock:package:core\-foundation\-sys@0\.8\.7\#50
      - Cargo\.lock:package:iana\-time\-zone\-haiku@0\.1\.2\#161
    - Cargo\.lock:package:num\-traits@0\.2\.19\#244
      - Cargo\.lock:package:autocfg@1\.5\.1\#13
    - Cargo\.lock:package:serde@1\.0\.229\#335
      - Cargo\.lock:package:serde\_core@1\.0\.229\#337
      - Cargo\.lock:package:serde\_derive@1\.0\.229\#338
  - Cargo\.lock:package:knx\-app@0\.1\.0\-alpha\.3\#193
    - Cargo\.lock:package:base64@0\.22\.1\#17
    - Cargo\.lock:package:chrono@0\.4\.45\#46 (repeated/cyclic)
    - Cargo\.lock:package:getrandom@0\.4\.3\#132
      - Cargo\.lock:package:cfg\-if@1\.0\.4\#44
      - Cargo\.lock:package:js\-sys@0\.3\.104\#189
      - Cargo\.lock:package:libc@0\.2\.189\#215
  - Cargo\.lock:package:knx\-build\-stamp@0\.1\.0\-alpha\.1\#194
    - Cargo\.lock:package:tempfile@3\.27\.0\#391
      - Cargo\.lock:package:fastrand@2\.5\.0\#102
      - Cargo\.lock:package:getrandom@0\.4\.3\#132 (repeated/cyclic)
      - Cargo\.lock:package:once\_cell@1\.21\.4\#265
  - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#196
    - Cargo\.lock:package:chrono@0\.4\.45\#46 (repeated/cyclic)
  - Cargo\.lock:package:knx\-csv@0\.1\.0\-alpha\.1\#197
    - Cargo\.lock:package:csv@1\.4\.0\#60
      - Cargo\.lock:package:csv\-core@0\.1\.13\#61
      - Cargo\.lock:package:itoa@1\.0\.18\#177
      - Cargo\.lock:package:ryu@1\.0\.23\#325

Display is truncated where limits apply; full measured graph remains in JSON.

### Availability notes

- Resolved records are a measured subset; lock inventory incomplete\.
- apps/knx\-web/package\-lock\.json: lock coverage unavailable; missing/stale records or unsupported static requirements\.
- apps/knx\-web/package\-lock\.json: missing npm dependency edge; depth unavailable\.


## Changes Since the Previous Run

Previous run (UTC): 2026-11-01T12:55:25.543088+00:00

| Metric | Previous | Current | Change |
| :--- | ---: | ---: | ---: |
| Tracked files | N/A | N/A | N/A |
| Logical project bytes | N/A | N/A | N/A |
| Source files | N/A | N/A | N/A |
| Source bytes | N/A | N/A | N/A |
| Physical source lines | N/A | N/A | N/A |
| Tracked-file directories | N/A | N/A | N/A |
| Maximum directory depth | N/A | N/A | N/A |
| Unique declared dependencies | 68 | 68 | +0 |
| Resolved packages | N/A | N/A | N/A |
| Packages with multiple versions | N/A | N/A | N/A |
| Git commits | 4299 | 4307 | +8 |
| Logged tasks | 5677 | 5679 | +2 |
| Logged tool calls | 182401 | 182649 | +248 |
| Input tokens | 451390653 | 451798295 | +407642 |
| Output tokens | 161150379 | 161530869 | +380490 |
| Cache-read tokens | 42178309285 | 42313535671 | +135226386 |
| Cache-write tokens | 1057792484 | 1065015671 | +7223187 |

### Dependency Changes

- **Added declarations:** 0
- **Removed declarations:** 0
- **Changed requirements:** 0
- **Changed locked versions:** N/A — incomplete source coverage in either run


These are inventory differences between successful local runs, not interval billing, code authorship or an agent-efficiency score. Removed/moved session logs can reduce counts. The generated report and private state are excluded from file-size measurements. Missing measurements remain N/A; snapshots stay in Git-ignored private state.
