/** Records the README hero clip: add a device, link it to a group address, zoomed in. */

// A real knx-server with the production frontend and the fictional sample
// house (playwright.readme-hero.config.ts), Graphite (dark) theme. Frames come
// from the CDP screencast as lossless PNGs; every action records a camera
// target, and the clip is cropped, eased and time-compressed afterwards with
// ffmpeg, so the zoom is post-production and the app runs at normal size.
//
// Run (offline, see docs/manual/development/01-contributing.md):
//   KNX_SERVER_BIN=../../target/release/knx-server README_HERO_GIF=<out.gif> \
//     unshare --user --map-root-user --net sh -c 'ip link set lo up && exec "$@"' sh \
//     npx playwright test -c playwright.readme-hero.config.ts
import { expect, test, type Locator, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";

const VIEW = { width: 1440, height: 748 }; // same aspect as the 900 × 467 GIF
// The screencast delivers CSS pixels whatever the device scale, so the camera
// never zooms closer than one source pixel per GIF pixel (ZOOM_WIDTH = OUT_W).
const OUT_W = 900;
const ZOOM_WIDTH = OUT_W;
const OUT_H = 467;
const FPS = 12;
// Every step stays on screen for at least HOLD_MS so a reader can follow it;
// MAX_SECONDS only guards against a runaway recording.
const HOLD_MS = 3000;
const MAX_SECONDS = 60;
const CAMERA_MS = 700;

type Rect = { x: number; y: number; w: number; h: number };
const FULL: Rect = { x: 0, y: 0, w: VIEW.width, h: VIEW.height };

/** A visible pointer with a click ripple: Chromium's screencast draws none. */
function installCursor() {
  const style = document.createElement("style");
  style.textContent = `
    #hero-cursor { position: fixed; left: 0; top: 0; z-index: 2147483647; pointer-events: none;
      width: 22px; height: 22px; transform: translate(-100px, -100px); }
    #hero-cursor svg { filter: drop-shadow(0 1px 2px #000a); }
    .hero-ripple { position: fixed; z-index: 2147483646; pointer-events: none; width: 34px; height: 34px;
      margin: -17px 0 0 -17px; border-radius: 50%; border: 2px solid #b79cff;
      animation: hero-ripple .45s ease-out forwards; }
    @keyframes hero-ripple { from { transform: scale(.3); opacity: 1; } to { transform: scale(1.4); opacity: 0; } }`;
  const cursor = document.createElement("div");
  cursor.id = "hero-cursor";
  cursor.innerHTML = '<svg viewBox="0 0 22 22" width="22" height="22"><path d="M3 2 L3 18 L7.5 14 L10.5 20.5 L13 19.4 L10.1 13 L16 13 Z" fill="#fff" stroke="#111" stroke-width="1.3" stroke-linejoin="round"/></svg>';
  const attach = () => { document.head.append(style); document.body.append(cursor); };
  if (document.body) attach(); else document.addEventListener("DOMContentLoaded", attach);
  addEventListener("mousemove", (event) => { cursor.style.transform = `translate(${event.clientX - 3}px, ${event.clientY - 2}px)`; }, true);
  addEventListener("mousedown", (event) => {
    const ripple = document.createElement("div");
    ripple.className = "hero-ripple";
    ripple.style.left = `${event.clientX}px`;
    ripple.style.top = `${event.clientY}px`;
    document.body.append(ripple);
    setTimeout(() => ripple.remove(), 500);
  }, true);
}

/** A camera rectangle around `box`, at least ZOOM_WIDTH wide, in the clip's aspect, inside the viewport. */
function frameAround(box: Rect): Rect {
  const aspect = VIEW.width / VIEW.height;
  const w = Math.min(Math.max(box.w * 1.5, box.h * 1.5 * aspect, ZOOM_WIDTH), VIEW.width);
  const h = w / aspect;
  const x = Math.min(Math.max(box.x + box.w / 2 - w / 2, 0), VIEW.width - w);
  const y = Math.min(Math.max(box.y + box.h / 2 - h / 2, 0), VIEW.height - h);
  return { x, y, w, h };
}

const union = (a: Rect, b: Rect): Rect => {
  const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y);
  return { x, y, w: Math.max(a.x + a.w, b.x + b.w) - x, h: Math.max(a.y + a.h, b.y + b.h) - y };
};

async function boxOf(locator: Locator): Promise<Rect> {
  await locator.scrollIntoViewIfNeeded();
  const b = (await locator.boundingBox())!;
  return { x: b.x, y: b.y, w: b.width, h: b.height };
}

/** Glide the pointer to an element and click it, as a person would. */
async function click(page: Page, locator: Locator): Promise<void> {
  const b = await boxOf(locator);
  await page.mouse.move(b.x + Math.min(b.w / 2, 60), b.y + b.h / 2, { steps: 12 });
  await page.mouse.down();
  await page.mouse.up();
}

test("records adding a device and linking it to a group address", async ({ page }) => {
  await page.setViewportSize(VIEW);
  await page.addInitScript(() => {
    localStorage.setItem("knx-desktop:theme", "graphite");
    localStorage.setItem("knx-desktop:ui-language", "en");
  });
  await page.addInitScript(installCursor);

  // Open the sample house before recording starts.
  await page.goto("/");
  const guide = page.getByRole("dialog", { name: "Welcome to KNXBench" });
  await expect(guide).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(guide).toBeHidden();
  await page.getByRole("button", { name: /Import ETS project/ }).first().click();
  await page.getByRole("button", { name: "sample-house.knxproj" }).click();
  await expect(page.getByText("Project status")).toBeVisible({ timeout: 15_000 });
  for (const dismiss of await page.getByRole("button", { name: "Dismiss" }).all()) await dismiss.click();
  const topology = page.getByRole("navigation", { name: "Navigation" }).getByRole("button", { name: "Topology", exact: true });
  await topology.click();
  await page.mouse.move(700, 420);
  await page.waitForTimeout(600);

  // Recording: lossless screencast frames with their wall-clock timestamps.
  const dir = test.info().outputPath("frames");
  mkdirSync(dir, { recursive: true });
  const frames: { t: number; file: string }[] = [];
  const cdp = await page.context().newCDPSession(page);
  cdp.on("Page.screencastFrame", (frame) => {
    const file = `${dir}/src-${String(frames.length).padStart(5, "0")}.png`;
    writeFileSync(file, Buffer.from(frame.data, "base64"));
    frames.push({ t: frame.metadata.timestamp! * 1000, file });
    void cdp.send("Page.screencastFrameAck", { sessionId: frame.sessionId }).catch(() => {});
  });
  await cdp.send("Page.startScreencast", { format: "png", maxWidth: VIEW.width, maxHeight: VIEW.height });
  const t0 = Date.now();
  const cameras: { t: number; rect: Rect }[] = [{ t: 0, rect: FULL }];
  const look = (rect: Rect) => cameras.push({ t: Date.now() - t0, rect });
  const hold = () => page.waitForTimeout(HOLD_MS);
  await page.waitForTimeout(500);

  // 1. Add a device to the ground-floor line.
  const add = page.getByRole("button", { name: "Device · Ground floor" });
  look(frameAround(await boxOf(add)));
  await page.waitForTimeout(250);
  await click(page, add);
  await page.getByText(/Push button 4-fold, flush mounted \(3\)/).waitFor();
  await hold();

  // 2. Pick the product.
  const product = page.getByText(/Push button 4-fold, flush mounted \(3\)/);
  await product.waitFor();
  look(frameAround(await boxOf(page.getByRole("listbox").first())));
  await page.waitForTimeout(250);
  await click(page, product);
  await hold();

  // 3. Name it, give it a free address, create it.
  const nameField = page.locator('input[value*="Push button"]').first();
  await nameField.waitFor();
  const assign = page.getByLabel(/Assign free addresses on the line/);
  const create = page.getByRole("button", { name: "Create", exact: true });
  look(frameAround(union(await boxOf(nameField), await boxOf(assign))));
  await click(page, nameField);
  await page.keyboard.press("Control+A");
  await nameField.pressSequentially("Push button dining", { delay: 90 });
  await hold();
  await click(page, assign);
  await hold();
  look(frameAround(union(await boxOf(nameField), await boxOf(create))));
  await click(page, create);

  // 4. Done; open the new device from the topology (wide shot for the view change).
  const done = page.getByRole("button", { name: "Done" });
  await done.waitFor();
  look(frameAround(union(await boxOf(page.getByText(/Device created/)), await boxOf(done))));
  await hold();
  await click(page, done);
  look(FULL);
  await click(page, topology);
  const card = page.locator("main").getByRole("button", { name: "1.1.3 Push button dining" }).last();
  await card.waitFor();
  look(frameAround(await boxOf(card)));
  await hold();
  await click(page, card);
  await hold();

  // 5. Expand the first button's object and link it to the kitchen light.
  await page.getByRole("tablist", { name: "Push button dining" }).waitFor();
  const channel = page.getByText("Button 1", { exact: true });
  look(frameAround(await boxOf(channel)));
  await click(page, channel);
  await click(page, page.getByText("Button 1: Switch", { exact: true }));
  await hold();
  const linkButton = page.getByRole("button", { name: "Link", exact: true });
  const linkRow = linkButton.locator("xpath=ancestor::li[1]");
  const groupSelect = linkRow.getByRole("combobox").first();
  await groupSelect.waitFor();
  look(frameAround(await boxOf(linkRow)));
  await page.waitForTimeout(500);
  await click(page, groupSelect);
  await groupSelect.selectOption({ label: "0/0/2 Kitchen light" });
  await hold();
  await click(page, linkButton);
  const unlink = page.getByRole("button", { name: /Unlink/ }).first();
  await unlink.waitFor();
  look(frameAround(await boxOf(unlink.locator("xpath=ancestor::li[1]"))));
  await hold();
  look(FULL);
  await hold();
  await cdp.send("Page.stopScreencast");
  const rawSeconds = (Date.now() - t0) / 1000;

  const out = process.env.README_HERO_GIF;
  if (!out) return;
  // Uniform time compression so the clip fits MAX_SECONDS; cameras ease over CAMERA_MS.
  const speed = Math.max(1, rawSeconds / MAX_SECONDS);
  const ease = (k: number) => (k < 0.5 ? 2 * k * k : 1 - (-2 * k + 2) ** 2 / 2);
  const cameraAt = (t: number): Rect => {
    let rect = cameras[0].rect;
    for (const { t: start, rect: target } of cameras.slice(1)) {
      if (t < start) break;
      const k = ease(Math.min(1, (t - start) / CAMERA_MS));
      rect = { x: rect.x + (target.x - rect.x) * k, y: rect.y + (target.y - rect.y) * k,
        w: rect.w + (target.w - rect.w) * k, h: rect.h + (target.h - rect.h) * k };
    }
    return rect;
  };
  const start = frames[0].t;
  const count = Math.floor((rawSeconds / speed) * FPS);
  for (let n = 0; n < count; n++) {
    const t = (n / FPS) * speed * 1000; // recording time in ms
    const source = frames.filter((f) => f.t - start <= t).at(-1) ?? frames[0];
    const c = cameraAt(t);
    const crop = `crop=${Math.round(c.w)}:${Math.round(c.h)}:${Math.round(c.x)}:${Math.round(c.y)}`;
    execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-i", source.file, "-vf", `${crop},scale=${OUT_W}:${OUT_H}:flags=lanczos`,
      `${dir}/out-${String(n).padStart(4, "0")}.png`]);
  }
  const filter = "split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle";
  execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-framerate", String(FPS), "-i", `${dir}/out-%04d.png`, "-vf", filter, "-loop", "0", out]);
  console.log(`hero: ${frames.length} source frames, ${rawSeconds.toFixed(1)} s recorded, ${(count / FPS).toFixed(1)} s clip at ${speed.toFixed(2)}x`);
});
