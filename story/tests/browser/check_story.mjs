// Real-browser acceptance checks for a built project-evolution preview (Playwright, Chromium).
// Usage: node story/tests/browser/check_story.mjs <story.html> <hostile.html> [receipt.json]
// PLAYWRIGHT_DIR may point at a node_modules directory that contains "playwright".
import { createRequire } from "module";
import { writeFileSync } from "fs";
import { pathToFileURL } from "url";

const require = createRequire(
  (process.env.PLAYWRIGHT_DIR || "/home/knxbench/.local/share/mise/installs/npm-playwright/latest/node_modules") + "/",
);
const playwright = require("playwright");
// STORY_BROWSER selects the engine: chromium (default), firefox or webkit.
const engine = process.env.STORY_BROWSER || "chromium";
if (!["chromium", "firefox", "webkit"].includes(engine)) {
  console.error(`unknown STORY_BROWSER ${engine}`);
  process.exit(2);
}

const [storyPath, hostilePath, receiptPath] = process.argv.slice(2);
if (!storyPath || !hostilePath) {
  console.error("usage: check_story.mjs <story.html> <hostile.html> [receipt.json]");
  process.exit(2);
}
const storyUrl = pathToFileURL(storyPath).href;
const hostileUrl = pathToFileURL(hostilePath).href;
const results = [];

function check(name, passed, detail) {
  results.push({ name, passed: Boolean(passed), detail });
  console.log(`${passed ? "PASS" : "FAIL"} ${name}${detail !== undefined ? ` — ${JSON.stringify(detail)}` : ""}`);
}

async function openPage(browser, viewport, options = {}) {
  const context = await browser.newContext({ viewport, ...options });
  const page = await context.newPage();
  const problems = [];
  page.on("console", (message) => { if (message.type() === "error") problems.push(message.text()); });
  page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
  page.on("dialog", async (dialog) => { problems.push(`dialog: ${dialog.message()}`); await dialog.dismiss(); });
  await page.addInitScript(() => {
    window.__cspViolations = [];
    document.addEventListener("securitypolicyviolation", (event) => window.__cspViolations.push(event.violatedDirective));
  });
  return { context, page, problems };
}

const totalSteps = (page) => page.evaluate(() => JSON.parse(document.getElementById("story-data").textContent).events.length);
const state = (page, key) => page.evaluate((k) => window.__storyState[k](), key);
// Ambient loops (signal pulses, letter swaps) are named "ambient-…" and never end on their own;
// every other effect (growth, retreat, refocus) must. Both kinds must stop when motion is off.
const runningAnimations = (page) => page.evaluate(() => document.getAnimations().filter((animation) =>
  animation.playState === "running" && !(animation.animationName || animation.id || "").startsWith("ambient")).length);
const runningAmbient = (page, name) => page.evaluate((n) => document.getAnimations().filter((animation) =>
  animation.playState === "running" && (animation.animationName || animation.id || "") === n).length, name);
async function waitForAmbient(page, name, timeout) {
  for (let waited = 0; waited <= timeout; waited += 100) {
    if (await runningAmbient(page, name)) return true;
    await page.waitForTimeout(100);
  }
  return false;
}
// Samples for the whole window, so a short effect between two samples cannot slip through.
async function ambientDuring(page, ms) {
  let seen = 0;
  for (let waited = 0; waited < ms; waited += 50) {
    seen = Math.max(seen, (await runningAmbient(page, "ambient-pulse")) + (await runningAmbient(page, "ambient-char-swap")));
    await page.waitForTimeout(50);
  }
  return seen;
}
const expectedVisible = (page, index) => page.evaluate((i) => {
  const data = JSON.parse(document.getElementById("story-data").textContent);
  const order = new Map(data.chapters.map((chapter, n) => [chapter.id, n]));
  return data.events.filter((event) => order.get(event.chapter) <= i).length;
}, index);
const visibleStoryNodes = (page) => page.$$eval("#story-graph .node:not(.is-hidden)", (nodes) => nodes.length);

async function scrollToChapter(page, number) {
  await page.evaluate((n) => document.getElementById(`chapter-${n}`).scrollIntoView({ block: "start" }), number);
  await page.waitForTimeout(250);
}

async function narrative(page, label) {
  const visibleCounts = [];
  const chapters = [];
  for (let n = 1; n <= 8; n += 1) {
    await scrollToChapter(page, n);
    chapters.push(await state(page, "chapter"));
    visibleCounts.push(await page.$$eval("#story-graph .node:not(.is-hidden)", (nodes) => nodes.length));
  }
  const total = await totalSteps(page);
  check(`${label}: scrolling advances through all eight chapters`, chapters.join() === "0,1,2,3,4,5,6,7", chapters);
  const grows = visibleCounts.every((count, index) => index === 0 || count > visibleCounts[index - 1]);
  check(`${label}: the tree grows with every chapter and ends complete`, grows && visibleCounts[7] === total, visibleCounts);
  const progress = await page.textContent("#graph-progress");
  check(`${label}: progress text reports the final state`, progress.includes(`Chapter 8 of 8 · ${total} of ${total} steps`), progress);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  check(`${label}: no horizontal page overflow`, overflow <= 1, overflow);
}

async function keyboardAndEvidence(page, label) {
  await scrollToChapter(page, 2);
  await page.focus("#next-chapter");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(400);
  const chapter = await state(page, "chapter");
  const focused = await page.evaluate(() => document.activeElement && document.activeElement.id);
  check(`${label}: keyboard Next chapter moves to chapter 3 and focuses its heading`,
    chapter === 2 && focused === "chapter-3-title", { chapter, focused });
  await page.keyboard.press("Tab");
  let reachedSummary = false;
  for (let i = 0; i < 12 && !reachedSummary; i += 1) {
    reachedSummary = await page.evaluate(() => document.activeElement && document.activeElement.tagName === "SUMMARY");
    if (!reachedSummary) await page.keyboard.press("Tab");
  }
  const outline = await page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
  await page.keyboard.press("Enter");
  const opened = await page.evaluate(() => document.activeElement.parentElement.open);
  const labels = await page.evaluate(() => Array.from(
    document.activeElement.parentElement.querySelectorAll(".source-label"), (node) => node.textContent));
  check(`${label}: Tab reaches an evidence disclosure with a visible focus ring`, reachedSummary && outline !== "none", outline);
  check(`${label}: Enter opens evidence and shows transformation labels`,
    opened && labels.some((text) => text.includes("Translated from German")), labels);
}

async function atlas(page, label, mobile) {
  await page.evaluate(() => document.getElementById("atlas").scrollIntoView());
  await page.fill("#atlas-search", "licence");
  await page.waitForTimeout(100);
  const results = await page.$$eval("#search-results button", (buttons) => buttons.map((b) => b.textContent));
  const matched = await page.$$eval("#atlas-canvas .node.is-match", (nodes) => nodes.length);
  const dimmed = await page.$$eval("#atlas-canvas .node.is-dimmed", (nodes) => nodes.length);
  check(`${label}: search finds licence steps and dims the rest`,
    results.length >= 2 && matched === results.length && dimmed === (await totalSteps(page)) - matched,
    { results, matched, dimmed });
  await page.press("#atlas-search", "Enter");
  await page.waitForTimeout(100);
  const selected = await state(page, "selected");
  const inspector = await page.textContent("#inspector");
  check(`${label}: Enter selects the first result and the inspector shows its evidence`,
    selected && inspector.includes(results[0]) && inspector.includes("Evidence"), selected);
  await page.fill("#atlas-search", "");
  await page.click('.chip[data-strand="bus"]');
  const pressed = await page.getAttribute('.chip[data-strand="bus"]', "aria-pressed");
  const focusDims = await page.$$eval("#atlas-canvas .node", (nodes) => nodes.every((node) => {
    const bus = ["live-bus-first", "test-transmitting-building", "first-device-programmed", "fail-closed-writes"]
      .includes(node.dataset.event);
    return bus === !node.classList.contains("is-dimmed");
  }));
  check(`${label}: strand focus highlights only the bus strand`, pressed === "true" && focusDims, pressed);
  await page.click('.chip[data-strand=""]');

  const before = await state(page, "view");
  await page.click("#zoom-in");
  const zoomed = await state(page, "view");
  check(`${label}: zoom button zooms in`, zoomed.w < before.w, { before: before.w, after: zoomed.w });
  await page.locator("#atlas-canvas").scrollIntoViewIfNeeded();
  const box = await page.locator("#atlas-canvas").boundingBox();
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  if (mobile && engine === "chromium") {
    // Real touch events need CDP; other engines get the same drag as a mouse gesture.
    const session = await page.context().newCDPSession(page);
    const touch = (type, x, y) => session.send("Input.dispatchTouchEvent", { type, touchPoints: type === "touchEnd" ? [] : [{ x, y }] });
    await touch("touchStart", cx, cy);
    for (let step = 1; step <= 6; step += 1) await touch("touchMove", cx - step * 15, cy - step * 10);
    await touch("touchEnd");
  } else {
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.mouse.move(cx - 60, cy - 40, { steps: 6 });
    await page.mouse.up();
  }
  const panned = await state(page, "view");
  check(`${label}: dragging pans the full view`, panned.x > zoomed.x && panned.y > zoomed.y, { zoomed, panned });

  await page.focus("#atlas-canvas");
  await page.keyboard.press("+");
  await page.keyboard.press("ArrowRight");
  const keyed = await state(page, "view");
  check(`${label}: keyboard + and arrows zoom and pan`, keyed.w < panned.w && keyed.x > panned.x, keyed);
  await page.keyboard.press("0");
  await page.keyboard.press("Tab");
  const nodeFocused = await page.evaluate(() => document.activeElement.classList.contains("node"));
  await page.keyboard.press("Enter");
  const keySelected = await state(page, "selected");
  check(`${label}: Tab reaches graph steps and Enter selects one`, nodeFocused && Boolean(keySelected), keySelected);
}

async function textAlternative(page, label) {
  await page.check("#text-only");
  const hidden = await page.evaluate(() => ["story-graph", "atlas"].map((id) => {
    const node = id === "atlas" ? document.getElementById(id) : document.querySelector(".story-graph");
    return getComputedStyle(node).display;
  }));
  const steps = await page.$$eval(".step-list li", (items) => items.length);
  const cards = await page.$$eval(".story-text .event-card", (items) => items.filter((i) => i.offsetParent).length);
  const total = await totalSteps(page);
  check(`${label}: text-only mode removes graphs but keeps every step`,
    hidden.every((d) => d === "none") && steps === total && cards === total, { hidden, steps, cards });
  await page.reload();
  const persisted = await page.isChecked("#text-only");
  check(`${label}: text-only preference persists after reload`, persisted, persisted);
  await page.uncheck("#text-only");
}

async function motion(browser) {
  const { context, page } = await openPage(browser, { width: 1440, height: 900 });
  await page.goto(storyUrl);
  await page.waitForTimeout(300);
  await page.click("#next-chapter");
  await page.waitForTimeout(160);
  const runningBefore = await runningAnimations(page);
  const midway = await page.evaluate(() => {
    const node = document.querySelector("#story-graph .node.is-new");
    return node ? Number(getComputedStyle(node).opacity) : null;
  });
  await page.check("#motion-off");
  const runningAfter = await runningAnimations(page);
  const settled = await page.evaluate(() => ({
    pendingNew: document.querySelectorAll(".is-new").length,
    activeOpacity: Array.from(document.querySelectorAll("#story-graph .node.is-active"),
      (node) => Number(getComputedStyle(node).opacity)),
  }));
  check("motion off cancels growth while it is running",
    runningBefore > 0 && midway !== null && midway < 1 && runningAfter === 0 && settled.pendingNew === 0 &&
    settled.activeOpacity.every((opacity) => opacity === 1),
    { runningBefore, midway, runningAfter, settled });
  await page.click("#next-chapter");
  await page.waitForTimeout(50);
  check("motion off prevents new growth effects", (await runningAnimations(page)) === 0);

  const stillWhileOff = await ambientDuring(page, 4500);
  check("motion off keeps connections and headlines still", stillWhileOff === 0, stillWhileOff);

  await page.uncheck("#motion-off");
  await page.click("#next-chapter");
  await page.waitForTimeout(160);
  const runningAgain = await runningAnimations(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.waitForSelector("#reduced-note", { state: "visible", timeout: 2000 }).catch(() => {});
  const runningReduced = (await runningAnimations(page)) + (await ambientDuring(page, 3000));
  const note = await page.isVisible("#reduced-note");
  check("OS reduced motion cancels growth and ambient loops while they run",
    runningAgain > 0 && runningReduced === 0 && note, { runningAgain, runningReduced, note });
  await page.click("#replay-growth");
  await page.waitForTimeout(50);
  check("replay does not animate under OS reduced motion", (await runningAnimations(page)) === 0);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.click("#replay-growth");
  await page.waitForTimeout(120);
  const replayRunning = await runningAnimations(page);
  await page.waitForTimeout(2600);
  const replayDone = await runningAnimations(page);
  check("replay grows again and every effect ends on its own", replayRunning > 0 && replayDone === 0,
    { replayRunning, replayDone });
  await context.close();
}

async function ambient(browser) {
  const { context, page } = await openPage(browser, { width: 1440, height: 900 });
  await page.goto(storyUrl);
  await scrollToChapter(page, 3);
  await page.waitForTimeout(1600);
  const pulses = await runningAmbient(page, "ambient-pulse");
  check("visible connections carry looping signal pulses", pulses > 0, pulses);
  const swapped = await waitForAmbient(page, "ambient-char-swap", 6000);
  const heading = await page.evaluate(() => {
    const h = document.getElementById("chapter-3-title");
    return { label: h.getAttribute("aria-label"), text: h.textContent, boxes: h.querySelectorAll(".char").length };
  });
  const title = await page.evaluate(() => JSON.parse(document.getElementById("story-data").textContent).chapters[2].title);
  const named = await page.getByRole("heading", { name: title, exact: true }).count();
  check("headline letters roll through in place and headings keep their text and name",
    swapped && heading.label === title && heading.text === title && heading.boxes > 0 && named === 1,
    { swapped, heading, named });
  await context.close();
}

async function scrollBack(browser) {
  const { context, page } = await openPage(browser, { width: 1440, height: 900 });
  await page.goto(storyUrl);
  for (const n of [1, 2, 3, 4]) {
    await scrollToChapter(page, n);
    await page.waitForTimeout(600);
  }
  await page.waitForTimeout(1200);
  await scrollToChapter(page, 2);
  const leaving = await page.$$eval("#story-graph .is-leaving", (nodes) => nodes.length);
  const retreating = await runningAnimations(page);
  await page.waitForTimeout(1600);
  const visible = await visibleStoryNodes(page);
  const expected = await expectedVisible(page, 1);
  const left = await page.$$eval("#story-graph .is-leaving", (nodes) => nodes.length);
  const settled = await runningAnimations(page);
  check("scrolling back retracts later steps with an animation and ends in the earlier chapter's state",
    leaving > 0 && retreating > 0 && visible === expected && left === 0 && settled === 0 &&
    (await state(page, "chapter")) === 1, { leaving, retreating, visible, expected, left, settled });

  await page.check("#motion-off");
  await scrollToChapter(page, 4);
  await scrollToChapter(page, 1);
  const instant = { leaving: await page.$$eval("#story-graph .is-leaving", (nodes) => nodes.length),
    visible: await visibleStoryNodes(page), expected: await expectedVisible(page, 0) };
  check("with motion off, scrolling back is immediate",
    instant.leaving === 0 && instant.visible === instant.expected, instant);
  await context.close();
}

async function noScript(browser) {
  const { context, page } = await openPage(browser, { width: 1024, height: 800 }, { javaScriptEnabled: false });
  await page.goto(storyUrl);
  const cards = await page.$$eval(".event-card", (items) => items.filter((i) => i.offsetParent).length);
  const steps = await page.$$eval(".step-list li", (items) => items.length);
  const graphHidden = await page.$eval(".story-graph", (node) => getComputedStyle(node).display === "none");
  check("without JavaScript the whole story and all evidence remain readable",
    cards === steps && steps > 0 && graphHidden, { cards, steps });
  await context.close();
}

async function hostile(browser) {
  const { context, page, problems } = await openPage(browser, { width: 1280, height: 900 });
  await page.goto(hostileUrl);
  await page.waitForTimeout(300);
  const titles = await page.$$eval(".story-text .event-card h3", (nodes) => nodes.map((node) => node.textContent));
  await page.evaluate(() => document.getElementById("atlas").scrollIntoView());
  await page.fill("#atlas-search", "pwned");
  await page.press("#atlas-search", "Enter");
  await page.waitForTimeout(200);
  const pwned = await page.evaluate(() => window.__pwned);
  const injected = await page.evaluate(() => document.querySelectorAll(
    ".story-text img, .story-text b, #inspector img, #inspector b, .story-text svg[onload], #search-results img").length);
  check("hostile text renders literally and never executes",
    pwned === undefined && injected === 0 && titles.some((t) => t.includes("<script>window.__pwned = 1</script>")) &&
    problems.filter((p) => p.startsWith("dialog")).length === 0, { pwned, injected, sample: titles[0] });
  await context.close();
}

const browser = await playwright[engine].launch();
// Firefox has no mobile emulation; it gets the phone viewport with touch support only.
const browserVersion = browser.version();
const mobileOptions = engine === "firefox" ? { hasTouch: true } : { isMobile: true, hasTouch: true, deviceScaleFactor: 2 };
for (const [label, viewport, mobile] of [["desktop 1440×900", { width: 1440, height: 900 }, false],
  ["mobile 390×844", { width: 390, height: 844 }, true]]) {
  const { context, page, problems } = await openPage(browser, viewport,
    mobile ? mobileOptions : {});
  await page.goto(storyUrl);
  await page.waitForTimeout(300);
  await narrative(page, label);
  await keyboardAndEvidence(page, label);
  await atlas(page, label, mobile);
  await textAlternative(page, label);
  const csp = await page.evaluate(() => window.__cspViolations);
  check(`${label}: no console errors, page errors or CSP violations`, problems.length === 0 && csp.length === 0,
    { problems, csp });
  await context.close();
}
await motion(browser);
await ambient(browser);
await scrollBack(browser);
await noScript(browser);
await hostile(browser);
await browser.close();

const failed = results.filter((result) => !result.passed);
const receipt = { story: storyPath, hostile: hostilePath, browser: `${engine} ${browserVersion} (Playwright)`, checks: results.length,
  failed: failed.length, results };
if (receiptPath) writeFileSync(receiptPath, JSON.stringify(receipt, null, 1));
console.log(`\n${results.length - failed.length}/${results.length} browser checks passed`);
process.exit(failed.length ? 1 : 0);
