/** Devices navigation browser contracts with strictly intercepted, fictional API data. */
import { expect, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

const devices = (count: number): DeviceNode[] => Array.from({ length: count }, (_, index) => ({
  id: index + 1, name: `Actuator ${index + 1}`, address: index < 2 ? `1.1.${index === 0 ? 2 : 10}` : null,
  description: `Fictional device ${index + 1}`, com_object_count: 0,
}));
function fixtureTree(nodes: DeviceNode[]): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "devices-browser", snapshot_revision: 1,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
      lines: [{ id: 2, name: "Ground floor", address: 1, devices: nodes }] }], unassigned: [],
      buildings: [{ id: 3, name: "Kitchen", kind: "Room", children: [], devices: nodes.slice(0, 1) }], group_ranges: [],
      group_addresses: [{ id: 500, name: "Kitchen light", address: "1/1/1", range: null, dpts: [],
        links: [{ device_id: 1, device_name: "Actuator 1", device_address: "1.1.2", com_object_id: 20,
          com_object_number: 1, com_object_name: "Switch", direction: "Receive" }] }],
    }] };
}
export async function intercept(page: Page, language: "en" | "de", count = 8, theme = "porcelain") {
  const nodes = devices(2); const tree = fixtureTree(nodes);
  const objects = makeObjects(count);
  const state = { tree, nodes, unexpected: [] as string[], requests: [] as string[], polls: 0, failCatalogue: false, objects, refuseDpt: false };
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    state.requests.push(`${method} ${path}`);
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme } };
    else if (path === "/api/project" && method === "GET") body = state.tree;
    else if (path === "/api/project/new" && method === "POST") body = state.tree;
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/com-object-dpt" && method === "POST") {
      const payload = route.request().postDataJSON();
      if (state.refuseDpt) { status = 422; body = { error: "Fictional refusal" }; }
      else { const object = state.objects.find(o => o.id === payload.comObjectId)!; object.dpt = payload.dpt;
        state.tree = { ...state.tree, snapshot_revision: state.tree.snapshot_revision! + 1, is_modified: true }; body = state.tree; }
    }
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/devices" && method === "GET") {
      if (state.failCatalogue) { status = 500; body = { error: "fixture product database unavailable" }; }
      else body = { schemaVersion: 1, serverIncarnation: "devices-browser", snapshotRevision: state.tree.snapshot_revision,
        devices: nodes.map((device) => ({ id: device.id, device, productRef: "Fictional-P-1", resolution: "Resolved",
          manufacturerId: "Fictional-M-1", manufacturerName: "Fictional Devices", productText: "Switch actuator", orderNumber: "SA-4",
          productTextLanguage: null, productSourceLanguage: null })) };
    } else if (/^\/api\/device\/\d+$/.test(path) && method === "GET") {
      const device = nodes.find((node) => node.id === Number(path.split("/").at(-1)));
      if (!device) { status = 400; body = { error: "device not found" }; }
      else body = { ...device, com_objects: state.objects, product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } };
    } else if (/^\/api\/device\/\d+\/parameters$/.test(path) && method === "GET") body = { programId: null, sections: [], stale: [], diagnostics: [] };
    else if (path === "/api/bus/monitor/telegrams" && method === "GET") {
      state.polls++;
      body = { sessionId: 1, serverIncarnation: "devices-browser", contextStatus: "current", projectOpen: true,
        status: "active", nextSince: 1, droppedBefore: 0, telegrams: Number(url.searchParams.get("since")) === 0 ? [{
          seq: 1, timestamp: "2026-10-08T12:00:00Z", source: "1.1.2", destination: "1/1/1", destinationName: "Kitchen light",
          service: "GroupValueWrite", rawPayload: "01", decoded: null, control: null,
        }] : [] };
    } else if (path === "/api/device-readiness" && method === "GET") body = { devices: nodes.map((node) => ({
      address: node.address, name: node.name, programRef: "fictional", readiness: "untested", category: null, detail: null, steps: 1, octets: 1,
    })), counts: { untested: nodes.length } };
    else if (path === "/api/device-address/availability" && method === "GET") body = { startAvailable: false, reason: "Device-specific recovery not implemented" };
    else if (path === "/api/bus/scan/results" && method === "GET") body = { sessionId: 4, status: "completed", error: null,
      nextSince: 1, completedCount: 1, totalCount: 1, omittedAddresses: [], excludedAddresses: [],
      results: [{ address: "1.1.2", outcome: { kind: "occupied", maskVersion: 1793 } }] };
    else if (path === "/api/bus/scan/comparison" && method === "GET") body = { unexpected: [], missing: [], excludedInProject: [] };
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else {
      state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict devices fixture only" };
    }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}
export async function openProject(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
}
export async function openList(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await expect(page.locator(".devices-workspace")).toBeVisible();
}

import type { ComObjectNode } from "../src/bindings/ComObjectNode";
function makeObjects(count: number): ComObjectNode[] {
  return Array.from({ length: count }, (_, index) => ({ id: index + 1, number: index + 1,
    name: `Object ${index + 1}`, function_text: `Function ${index + 1}`, description: index === 0 ? "Hidden boiler marker" : null,
    description_layer: null, dpt: index % 3 === 0 ? "DPST-1-1" : null, program_dpt: index % 3 === 1 ? "DPST-9-1" : null,
    dpt_layer: index % 3 === 0 ? "Program" : null, dpt_text: index % 3 === 1 ? "Temperature" : null, read: false, write: true, transmit: false,
    update: false, communication: true, read_on_init: false, is_active: true,
    activation: index % 4 === 0 ? "Active" : index % 4 === 1 ? "Inactive" : index % 4 === 2 ? "Undetermined" : "NotEvaluated",
    channel: index % 4 === 0 ? { key: `channel-${index % 5}`, order: index % 5, kind: "Channel", text: `Lighting ${index % 5}`, name: `Raw ${index % 5}`, number: "A/05" } : null,
    links: index % 2 === 0 ? [{ ga_id: 500, address: "1/1/1", name: "Kitchen light", direction: "Send" }] : [],
  }));
}
