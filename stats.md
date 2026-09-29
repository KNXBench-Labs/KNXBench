# Project Statistics

**Last update:** 2026-09-29 15:41:02 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **51.729.942 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **43.108 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **352.7 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **17.555 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **103.459 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **5.2 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **21258.9 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **2.069.197 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **227.1 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **372.2 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **295.796.220 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.90x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **517.3 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **206.919 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **20.691 kilometers** (about **0.52x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **8.28 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 In plain text ASCII, this represents **77.08 GB of raw source text data**.
- 🐦 At 280 characters each, the generated and processed text would fill about **295.599.672 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **1.034.598.854 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **54.815 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **2.069 kWh** and **682.8 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 1.641
- **Merges:** 181
- **Pushes/Sync:** 1.640
- **Lines Added (+):** 452.243
- **Lines Deleted (-):** 89.566

## Session Time & Execution Analysis
| Provider | Thinking / Reasoning Time | Generation Time | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 196h 38m 16s | 4.327 |
| **Codex** | N/A | 171h 36m 49s | 495 |
| **TOTAL** | **164h 39m 19s** | **368h 15m 5s** | **4.822** |

`N/A` means the source logs do not provide a meaningful measurable duration;
it does not mean the model achieved enlightenment instantaneously. Idle/wait time
is omitted because the available providers do not record it consistently enough
for a useful comparison.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 943.378 | 62.706.774 | 14.690.805.858 | 668.330.006 |
| **Codex** | 1.983.196.034 | 11.242.281 | 3.274.752.768 | 0 |
| **TOTAL** | **1.984.139.412** | **73.949.055** | **17.965.558.626** | **668.330.006** |

### 🚀 GRAND TOTAL CONSUMPTION
**20.691.977.099 Total Tokens**

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Codex CLI | 3.855.221.595 | 1.958.995.152 | 7.649.291 | 1.888.577.152 | 99h 32m 10s | 443 | 730 |
| Hermes Agent via Headroom | 2.569.674.055 | 22.808.719 | 10.785.159 | 2.376.600.602 | 48h 2m 23s | 67 | 3.913 |
| Hermes Agent (direct) | 59.255.761 | 1.815.874 | 197.391 | 57.242.496 | 29h 9m 57s | 6 | 301 |
| Claude Code Cloud | 24.299.395 | 7.787 | 128.182 | 23.746.860 | 47m 53s | 4 | 177 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Sonnet 5 (`claude-sonnet-5`) | 10.199.499.647 | 356.049 | 33.161.909 | 9.842.273.676 | 85h 11m 49s | 1.592 | 49.371 |
| Claude Opus 5 (`claude-opus-5`) | 3.932.069.670 | 136.736 | 21.806.057 | 3.719.638.821 | 101h 51m 24s | 1.338 | 23.265 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 3.833.771.299 | 1.622.282.256 | 7.229.843 | 2.204.259.200 | 119h 40m 17s | 227 | 1.033 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 1.048.921.426 | 387.378 | 6.947.503 | 907.241.848 | 5h 3m 56s | 21 | 1.312 |
| gpt-6-sol-900k | 402.841.948 | 4.389.990 | 790.006 | 397.661.952 | 0s | 1 | 145 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 303.792.963 | 95.846.543 | 1.276.596 | 206.669.824 | 6h 20m 37s | 46 | 2.293 |
| GPT-6 Astra (`gpt-6-astra`) | 279.231.923 | 121.156.591 | 800.452 | 157.274.880 | 37h 25m 50s | 14 | 169 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 268.157.389 | 137.494.936 | 861.493 | 129.800.960 | 8h 6m 6s | 32 | 33 |
| GPT-6 Sol (`gpt-6-sol`) | 181.395.561 | 2.025.718 | 283.891 | 179.085.952 | 0s | 1 | 39 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| <synthetic> | 0 | 0 | 0 | 0 | 57m 51s | 138 | 0 |
| Claude Sonnet 4.6 (`claude-sonnet-4-6`) | 0 | 0 | 0 | 0 | 2s | 1 | 0 |
| unknown | 0 | 0 | 0 | 0 | 3m 57s | 1.393 | 0 |

Model IDs are discovered directly from Claude, Codex and Hermes usage records;
display names are loaded dynamically from every available Hermes model cache.
There is no fixed model allowlist, so future persisted model IDs appear automatically
and unknown IDs remain visible verbatim instead of being discarded.

### Latest Models Discovered in Hermes

| Model | Model ID | Released | Project usage status |
| :--- | :--- | :--- | :--- |
| Claude Sonnet 5.5 | `claude-sonnet-5.5` | 2026-09-28 | available; no selected-project usage |
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
| Claude Fable 5.1 | `claude-fable-5-1@default` | 2026-09-01 | available; no selected-project usage |
| Claude Fable 5.1 | `claude-fable-5-1` | 2026-09-01 | used in selected projects |
| Daybreak Red | `gpt-daybreak-red-latest` | 2026-08-07 | available; no selected-project usage |

This catalogue is generated from the newest 20 Claude/GPT entries in the local
Hermes model caches. **Available** is not reported as **used**: a cached model with
no attributable activity remains labelled `available; no selected-project usage`.
That makes newly published models visible without inventing token consumption.

## External AI Integrations

| Integration | Detection status | Attributable tool calls |
| :--- | :--- | ---: |
| Laya | configured; no attributable tool calls | 0 |

Laya is detected from Hermes configuration and executed Laya MCP tool names. Its
router-selected model work remains accounted under the concrete model recorded by
the resulting Claude/Codex/Hermes session; only explicit Laya MCP calls are counted
in the integration row, avoiding double-counted tokens.

## Effort Breakdown

| Effort | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| high | 15.830.239.851 | 726.114.915 | 58.690.212 | 14.514.643.383 | 203h 21m 49s | 2.833 | 69.308 |
| medium | 3.321.464.378 | 1.253.053.792 | 8.811.126 | 1.978.811.068 | 135h 15m 48s | 249 | 3.601 |
| xhigh | 1.305.466.851 | 2.075.428 | 5.627.517 | 1.257.829.693 | 25h 6m 59s | 318 | 4.582 |
| unknown | 216.504.459 | 644.706 | 518.043 | 199.148.789 | 3h 58m 0s | 1.418 | 2.506 |
| max | 18.301.560 | 2.250.571 | 302.157 | 15.125.693 | 31m 33s | 4 | 111 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | Bash | 55.117 |
| 2 | terminal | 10.134 |
| 3 | Read | 10.084 |
| 4 | read_file | 8.178 |
| 5 | Edit | 6.008 |
| 6 | execute_code | 2.666 |
| 7 | patch | 2.594 |
| 8 | search_files | 2.426 |
| 9 | Write | 1.169 |
| 10 | tool_call | 1.089 |
| 11 | Agent | 1.001 |
| 12 | ToolSearch | 582 |
| 13 | write_file | 512 |
| 14 | skill_view | 489 |
| 15 | Monitor | 372 |
| 16 | wait_agent | 368 |
| 17 | SendMessage | 194 |
| 18 | Skill | 189 |
| 19 | TaskStop | 167 |
| 20 | delegate_task | 147 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | knx-live-bus-operations | 41 |
| 2 | compressed-tool-output-recovery | 39 |
| 3 | hermes-agent | 38 |
| 4 | superpowers:subagent-driven-development | 35 |
| 5 | test-driven-development | 35 |
| 6 | superpowers:finishing-a-development-branch | 31 |
| 7 | superpowers:brainstorming | 28 |
| 8 | autonomous-goal-boundaries | 25 |
| 9 | superpowers:writing-plans | 24 |
| 10 | repository-delivery | 23 |
| 11 | requesting-code-review | 22 |
| 12 | superpowers:using-git-worktrees | 19 |
| 13 | codex | 18 |
| 14 | knx-spec | 16 |
| 15 | software-development/codebase-inspection | 15 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 45,754.1 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 5.05:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 136.5 | AI output ÷ added and deleted Git lines |
| Git commit density | 12,609,370.6 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $7,856.91 | 95.6% | $39,665.18 |
| Codex | $5,945.31 | 62.3% | $7,368.19 |
| **TOTAL** | **$13,802.22** | **87.1%** | **$47,033.37** |

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

- **Prime hour:** 16:00–17:00 local time (338 logged activity events)
- **Prime weekday:** Sunday (1.275 logged activity events)
- **Session velocity:** 3,346.9 output tokens per active generation minute
- **Autonomy input/output ratio:** 26.83:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 16.61 logged tool calls per task (80.108 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 545 | 11.3% |
| Morning (06-11) | 1.467 | 30.4% |
| Afternoon (12-17) | 1.628 | 33.8% |
| Evening (18-23) | 1.182 | 24.5% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 526 | 10.9% |
| Tuesday | 299 | 6.2% |
| Wednesday | 241 | 5.0% |
| Thursday | 825 | 17.1% |
| Friday | 581 | 12.0% |
| Saturday | 1.075 | 22.3% |
| Sunday | 1.275 | 26.4% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 2,069.2 kWh
- **Estimated CO₂ equivalent:** 682.8 kg CO₂e
- **Estimated physical keystrokes avoided:** 295.796.220

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.
