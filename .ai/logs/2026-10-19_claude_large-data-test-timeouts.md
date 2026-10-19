# Large-data unit tests stop measuring the host (19 October 2026)

Test hygiene for the Alpha 15 package set. Four Vitest cases render thousands of
rows and assert no elapsed time, yet ran under Vitest's 5 s default timeout:

| Test | Alone (load ≈12) | Full suite under parallel builds (load ≈19–20) |
| --- | --- | --- |
| `LogPanel.test.tsx` renders 16000 entries… | 2.8 s | 5.3 s, 6.1 s (timed out twice) |
| `ResizableTable.test.tsx` invalidates only a changed cell… | 1.4 s | 5.4 s (timed out) |
| `BusMonitorPanel.test.tsx` bounds a large captured batch… | 0.9 s | 6.5 s (timed out) |
| `GroupAddressWorkspace.test.tsx` renders a visible bounded window… | 3.1 s | 6.4 s (timed out) |

Five timeouts in four full-suite runs, spread over three package worktrees, none
in a file the packages changed. Each case now passes `LARGE_DATA_TIMEOUT_MS`
(30 s). No assertion, budget or production code changed. Rule recorded in
`docs/DEVELOPER_PITFALLS.md` (Web app).
