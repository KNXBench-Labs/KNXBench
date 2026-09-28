# Project Statistics

**Last update:** 2026-09-28 18:27:54 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **47.214.542 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **39.345 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **321.9 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **16.023 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **94.429 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **4.7 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **19403.2 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.888.581 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **207.3 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **339.7 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **257.260.864 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.30x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **472.1 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **188.858 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **18.885 kilometers** (about **0.47x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **7.55 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 In plain text ASCII, this represents **70.36 GB of raw source text data**.
- 🐦 At 280 characters each, the generated and processed text would fill about **269.797.383 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **944.290.843 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **50.030 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.888 kWh** and **623.2 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 1.554
- **Merges:** 173
- **Pushes/Sync:** 1.554
- **Lines Added (+):** 422.512
- **Lines Deleted (-):** 88.470

## Session Time & Execution Analysis
| Provider | Thinking / Reasoning Time | Generation Time | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 190h 42m 40s | 4.302 |
| **Codex** | N/A | 174h 7m 31s | 562 |
| **TOTAL** | **164h 39m 19s** | **364h 50m 11s** | **4.864** |

`N/A` means the source logs do not provide a meaningful measurable duration;
it does not mean the model achieved enlightenment instantaneously. Idle/wait time
is omitted because the available providers do not record it consistently enough
for a useful comparison.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 511.880 | 55.189.032 | 13.619.391.516 | 508.433.865 |
| **Codex** | 1.922.971.248 | 9.126.184 | 2.770.193.152 | 0 |
| **TOTAL** | **1.923.483.128** | **64.315.216** | **16.389.584.668** | **508.433.865** |

### 🚀 GRAND TOTAL CONSUMPTION
**18.885.816.877 Total Tokens**

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Codex CLI | 3.752.612.183 | 1.905.611.498 | 6.741.229 | 1.840.259.456 | 102h 2m 51s | 511 | 783 |
| Hermes Agent via Headroom | 890.422.640 | 15.543.876 | 2.187.564 | 872.691.200 | 42h 54m 44s | 46 | 2.520 |
| Hermes Agent (direct) | 59.255.761 | 1.815.874 | 197.391 | 57.242.496 | 29h 9m 54s | 5 | 301 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Sonnet 5 (`claude-sonnet-5`) | 10.194.076.112 | 355.925 | 33.082.380 | 9.840.530.452 | 84h 20m 12s | 1.589 | 49.274 |
| Claude Opus 5 (`claude-opus-5`) | 3.747.154.908 | 92.740 | 21.315.347 | 3.557.209.551 | 101h 51m 24s | 1.338 | 23.265 |
| GPT-5.6 Sol (`gpt-5.6-sol`) | 3.399.087.540 | 1.413.810.461 | 5.664.791 | 1.979.612.288 | 108h 36m 35s | 176 | 1.005 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 533.865.343 | 272.844.357 | 1.368.890 | 259.652.096 | 20h 4m 2s | 174 | 87 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 314.247.192 | 100.759.604 | 989.412 | 212.498.176 | 6h 36m 50s | 55 | 2.293 |
| GPT-6 Astra (`gpt-6-astra`) | 304.475.205 | 133.938.868 | 876.433 | 169.659.904 | 38h 47m 3s | 21 | 180 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| GPT-6 Sol (`gpt-6-sol`) | 150.615.304 | 1.617.958 | 226.658 | 148.770.688 | 0s | 1 | 39 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| <synthetic> | 0 | 0 | 0 | 0 | 57m 51s | 138 | 0 |
| unknown | 0 | 0 | 0 | 0 | 2m 58s | 1.354 | 0 |

Model IDs are discovered directly from Claude, Codex and Hermes usage records;
display names are loaded dynamically from every available Hermes model cache.
There is no fixed model allowlist, so future persisted model IDs appear automatically
and unknown IDs remain visible verbatim instead of being discarded.

### Latest Models Discovered in Hermes

| Model | Model ID | Released | Project usage status |
| :--- | :--- | :--- | :--- |
| GPT-6 Sol (EU) | `gpt-6-sol@eu` | 2026-09-22 | available; no selected-project usage |
| GPT-6 Sol | `gpt-6-sol` | 2026-09-22 | used in selected projects |
| GPT-6 Luna (EU) | `gpt-6-luna@eu` | 2026-09-22 | available; no selected-project usage |
| GPT-6 Luna | `gpt-6-luna` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 | `claude-opus-5.5` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 (EU) | `claude-opus-5-5@eu` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 | `claude-opus-5-5@default` | 2026-09-22 | available; no selected-project usage |
| Claude Opus 5.5 | `claude-opus-5-5` | 2026-09-22 | available; no selected-project usage |
| GPT Image 2.5 Sunburst | `gpt-image-2.5-sunburst` | 2026-09-08 | available; no selected-project usage |
| GPT Image 2.5 Flare | `gpt-image-2.5-flare` | 2026-09-08 | available; no selected-project usage |
| GPT-6 Astra | `gpt-6-astra` | 2026-09-04 | used in selected projects |
| Claude Fable 5.1 (EU) | `claude-fable-5.1@eu` | 2026-09-01 | available; no selected-project usage |
| Claude Fable 5.1 | `claude-fable-5.1` | 2026-09-01 | available; no selected-project usage |
| Claude Fable 5.1 | `claude-fable-5-1@default` | 2026-09-01 | available; no selected-project usage |
| Claude Fable 5.1 | `claude-fable-5-1` | 2026-09-01 | used in selected projects |
| Daybreak Red | `gpt-daybreak-red-latest` | 2026-08-07 | available; no selected-project usage |
| Daybreak Blue | `gpt-daybreak-blue-latest` | 2026-08-07 | available; no selected-project usage |
| Claude Opus 5 (EU) | `claude-opus-5@eu` | 2026-07-24 | available; no selected-project usage |
| Claude Opus 5 | `claude-opus-5@default` | 2026-07-24 | available; no selected-project usage |
| claude-opus-5-thinking | `claude-opus-5-thinking` | 2026-07-24 | available; no selected-project usage |

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
| high | 14.442.419.652 | 525.187.882 | 53.314.691 | 13.411.816.921 | 189h 51m 26s | 2.782 | 68.440 |
| medium | 2.899.763.510 | 1.379.291.917 | 4.598.953 | 1.515.872.640 | 144h 45m 26s | 380 | 3.116 |
| xhigh | 1.305.466.851 | 2.075.428 | 5.627.517 | 1.257.829.693 | 25h 6m 59s | 318 | 4.582 |
| unknown | 219.865.304 | 14.677.330 | 471.898 | 188.939.721 | 4h 33m 52s | 1.380 | 2.342 |
| max | 18.301.560 | 2.250.571 | 302.157 | 15.125.693 | 31m 33s | 4 | 111 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | Bash | 54.960 |
| 2 | Read | 10.081 |
| 3 | Edit | 6.004 |
| 4 | read_file | 1.270 |
| 5 | Write | 1.165 |
| 6 | Agent | 1.001 |
| 7 | terminal | 838 |
| 8 | search_files | 767 |
| 9 | ToolSearch | 578 |
| 10 | Monitor | 371 |
| 11 | wait_agent | 364 |
| 12 | patch | 351 |
| 13 | SendMessage | 194 |
| 14 | Skill | 189 |
| 15 | TaskStop | 167 |
| 16 | skill_view | 155 |
| 17 | send_message | 141 |
| 18 | tool_call | 104 |
| 19 | ListAgents | 98 |
| 20 | spawn_agent | 87 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | superpowers:subagent-driven-development | 35 |
| 2 | superpowers:finishing-a-development-branch | 31 |
| 3 | superpowers:brainstorming | 28 |
| 4 | superpowers:writing-plans | 24 |
| 5 | superpowers:using-git-worktrees | 19 |
| 6 | hermes-agent | 17 |
| 7 | knx-spec | 16 |
| 8 | software-development/codebase-inspection | 15 |
| 9 | software-development:codebase-inspection | 11 |
| 10 | software-development/private-corpus-regression-testing | 10 |
| 11 | software-development/delegated-code-verification | 8 |
| 12 | software-development/test-driven-development | 7 |
| 13 | requesting-code-review | 6 |
| 14 | run | 6 |
| 15 | update-config | 6 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 44,698.9 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.78:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 125.9 | AI output ÷ added and deleted Git lines |
| Git commit density | 12,153,035.3 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $6,821.82 | 96.4% | $36,772.36 |
| Codex | $5,636.87 | 59.0% | $6,232.93 |
| **TOTAL** | **$12,458.68** | **87.1%** | **$43,005.29** |

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

- **Prime hour:** 16:00–17:00 local time (348 logged activity events)
- **Prime weekday:** Sunday (1.265 logged activity events)
- **Session velocity:** 2,938.1 output tokens per active generation minute
- **Autonomy input/output ratio:** 29.91:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 16.16 logged tool calls per task (78.591 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 526 | 10.8% |
| Morning (06-11) | 1.472 | 30.3% |
| Afternoon (12-17) | 1.682 | 34.6% |
| Evening (18-23) | 1.184 | 24.3% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 526 | 10.8% |
| Tuesday | 389 | 8.0% |
| Wednesday | 293 | 6.0% |
| Thursday | 742 | 15.3% |
| Friday | 575 | 11.8% |
| Saturday | 1.074 | 22.1% |
| Sunday | 1.265 | 26.0% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 1,888.6 kWh
- **Estimated CO₂ equivalent:** 623.2 kg CO₂e
- **Estimated physical keystrokes avoided:** 257.260.864

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.
