# Project Statistics

**Last update:** 2026-09-29 17:48:21 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **47.324.619 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **39.437 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **322.7 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **16.060 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **94.649 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **4.7 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **19448.5 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.892.984 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **207.8 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **340.5 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **282.717.504 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.31x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **473.2 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **189.298 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **18.929 kilometers** (about **0.47x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **7.57 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 At an assumed 4 bytes per token, this is **70.52 GiB of text equivalent**, not measured network traffic.
- 🐦 At 280 characters each, the generated and processed text would fill about **270.426.399 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **946.492.399 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **50.146 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.892 kWh** and **624.7 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 1.648
- **Merges:** 181
- **Pushes/Sync:** 1.647
- **Lines Added (+):** 453.273
- **Lines Deleted (-):** 89.740

## Session Time & Execution Analysis
| Provider | Thinking / Reasoning Time | Generation Time | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 196h 38m 16s | 4.327 |
| **Codex** | N/A | 171h 36m 49s | 496 |
| **TOTAL** | **164h 39m 19s** | **368h 15m 5s** | **4.823** |

`N/A` means the source logs do not provide a meaningful measurable duration;
it does not mean the model achieved enlightenment instantaneously. Idle/wait time
is omitted because the available providers do not record it consistently enough
for a useful comparison.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 977.422 | 62.986.089 | 14.733.424.809 | 673.804.311 |
| **Codex** | 1.014.223.503 | 7.693.287 | 2.436.738.560 | 0 |
| **TOTAL** | **1.015.200.925** | **70.679.376** | **17.170.163.369** | **673.804.311** |

### 🚀 GRAND TOTAL CONSUMPTION
**18.929.847.981 Total Tokens**

## Weekly Usage Trend

| Week starting (UTC Monday) | Tokens | Output tokens | Tasks |
| :--- | ---: | ---: | ---: |
| 2026-09-28 | 552.422.383 | 2.105.734 | 14 |
| 2026-09-21 | 3.720.425.807 | 13.026.524 | 381 |
| 2026-09-14 | 4.367.001.624 | 17.583.909 | 1.222 |
| 2026-09-07 | 6.570.282.534 | 28.442.776 | 1.935 |
| 2026-08-31 | 3.695.416.238 | 9.392.251 | 1.271 |

Dated: **18.905.548.586 tokens / 4.823 tasks**;
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

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Hermes Agent via Headroom | 2.717.727.305 | 24.595.997 | 11.307.955 | 2.516.869.473 | 48h 2m 23s | 68 | 4.310 |
| Codex CLI | 1.945.039.227 | 988.269.387 | 3.856.816 | 952.913.024 | 99h 32m 10s | 443 | 730 |
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
| GPT-5.6 Sol (`gpt-5.6-sol`) | 2.260.892.486 | 822.049.580 | 4.288.410 | 1.434.554.496 | 119h 40m 17s | 226 | 876 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 1.097.328.041 | 421.422 | 7.226.818 | 949.860.799 | 5h 3m 56s | 21 | 1.394 |
| gpt-6-sol-900k | 498.568.303 | 5.960.595 | 996.700 | 491.611.008 | 0s | 3 | 617 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 219.552.850 | 53.107.771 | 1.216.535 | 165.228.544 | 6h 20m 37s | 46 | 2.293 |
| GPT-6 Sol (`gpt-6-sol`) | 181.395.561 | 2.025.718 | 283.891 | 179.085.952 | 0s | 1 | 39 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| GPT-6 Astra (`gpt-6-astra`) | 163.031.658 | 61.760.130 | 474.216 | 100.797.312 | 37h 25m 50s | 14 | 169 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 135.214.492 | 69.319.709 | 433.535 | 65.461.248 | 8h 6m 6s | 32 | 33 |
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
| high | 15.236.395.734 | 374.431.308 | 56.930.214 | 14.274.242.871 | 203h 21m 49s | 2.834 | 69.623 |
| medium | 2.157.174.932 | 637.860.735 | 7.318.761 | 1.425.732.739 | 135h 15m 48s | 249 | 3.683 |
| xhigh | 1.303.655.495 | 1.137.024 | 5.622.789 | 1.256.961.469 | 25h 6m 59s | 318 | 4.582 |
| unknown | 216.504.459 | 644.706 | 518.043 | 199.148.789 | 3h 58m 0s | 1.418 | 2.506 |
| max | 16.117.361 | 1.127.152 | 289.569 | 14.077.501 | 31m 33s | 4 | 111 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | Bash | 55.117 |
| 2 | terminal | 10.674 |
| 3 | Read | 10.084 |
| 4 | read_file | 8.571 |
| 5 | Edit | 6.008 |
| 6 | patch | 2.785 |
| 7 | execute_code | 2.745 |
| 8 | search_files | 2.554 |
| 9 | Write | 1.169 |
| 10 | tool_call | 1.124 |
| 11 | Agent | 1.001 |
| 12 | ToolSearch | 582 |
| 13 | write_file | 526 |
| 14 | skill_view | 522 |
| 15 | Monitor | 372 |
| 16 | wait_agent | 368 |
| 17 | SendMessage | 194 |
| 18 | Skill | 189 |
| 19 | TaskStop | 167 |
| 20 | delegate_task | 147 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | compressed-tool-output-recovery | 42 |
| 2 | knx-live-bus-operations | 42 |
| 3 | hermes-agent | 38 |
| 4 | test-driven-development | 38 |
| 5 | superpowers:subagent-driven-development | 35 |
| 6 | superpowers:finishing-a-development-branch | 31 |
| 7 | superpowers:brainstorming | 28 |
| 8 | repository-delivery | 27 |
| 9 | autonomous-goal-boundaries | 26 |
| 10 | superpowers:writing-plans | 24 |
| 11 | requesting-code-review | 23 |
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
| Tokens per committed added line | 41,762.6 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 5.05:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 130.2 | AI output ÷ added and deleted Git lines |
| Git commit density | 11,486,558.2 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $7,894.52 | 95.6% | $39,780.25 |
| Codex | $3,260.14 | 70.6% | $5,482.66 |
| **TOTAL** | **$11,154.66** | **91.0%** | **$45,262.91** |

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
- **Session velocity:** 3,198.9 output tokens per active generation minute
- **Autonomy input/output ratio:** 14.36:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 16.69 logged tool calls per task (80.505 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 545 | 11.3% |
| Morning (06-11) | 1.467 | 30.4% |
| Afternoon (12-17) | 1.629 | 33.8% |
| Evening (18-23) | 1.182 | 24.5% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 526 | 10.9% |
| Tuesday | 300 | 6.2% |
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

- **Estimated inference energy:** 1,893.0 kWh
- **Estimated CO₂ equivalent:** 624.7 kg CO₂e
- **Estimated physical keystrokes avoided:** 282.717.504

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.
