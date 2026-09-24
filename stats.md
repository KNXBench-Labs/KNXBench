# Project Statistics

**Last update:** 2026-09-24 09:16:08 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the final section remain under observation.

## Git Statistics (Current Repository)
- **Commits:** 1.388
- **Merges:** 146
- **Pushes/Sync:** 1.387
- **Lines Added (+):** 378.263
- **Lines Deleted (-):** 84.058

## Session Time & Execution Analysis
| Provider | Thinking / Reasoning Time | Generation Time | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 190h 42m 40s | 4.302 |
| **Codex** | N/A | 132h 53m 55s | 540 |
| **TOTAL** | **164h 39m 19s** | **323h 36m 36s** | **4.842** |

`N/A` means the source logs do not provide a meaningful measurable duration;
it does not mean the model achieved enlightenment instantaneously. Idle/wait time
is omitted because the available providers do not record it consistently enough
for a useful comparison.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 511.880 | 55.189.032 | 13.619.391.516 | 508.433.865 |
| **Codex** | 1.912.824.064 | 7.892.282 | 2.308.010.368 | 0 |
| **TOTAL** | **1.913.335.944** | **63.081.314** | **15.927.401.884** | **508.433.865** |

### 🚀 GRAND TOTAL CONSUMPTION
**18.412.253.007 Total Tokens**

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Codex CLI | 3.750.109.161 | 1.904.262.434 | 6.732.615 | 1.839.114.112 | 101h 59m 29s | 509 | 783 |
| Hermes Agent via Headroom | 419.361.792 | 6.745.756 | 962.276 | 411.653.760 | 1h 44m 30s | 26 | 1.424 |
| Hermes Agent (direct) | 59.255.761 | 1.815.874 | 197.391 | 57.242.496 | 29h 9m 54s | 5 | 301 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and appears only when matching usage exists. Direct
Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-sonnet-5 | 10.194.076.112 | 355.925 | 33.082.380 | 9.840.530.452 | 84h 20m 12s | 1.589 | 49.274 |
| claude-opus-5 | 3.747.154.908 | 92.740 | 21.315.347 | 3.557.209.551 | 101h 51m 24s | 1.338 | 23.265 |
| gpt-5.6-sol | 3.127.303.209 | 1.409.381.480 | 5.173.057 | 1.712.748.672 | 69h 6m 4s | 175 | 990 |
| gpt-5.6-terra | 533.865.343 | 272.844.357 | 1.368.890 | 259.652.096 | 20h 4m 2s | 174 | 87 |
| gpt-6-astra | 304.475.205 | 133.938.868 | 876.433 | 169.659.904 | 38h 47m 3s | 21 | 180 |
| gpt-5.6-luna | 256.865.202 | 96.494.756 | 460.302 | 159.910.144 | 4h 53m 47s | 36 | 1.251 |
| claude-haiku-4-5-20251001 | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| claude-fable-5-1 | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| gpt-6-sol | 6.217.755 | 164.603 | 13.600 | 6.039.552 | 0s | 0 | 0 |
| <synthetic> | 0 | 0 | 0 | 0 | 57m 51s | 138 | 0 |
| unknown | 0 | 0 | 0 | 0 | 2m 57s | 1.353 | 0 |

## Effort Breakdown

| Effort | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| high | 13.699.236.136 | 515.446.033 | 51.511.808 | 12.680.178.137 | 189h 51m 26s | 2.781 | 68.299 |
| medium | 3.169.383.156 | 1.378.886.582 | 5.167.934 | 1.785.328.640 | 103h 31m 51s | 360 | 2.161 |
| xhigh | 1.305.466.851 | 2.075.428 | 5.627.517 | 1.257.829.693 | 25h 6m 59s | 318 | 4.582 |
| unknown | 219.865.304 | 14.677.330 | 471.898 | 188.939.721 | 4h 33m 50s | 1.379 | 2.342 |
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
| 4 | read_file | 1.499 |
| 5 | Write | 1.165 |
| 6 | terminal | 1.037 |
| 7 | search_files | 1.015 |
| 8 | Agent | 1.001 |
| 9 | patch | 642 |
| 10 | ToolSearch | 578 |
| 11 | Monitor | 371 |
| 12 | wait_agent | 364 |
| 13 | SendMessage | 194 |
| 14 | Skill | 189 |
| 15 | TaskStop | 167 |
| 16 | send_message | 141 |
| 17 | skill_view | 125 |
| 18 | tool_call | 125 |
| 19 | ListAgents | 98 |
| 20 | spawn_agent | 87 |

### Top Skills

| Rank | Skill | Calls |
| ---: | :--- | ---: |
| 1 | superpowers:subagent-driven-development | 35 |
| 2 | superpowers:finishing-a-development-branch | 31 |
| 3 | superpowers:brainstorming | 28 |
| 4 | superpowers:writing-plans | 24 |
| 5 | hermes-agent | 19 |
| 6 | superpowers:using-git-worktrees | 19 |
| 7 | knx-spec | 16 |
| 8 | codex | 13 |
| 9 | requesting-code-review | 11 |
| 10 | software-development/codebase-inspection | 11 |
| 11 | software-development:codebase-inspection | 8 |
| 12 | run | 6 |
| 13 | software-development/test-driven-development | 6 |
| 14 | software-development:systematic-debugging | 6 |
| 15 | test-driven-development | 6 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 48,675.8 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.50:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 136.4 | AI output ÷ added and deleted Git lines |
| Git commit density | 13,265,312.0 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $6,821.82 | 96.4% | $36,772.36 |
| Codex | $5,477.45 | 54.7% | $5,193.02 |
| **TOTAL** | **$12,299.26** | **86.8%** | **$41,965.38** |

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
- **Prime weekday:** Sunday (1.264 logged activity events)
- **Session velocity:** 3,248.8 output tokens per active generation minute
- **Autonomy input/output ratio:** 30.33:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 16.00 logged tool calls per task (77.495 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 526 | 10.9% |
| Morning (06-11) | 1.466 | 30.3% |
| Afternoon (12-17) | 1.668 | 34.4% |
| Evening (18-23) | 1.182 | 24.4% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 526 | 10.9% |
| Tuesday | 389 | 8.0% |
| Wednesday | 294 | 6.1% |
| Thursday | 721 | 14.9% |
| Friday | 574 | 11.9% |
| Saturday | 1.074 | 22.2% |
| Sunday | 1.264 | 26.1% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 1,841.2 kWh
- **Estimated CO₂ equivalent:** 607.6 kg CO₂e
- **Estimated physical keystrokes avoided:** 252.325.256

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.

## 💡 Fun Facts
- 📚 **46.030.632 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **38.358 times**).
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.14x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **460.3 grams of fertile dirt**.
- 🌌 If every token were 1 millimeter, the line would stretch **18.412 kilometers** (about **0.46x around Earth**).
- 💾 In plain text ASCII, this represents **68.59 GB of raw source text data**.
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **18916.7 human-years worth of typing**.
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **313.8 full sets**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **184.122 ants** their own full context window.
- 💧 A drop of water contains ~1.5 sextillion molecules. Token count equals **368.2x the drops in an Olympic pool** (50M drops ≈ 2.5L).
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.841.225 cups of espresso**.
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.841 kWh** and **607.6 kg CO₂e**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **252.325.256 physical keystrokes** (backspace heroics not included).

*No ants were harmed in the making of these statistics — at least none that I know of.*
