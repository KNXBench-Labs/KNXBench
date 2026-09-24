# Project Statistics

**Last update:** 2026-09-24 09:00:17

## Git Statistics (Current Repository)
- **Commits:** 1.386
- **Merges:** 146
- **Pushes/Sync:** 1.386
- **Lines Added (+):** 378.073
- **Lines Deleted (-):** 83.978

## Session Time & Execution Analysis
| Provider | Thinking / Reasoning Time | Generation Time | User Idle / Wait Time | Executed Tasks |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 190h 42m 40s | 0s | 4.302 |
| **Codex** | 0s | 132h 53m 55s | 0s | 539 |
| **TOTAL** | **164h 39m 19s** | **323h 36m 36s** | **0s** | **4.841** |

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 511.880 | 55.189.032 | 13.619.391.516 | 508.433.865 |
| **Codex** | 1.912.683.740 | 7.878.593 | 2.301.105.792 | 0 |
| **TOTAL** | **1.913.195.620** | **63.067.625** | **15.920.497.308** | **508.433.865** |

### 🚀 GRAND TOTAL CONSUMPTION
**18.405.194.418 Total Tokens**

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Codex CLI | 3.750.109.161 | 1.904.262.434 | 6.732.615 | 1.839.114.112 | 101h 59m 29s | 509 | 783 |
| Hermes Agent via Headroom | 412.303.203 | 6.605.432 | 948.587 | 404.749.184 | 1h 44m 30s | 25 | 1.334 |
| Hermes Agent (direct) | 59.255.761 | 1.815.874 | 197.391 | 57.242.496 | 29h 9m 54s | 5 | 301 |
| Caveman | 0 | 0 | 0 | 0 | 0s | 0 | 0 |

Hermes sessions routed through a billing base URL on `127.0.0.1:8787` are
classified as **Hermes Agent via Headroom**. Caveman is detected from Caveman/
`glm-5.2` model identifiers and remains visible with zeroes when configured but
unused. Direct Claude Code and Codex CLI logs remain separate systems while their
tokens still roll up into the Claude/Codex provider totals above.

## Model Breakdown

| Model | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-sonnet-5 | 10.194.076.112 | 355.925 | 33.082.380 | 9.840.530.452 | 84h 20m 12s | 1.589 | 49.274 |
| claude-opus-5 | 3.747.154.908 | 92.740 | 21.315.347 | 3.557.209.551 | 101h 51m 24s | 1.338 | 23.265 |
| gpt-5.6-sol | 3.122.776.535 | 1.409.347.360 | 5.166.391 | 1.708.262.784 | 69h 6m 4s | 175 | 961 |
| gpt-5.6-terra | 533.865.343 | 272.844.357 | 1.368.890 | 259.652.096 | 20h 4m 2s | 174 | 87 |
| gpt-6-astra | 304.475.205 | 133.938.868 | 876.433 | 169.659.904 | 38h 47m 3s | 21 | 180 |
| gpt-5.6-luna | 254.333.287 | 96.388.552 | 453.279 | 157.491.456 | 4h 53m 47s | 35 | 1.190 |
| claude-haiku-4-5-20251001 | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| claude-fable-5-1 | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| gpt-6-sol | 6.217.755 | 164.603 | 13.600 | 6.039.552 | 0s | 0 | 0 |
| <synthetic> | 0 | 0 | 0 | 0 | 57m 51s | 138 | 0 |
| unknown | 0 | 0 | 0 | 0 | 2m 57s | 1.353 | 0 |

## Effort Breakdown

| Effort | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| high | 13.699.236.136 | 515.446.033 | 51.511.808 | 12.680.178.137 | 189h 51m 26s | 2.781 | 68.299 |
| medium | 3.162.324.567 | 1.378.746.258 | 5.154.245 | 1.778.424.064 | 103h 31m 51s | 359 | 2.071 |
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
| 4 | read_file | 1.465 |
| 5 | Write | 1.165 |
| 6 | terminal | 1.026 |
| 7 | Agent | 1.001 |
| 8 | search_files | 985 |
| 9 | patch | 634 |
| 10 | ToolSearch | 578 |
| 11 | Monitor | 371 |
| 12 | wait_agent | 364 |
| 13 | SendMessage | 194 |
| 14 | Skill | 189 |
| 15 | TaskStop | 167 |
| 16 | send_message | 141 |
| 17 | tool_call | 125 |
| 18 | skill_view | 121 |
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
| 10 | software-development/codebase-inspection | 10 |
| 11 | software-development:codebase-inspection | 8 |
| 12 | run | 6 |
| 13 | software-development:systematic-debugging | 6 |
| 14 | test-driven-development | 6 |
| 15 | update-config | 6 |

Tool rankings use executed Claude tool-use blocks, Codex function-call events and
Hermes `messages.tool_calls`. Hermes `sessions.tool_names` is an availability
inventory and is deliberately not counted as execution. Skill rankings recognize
executed `Skill`/`skill_view` calls with attributable skill arguments; unattributable
or older log records remain in the overall tool count without inventing a skill name.

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 48,681.6 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.50:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 136.5 | AI output ÷ added and deleted Git lines |
| Git commit density | 13,279,361.1 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $6,821.82 | 96.4% | $36,772.36 |
| Codex | $5,475.16 | 54.6% | $5,177.49 |
| **TOTAL** | **$12,296.98** | **86.8%** | **$41,949.85** |

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
- **Session velocity:** 3,248.1 output tokens per active generation minute
- **Autonomy input/output ratio:** 30.34:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 15.99 logged tool calls per task (77.405 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 526 | 10.9% |
| Morning (06-11) | 1.465 | 30.3% |
| Afternoon (12-17) | 1.668 | 34.5% |
| Evening (18-23) | 1.182 | 24.4% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 526 | 10.9% |
| Tuesday | 389 | 8.0% |
| Wednesday | 294 | 6.1% |
| Thursday | 720 | 14.9% |
| Friday | 574 | 11.9% |
| Saturday | 1.074 | 22.2% |
| Sunday | 1.264 | 26.1% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 1,840.5 kWh
- **Estimated CO₂ equivalent:** 607.4 kg CO₂e
- **Estimated physical keystrokes avoided:** 252.270.500

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.

## 💡 Fun Facts
- 📚 **46.012.986 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **38.344 times**).
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.14x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **460.1 grams of fertile dirt**.
- 🌌 If every token were 1 millimeter, the line would stretch **18.405 kilometers** (about **0.46x around Earth**).
- 💾 In plain text ASCII, this represents **68.56 GB of raw source text data**.
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **18909.4 human-years worth of typing**.
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **313.7 full sets**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **184.051 ants** their own full context window.
- 💧 A drop of water contains ~1.5 sextillion molecules. Token count equals **368.1x the drops in an Olympic pool** (50M drops ≈ 2.5L).
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.840.519 cups of espresso**.
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.840 kWh** and **607.4 kg CO₂e**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **252.270.500 physical keystrokes** (backspace heroics not included).

*No ants were harmed in the making of these statistics — at least none that I know of.*
