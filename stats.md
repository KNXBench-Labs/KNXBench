# Project Statistics

**Last update:** 2026-09-24 11:14:43 CEST (UTC+02:00)

Welcome to the numerical engine room of KNXBench: this page counts commits,
tokens, agents, models, tools, caffeine-adjacent productivity and several things
no reasonable person would normally measure. It explains where the project's AI
effort went, how much code churn came back out, and whether the prompt cache is
quietly saving a small imaginary fortune. The figures are generated from local
Git and session logs; the ants in the fun-fact section remain under observation.

## 💡 Fun Facts
- 📚 **46.186.096 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **38.488 times**).
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.16x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **461.9 grams of fertile dirt**.
- 🌌 If every token were 1 millimeter, the line would stretch **18.474 kilometers** (about **0.46x around Earth**).
- 💾 In plain text ASCII, this represents **68.82 GB of raw source text data**.
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **18980.6 human-years worth of typing**.
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **314.9 full sets**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **184.744 ants** their own full context window.
- 💧 If every token were one microliter of water, they would fill about **7.39 Olympic swimming pools**.
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.847.443 cups of espresso**.
- 📖 At 90,000 words per novel, the processed text equals roughly **153.953 novels** — an editor has just fainted somewhere.
- 🎭 Shakespeare's complete works contain roughly 884,000 words; this is about **15.674 complete Bards**.
- ⛪ At roughly 783,000 words per King James Bible, the token history equals **17.695 copies**.
- 🇷🇺 *War and Peace* contains roughly 587,000 words; the project has processed about **23.604 Napoleonic epics**.
- 🐦 At 280 characters each, the generated and processed text would fill about **263.920.549 maximally packed posts**.
- 📱 At 160 characters per SMS, it equals approximately **461.860.962 text messages**. Roaming charges not included.
- 🗂️ At 80 characters per punched card, this would require **923.721.924 cards** and a warehouse-sized debugging session.
- 📄 Printed at 300 words per page and 500 sheets per ream, it would consume roughly **92.372 reams of paper**.
- 📚 Binding those 90,000-word novels at 3 cm each would create about **4.6 kilometers of bookshelf**.
- 🎧 Read aloud at 150 words per minute, the corpus would last about **1.539.536 audiobook hours**.
- 🗣️ Spoken continuously at 130 words per minute, it would take **202.8 years** to say everything out loud.
- 👓 Read at 238 words per minute for eight hours a day, it would occupy about **332.3 reader-years**.
- 💽 Stored as plain text on 1.44 MB floppy disks, it would need roughly **48.940 disks** — please label them carefully.
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.847 kWh** and **609.7 kg CO₂e**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **252.721.688 physical keystrokes** (backspace heroics not included).

*No ants were harmed in the making of these statistics — at least none that I know of.*

## Git Statistics (Current Repository)
- **Commits:** 1.393
- **Merges:** 147
- **Pushes/Sync:** 1.389
- **Lines Added (+):** 382.045
- **Lines Deleted (-):** 84.341

## Session Time & Execution Analysis
| Provider | Thinking / Reasoning Time | Generation Time | Executed Tasks |
| :--- | ---: | ---: | ---: |
| **Claude** | 164h 39m 19s | 190h 42m 40s | 4.302 |
| **Codex** | N/A | 133h 24m 15s | 543 |
| **TOTAL** | **164h 39m 19s** | **324h 6m 56s** | **4.845** |

`N/A` means the source logs do not provide a meaningful measurable duration;
it does not mean the model achieved enlightenment instantaneously. Idle/wait time
is omitted because the available providers do not record it consistently enough
for a useful comparison.

## Token Consumption (Selected Projects)

| Provider | Input Tokens | Output Tokens | Cache Read Tokens | Cache Write Tokens |
| :--- | ---: | ---: | ---: | ---: |
| **Claude** | 511.880 | 55.189.032 | 13.619.391.516 | 508.433.865 |
| **Codex** | 1.914.851.692 | 7.991.390 | 2.368.069.120 | 0 |
| **TOTAL** | **1.915.363.572** | **63.180.422** | **15.987.460.636** | **508.433.865** |

### 🚀 GRAND TOTAL CONSUMPTION
**18.474.438.495 Total Tokens**

## Agent & System Breakdown

| Agent / transport | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Claude Code | 14.183.526.293 | 511.880 | 55.189.032 | 13.619.391.516 | 190h 42m 40s | 4.302 | 74.987 |
| Codex CLI | 3.752.612.183 | 1.905.611.498 | 6.741.229 | 1.840.259.456 | 102h 2m 51s | 511 | 783 |
| Hermes Agent via Headroom | 479.044.258 | 7.424.320 | 1.052.770 | 470.567.168 | 2h 11m 28s | 27 | 1.777 |
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
| gpt-5.6-sol | 3.174.497.307 | 1.411.055.823 | 5.236.300 | 1.758.205.184 | 69h 9m 25s | 176 | 1.149 |
| gpt-5.6-terra | 533.865.343 | 272.844.357 | 1.368.890 | 259.652.096 | 20h 4m 2s | 174 | 87 |
| gpt-6-astra | 304.475.205 | 133.938.868 | 876.433 | 169.659.904 | 38h 47m 3s | 21 | 180 |
| gpt-5.6-luna | 271.856.592 | 96.848.041 | 496.167 | 174.512.384 | 5h 20m 45s | 37 | 1.445 |
| claude-haiku-4-5-20251001 | 181.190.782 | 46.428 | 358.390 | 165.009.609 | 3h 1m 22s | 12 | 2.134 |
| claude-fable-5-1 | 61.104.491 | 16.787 | 432.915 | 56.641.904 | 31m 49s | 6 | 314 |
| gpt-6-sol | 6.217.755 | 164.603 | 13.600 | 6.039.552 | 0s | 0 | 0 |
| <synthetic> | 0 | 0 | 0 | 0 | 57m 51s | 138 | 0 |
| unknown | 0 | 0 | 0 | 0 | 2m 58s | 1.354 | 0 |

## Effort Breakdown

| Effort | Total tokens | Input | Output | Cache read | Generation time | Tasks | Tool calls |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| high | 13.699.236.136 | 515.446.033 | 51.511.808 | 12.680.178.137 | 189h 51m 26s | 2.781 | 68.299 |
| medium | 3.231.568.644 | 1.380.914.210 | 5.267.042 | 1.845.387.392 | 104h 2m 10s | 362 | 2.514 |
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
| 4 | read_file | 1.624 |
| 5 | terminal | 1.189 |
| 6 | Write | 1.165 |
| 7 | search_files | 1.098 |
| 8 | Agent | 1.001 |
| 9 | patch | 715 |
| 10 | ToolSearch | 578 |
| 11 | Monitor | 371 |
| 12 | wait_agent | 364 |
| 13 | tool_call | 196 |
| 14 | SendMessage | 194 |
| 15 | Skill | 189 |
| 16 | TaskStop | 167 |
| 17 | send_message | 141 |
| 18 | skill_view | 131 |
| 19 | ListAgents | 98 |
| 20 | write_file | 93 |

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
| 8 | codex | 14 |
| 9 | software-development/codebase-inspection | 12 |
| 10 | requesting-code-review | 11 |
| 11 | software-development:codebase-inspection | 8 |
| 12 | software-development/test-driven-development | 7 |
| 13 | run | 6 |
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
| Tokens per committed added line | 48,356.7 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.53:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 135.5 | AI output ÷ added and deleted Git lines |
| Git commit density | 13,262,339.2 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $6,821.82 | 96.4% | $36,772.36 |
| Codex | $5,499.02 | 55.3% | $5,328.16 |
| **TOTAL** | **$12,320.83** | **86.8%** | **$42,100.51** |

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
- **Session velocity:** 3,248.9 output tokens per active generation minute
- **Autonomy input/output ratio:** 30.32:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 16.07 logged tool calls per task (77.848 calls)

### Activity by Time of Day

| Local time | Activity events | Share |
| :--- | ---: | ---: |
| Night (00-05) | 526 | 10.9% |
| Morning (06-11) | 1.469 | 30.3% |
| Afternoon (12-17) | 1.668 | 34.4% |
| Evening (18-23) | 1.182 | 24.4% |

### Activity by Weekday

| Weekday | Activity events | Share |
| :--- | ---: | ---: |
| Monday | 526 | 10.9% |
| Tuesday | 389 | 8.0% |
| Wednesday | 294 | 6.1% |
| Thursday | 724 | 14.9% |
| Friday | 574 | 11.8% |
| Saturday | 1.074 | 22.2% |
| Sunday | 1.264 | 26.1% |

Activity events are timestamped prompts/tasks found in the selected logs. Session
velocity uses logged output tokens and measured generation/session time; tool-call
coverage depends on what each provider records and is therefore an autonomy indicator,
not a billing-grade audit.

## Resource Scenario

- **Estimated inference energy:** 1,847.4 kWh
- **Estimated CO₂ equivalent:** 609.7 kg CO₂e
- **Estimated physical keystrokes avoided:** 252.721.688

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.
