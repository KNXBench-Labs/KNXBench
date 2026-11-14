/** Composes original fictional server responses with the mounted production parameter editor. */
import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { ParameterPanel } from "../src/api";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { openExplorerBranches } from "./disclosure-fixture";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Phase = "initial" | "applied" | "changed" | "undo";
type Language = "en" | "de";
const captureRoot = process.env.KNXBENCH_PARAMETER_NAVIGATION_CAPTURE_DIR;

async function capturedFixture(page: Page, language: Language) {
  const bodies = new Map<string, string>();
  const hashes = new Map<string, string>();
  const body = (name: string) => {
    if (!bodies.has(name)) {
      const bytes = readFileSync(join(captureRoot!,language,`${name}.json`));
      bodies.set(name,bytes.toString("utf8"));
      hashes.set(name,createHash("sha256").update(bytes).digest("hex"));
    }
    return bodies.get(name)!;
  };
  const tree = (phase: Phase): ProjectTree => JSON.parse(body(phase === "undo" ? "undo-tree" : `project-${phase}`));
  // Independent semantic pins; captures are never rewritten to satisfy a fixture DTO.
  const initial: ParameterPanel = JSON.parse(body("initial"));
  expect(initial.sections[0].fields.map(field => field.etsId)).toEqual(["NAV-C","NAV-A","NAV-B","NAV-L"]);
  expect(initial.sections[0].fields[1].group?.container?.elementId).toBe("NAV-ARC-A");
  expect(initial.sections[0].fields[2].group?.container?.elementId).toBe("NAV-ARC-B");
  const state = { phase: "initial" as Phase, writes: [] as Record<string,unknown>[], unexpected: [] as string[] };
  const origin = new URL(String(test.info().project.use.baseURL)).origin;
  page.on("request", request => { if (new URL(request.url()).origin !== origin) state.unexpected.push(request.url()); });
  await page.route("https://api.github.com/**", route => { state.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", route => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    const ok = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const original = (name: string) => route.fulfill({ status: 200, contentType: "application/json", body: body(name) });
    const achievements = achievementsFixtureAnswer(method,path); if (achievements !== undefined) return ok(achievements);
    const activity = busActivityFixtureAnswer(method,path); if (activity !== undefined) return ok(activity);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme: "porcelain", motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" },
    } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+fictional-navigation" });
    if (path === "/api/project" && method === "GET") return original(state.phase === "undo" ? "undo-tree" : `project-${state.phase}`);
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics") return ok(statusMetricsFixtureBody(tree(state.phase)));
    if (path === "/api/log" || path === "/api/product-languages" || path === "/api/catalog/manufacturers" || path === "/api/catalog/items") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/device-download/status","/api/device-download/batch/status","/api/device-address/status"].includes(path)) return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no fictional operation"}' });
    if (path === "/api/device-system2/status") return ok(null);
    if (path === "/api/devices") return ok({ schemaVersion: 1, serverIncarnation: tree(state.phase).server_incarnation, snapshotRevision: tree(state.phase).snapshot_revision, devices: [] });
    if (path === "/api/device/1") return original("device");
    if (path === "/api/device/1/parameter-editor-source") {
      expect(url.searchParams.get("etsId")).toBe("NAV-A");
      return original(`source-${state.phase}`);
    }
    if (path === "/api/device/1/parameters" && method === "GET") return original(state.phase);
    if (path === "/api/device/1/parameters" && method === "POST") {
      const write = route.request().postDataJSON() as Record<string,unknown>;
      const current = tree(state.phase);
      expect({ server:write.serverIncarnation, project:write.projectIncarnation, revision:write.snapshotRevision })
        .toEqual({ server:current.server_incarnation, project:current.project_incarnation, revision:current.snapshot_revision });
      const target = state.phase === "initial" ? { etsId:"NAV-A", raw:"24", expectedValue:"10" }
        : { etsId:"NAV-C", raw:"1", expectedValue:"0" };
      expect({ etsId:write.etsId,raw:write.raw,expectedValue:write.expectedValue }).toEqual(target);
      state.writes.push(write);
      state.phase = state.phase === "initial" ? "applied" : "changed";
      return original(state.phase);
    }
    if (path === "/api/undo" && method === "POST") { expect(state.phase).toBe("changed"); state.phase="undo"; return original("undo-tree"); }
    state.unexpected.push(`${method} ${path}`);
    return route.fulfill({ status:404, contentType:"application/json", body:'{"error":"strict fictional capture fixture"}' });
  });
  return { state, verify: () => {
    for (const [name,hash] of hashes) expect(createHash("sha256").update(readFileSync(join(captureRoot!,language,`${name}.json`))).digest("hex")).toBe(hash);
    test.info().annotations.push({ type:"producer-captures", description:JSON.stringify(Object.fromEntries(hashes)) });
  } };
}

/** Runs one exact-response case; its owning spec registers the test for gate inventory. */
export async function runParameterNavigationCapture(page: Page, language: Language, width: number) {
      test.skip(!captureRoot,"requires runtime-produced fictional HTTP bodies; no fabricated DTO substitute");
      const m = language === "de" ? de : en;
      await page.setViewportSize({ width,height:1000 });
      const errors: string[] = []; page.on("pageerror",error => errors.push(error.message));
      const { state,verify } = await capturedFixture(page,language);
      await page.goto("/?view=editor");
      await openExplorerBranches(page);
      await page.locator(".project-explorer").getByText("Device 1",{ exact:false }).first().click();
      await page.getByRole("tab",{ name:m["workbench.parameters"],exact:true }).click();
      const workspace = page.locator(".parameter-workspace:visible");
      const nav = workspace.locator(".parameter-workspace-nav");
      const containers = nav.locator(".parameter-workspace-container-toggle");
      await expect(containers).toHaveCount(2);
      const caption = language === "de" ? "Bogen Fable" : "Arc Fable";
      await expect(containers.nth(0)).toContainText(caption); await expect(containers.nth(1)).toContainText(caption);
      expect(await nav.locator(".parameter-workspace-nav-entry").evaluateAll(entries => entries.map(entry => entry.getAttribute("data-container-key"))))
        .toEqual([null,expect.any(String),expect.any(String)]);
      const keys = await nav.locator("[data-container-key]").evaluateAll(entries => entries.map(entry => entry.getAttribute("data-container-key")));
      expect(new Set(keys).size).toBe(2);
      await expect(containers.nth(0)).toHaveAttribute("aria-expanded","false");
      await containers.nth(0).focus(); await page.keyboard.press("Enter");
      await expect(containers.nth(0)).toBeFocused();
      await expect(containers.nth(0)).toHaveAttribute("aria-expanded","true");
      const amber = nav.getByRole("button",{ name:/Amber page/ });
      await amber.click();
      const amberRow = workspace.locator('[data-parameter-row="NAV-A"]');
      await amberRow.locator(".parameter-checked-open").click();
      const draft = amberRow.getByRole("textbox",{ name:"Amber count",exact:true });
      await expect(draft).toHaveValue("10"); await draft.fill("24");
      await draft.evaluate(node => node.setAttribute("data-retained-draft","same-editor"));
      await containers.nth(0).focus(); await page.keyboard.press("Space");
      await expect(containers.nth(0)).toHaveAttribute("aria-expanded","false");
      await expect(draft).toHaveValue("24");
      expect(state.writes).toEqual([]);
      const search = workspace.locator('input[type="search"]');
      await search.fill(`${caption} Amber`);
      await expect(amberRow.locator(".parameter-workspace-result-path")).toHaveText(`${m["parameters.deviceScope"]} / ${caption} / Amber page`);
      await expect(draft).toHaveValue("24"); await search.fill("no such fictional parameter");
      await expect(workspace.locator(".parameter-workspace-empty")).toBeVisible();
      await search.fill(""); await expect(containers.nth(0)).toHaveAttribute("aria-expanded","false");
      expect(state.writes).toEqual([]);
      await amberRow.getByRole("button",{ name:language === "de" ? "Wert übernehmen" : "Apply value",exact:true }).click();
      await expect(draft).toHaveValue("24");
      await expect(amberRow).toContainText(language === "de" ? "Übernommen." : "Applied.");
      expect(state.writes.map(write => write.etsId)).toEqual(["NAV-A"]);
      await draft.fill("25");
      await nav.getByRole("button",{ name:/Overview/ }).click();
      const control = workspace.locator('[data-parameter-row="NAV-C"] input[type="number"]');
      await control.fill("1"); await control.press("Enter");
      await expect(containers.nth(0)).toHaveAttribute("aria-expanded","false");
      await containers.nth(0).click();
      await expect(nav.getByRole("button",{ name:/Extra page/ })).toBeVisible();
      await amber.click(); await expect(draft).toHaveValue("25");
      await expect(draft).toHaveAttribute("data-retained-draft","same-editor");
      // An unrelated revision with an identical field signature adopts the new
      // binding without throwing away the draft; the existing editor owns this policy.
      await expect(amberRow.getByRole("button",{ name:language === "de" ? "Wert übernehmen" : "Apply value",exact:true })).toBeEnabled();
      await draft.focus();
      await expect(workspace.locator(".parameter-workspace-context")).toContainText(`${caption} / Amber page`);
      await page.getByRole("button",{ name:m["toolbar.undo"],exact:true }).click();
      await expect(nav.getByRole("button",{ name:/Extra page/ })).toHaveCount(0);
      await expect(draft).toHaveValue("25"); await expect(draft).toHaveAttribute("data-retained-draft","same-editor");
      await containers.nth(1).click(); await nav.getByRole("button",{ name:/Blue page/ }).click();
      const grid = workspace.locator('.parameter-workspace-rows[data-layout="matrix"]');
      await expect(grid).toBeVisible(); await expect(grid.locator('[data-parameter-row="NAV-B"] input')).toHaveValue("30");
      expect(state.writes.map(write => ({ etsId:write.etsId,raw:write.raw }))).toEqual([{ etsId:"NAV-A",raw:"24" },{ etsId:"NAV-C",raw:"1" }]);
      expect(await page.evaluate(() => document.documentElement.scrollWidth-innerWidth)).toBeLessThanOrEqual(1);
      await expect(containers.nth(1)).toBeVisible();
      const shot = test.info().outputPath(`navigation-${language}-${width}.png`);
      await workspace.screenshot({ path:shot });
      await test.info().attach("fictional-navigation",{ path:shot,contentType:"image/png" });
      expect(state.unexpected).toEqual([]); expect(errors).toEqual([]); verify();
}
