# U20 telegram-flow view — screenshots

Two captures of the real `BusMonitorPanel` flow tab with the Dimmer node selected
(Inspector open), rendered by `e2e/telegram-flow-fixture.html` in headless
Chromium. All data is synthetic and intercepted by Playwright: one switch writing
`On` on 1/0/1 to its configured member, and one unknown sender (1.1.9) writing
on 2/1/7, which has no resolved member.

| File | Theme |
| --- | --- |
| `flow-porcelain.png` | Porcelain (light) |
| `flow-graphite.png` | Graphite (dark) |

They were inspected for visible lines and arrowheads, readable labels and
badges, and an Inspector that fits its column. Two defects were found and fixed
before these captures: arrowheads hidden under node text, and a flag table too
wide for the Inspector. These are browser fixture captures; they say nothing
about the packaged WebKitGTK app.
