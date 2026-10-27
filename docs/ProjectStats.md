# Project Statistics

**Last update:** 2026-10-27 05:37:45 CET (UTC+01:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **97.713.157 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **81.427 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **666.2 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **33.160 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **195.426 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **9.8 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **40156.1 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **3.908.526 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **429.0 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **703.0 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **575.561.816 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **13.03x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **977.1 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **390.852 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **39.085 kilometers** (about **0.98x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **15.63 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **145.60 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **558.360.898 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **1.954.263.144 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **103.540 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **3.908 kWh** and **1289.8 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 4.030
- **Merges:** 626
- **Pushes/Sync:** 4.030
- **Lines Added (+):** 1.230.141
- **Lines Deleted (-):** 287.803

## Git Text Churn Distribution

| P50 changed text lines | P90 | P95 |
| ---: | ---: | ---: |
| 100 | 857 | 1.423 |

**3.343** measured commits;
**687** unavailable (for example, merge-only
history or binary/unsupported `--numstat` entries). A zero means Git explicitly
reported zero text-line changes; missing data is not turned into zero. This
counts historical additions + deletions per commit, **not AI-attributed** work
or surviving source lines. Branch lifetime cannot be inferred from this table.

## Coding Statistics (Current Working Tree)

**Git-tracked source files:** 2.011; **physical lines:** 640.063;
**code:** 547.537; **comments:** 55.732; **blank:** 36.794.
**Code/comment ratio:** 9.82:1; **comment share of nonblank lines:** 9.2%.
Tracked source paths unavailable in the working tree: **0**;
excluded generated/vendor/build source paths: **31**.

| Language | Files | Code lines | Comment lines | Blank lines |
| :--- | ---: | ---: | ---: | ---: |
| Rust | 1.015 | 402.343 | 43.039 | 24.793 |
| TSX | 318 | 59.886 | 5.099 | 4.964 |
| TypeScript | 526 | 58.752 | 6.443 | 4.191 |
| Python | 86 | 17.548 | 319 | 2.219 |
| CSS | 33 | 5.325 | 670 | 484 |
| JavaScript | 28 | 3.460 | 110 | 115 |
| Shell | 5 | 223 | 52 | 28 |

### Direct-source syntax structures

| Language | Parsed files | Functions/methods | Loops | Type declarations | Branch constructs |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Rust | 1.015/1.015 | 16.185 | 4.136 | 2.392 | 9.746 |
| TSX | 0/318 | N/A | N/A | N/A | N/A |
| TypeScript | 0/526 | N/A | N/A | N/A | N/A |
| Python | 86/86 | 1.193 | 664 | 114 | 1.308 |
| CSS | N/A (no parser) | N/A | N/A | N/A | N/A |
| JavaScript | 0/28 | N/A | N/A | N/A | N/A |
| Shell | N/A (no parser) | N/A | N/A | N/A | N/A |
| **Supported-source subtotal** | **1.101/1.973** | **N/A** | **N/A** | **N/A** | **N/A** |

Structure coverage is **1.101/1.973 supported source files**;
**38** source files have no function/type/loop structural parser (for example CSS/Shell).
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
comments/blanks; **11** nested
code-fence example lines are also classified as comments, not executable
source. Python docstrings remain code lines. Tests are included. Untracked
source, non-programming files, known generated/vendor/build directories and
binary files are excluded.
This is a **current checkout** inventory, not Git history, AI attribution or
proof that all lines execute. Line classification follows tokei's syntax rules.

### CSS syntax structures

| Parsed CSS files | Rule nodes | At-rule nodes | Declaration nodes |
| :--- | ---: | ---: | ---: |
| 0/33 | N/A | N/A | N/A |

Optional PostCSS 8 parser: `unavailable`.
Unavailable CSS files: **33**. Partial coverage
makes all CSS totals **N/A**, not zero or a successful-file subtotal. Rule nodes
include nested selectors and keyframe steps; declarations include custom
properties. Comments and string contents are not nodes of those types. These
are syntax counts, **not compiler/browser validation**, computed without
plugins or source-map/resource loading. CSS counts are a separate metric family,
not functions, loops or type declarations. Shell has no admitted AST parser.

## Session Time & Execution Analysis
| Provider | Reasoning-adjacent gap estimate | Execution interval proxy | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 157h 43m 41s | 182h 25m 54s | 4.136 |
| **Codex** | N/A | 127h 20m 20s | 1.282 |
| **Other** | N/A | N/A | 20 |
| **TOTAL** | **157h 43m 41s** | **309h 46m 14s** | **5.438** |

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
| 457 | 9m 11s | 31m 12s | 45m 13s | 271h 5m 10s |

**457** measured Hermes sessions;
**324** unavailable (open sessions,
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
| **Claude** | 6.865.370 | 87.388.317 | 21.620.878.508 | 975.833.130 |
| **Codex** | 376.021.642 | 53.311.784 | 15.111.516.288 | 0 |
| **Other** | 9.663.585 | 3.190.353 | 840.593.920 | 0 |
| **TOTAL** | **392.550.597** | **143.890.454** | **37.572.988.716** | **975.833.130** |

### 🚀 GRAND TOTAL CONSUMPTION
**39.085.262.897 Total Tokens**

## Weekly Usage Trend

| Day (UTC) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| Monday | 7.114.450.839 | 24.159.068 | 642 |
| Tuesday | 3.921.081.331 | 11.958.500 | 545 |
| Wednesday | 3.450.171.848 | 12.427.754 | 264 |
| Thursday | 5.346.962.794 | 20.577.977 | 689 |
| Friday | 4.183.681.689 | 17.446.226 | 469 |
| Saturday | 7.298.501.163 | 24.793.045 | 1.250 |
| Sunday | 7.770.413.233 | 32.527.884 | 1.501 |

Dated: **39.085.262.897 tokens / 5.360 tasks**;
unallocated: **0 tokens / 78 tasks**.
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
| Hermes Agent (direct) | 15.783.219.845 | 177.548.506 | 46.346.879 | 15.207.767.297 | 0s | 613 | 64.864 |
| Claude Code | 13.010.773.739 | 300.028 | 53.700.724 | 12.473.286.738 | 182h 25m 54s | 3.946 | 73.471 |
| Hermes Agent via Headroom | 9.042.664.931 | 169.165.039 | 38.811.589 | 8.693.898.585 | 0s | 168 | 19.188 |
| Codex CLI | 1.248.604.382 | 45.537.024 | 5.031.262 | 1.198.036.096 | 127h 20m 20s | 711 | 840 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Execution interval proxy | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| GPT-6.1 Sol (`gpt-6.1-sol`) | 9.929.553.221 | 260.508.815 | 36.534.838 | 9.632.509.568 | 10h 31m 54s | 496 | 45.316 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 9.747.682.213 | 4.176.677 | 34.246.145 | 9.229.447.444 | 3h 18m 13s | 199 | 23.843 |
| Claude Sonnet 5 (`claude-sonnet-5`) | 8.638.737.325 | 162.531 | 28.854.876 | 8.318.199.041 | 72h 28m 43s | 1.317 | 44.974 |
| Claude Opus 5 (`claude-opus-5`) | 3.767.184.218 | 100.370 | 20.790.313 | 3.568.644.548 | 100h 14m 31s | 1.298 | 22.640 |
| gpt-6.1-sol-900k | 2.257.754.690 | 40.287.780 | 6.901.534 | 2.210.565.376 | 0s | 58 | 10.014 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 1.500.242.387 | 36.794.050 | 4.437.009 | 1.459.011.328 | 82h 40m 21s | 244 | 894 |
| gpt-6-sol-900k | 1.217.683.037 | 13.755.988 | 2.635.145 | 1.201.291.904 | 0s | 3 | 624 |
| DeepSeek V4 Pro (`deepseek-v4-pro`) | 853.447.858 | 9.663.585 | 3.190.353 | 840.593.920 | 0s | 20 | 3.338 |
| Claude Sonnet 5.5 (`claude-sonnet-5-5`) | 319.136.547 | 4.116 | 1.796.186 | 307.155.247 | 2h 40m 59s | 16 | 1.931 |
| GPT-6 Sol (`gpt-6-sol`) | 207.689.703 | 2.367.661 | 372.410 | 204.949.632 | 56m 37s | 5 | 125 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 183.022.732 | 13.937.809 | 2.164.987 | 166.919.936 | 3h 0m 5s | 55 | 2.293 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 153.728.019 | 38.190 | 330.177 | 140.766.004 | 2h 37m 49s | 11 | 1.781 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 136.567.795 | 6.447.704 | 685.851 | 129.434.240 | 20h 9m 9s | 174 | 87 |
| GPT-6 Astra (`gpt-6-astra`) | 111.635.050 | 4.288.522 | 512.224 | 106.834.304 | 9h 57m 13s | 21 | 183 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| Claude Haiku 5.5 (`claude-haiku-5-5`) | 93.611 | 12 | 5.491 | 24.320 | 12s | 0 | 6 |
| <synthetic> | 0 | 0 | 0 | 0 | 33m 35s | 113 | 0 |
| Claude Sonnet 4.6 (`claude-sonnet-4-6`) | 0 | 0 | 0 | 0 | 0s | 1 | 0 |
| GPT-5.5 (`gpt-5.5`) | 0 | 0 | 0 | 0 | 2s | 1 | 0 |
| unknown | 0 | 0 | 0 | 0 | 4m 56s | 1.400 | 0 |

Model IDs are discovered directly from Claude, Codex and Hermes usage records;
display names are loaded dynamically from every available Hermes model cache.
There is no fixed model allowlist, so future persisted model IDs appear automatically
and unknown IDs remain visible verbatim instead of being discarded.

## Model & Cache Trend

| Relative week | Model | Tokens | Cache read | Cache-read share |
| :--- | :--- | ---: | ---: | ---: |
| Latest observed week | GPT-6.1 Sol (`gpt-6.1-sol`) | 363.923.131 | 357.163.520 | 98.3% |
| 1 week earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 4.177.581.004 | 4.055.693.824 | 97.4% |
| 2 weeks earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 2.440.227.752 | 2.383.230.336 | 97.9% |
| 3 weeks earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 1.356.571.205 | 1.306.139.648 | 96.9% |
| 4 weeks earlier | GPT-6.1 Sol (`gpt-6.1-sol`) | 1.591.250.129 | 1.530.282.240 | 96.8% |
| Latest observed week | Claude Opus 5.5 (`claude-opus-5-5`) | 427.278.507 | 409.741.335 | 96.3% |
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
**39.085.262.897 tokens**; **0 tokens**
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
| high | 35.971.138.443 | 344.390.391 | 131.884.992 | 34.595.674.404 | 206h 56m 25s | 3.281 | 144.225 |
| medium | 2.228.346.549 | 44.981.463 | 7.353.018 | 2.141.972.074 | 75h 22m 51s | 453 | 8.488 |
| xhigh | 712.876.041 | 2.212.187 | 4.237.172 | 676.415.850 | 22h 57m 36s | 277 | 3.657 |
| unknown | 171.765.857 | 891.329 | 402.684 | 157.878.196 | 4h 9m 10s | 1.427 | 1.991 |
| max | 1.136.007 | 75.227 | 12.588 | 1.048.192 | 19m 17s | 0 | 2 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | read_file | 77.262 |
| 2 | terminal | 56.260 |
| 3 | Bash | 53.919 |
| 4 | execute_code | 28.788 |
| 5 | patch | 17.299 |
| 6 | skill_view | 15.265 |
| 7 | search_files | 12.409 |
| 8 | Read | 9.987 |
| 9 | tool_call | 7.569 |
| 10 | write_file | 6.628 |
| 11 | Edit | 5.320 |
| 12 | vision_analyze | 1.873 |
| 13 | tool_describe | 1.135 |
| 14 | Write | 959 |
| 15 | Agent | 886 |
| 16 | delegate_task | 849 |
| 17 | Grep | 619 |
| 18 | ToolSearch | 564 |
| 19 | skill_manage | 532 |
| 20 | Monitor | 410 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | repository-delivery | 1.491 |
| 2 | compressed-tool-output-recovery | 1.397 |
| 3 | autonomous-goal-boundaries | 1.339 |
| 4 | test-driven-development | 1.291 |
| 5 | cross-layer-contract-verification | 1.012 |
| 6 | playwright | 812 |
| 7 | frontend-design | 756 |
| 8 | requesting-code-review | 720 |
| 9 | systematic-debugging | 714 |
| 10 | private-corpus-regression-testing | 668 |
| 11 | data-integrity-feature-verification | 499 |
| 12 | project-completion-auditing | 494 |
| 13 | safety-confirmation-gates | 481 |
| 14 | delegated-code-verification | 349 |
| 15 | grounded-citations | 327 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 31,773.0 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.27:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 94.8 | AI output ÷ added and deleted Git lines |
| Git commit density | 9,698,576.4 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $11,477.06 | 95.7% | $58,376.37 |
| Codex | $5,517.61 | 97.6% | $34,000.91 |
| Other | N/A | N/A | N/A |
| **PRICED SUBTOTAL** | **$16,994.67** | **96.5%** | **$92,377.28** |

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

- **Prime hour:** 16:00–17:00 local time (398 logged activity events)
- **Prime weekday:** Sunday (1.501 logged activity events)
- **Generation throughput:** N/A — output tokens and measured generation seconds are not joined per event across all sources
- **Autonomy input/output ratio:** 2.73:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 29.12 logged tool calls per task (158.363 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 615 | 11.5% |
| Morning (06-11) | 1.538 | 28.7% |
| Afternoon (12-17) | 1.811 | 33.8% |
| Evening (18-23) | 1.396 | 26.0% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 691 | 12.9% |
| Tuesday | 556 | 10.4% |
| Wednesday | 259 | 4.8% |
| Thursday | 667 | 12.4% |
| Friday | 479 | 8.9% |
| Saturday | 1.207 | 22.5% |
| Sunday | 1.501 | 28.0% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 3,908.5 kWh
- **Estimated CO₂ equivalent:** 1,289.8 kg CO₂e
- **Estimated physical keystrokes avoided:** 575.561.816

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.


## Project Size & File Inventory

Scope: **partial — measured subset only**, no tracked index/stat changes detected; HEAD revision: <code>eabea810cf95d5bb2e45b8662a0ee55b284a8f43</code>.

Git index/stat metadata only; conservative, without content comparisons. Secret/cache/report exclusions and submodule working contents are not assessed.

Logical bytes are file lengths, not disk allocation. Source suffixes are counted independently of categories; tests/docs take category precedence. Source lines are streaming physical UTF-8 text lines, including comments and blanks, not parser/tokei code counts.

Directories are unique parents of measured tracked files, including the root at depth zero. File depth means its parent directory depth. Name/path lengths count Unicode characters (not graphemes) separately from filesystem-encoded bytes, excluding the absolute root prefix.

| Metric | Value |
| --- | --- |
| Files (measured) | 3,309 |
| Logical bytes (measured) | 80,638,221 |
| Source files | 2,002 |
| Source logical bytes | 27,198,662 |
| Source physical lines | 639,257 |
| Tracked directories (including root) | 280 |
| Maximum directory depth | 8 |

| Category | Files | Logical bytes |
| --- | --- | --- |
| source | 1,185 | 17,206,312 |
| tests | 819 | 13,183,357 |
| docs | 1,115 | 36,133,895 |
| config | 97 | 1,530,943 |
| assets | 31 | 3,371,820 |
| other | 62 | 9,211,894 |

Readable UTF-8 source files: **2,002**; median file lines: **160.5**; p95 file lines: **994** (nearest-rank). Unreadable source files are excluded from distributions; their aggregate source lines are N/A.

### Extension distribution (top 10 by bytes)

| Extension | Files | Logical bytes |
| --- | --- | --- |
| <code>.rs</code> | 1,007 | 17,934,921 |
| <code>.png</code> | 95 | 13,239,652 |
| <code>.gif</code> | 5 | 11,482,795 |
| <code>.md</code> | 853 | 9,203,621 |
| <code>.sqlite</code> | 11 | 6,905,856 |
| <code>.ts</code> | 525 | 4,011,508 |
| <code>.tsx</code> | 318 | 3,630,789 |
| <code>.json</code> | 210 | 3,323,178 |
| <code>.knxdb</code> | 3 | 3,059,712 |
| <code>.mp4</code> | 3 | 2,147,882 |

### File and path extremes

| Extreme | Path/name | Measurement |
| --- | --- | --- |
| Longest filename | <code>docs/research/commissioning/2026-09-29-download-coverage-re-measured-signed-and-text-values-honest-mask.md</code> | 78 Unicode characters; 78 filesystem-encoded bytes |
| Longest relative path | <code>crates/knx-app/tests/it/system2&#95;validation&#95;preparation/configured&#95;hardware/pid7&#95;policy/pid7&#95;failure&#95;diagnostic.rs</code> | 113 Unicode characters; 113 filesystem-encoded bytes |
| Deepest file | <code>crates/knx-net/src/commissioning/memory&#95;download/tests/region&#95;reads/budget/note.rs</code> | Parent depth 8 |
| Shortest nonempty readable source | <code>apps/knx-web/src/vite-env.d.ts</code> | 1 physical lines; 38 bytes |
| Longest readable source line | <code>tools/design-studies/group-addresses/style.css</code> | Line 4; 14,666 Unicode characters (without newline) |
| Most common basename (case-sensitive) | <code>mod.rs</code> | 21 files |

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
| <code>apps/knx-server/src/domain.rs</code> | 367,917 | 9,087 |
| <code>crates/knx-net/src/commissioning.rs</code> | 275,342 | 6,418 |
| <code>crates/knx-core/src/dpt/codec.rs</code> | 238,140 | 5,961 |
| <code>apps/knx-cli/src/main.rs</code> | 208,683 | 5,370 |
| <code>crates/knx-productdb/src/migration.rs</code> | 223,139 | 5,119 |
| <code>apps/knx-web/src/styles.css</code> | 205,419 | 4,943 |
| <code>apps/knx-web/src/App.test.tsx</code> | 218,728 | 4,651 |
| <code>apps/knx-web/src/messages/en.ts</code> | 349,188 | 4,462 |
| <code>crates/knx-productdb/tests/it/dynamic&#95;tree.rs</code> | 177,414 | 4,458 |

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
- Excluded ignored: 431.
- Excluded report or state: 1.
- Excluded secret or credential: 11.
- Excluded symlink: 1.
- Unavailable tracked working files: 1; unreadable source files: 0.
- Excluded tracked files&#58; generated or cache=31, ignored=431, report or state=1, secret or credential=11, symlink=1.
- Partial inventory&#58; unavailable working files are omitted; unreadable source lines are N/A.


## Package Dependencies

Offline, Git-tracked declarations and lockfiles only; no package code is executed.

| Metric | Value |
|---|---:|
| Unique declared dependencies | 67 |
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
Other 57 package names omitted; full records remain in JSON.

### Multiple locked versions (up to 5; observed subset if coverage is incomplete)

- cargo / windows\-sys: 0\.45\.0, 0\.52\.0, 0\.59\.0; other versions omitted
- cargo / base64: 0\.21\.7, 0\.22\.1, 0\.23\.1
- cargo / getrandom: 0\.2\.17, 0\.3\.4, 0\.4\.3
- cargo / hashbrown: 0\.12\.3, 0\.16\.1, 0\.17\.1
- cargo / proc\-macro\-crate: 1\.3\.1, 2\.0\.2, 3\.5\.0

### Compact resolved tree

At most 3 roots, 10 direct branches, display depth 3 and 30 tree lines; repeated nodes are not expanded.

- Cargo\.lock:package:knx\-cli@0\.1\.0\-alpha\.17\#194
  - Cargo\.lock:package:chrono@0\.4\.45\#45
    - Cargo\.lock:package:iana\-time\-zone@0\.1\.65\#159
      - Cargo\.lock:package:android\_system\_properties@0\.1\.6\#4
      - Cargo\.lock:package:core\-foundation\-sys@0\.8\.7\#49
      - Cargo\.lock:package:iana\-time\-zone\-haiku@0\.1\.2\#160
    - Cargo\.lock:package:num\-traits@0\.2\.19\#242
      - Cargo\.lock:package:autocfg@1\.5\.1\#13
    - Cargo\.lock:package:serde@1\.0\.229\#333
      - Cargo\.lock:package:serde\_core@1\.0\.229\#335
      - Cargo\.lock:package:serde\_derive@1\.0\.229\#336
  - Cargo\.lock:package:knx\-app@0\.1\.0\-alpha\.3\#192
    - Cargo\.lock:package:base64@0\.22\.1\#17
    - Cargo\.lock:package:chrono@0\.4\.45\#45 (repeated/cyclic)
    - Cargo\.lock:package:getrandom@0\.4\.3\#131
      - Cargo\.lock:package:cfg\-if@1\.0\.4\#44
      - Cargo\.lock:package:libc@0\.2\.189\#214
      - Cargo\.lock:package:r\-efi@6\.0\.0\#298
  - Cargo\.lock:package:knx\-build\-stamp@0\.1\.0\-alpha\.1\#193
    - Cargo\.lock:package:tempfile@3\.27\.0\#389
      - Cargo\.lock:package:fastrand@2\.5\.0\#101
      - Cargo\.lock:package:getrandom@0\.4\.3\#131 (repeated/cyclic)
      - Cargo\.lock:package:once\_cell@1\.21\.4\#263
  - Cargo\.lock:package:knx\-core@0\.1\.0\-alpha\.1\#195
    - Cargo\.lock:package:chrono@0\.4\.45\#45 (repeated/cyclic)
  - Cargo\.lock:package:knx\-csv@0\.1\.0\-alpha\.1\#196
    - Cargo\.lock:package:csv@1\.4\.0\#59
      - Cargo\.lock:package:csv\-core@0\.1\.13\#60
      - Cargo\.lock:package:itoa@1\.0\.18\#176
      - Cargo\.lock:package:ryu@1\.0\.23\#323

Display is truncated where limits apply; full measured graph remains in JSON.

### Availability notes

- Resolved records are a measured subset; lock inventory incomplete\.
- apps/knx\-web/package\-lock\.json: lock coverage unavailable; missing/stale records or unsupported static requirements\.
- apps/knx\-web/package\-lock\.json: missing npm dependency edge; depth unavailable\.


## Changes Since the Previous Run

Baseline recorded; no previous compatible snapshot exists. Changes are N/A, not zero.

These are inventory differences between successful local runs, not interval billing, code authorship or an agent-efficiency score. Removed/moved session logs can reduce counts. The generated report and private state are excluded from file-size measurements. Missing measurements remain N/A; snapshots stay in Git-ignored private state.
