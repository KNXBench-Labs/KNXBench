/** Real-DOM checks of the device configuration panel against an in-memory mocked profile API. */
import { expect, test, type Page, type Route } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

const MODULE_SHA = "a".repeat(64);
const ORIGINAL = '{ "known" : 1,\n  "unknown_member": [true, null] }';
const b64 = (text: string) => Buffer.from(text, "utf8").toString("base64url");
const fromB64 = (value: string) => Buffer.from(value, "base64url").toString("utf8");
const sha = (digit: number) => String(digit).repeat(64);

/** One device, a stored document and a revision that moves on apply or an external change. */
function mockServer() {
  const state = { revision: 7, document: ORIGINAL as string | null, digit: 1, applied: [] as string[], cancelled: [] as string[], previews: [] as { action: { kind: string; draft?: { documentB64: string } } }[], previewGate: null as Promise<void> | null };
  const profile = () => ({ device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: MODULE_SHA, documentBytes: Buffer.byteLength(state.document ?? ""), documentSha256: sha(state.digit) });
  const json = (route: Route, value: unknown) => route.fulfill({ contentType: "application/json", body: JSON.stringify(value) });
  const install = async (page: Page) => {
    await page.route("**/api/**", async (route) => {
      const url = new URL(route.request().url());
      const body = route.request().postDataJSON() as { action?: { kind: string; draft?: { documentB64: string } }; token?: string } | null;
      if (url.pathname === "/api/project") return json(route, { server_incarnation: "device-config-fixture", project_incarnation: 1, snapshot_revision: state.revision });
      if (url.pathname.endsWith("/devices/3")) {
        return json(route, { device: 3, serverIncarnation: "device-config-fixture", projectIncarnation: 1, snapshotRevision: state.revision, persistence: "native-working-history",
          profile: state.document === null ? null : { ...profile(), documentB64: b64(state.document), documentError: null } });
      }
      if (url.pathname.endsWith("/preview")) {
        state.previews.push({ action: body!.action! });
        const gate = state.previewGate;
        if (gate) await gate;
        return json(route, { token: `tok${state.previews.length}`, action: body!.action, previous: state.document === null ? null : profile(), noOp: false, expiresInSeconds: 300 });
      }
      if (url.pathname.endsWith("/apply")) {
        const action = state.previews[Number(body!.token!.slice(3)) - 1].action;
        state.document = action.kind === "remove" ? null : fromB64(action.draft!.documentB64);
        state.applied.push(state.document ?? "<removed>");
        state.revision += 1; state.digit += 1;
        return json(route, { changed: true, nativeGeneration: state.applied.length, snapshotRevision: state.revision, persistence: "native-working-history" });
      }
      if (url.pathname.endsWith("/cancel")) { state.cancelled.push(body!.token!); return json(route, { cancelled: true }); }
      return route.fulfill({ status: 404, contentType: "application/json", body: "{}" });
    });
  };
  /** Changes stored content as an undo or another editor would, then lets the panel see the new revision. */
  const changeExternally = (page: Page, document: string) => {
    state.document = document; state.revision += 1; state.digit += 1;
    return page.evaluate(() => window.dispatchEvent(new Event("device-config-external-change")));
  };
  return { state, install, changeExternally };
}

for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  const open = async (page: Page) => {
    const server = mockServer();
    await server.install(page);
    await page.goto(`/e2e/device-config-fixture.html?lang=${language}`);
    await expect(page.locator("textarea")).toHaveValue(ORIGINAL);
    return server;
  };
  const press = (page: Page, key: "preview" | "apply" | "remove" | "keepDraft" | "loadStored") => page.getByRole("button", { name: m[`deviceConfig.${key}`], exact: true }).click();
  const staleAlert = (page: Page) => page.locator(".device-config-stale");

  test(`${language} shows the stored text verbatim and applies an edit without a stale alert`, async ({ page }) => {
    const server = await open(page);
    await page.locator("textarea").fill('{"edited":1}');
    await expect(page.locator(".device-config-dirty")).toBeVisible();
    await press(page, "preview");
    await expect(page.getByRole("group", { name: m["deviceConfig.confirmLabel"] })).toBeVisible();
    await press(page, "apply");
    await expect.poll(() => server.state.document).toBe('{"edited":1}');
    await expect(page.getByRole("group", { name: m["deviceConfig.confirmLabel"] })).toBeHidden();
    await expect(page.locator(".device-config-dirty")).toBeHidden();
    await expect(page.locator("textarea")).toHaveValue('{"edited":1}');
    await expect(staleAlert(page)).toHaveCount(0);
    await expect(page.getByRole("button", { name: m["deviceConfig.preview"], exact: true })).toBeEnabled();
  });

  test(`${language} an edit during an in-flight preview cannot apply the older text`, async ({ page }) => {
    const server = await open(page);
    let release!: () => void;
    server.state.previewGate = new Promise<void>((resolve) => { release = resolve; });
    await page.locator("textarea").fill('{"a":1}');
    await press(page, "preview");
    await expect.poll(() => server.state.previews.length).toBe(1);
    await page.locator("textarea").fill('{"b":2}');
    release();
    await expect.poll(() => server.state.cancelled).toEqual(["tok1"]);
    await expect(page.getByRole("group", { name: m["deviceConfig.confirmLabel"] })).toHaveCount(0);
    await expect(page.locator("textarea")).toHaveValue('{"b":2}');
    expect(server.state.applied).toEqual([]);
    server.state.previewGate = null;
    await press(page, "preview");
    await press(page, "apply");
    await expect.poll(() => server.state.document).toBe('{"b":2}');
  });

  test(`${language} an external revision change keeps a dirty draft and offers both choices`, async ({ page }) => {
    const server = await open(page);
    await page.locator("textarea").fill('{"mine":true}');
    await server.changeExternally(page, '{"undone":1}');
    await expect(staleAlert(page)).toBeVisible();
    await expect(page.locator("textarea")).toHaveValue('{"mine":true}');
    await expect(page.getByRole("button", { name: m["deviceConfig.preview"], exact: true })).toBeDisabled();
    await press(page, "keepDraft");
    await expect(staleAlert(page)).toHaveCount(0);
    await expect(page.locator("textarea")).toHaveValue('{"mine":true}');
    await expect(page.getByRole("button", { name: m["deviceConfig.preview"], exact: true })).toBeEnabled();

    await server.changeExternally(page, '{"third":3}');
    await expect(staleAlert(page)).toBeVisible();
    await press(page, "loadStored");
    await expect(page.locator("textarea")).toHaveValue('{"third":3}');
    await expect(staleAlert(page)).toHaveCount(0);
    await expect(page.locator(".device-config-dirty")).toBeHidden();
  });
}
