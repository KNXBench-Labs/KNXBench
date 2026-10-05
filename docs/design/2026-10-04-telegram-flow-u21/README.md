# U21 telegram-flow load measurements

Real measurements of the bus monitor's Flow view with synthetic, intercepted
traffic, produced by `apps/knx-web/e2e/flow-load.load.ts` with
`playwright.load.config.ts`. The fixture is built for **production** (`vite
build`, `vite.study.config.ts`) and served by `vite preview`. Machine: AMD Ryzen
7 5800X (16 threads), headless Chromium 152.0.7977.82 on Linux. Each scenario
ran once, so the numbers are single samples, not distributions.

| File | Content |
| --- | --- |
| `measurements.json` | Final run, with the 30 fps drawing cap (`FRAME_INTERVAL_MS = 32`) |
| `measurements-before-frame-cap.json` | The same scenarios before the cap (60 fps), the basis for it |
| `measurements-ar21-after.json` | AR21 corrections (2026-10-05): all scenarios incl. the §7 starting load |
| `measurements-ar21-before.json` | The §7 scenarios with the flow sources of `origin/main` before the corrections |
| `hub-before.png`, `hub-after.png` | A sender on 12 group addresses with 24 receivers, before and after hub separation |

## Scenarios

- **dense-burst-motion:** 300 devices and 120 groups in the snapshot, 200
  telegrams/s for 15 s (about 3,650 telegrams; 230 nodes and 240 edges drawn),
  motion on.
- **dense-burst-motion-off:** the same burst with the app's Motion set to Off.
- **long-session-motion:** 60 devices, 40 groups, 10 telegrams/s for 180 s,
  motion on.

Five marker telegrams per run go to a group that no other traffic uses. The
**marker lag** is the time from the fake server answering the poll that
carries a marker to the marker value appearing in a node badge (client
admission plus rendering; the 1 s poll interval is not included). **Main
thread** is Chromium's `TaskDuration` divided by wall time. **Heap** is
`JSHeapUsedSize` after a forced garbage collection, at the start, at five
points and at the end.

## Results

| Scenario | Main thread 60 fps → 30 fps | Long tasks | Marker lag max | Heap after GC (start → end) |
| --- | --- | --- | --- | --- |
| Burst, motion on | 0.89 → **0.69** | 1 (52 ms) → 0 | 156 → 148 ms | 4.3 → 6.9 MiB, flat from the 2nd sample |
| Burst, motion off | 0.06 → 0.06 | 1 (≈ 65 ms) | ≈ 20 ms | 4.2 → 6.4 MiB |
| Session, motion on | 0.35 → **0.21** | 0 | 13 → 10 ms | 3.3 → 5.3 MiB, flattening (includes the monitor's 1,000-row capture) |

Reduced rendering in the burst: all telegrams were drawn as bundled pulses
(more than 24 per batch), and about 1,500 of them found no free pulse element
(the cap is 160) and were counted instead. The view says so; values and
counts stay complete.

## Reading

- The data path is cheap: with motion off the burst keeps the main thread
  about 6 % busy and shows a value about 20 ms after its poll answer.
- Motion is the cost, and a CPU profile showed it to be mostly native work
  (style, layout, paint of the SVG), not JavaScript. That cost grows with
  drawn frames, so drawing is capped at about 30 fps: −35 % main-thread time
  in the quiet session and −23 % in the burst, with no long task left.
- Even capped, motion keeps a fast desktop CPU's main thread about 21 % busy
  at 10 telegrams/s. Slower hardware and the packaged WebKitGTK app were not
  measured (KNOWN_LIMITATIONS §154). Motion Off and the OS reduce preference
  remove that cost entirely.
- `frames.count` counts animation-frame callbacks, including the ones the cap
  skips, not drawn frames.

## History of this measurement

The first two attempts were not usable and are not published. In the first,
the Motion level was set by an init script, and that attribute is lost when
the document is parsed: "motion off" actually ran with motion on, and the
markers shared a group with generated traffic, so newer values replaced them
in the same batch. The second ran on the development build (React
development checks). Fixing the first problem also exposed an e2e test that
had passed without testing anything; it now checks the attribute and counts
frames.

## AR21 corrections (2026-10-05)

Three scenarios at the §7 starting load were added: `target-load-motion`,
`target-load-motion-off` (500 devices, 1,250 groups, 1,000 telegrams/s for
15 s) and `target-load-motion-60s`. Group 0 now goes to two marker devices of
its own: at high rates a marker on a busy device was pushed out of its
three-value badge before it could be seen. Groups beyond the device count
shift their targets, so the §7 load reaches ~2,500 distinct pairs instead of
repeating ~1,000. For the U21 scenarios this adds two devices and changes the
members of group 0 only. `FLOW_LOAD_FILE` names the output file.

Both AR21 files were measured on a host shared with other sessions (load
average 16–21 on 16 threads), so they are not directly comparable with the
U21 files above; compare before with after. Results and reading:
TELEGRAM_FLOW_VISUALIZATION §14. `hub-before.png` was taken with a scratch
copy of the hub scenario of `e2e/telegram-flow-hub.e2e.ts` after 25 s with the
`origin/main` flow sources; `hub-after.png` is the screenshot of that test.
