/** Browser proof of note-aware deletes: named notes, explicit choices, bound apply. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { openExplorerBranches } from "./disclosure-fixture";

// Strictly fictional: names, addresses and notes are made up here.
const address = (id: number, raw: string, name: string) => ({ id, name, address: raw, range: null, dpts: [], links: [] });
const tree = (revision: number, deleted: boolean) => ({
  schema_version: 16, errors: 0, warnings: 0, can_undo: deleted, can_redo: false, is_modified: deleted,
  group_address_style: "ThreeLevel", server_incarnation: "note-fixture", project_incarnation: 5, snapshot_revision: revision,
  installations: [{ id: 0, name: "Fixture house", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: deleted ? [address(2, "1/1/2", "Hall light")] : [address(1, "1/1/1", "Living light"), address(2, "1/1/2", "Hall light")] }],
});
const notes = [
  { id: 4, title: "Wiring check", target: { kind: "groupAddress", id: 1 } },
  { id: 6, title: "Scene 3 uses this address", target: { kind: "groupAddress", id: 1 } },
];
const refusal = {
  kind: "noteDispositionRequired",
  error: "this change would leave 2 project notes without their target: …",
  notes,
  removedTargets: [{ kind: "groupAddress", id: 1 }],
  serverIncarnation: "note-fixture", projectIncarnation: 5, snapshotRevision: 1,
};
const targets = {
  serverIncarnation: "note-fixture", projectIncarnation: 5, snapshotRevision: 1,
  targets: [
    { target: { kind: "installation", id: 0 }, owner: [], parts: ["Fixture house"] },
    { target: { kind: "groupAddress", id: 1 }, owner: [], parts: ["1/1/1", "Living light"] },
    { target: { kind: "groupAddress", id: 2 }, owner: [], parts: ["1/1/2", "Hall light"] },
  ],
};

async function intercept(page: Page, language: "en" | "de") {
  const state = { revision: 1, open: false, deleted: false, unexpected: [] as string[], writes: [] as { method: string; path: string; body: unknown }[] };
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname; const method = request.method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (state.open) body = tree(state.revision, state.deleted); else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { state.open = true; body = tree(state.revision, state.deleted); }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/project/notes/targets" && method === "GET") body = targets;
    else if (path === "/api/group-addresses/1" && method === "DELETE") {
      state.writes.push({ method, path, body: null });
      status = 409; body = refusal;
    } else if (path === "/api/project/delete-with-notes" && method === "POST") {
      const sent = request.postDataJSON() as Record<string, unknown>;
      state.writes.push({ method, path, body: sent });
      if (sent.snapshotRevision !== state.revision) { status = 409; body = { kind: "noteDispositionStale", error: "changed" }; }
      else { state.revision += 1; state.deleted = true; body = tree(state.revision, true); }
    }
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict note disposition fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openProject(page: Page, m: Record<keyof typeof en, string>) {
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
}

async function deleteLivingLight(page: Page, m: Record<keyof typeof en, string>) {
  await openExplorerBranches(page);
  await page.locator("button.tree-label", { hasText: "Living light" }).click();
  await page.locator(".inspector").getByRole("button", { name: m["inspector.delete"], exact: true }).click();
}

async function expectInViewport(page: Page, width: number, title: string) {
  const panel = page.getByRole("dialog", { name: title, exact: true });
  const box = await panel.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(width);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
  expect(await panel.evaluate((node) => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
  for (const overflow of await panel.evaluate((node) => [...node.querySelectorAll<HTMLElement>("fieldset, select")]
    .map((element) => element.getBoundingClientRect().right - node.getBoundingClientRect().right))) {
    expect(overflow).toBeLessThanOrEqual(1);
  }
  // Radios stay radio-sized next to their words (the shared panel input rule is full width).
  for (const radio of await panel.getByRole("radio").all()) expect((await radio.boundingBox())!.width).toBeLessThan(40);
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  test(`${language} decides each stranded note before deleting at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language);
    await openProject(page, m);
    await deleteLivingLight(page, m);

    const dialog = page.getByRole("dialog", { name: m["noteDisposition.title"], exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText(m["noteDisposition.intro.other"].replace("{count}", "2"));
    const target = m["noteDisposition.on"].replace("{target}", `${m["notes.target.groupAddress"]}: 1/1/1 Living light`);
    for (const note of notes) {
      const group = dialog.getByRole("group", { name: `${note.title} ${target}` });
      await expect(group).toBeVisible();
      await expect(group.getByRole("radio", { name: m["noteDisposition.keep"], exact: true })).toBeChecked();
    }
    // The deleted address itself is never offered as a move target.
    const select = dialog.getByRole("combobox", { name: m["noteDisposition.moveTarget"].replace("{title}", notes[0].title) });
    await expect(select.locator("option")).toHaveText([m["noteDisposition.moveNone"], `${m["notes.target.installation"]}: Fixture house`,
      `${m["notes.target.groupAddress"]}: 1/1/2 Hall light`]);
    await expectInViewport(page, width, m["noteDisposition.title"]);
    if (process.env.KNX_NOTE_DISPOSITION_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_NOTE_DISPOSITION_CAPTURE_DIR, `note-disposition-${language}-${width}.png`) });

    const first = dialog.getByRole("group", { name: `${notes[0].title} ${target}` });
    await first.getByRole("radio", { name: m["noteDisposition.move"], exact: true }).check();
    const confirm = dialog.getByRole("button", { name: m["noteDisposition.confirm"], exact: true });
    await expect(confirm).toBeDisabled();
    await expect(dialog).toContainText(m["noteDisposition.incomplete"]);
    await select.selectOption({ label: `${m["notes.target.groupAddress"]}: 1/1/2 Hall light` });
    await dialog.getByRole("group", { name: `${notes[1].title} ${target}` }).getByRole("radio", { name: m["noteDisposition.delete"], exact: true }).check();
    await expectInViewport(page, width, m["noteDisposition.title"]);
    if (process.env.KNX_NOTE_DISPOSITION_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_NOTE_DISPOSITION_CAPTURE_DIR, `note-disposition-chosen-${language}-${width}.png`) });
    // A long dialog scrolls inside the shared panel cap; both actions stay reachable.
    for (const name of [m["noteDisposition.cancel"], m["noteDisposition.confirm"]]) {
      const action = dialog.getByRole("button", { name, exact: true });
      await action.scrollIntoViewIfNeeded();
      await expect(action).toBeInViewport();
    }
    await confirm.click();

    await expect(dialog).toHaveCount(0);
    await expect(page.locator("button.tree-label", { hasText: "Living light" })).toHaveCount(0);
    await expect(page.locator("button.tree-label", { hasText: "Hall light" })).toBeVisible();
    expect(state.writes.map((write) => `${write.method} ${write.path}`)).toEqual(["DELETE /api/group-addresses/1", "POST /api/project/delete-with-notes"]);
    expect(state.writes[1].body).toEqual({
      serverIncarnation: "note-fixture", projectIncarnation: 5, snapshotRevision: 1,
      delete: { kind: "groupAddresses", ids: [1] },
      choices: [
        { note: 4, action: { kind: "move", target: { kind: "groupAddress", id: 2 } } },
        { note: 6, action: { kind: "delete" } },
      ],
    });
    expect(state.unexpected).toEqual([]);
  });
}

test("cancel deletes nothing and returns focus to the delete button", async ({ page }) => {
  const state = await intercept(page, "en");
  await openProject(page, en);
  await deleteLivingLight(page, en);
  const dialog = page.getByRole("dialog", { name: en["noteDisposition.title"], exact: true });
  await expect(dialog.getByRole("radio", { name: en["noteDisposition.keep"], exact: true }).first()).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".inspector").getByRole("button", { name: en["inspector.delete"], exact: true })).toBeFocused();
  await expect(page.locator("button.tree-label", { hasText: "Living light" })).toBeVisible();
  expect(state.writes.map((write) => write.path)).toEqual(["/api/group-addresses/1"]);
  expect(state.unexpected).toEqual([]);
});
