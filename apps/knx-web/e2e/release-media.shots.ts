/** Captures Alpha 14 website stills from the real server and a fictional project. */
import {expect, test, type Page} from "@playwright/test";
import {execFileSync} from "node:child_process";
import {mkdirSync, writeFileSync} from "node:fs";
import {resolve} from "node:path";
import {fileURLToPath} from "node:url";
import {openNavigationSections} from "./disclosure-fixture";
import { openFileSubmenu } from "./file-menu";

const target = process.env.KNX_RELEASE_MEDIA_DIR;
if (!target) throw new Error("set KNX_RELEASE_MEDIA_DIR to a task-owned capture directory");
mkdirSync(target, {recursive: true});
const repo = fileURLToPath(new URL("../../..", import.meta.url));

/** A vendor-style ZIP: the fictional demo package plus a fictional manual. */
function vendorZip(): Buffer {
  const script = [
    "import io, sys, zipfile",
    "out = io.BytesIO()",
    "with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as z:",
    "    z.write(sys.argv[1], 'Example Devices/fictional-demo-devices.knxprod')",
    "    z.writestr('Example Devices/Manual - fictional demo devices.txt',",
    "               'Fictional manual. Every device in this package is invented.\\n')",
    "sys.stdout.buffer.write(out.getvalue())",
  ].join("\n");
  return execFileSync("python3", ["-c", script, resolve(repo, "demos/1.0.0/fictional-demo-devices.knxprod")]);
}

/** Records the page as lossless screencast frames and encodes one H.264 clip. */
async function recordClip(page: Page, name: string, actions: () => Promise<void>): Promise<void> {
  const dir = test.info().outputPath(`frames-${name}`);
  mkdirSync(dir, {recursive: true});
  const frames: {path: string; time: number}[] = [];
  const cdp = await page.context().newCDPSession(page);
  cdp.on("Page.screencastFrame", frame => {
    const path = `${dir}/${String(frames.length).padStart(5, "0")}.png`;
    writeFileSync(path, Buffer.from(frame.data, "base64"));
    frames.push({path, time: frame.metadata.timestamp!});
    void cdp.send("Page.screencastFrameAck", {sessionId: frame.sessionId}).catch(() => {});
  });
  await cdp.send("Page.startScreencast", {format: "png"});
  try {
    await page.waitForTimeout(800);
    await actions();
    await page.waitForTimeout(1500);
  } finally {
    await cdp.send("Page.stopScreencast");
    await cdp.detach();
  }
  expect(frames.length).toBeGreaterThan(10);
  const list = frames.map((frame, i) => {
    const duration = i + 1 < frames.length ? Math.max(0.02, frames[i + 1].time - frame.time) : 1.5;
    return `file '${frame.path}'\nduration ${duration.toFixed(4)}\n`;
  }).join("") + `file '${frames[frames.length - 1].path}'\n`;
  writeFileSync(`${dir}/frames.txt`, list);
  execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", `${dir}/frames.txt`,
    "-vf", "fps=25,format=yuv420p", "-c:v", "libx264", "-crf", "20", "-preset", "slow", "-an",
    "-movflags", "+faststart", "-map_metadata", "-1", resolve(target!, name + ".mp4")]);
}

async function dismissToasts(page: Page): Promise<void> {
  const buttons = page.getByRole("button", {name: "Dismiss", exact: true});
  while (await buttons.count()) {
    const node = await buttons.first().elementHandle();
    if (!node) continue;
    await node.click();
    await expect.poll(() => node.evaluate(element => element.isConnected)).toBe(false);
    await node.dispose();
  }
}

async function shot(page: Page, name: string): Promise<void> {
  await dismissToasts(page);
  await page.waitForTimeout(400);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await page.screenshot({path: resolve(target!, name + ".png"), animations: "disabled"});
}

async function nav(page: Page, name: string): Promise<void> {
  // Since Alpha 14 the navigation blocks start folded; open them the way a user would.
  await openNavigationSections(page);
  await page.getByRole("navigation", {name: "Navigation", exact: true})
    .getByRole("button", {name, exact: true}).click();
}

async function theme(page: Page, value: string): Promise<void> {
  await page.getByRole("button", {name: "Settings", exact: true}).first().click();
  const settings = page.getByRole("dialog");
  await settings.getByLabel("Theme", {exact: true}).selectOption(value);
  await expect(page.locator("html")).toHaveAttribute("data-theme", value);
  await expect(settings.getByRole("status").filter({hasText: "Theme saved."})).toBeVisible();
  await settings.focus();
  await page.keyboard.press("Escape");
  await expect(settings).toBeHidden();
}

async function kitchenLight(page: Page): Promise<void> {
  await nav(page, "Group addresses");
  await page.locator("[data-ga-expand-all]").click();
  await page.getByRole("button", {name: "0/0/2", exact: true}).first().click();
  await expect(page.locator(".workbench-pane-right").getByRole("textbox", {name: "Name", exact: true}))
    .toHaveValue("Kitchen light");
}

async function addNote(page: Page, target: string, title: string, body: string): Promise<void> {
  await nav(page, "Notes");
  const editor = page.locator(".notes-workspace section").first();
  await editor.getByRole("button", {name: "New note", exact: true}).click();
  await editor.getByLabel("Title", {exact: true}).fill(title);
  await editor.getByLabel("Note text", {exact: true}).fill(body);
  const select = editor.locator("select");
  const value = await select.locator("option").filter({hasText: target}).first().getAttribute("value");
  await select.selectOption(value!);
  await editor.getByRole("button", {name: "Preview change", exact: true}).click();
  const review = editor.getByRole("region", {name: "Review change"});
  await expect(review).toContainText("Action: Create note");
  await review.getByRole("button", {name: "Confirm change", exact: true}).click();
  await expect(editor.locator("nav li button").filter({hasText: title})).toBeVisible();
}

test("Alpha 14 website stills, real application and fictional project", async ({page}) => {
  const errors: string[] = [];
  const forbidden: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  page.on("request", request => {
    const path = new URL(request.url()).pathname;
    if (/\/api\/(support\/diagnose|support\/export|device-diagnostics\/(start|export)|device-download\/start|device-address\/start|catalog\/(install|apply))/.test(path)) {
      forbidden.push(path);
    }
  });
  // This test must run in the isolated loopback-only namespace documented in the config.
  // No application responses are replaced; the sample is imported by the actual server.
  await page.addInitScript(() => {
    localStorage.setItem("knx-desktop:theme", "graphite");
    localStorage.setItem("knx-desktop:ui-language", "en");
  });
  await page.goto("/");
  const guide = page.getByRole("dialog", {name: "Welcome to KNXBench", exact: true});
  await expect(guide).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(guide).toBeHidden();
  await page.getByRole("button", {name: /Import ETS project/}).first().click();
  await page.getByRole("button", {name: "sample-house.knxproj", exact: true}).click();
  await expect(page.getByText("Project status", {exact: true})).toBeVisible({timeout: 20_000});
  await dismissToasts(page);

  // Clip: filter the group addresses, inspect the kitchen light, clear the filter.
  await page.setViewportSize({width: 1440, height: 900});
  await nav(page, "Group addresses");
  await page.locator("[data-ga-expand-all]").click();
  await dismissToasts(page);
  await recordClip(page, "group-details", async () => {
    await page.locator(".address-filter").pressSequentially("Kitchen", {delay: 140});
    await page.waitForTimeout(900);
    await page.getByRole("button", {name: "0/0/2", exact: true}).first().click();
    await expect(page.locator(".workbench-pane-right").getByRole("textbox", {name: "Name", exact: true}))
      .toHaveValue("Kitchen light");
    await page.waitForTimeout(2200);
    await page.locator(".address-filter").fill("");
  });
  await page.setViewportSize({width: 1440, height: 1080});

  await kitchenLight(page);
  await shot(page, "graphite-addresses");

  // Parameter workspace: area register, aligned rows, information pane.
  await nav(page, "Devices");
  await page.getByRole("button", {name: /Switch actuator ground floor/}).first().click();
  const tabs = page.getByRole("tablist", {name: "Switch actuator ground floor", exact: true});
  await tabs.getByRole("tab", {name: "Parameters", exact: true}).click();
  await expect(page.locator(".parameter-workspace")).toBeVisible();
  // The register starts closed; open the first channel area like a user would.
  const areas = page.locator(".parameter-workspace-nav button:visible");
  await areas.nth(Math.min(1, (await areas.count()) - 1)).click();
  await expect(page.locator(".parameter-workspace-closed")).toHaveCount(0);
  await shot(page, "graphite-parameters");

  // Notes ask before a delete: a new, unlinked address gets two notes, then Delete.
  await kitchenLight(page);
  await page.getByRole("button", {name: "New group address in Switching", exact: true}).first().click();
  const create = page.getByRole("dialog", {name: "New group address in this range", exact: true});
  await expect(create.locator("[data-ga-next-free]")).toHaveText("0/0/5");
  await create.getByLabel("Name", {exact: true}).fill("Dining light");
  await create.getByLabel("Name", {exact: true}).press("Enter");
  await expect(create).toHaveCount(0);
  await addNote(page, "0/0/5 Dining light", "Dining light waits for its actuator",
    "Channel D of the ground-floor actuator is reserved for it.");
  await addNote(page, "0/0/5 Dining light", "Scene 3 will use this address",
    "Add it to the evening scene once the actuator is linked.");
  await nav(page, "Group addresses");
  await page.locator("[data-ga-expand-all]").click();
  await page.getByRole("button", {name: "0/0/5", exact: true}).first().click();
  await page.locator(".inspector").getByRole("button", {name: "Delete", exact: true}).click();
  const decision = page.getByRole("dialog", {name: "Notes need a decision", exact: true});
  await expect(decision).toBeVisible();
  await expect(decision.getByRole("radio")).not.toHaveCount(0);
  await shot(page, "graphite-note-decision");
  await decision.getByRole("button", {name: "Cancel", exact: true}).click();
  await expect(decision).toBeHidden();

  // Vendor ZIP: preview only, nothing is installed.
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("option", {name: /^Product catalog/}).click();
  const catalogue = page.getByRole("region", {name: "Device catalog", exact: true});
  await expect(catalogue).toBeVisible();
  await catalogue.locator('input[type="file"]').setInputFiles({
    name: "example-devices-vendor.zip", mimeType: "application/zip", buffer: vendorZip(),
  });
  const wrapper = catalogue.getByRole("region", {name: "Packages in this ZIP"});
  await expect(wrapper).toBeVisible({timeout: 20_000});
  await wrapper.scrollIntoViewIfNeeded();
  await shot(page, "graphite-vendor-zip");

  // Diagnostic export: device, interface and the read-only preview.
  await page.locator(".file-menu > summary").click();
  await openFileSubmenu(page, "help");
  await page.locator(".file-menu [data-diagnostics-open]").click();
  const diagnostics = page.getByRole("dialog", {name: "Export diagnostic data", exact: true});
  await expect(diagnostics).toBeVisible();
  const device = diagnostics.locator("[data-diagnostics-device]");
  const first = await device.locator("option").filter({hasText: "1.1.1"}).first().getAttribute("value");
  await device.selectOption(first!);
  await diagnostics.locator("[data-diagnostics-gateway]").fill("192.0.2.10:3671");
  await diagnostics.locator("[data-diagnostics-preview]").click();
  await expect(diagnostics.locator("[data-diagnostics-review]")).toContainText("1.1.1");
  await shot(page, "graphite-diagnostics");
  await diagnostics.getByRole("button", {name: "Close", exact: true}).click();
  await expect(diagnostics).toBeHidden();

  for (const value of ["lcars", "user-modern-retro-green-crt"]) {
    await theme(page, value);
    await kitchenLight(page);
    await shot(page, value + "-addresses");
  }
  expect(errors).toEqual([]);
  expect(forbidden).toEqual([]);
});
