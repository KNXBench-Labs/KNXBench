# Project Statistics

**Last update:** 2026-09-24 08:46:47

## Git Statistics (Current Repository)
- **Commits:** 1.385
- **Merges:** 146
- **Pushes/Sync:** 1.385
- **Lines Added (+):** 377.972
- **Lines Deleted (-):** 83.956

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
| **Codex** | 1.912.667.915 | 7.875.093 | 2.298.732.672 | 0 |
| **TOTAL** | **1.913.179.795** | **63.064.125** | **15.918.124.188** | **508.433.865** |

### 🚀 GRAND TOTAL CONSUMPTION
**18.402.801.973 Total Tokens**

## AI Efficiency & Code Yield

| Metric | Value | Interpretation |
| :--- | ---: | :--- |
| Tokens per committed added line | 48,688.3 | All selected input/output/cache tokens ÷ historical Git additions |
| Code churn ratio (added ÷ deleted) | 4.50:1 | Above 1 means the history added more lines than it removed |
| Output tokens per changed line | 136.5 | AI output ÷ added and deleted Git lines |
| Git commit density | 13,287,221.6 tokens/commit | Total selected token volume ÷ commits |

> Git `--numstat` measures historical committed additions/deletions, not surviving
> present-day source lines. Generated files, documentation and vendored changes are
> included when they are part of Git history.

## Estimated API-equivalent Cost & Cache Efficiency

| Provider | Estimated API-equivalent cost | Cache share of logical input | Estimated cache savings |
| :--- | ---: | ---: | ---: |
| Claude | $6,821.82 | 96.4% | $36,772.36 |
| Codex | $5,474.48 | 54.6% | $5,172.15 |
| **TOTAL** | **$12,296.29** | **86.8%** | **$41,944.51** |

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
- **Session velocity:** 3,247.9 output tokens per active generation minute
- **Autonomy input/output ratio:** 30.34:1 recorded non-cache input tokens per output token
- **Tool autonomy:** 15.99 logged tool calls per task (77.401 calls)

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

- **Estimated inference energy:** 1,840.3 kWh
- **Estimated CO₂ equivalent:** 607.3 kg CO₂e
- **Estimated physical keystrokes avoided:** 252.256.500

The environmental estimate is deliberately rough and transparent: **0.1 Wh
per 1,000 processed tokens** and **330 g CO₂e/kWh**. Real energy varies
dramatically with model, hardware, batching, context length, provider grid mix and
reasoning effort. It excludes training, networking and client hardware and must not be
presented as measured provider emissions.

## 💡 Fun Facts
- 📚 **46.007.004 book pages** of code & docs generated and processed (equivalent to reading *The Lord of the Rings* trilogy **38.339 times**).
- 🧬 Human DNA contains ~3 billion base pairs. This project's token history is **6.13x the length of the human genome**.
- 🦠 A single gram of soil contains ~40 million bacteria. Token volume equals the bacterial population in **460.1 grams of fertile dirt**.
- 🌌 If every token were 1 millimeter, the line would stretch **18.402 kilometers** (about **0.46x around Earth**).
- 💾 In plain text ASCII, this represents **68.56 GB of raw source text data**.
- ⌨️ Average professional coding speed is ~2,000 words/day. The project processed **18907.0 human-years worth of typing**.
- 📜 The entire *Encyclopædia Britannica* contains ~44 million words. This repository processed **313.7 full sets**.
- 🐜 An ant colony has ~100,000 ants. Enough tokens were processed to give **184.028 ants** their own full context window.
- 💧 A drop of water contains ~1.5 sextillion molecules. Token count equals **368.1x the drops in an Olympic pool** (50M drops ≈ 2.5L).
- ☕ At 10,000 tokens per manual coding sprint, completing this via human effort would take **1.840.280 cups of espresso**.
- ⚡ At the stated inference scenario, the processed tokens represent roughly **1.840 kWh** and **607.3 kg CO₂e**.
- ⌨️ At roughly four characters per output token, AI generation avoided about **252.256.500 physical keystrokes** (backspace heroics not included).

*No ants were harmed in the making of these statistics — at least none that I know of.*
