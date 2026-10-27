/** Captures the website highlight stills from the real server and a fictional project. */
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

test("website highlight stills, real application and fictional project", async ({page}) => {
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

  // Highlight: the imported project's status page, straight after the import.
  await shot(page, "graphite-import");

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

  // Highlight: Ctrl+K search across devices and group addresses.
  await page.keyboard.press("Control+k");
  const search = page.getByRole("dialog", {name: "Search"});
  await expect(search).toBeVisible();
  await page.keyboard.type("light", {delay: 60});
  await page.waitForTimeout(600);
  await shot(page, "graphite-search");
  await page.keyboard.press("Escape");
  await expect(search).toBeHidden();

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

  // Highlight: the add-device wizard previews name and address before anything exists.
  await openNavigationSections(page);
  const explorer = page.locator(".project-explorer");
  await explorer.getByRole("button", {name: "Expand all", exact: true}).click();
  const line = page.locator("li").filter({
    has: page.locator("button.tree-label", {hasText: /^Line 1: Ground floor$/}),
  }).last();
  await line.getByRole("button", {name: "+ Add device", exact: true}).click();
  const wizard = page.getByRole("dialog", {name: "Add device", exact: true});
  await expect(wizard).toBeVisible();
  await wizard.getByRole("button", {name: /Switch actuator 4-fold, DIN rail/}).click();
  await wizard.getByRole("button", {name: "Next", exact: true}).click();
  await wizard.getByRole("button", {name: "Next", exact: true}).click();
  await wizard.getByLabel("Device name", {exact: true}).fill("Dining room actuator");
  await wizard.getByLabel(/Assign free addresses on the line/).check();
  await wizard.getByRole("button", {name: "Next", exact: true}).click();
  await expect(wizard.getByText("1 device will be created")).toBeVisible();
  await shot(page, "graphite-device-wizard");
  // Cancelling a filled wizard asks before discarding; nothing is created.
  await wizard.getByRole("button", {name: "Cancel", exact: true}).click();
  await wizard.locator(".project-wizard-discard .danger-action").click();
  await expect(wizard).toBeHidden();

  // Highlight: named versions on the project history timeline; versions need a saved .knxdb.
  await page.getByRole("button", {name: "Save", exact: true}).first().click();
  const save = page.getByRole("dialog", {name: "Save as", exact: true});
  await expect(save).toBeVisible();
  await save.getByPlaceholder("filename").fill("sample-house.knxdb");
  await save.getByRole("button", {name: "Save", exact: true}).click();
  await expect(save).toBeHidden();
  await expect.poll(async () => (await (await page.request.get("/api/project")).json()).is_modified).toBe(false);
  await dismissToasts(page);
  await page.locator(".file-menu > summary").click();
  await openFileSubmenu(page, "history");
  await page.getByRole("menuitem", {name: "Project history", exact: true}).click();
  const history = page.getByRole("dialog", {name: "Project history", exact: true});
  await expect(history).toBeVisible();
  await history.getByLabel("Version name", {exact: true}).fill("Before the kitchen redesign");
  await history.getByRole("button", {name: "Create version", exact: true}).click();
  await expect(history.getByText("Before the kitchen redesign", {exact: true}).first()).toBeVisible();
  await shot(page, "graphite-history");
  await history.getByRole("button", {name: "Close", exact: true}).click();
  await expect(history).toBeHidden();

  // Optional themes, before the catalogue job's status pill joins the header.
  for (const value of ["lcars", "user-modern-retro-green-crt"]) {
    await theme(page, value);
    await kitchenLight(page);
    await shot(page, value + "-addresses");
  }
  await theme(page, "graphite");

  // Vendor ZIP: preview only, nothing is installed.
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("option", {name: /^Product catalog/}).click();
  const catalogue = page.getByRole("region", {name: "Device catalog", exact: true});
  await expect(catalogue).toBeVisible();
  await catalogue.locator('input[type="file"]').setInputFiles({
    name: "example-devices-vendor.zip", mimeType: "application/zip", buffer: vendorZip(),
  });
  // Since Alpha 16 a catalogue job prepares the preview first; it never installs on its own.
  await catalogue.getByRole("button", {name: "Upload once and prepare preview", exact: true}).click();
  const job = catalogue.getByRole("region", {name: "Catalogue update"}).filter({hasText: "Bound preview"});
  await expect(job.getByText("Nothing is installed yet.", {exact: true})).toBeVisible({timeout: 20_000});
  await expect(job.getByRole("button", {name: "Back up and install", exact: true})).toBeDisabled();
  await job.getByRole("heading", {name: "example-devices-vendor.zip"}).scrollIntoViewIfNeeded();
  await shot(page, "graphite-vendor-zip");

  expect(errors).toEqual([]);
  expect(forbidden).toEqual([]);
});
