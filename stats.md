# Project Statistics

**Last update:** 2026-09-28 19:33:39 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
### 📚 Books, Pages & Literary Suffering
- 📚 **47.331.573 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **39.442 times**).
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **322.7 full sets**.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **16.062 complete Bards**.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **94.663 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **4.7 kilometers of bookshelf**.

### ⏱️ Human Time & Manual Effort
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **19451.3 human-years worth of typing**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.893.262 cups of espresso**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **207.8 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **340.5 reader-years**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **258.059.240 physical keystrokes** (backspace heroics not included).

### 🌍 Scale, Biology & Suspicious Liquids
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.31x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **473.3 grams of fertile dirt**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **189.326 ants** their own full context window.
- 🌌 If every token were 1 millimeter, the line would stretch **18.932 kilometers** (about **0.47x around Earth**).
- 💧 If every token were one microliter of water, they would fill about **7.57 Olympic swimming pools**.

### 💾 Messages & Retro Storage
- 💾 In plain text ASCII, this represents **70.53 GB of raw source text data**.
- 🐦 At 280 characters each, the generated and processed text would fill about **270.466.135 maximally packed posts**.
- 🗂️ At 80 characters per punched card, this would require **946.631.474 cards** and a warehouse-sized debugging session.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **50.154 disks** — please label them carefully.

### ⚡ Resources & Emissions
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.893 kWh** and **624.8 kg CO₂e**.

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 1.560
- **Merges:** 173
- **Pushes/Sync:** 1.559
- **Lines Added (+):** 424.075
- **Lines Deleted (-):** 88.626

## Session Time & Execution Analysis
| Provider | Thinking / Reasoning Time | Generation Time | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 191h 30m 33s | 4.306 |
| **Codex** | N/A | 174h 7m 31s | 562 |
| **TOTAL** | **164h 39m 19s** | **365h 38m 4s** | **4.868** |

`N/A` means the source logs do not provide a meaningful measurable duration;
it does not mean the model achieved enlightenment instantaneously. Idle/wait time
is omitted because the available providers do not record it consistently enough
for a useful comparison.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 519.667 | 55.317.214 | 13.643.138.376 | 508.850.431 |
| **Codex** | 1.923.519.797 | 9.197.596 | 2.792.086.400 | 0 |
| **TOTAL** | **1.924.039.464** | **64.514.810** | **16.435.224.776** | **508.850.431** |

### 🚀 GRAND TOTAL CONSUMPTION
**18.932.629.481 Total Tokens**

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Codex CLI | 3.752.612.183 | 1.905.611.498 | 6.741.229 | 1.840.259.456 | 102h 2m 51s | 511 | 783 |
| Hermes Agent via Headroom | 912.935.849 | 16.092.425 | 2.258.976 | 894.584.448 | 42h 54m 44s | 46 | 2.493 |
| Hermes Agent (direct) | 59.255.761 | 1.815.874 | 197.391 | 57.242.496 | 29h 9m 54s | 5 | 301 |
| Claude Code Cloud | 24.299.395 | 7.787 | 128.182 | 23.746.860 | 47m 53s | 4 | 177 |

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
| GPT-5.6 Sol (`gpt-5.6-sol`) | 3.421.470.629 | 1.414.247.616 | 5.717.477 | 2.001.505.536 | 108h 36m 35s | 176 | 978 |
| GPT-5.6 Terra (`gpt-5.6-terra`) | 533.865.343 | 272.844.357 | 1.368.890 | 259.652.096 | 20h 4m 2s | 174 | 87 |
| GPT-5.6 Luna (`gpt-5.6-luna`) | 314.377.312 | 100.870.998 | 1.008.138 | 212.498.176 | 6h 36m 50s | 55 | 2.293 |
| GPT-6 Astra (`gpt-6-astra`) | 304.475.205 | 133.938.868 | 876.433 | 169.659.904 | 38h 47m 3s | 21 | 180 |
| Claude Haiku 4.5 (`claude-haiku-4-5-20251001`) | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| GPT-6 Sol (`gpt-6-sol`) | 150.615.304 | 1.617.958 | 226.658 | 148.770.688 | 0s | 1 | 39 |
| Claude Fable 5.1 (`claude-fable-5-1`) | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| Claude Opus 5.5 (`claude-opus-5-5`) | 24.299.395 | 7.787 | 128.182 | 23.746.860 | 47m 53s | 4 | 177 |
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
| Claude Opus 5.5 | `claude-opus-5-5` | 2026-09-22 | used in selected projects |
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
| high | 14.464.379.639 | 525.291.765 | 53.277.547 | 13.433.710.169 | 189h 51m 26s | 2.782 | 68.413 |
| medium | 2.900.316.732 | 1.379.736.583 | 4.707.509 | 1.515.872.640 | 144h 45m 26s | 380 | 3.116 |
| xhigh | 1.305.466.851 | 2.075.428 | 5.627.517 | 1.257.829.693 | 25h 6m 59s | 318 | 4.582 |
| unknown | 244.164.699 | 14.685.117 | 600.080 | 212.686.581 | 5h 21m 45s | 1.384 | 2.519 |
| max | 18.301.560 | 2.250.571 | 302.157 | 15.125.693 | 31m 33s | 4 | 111 |
| low | 0 | 0 | 0 | 0 | 53s | 0 | 0 |

`unknown` means that the originating log/session did not persist an explicit
effort value. It is retained rather than guessed from model names or response size.

## Tool & Skill Usage

### Top Tools

| Rank | Tool | Calls |
| ---: | :--- | ---: |
| 1 | Bash | 55.117 |
| 2 | Read | 10.084 |
| 3 | Edit | 6.008 |
| 4 | read_file | 1.312 |
| 5 | Write | 1.169 |
| 6 | Agent | 1.001 |
| 7 | terminal | 911 |
| 8 | search_files | 786 |
| 9 | ToolSearch | 582 |
| 10 | Monitor | 372 |
| 11 | wait_agent | 364 |
| 12 | patch | 357 |
| 13 | SendMessage | 194 |
| 14 | Skill | 189 |
| 15 | TaskStop | 167 |
| 16 | skill_view | 163 |
| 17 | send_message | 141 |
| 18 | tool_call | 122 |
| 19 | execute_code | 100 |
| 20 | ListAgents | 98 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | superpowers:subagent-driven-development | 35 |
| 2 | superpowers:finishing-a-development-branch | 31 |
| 3 | superpowers:brainstorming | 28 |
| 4 | superpowers:writing-plans | 24 |
| 5 | superpowers:using-git-worktrees | 19 |
| 6 | hermes-agent | 18 |
| 7 | knx-spec | 16 |
| 8 | software-development/codebase-inspection | 15 |
| 9 | software-development:codebase-inspection | 11 |
| 10 | software-development/private-corpus-regression-testing | 10 |
| 11 | software-development/delegated-code-verification | 8 |
| 12 | local-telemetry-collectors | 7 |
| 13 | software-development/test-driven-development | 7 |
| 14 | systematic-debugging | 7 |
| 15 | requesting-code-review | 6 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 44,644.5 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.78:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 125.8 | AI output ÷ added and deleted Git lines |
| Git commit density | 12,136,300.9 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $6,832.45 | 96.4% | $36,836.47 |
| Codex | $5,644.79 | 59.2% | $6,282.19 |
| **TOTAL** | **$12,477.23** | **87.1%** | **$43,118.67** |

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
- **Session velocity:** 2,940.8 output tokens per active generation minute
- **Autonomy input/output ratio:** 29.82:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 16.18 logged tool calls per task (78.741 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 526 | 10.8% |
| Morning (06-11) | 1.474 | 30.3% |
| Afternoon (12-17) | 1.684 | 34.6% |
| Evening (18-23) | 1.184 | 24.3% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 530 | 10.9% |
| Tuesday | 389 | 8.0% |
| Wednesday | 293 | 6.0% |
| Thursday | 742 | 15.2% |
| Friday | 575 | 11.8% |
| Saturday | 1.074 | 22.1% |
| Sunday | 1.265 | 26.0% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 1,893.3 kWh
- **Estimated CO₂ equivalent:** 624.8 kg CO₂e
- **Estimated physical keystrokes avoided:** 258.059.240

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.
