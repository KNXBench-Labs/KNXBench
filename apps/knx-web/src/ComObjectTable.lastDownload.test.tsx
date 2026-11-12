/** Object-table marks, captions and summary for links/flags since the last download (ADR-0174). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { DownloadObjectDifference } from "./bindings/DownloadObjectDifference";
import type { DownloadObjectFlags } from "./bindings/DownloadObjectFlags";
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";
import type { ProjectTree } from "./bindings/ProjectTree";
import ComObjectTable from "./ComObjectTable";
import { formatLogTime } from "./logRows";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }),
  setComObjectDpt: vi.fn(), setComObjectDescription: vi.fn(), setComObjectFlag: vi.fn(), unlinkComObject: vi.fn(), linkComObject: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { DeviceWorkspace } from "./deviceComparisons.fixture";

const kitchen = { key: "c-kitchen", kind: "Channel" as const, text: "Kitchen", name: null, number: null, order: 0 };
const hall = { key: "c-hall", kind: "Channel" as const, text: "Hall", name: null, number: null, order: 1 };
function object(id: number, overrides: Partial<ComObjectNode> = {}): ComObjectNode {
  return { saved_instance_priority: { status: "unavailable", reason: "NotCaptured" }, effective_priority: { status: "unresolved", reason: "InstanceUnavailable", source: null, raw: null },
    id, number: id, name: `Object ${id}`, function_text: null, description: null, description_layer: null, dpt: null, dpt_layer: null, program_dpt: null,
    dpt_text: null, is_active: true, activation: "Active", channel: kitchen, read: false, write: false, transmit: false, update: false,
    communication: true, read_on_init: false, links: [], ...overrides };
}
const flags = (overrides: Partial<DownloadObjectFlags> = {}): DownloadObjectFlags =>
  ({ read: false, write: true, transmit: false, update: false, communication: true, read_on_init: null, ...overrides });
const at = "2026-11-12T09:30:00.000Z";
// 1/2/3 = 0x0A03, 1/2/4 = 0x0A04, 2/0/1 = 0x1001.
const links: DownloadObjectDifference = { number: 1, instance_id: 21, flags: null, links: [
  { group_address: 0x0a03, downloaded: null, current: true },
  { group_address: 0x0a04, downloaded: false, current: null },
  { group_address: 0x1001, downloaded: false, current: true },
] };
const changedFlags: DownloadObjectDifference = { number: 2, instance_id: 22, links: [],
  flags: { downloaded: flags({ read: true }), current: flags() } };
const newlyActive: DownloadObjectDifference = { number: 3, instance_id: 23, links: [], flags: { downloaded: null, current: flags() } };
const noLongerActive: DownloadObjectDifference = { number: 4, instance_id: 24, links: [{ group_address: 0x0a03, downloaded: true, current: null }],
  flags: { downloaded: flags({ transmit: true }), current: null } };
const orphan: DownloadObjectDifference = { number: 9, instance_id: null, links: [], flags: { downloaded: flags(), current: null } };
const usable: ObjectLastDownload = { links: { state: "usable", at }, flags: { state: "usable", at },
  differing: [links, changedFlags, newlyActive, noLongerActive, orphan],
  instances: { 21: links, 22: changedFlags, 23: newlyActive, 24: noLongerActive }, differing_objects: 5, no_selected_row: 1 };
// Rows keyed by id, deliberately not by number: id 21 has number 1, and id 1 must stay unmarked.
const objects = [object(21, { number: 1 }), object(22, { number: 2 }), object(23, { number: 3, channel: hall }),
  object(24, { number: 4, channel: hall, activation: "Inactive", is_active: false }), object(1, { number: 7, channel: hall })];

beforeEach(() => { resetSettingsForTests(); settingsStorage.setItem("uiLanguage", "en"); resetUiLanguageForTests(); });
afterEach(() => { document.body.innerHTML = ""; });

async function render(lastDownload: ObjectLastDownload | undefined, rows = objects, addressStyle = "ThreeLevel") {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  const draw = async (next: ObjectLastDownload | undefined) => { await act(async () => root.render(
    <ComObjectTable objects={rows} lastDownload={next} addressStyle={addressStyle} editor={com => <p className="stub-editor">{com.id}</p>} />)); };
  await draw(lastDownload);
  const row = (id: number) => host.querySelector<HTMLTableRowElement>(`tr[data-object-id="${id}"]`)!;
  const summary = () => host.querySelector<HTMLParagraphElement>(".com-last-download");
  const set = async (el: HTMLSelectElement | HTMLInputElement, value: string) => { await act(async () => {
    Object.getOwnPropertyDescriptor(el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype, "value")!.set!.call(el, value);
    el.dispatchEvent(new Event(el instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
  }); };
  const flat = async () => set(host.querySelector<HTMLSelectElement>("select.com-table-view")!, "flat");
  const open = async (id: number) => { await act(async () => { row(id).querySelector<HTMLElement>("summary")!.click(); }); };
  const editorRow = (id: number) => host.querySelector<HTMLTableRowElement>(`tr[data-editor-id="${id}"]`)!;
  return { host, row, summary, set, flat, open, editorRow, draw, cleanup: async () => { await act(async () => root.unmount()); host.remove(); } };
}
const description = (host: HTMLElement, el: Element) => (el.getAttribute("aria-describedby") ?? "").split(" ").filter(Boolean)
  .map(id => host.ownerDocument.getElementById(id)?.textContent ?? "").join(" ");

describe("object rows", () => {
  it("marks exactly the rows the server selected, by id, with a caption that describes the editor toggle", async () => {
    const v = await render(usable);
    for (const [id, key] of [[21, "comTable.lastDownload.row.links"], [22, "comTable.lastDownload.row.flags"],
      [23, "comTable.lastDownload.row.flags"], [24, "comTable.lastDownload.row.both"]] as const) {
      const row = v.row(id);
      expect(row.dataset.differsFromDownload, `row ${id}`).toBe("true");
      expect(row.querySelector(".com-object-download-note")?.textContent).toBe(en[key]);
      const mark = row.querySelector(".com-object-download-mark")!;
      expect(mark.getAttribute("aria-hidden")).toBe("true");
      expect(mark.getAttribute("title")).toBe(en[key]);
      const summary = row.querySelector("summary")!;
      // The name stays the toggle's label; the caption is its description.
      expect(summary.getAttribute("aria-label")).toBe(en["comTable.edit"].replace("{number}", String(Number(row.querySelector("summary")!.textContent))));
      expect(description(v.host, summary)).toBe(en[key]);
    }
    // Object number 1 belongs to id 21; the row with id 1 is unmarked.
    expect(v.row(1).dataset.differsFromDownload).toBeUndefined();
    expect(v.row(1).querySelector(".com-object-download-mark, .com-object-download-note")).toBeNull();
    expect(v.row(1).querySelector("summary")!.hasAttribute("aria-describedby")).toBe(false);
    await v.cleanup();
  });

  it("lists added, struck-through removed and redirected links in the editor row", async () => {
    const v = await render(usable);
    await v.flat();
    await v.open(21);
    const diff = v.editorRow(21).querySelector(".com-object-download-diff")!;
    expect(diff.getAttribute("aria-label")).toBe(en["comTable.lastDownload.heading"]);
    const items = [...diff.querySelectorAll("li")];
    expect(items.map(item => item.dataset.linkChange)).toEqual(["added", "removed", "redirected"]);
    expect(items[0].textContent).toBe(`${en["comTable.lastDownload.linkAdded"]} 1/2/3 (${en["inspector.direction.send"]})`);
    expect(items[1].querySelector("del")?.textContent).toBe("1/2/4");
    expect(items[1].textContent).toBe(`${en["comTable.lastDownload.linkRemoved"]} 1/2/4 (${en["inspector.direction.receive"]})`);
    expect(items[2].textContent).toBe(en["comTable.lastDownload.linkRedirected"].replace("{address}", "2/0/1")
      .replace("{downloaded}", en["inspector.direction.receive"]).replace("{current}", en["inspector.direction.send"]));
    // The details sit above the ordinary editor, which still mounts.
    expect(v.editorRow(21).querySelector(".stub-editor")?.textContent).toBe("21");
    await v.cleanup();
  });

  it("shows raw addresses in the project's address style", async () => {
    const v = await render(usable, objects, "TwoLevel");
    await v.flat();
    await v.open(21);
    expect(v.editorRow(21).querySelector('[data-link-change="added"] .mono')?.textContent).toBe("1/515");
    await v.cleanup();
  });

  it("says when flags differ and names presence on each side explicitly", async () => {
    const v = await render(usable);
    await v.flat();
    await v.open(22); await v.open(23); await v.open(24);
    const changed = v.editorRow(22).querySelector('[data-flag-change="changed"]')!;
    expect(changed.textContent).toBe(`${en["comTable.lastDownload.flagsAt"]} R W C`
      + `${en["inspector.comFlag.read"]}, ${en["inspector.comFlag.write"]}, ${en["inspector.comFlag.communication"]}`);
    expect(changed.querySelector('[aria-hidden="true"]')?.textContent).toBe("R W C");
    expect(changed.querySelector(".sr-only")?.textContent).toBe("Read, Write, Communication");
    expect(v.editorRow(23).querySelector('[data-flag-change="notActiveThen"]')?.textContent).toBe(en["comTable.lastDownload.notActiveThen"]);
    expect(v.editorRow(23).querySelector("[data-link-change]")).toBeNull();
    const gone = v.editorRow(24).querySelector('[data-flag-change="notActiveNow"]')!;
    expect(gone.textContent).toContain(en["comTable.lastDownload.notActiveNow"]);
    expect(gone.querySelector('[aria-hidden="true"]')?.textContent).toBe("W T C");
    expect(v.editorRow(24).querySelector('[data-link-change="removed"] del')?.textContent).toBe("1/2/3");
    await v.cleanup();
  });

  it("does not mark a row whose link entry lists no change", async () => {
    const same: DownloadObjectDifference = { number: 2, instance_id: 22, flags: null, links: [{ group_address: 0x0a03, downloaded: true, current: true }] };
    const v = await render({ ...usable, differing: [same], instances: { 22: same }, differing_objects: 1, no_selected_row: 0 });
    expect(v.row(22).dataset.differsFromDownload).toBeUndefined();
    expect(v.host.querySelector(".com-object-download-mark")).toBeNull();
    await v.cleanup();
  });

  it("never invents a read-on-init flag and says none when no flag was set", async () => {
    const none: DownloadObjectDifference = { number: 2, instance_id: 22, links: [],
      flags: { downloaded: { read: false, write: false, transmit: false, update: false, communication: false, read_on_init: null }, current: flags() } };
    const v = await render({ ...usable, differing: [none], instances: { 22: none }, differing_objects: 1, no_selected_row: 0 });
    await v.flat(); await v.open(22);
    expect(v.editorRow(22).querySelector('[data-flag-change="changed"]')?.textContent)
      .toBe(`${en["comTable.lastDownload.flagsAt"]} ${en["comTable.lastDownload.noFlags"]}`);
    await v.cleanup();
  });
});

describe("summary line", () => {
  const time = () => formatLogTime(Date.parse(at), "en", new Date())!;
  const sections = () => `${en["comTable.lastDownload.links.usable"].replace("{time}", time())} ${en["comTable.lastDownload.flags.usable"].replace("{time}", time())}`;

  it("counts differing objects, hidden rows and rows the server could not select", async () => {
    const v = await render(usable);
    // Grouped view starts collapsed: all four selected rows are hidden.
    expect(v.summary()?.textContent).toBe(`${en["comTable.lastDownload.differs.other"].replace("{count}", "5")} ${
      en["comTable.lastDownload.hidden.other"].replace("{count}", "4")} ${en["comTable.lastDownload.noRow.one"].replace("{count}", "1")} ${sections()}`);
    expect(v.summary()?.dataset.differs).toBe("true");
    expect(v.summary()?.querySelector(".com-last-download-dot")?.getAttribute("aria-hidden")).toBe("true");
    expect(v.summary()?.hasAttribute("role")).toBe(false);
    await v.flat();
    expect(v.summary()?.textContent).toBe(`${en["comTable.lastDownload.differs.other"].replace("{count}", "5")} ${
      en["comTable.lastDownload.noRow.one"].replace("{count}", "1")} ${sections()}`);
    // A filter that hides one differing row counts it.
    await v.set(v.host.querySelector<HTMLInputElement>('input[type="search"]')!, "Object 22");
    expect(v.summary()?.textContent).toContain(en["comTable.lastDownload.hidden.other"].replace("{count}", "3"));
    await v.cleanup();
  });

  it("marks a collapsed group that holds differing rows, not its clean neighbour", async () => {
    const clean = object(30, { channel: { ...hall, key: "c-clean", text: "Clean" } });
    const v = await render(usable, [...objects, clean]);
    const groups = [...v.host.querySelectorAll<HTMLButtonElement>(".com-object-channel-summary")];
    const byText = (text: string) => groups.find(group => group.textContent!.includes(text))!;
    for (const marked of ["Kitchen", "Hall"]) {
      expect(byText(marked).dataset.differsFromDownload).toBe("true");
      expect(byText(marked).getAttribute("aria-expanded")).toBe("false");
      expect(description(v.host, byText(marked))).toBe(en["comTable.lastDownload.group"]);
      expect(byText(marked).querySelector(".com-object-download-mark")?.getAttribute("aria-hidden")).toBe("true");
    }
    expect(byText("Clean").dataset.differsFromDownload).toBeUndefined();
    expect(byText("Clean").hasAttribute("aria-describedby")).toBe(false);
    expect(byText("Clean").querySelector(".com-object-download-mark")).toBeNull();
    await v.cleanup();
  });

  it("keeps usable link marks when flags cannot be compared, and says so", async () => {
    const v = await render({ links: { state: "usable", at }, flags: { state: "currentUnreadable" }, differing: [links],
      instances: { 21: links }, differing_objects: 1, no_selected_row: 0 });
    expect(v.summary()?.dataset.flags).toBe("currentUnreadable");
    expect(v.summary()?.textContent).toBe(`${en["comTable.lastDownload.differs.one"].replace("{count}", "1")} ${
      en["comTable.lastDownload.hidden.one"].replace("{count}", "1")} ${en["comTable.lastDownload.links.usable"].replace("{time}", time())} ${
      en["comTable.lastDownload.flags.currentUnreadable"]}`);
    expect(v.row(21).dataset.differsFromDownload).toBe("true");
    await v.cleanup();
  });

  it("says nothing differs when both sections compare clean", async () => {
    const v = await render({ ...usable, differing: [], instances: {}, differing_objects: 0, no_selected_row: 0 });
    expect(v.summary()?.textContent).toBe(`${en["comTable.lastDownload.noneDiffer"]} ${sections()}`);
    expect(v.summary()?.dataset.differs).toBeUndefined();
    expect(v.summary()?.querySelector(".com-last-download-dot")).toBeNull();
    expect(v.host.querySelector(".com-object-download-mark")).toBeNull();
    await v.cleanup();
  });

  for (const [links_, flags_] of [["absent", "absent"], ["otherProgram", "productUnavailable"], ["productUnavailable", "otherProgram"],
    ["currentUnreadable", "absent"]] as const) {
    it(`gives a plain sentence per unusable section (${links_}/${flags_}) and no count or marks`, async () => {
      const v = await render({ links: { state: links_ }, flags: { state: flags_ }, differing: [], instances: {}, differing_objects: 0, no_selected_row: 0 });
      expect(v.summary()?.textContent).toBe(`${en[`comTable.lastDownload.links.${links_}`]} ${en[`comTable.lastDownload.flags.${flags_}`]}`);
      expect(v.host.querySelector(".com-object-download-mark, .com-last-download-dot")).toBeNull();
      await v.cleanup();
    });
  }

  it("shows nothing without a comparison (older server or edit response)", async () => {
    const v = await render(undefined);
    expect(v.summary()).toBeNull();
    expect(v.host.querySelector(".com-object-download-mark, .com-object-download-note, [data-differs-from-download]")).toBeNull();
    await v.cleanup();
  });

  it("speaks German", async () => {
    settingsStorage.setItem("uiLanguage", "de"); resetUiLanguageForTests();
    const v = await render(usable);
    const deTime = formatLogTime(Date.parse(at), "de", new Date())!;
    expect(v.summary()?.textContent).toBe(`${de["comTable.lastDownload.differs.other"].replace("{count}", "5")} ${
      de["comTable.lastDownload.hidden.other"].replace("{count}", "4")} ${de["comTable.lastDownload.noRow.one"].replace("{count}", "1")} ${
      de["comTable.lastDownload.links.usable"].replace("{time}", deTime)} ${de["comTable.lastDownload.flags.usable"].replace("{time}", deTime)}`);
    expect(v.row(24).querySelector(".com-object-download-note")?.textContent).toBe(de["comTable.lastDownload.row.both"]);
    await v.flat(); await v.open(24);
    expect(v.editorRow(24).querySelector('[data-flag-change="notActiveNow"] .sr-only')?.textContent).toBe("Schreiben, Übertragen, Kommunikation");
    await v.cleanup();
  });

  it("replaces marks with the next read and drops them for a comparison without them", async () => {
    const v = await render(usable);
    expect(v.row(22).dataset.differsFromDownload).toBe("true");
    await v.draw({ ...usable, differing: [links], instances: { 21: links }, differing_objects: 1, no_selected_row: 0 });
    expect(v.row(22).dataset.differsFromDownload).toBeUndefined();
    expect(v.row(21).dataset.differsFromDownload).toBe("true");
    await v.draw(undefined);
    expect(v.host.querySelector("[data-differs-from-download]")).toBeNull();
    await v.cleanup();
  });
});

it("the device editor passes the device detail's comparison and address style to the table", async () => {
  const tree: ProjectTree = { schema_version: 11, installations: [], group_address_style: "TwoLevel", errors: 0, warnings: 0,
    can_undo: false, can_redo: false, is_modified: false };
  const detail: DeviceDetail = { id: 9, name: "Example", address: null, description: null, com_objects: objects,
    product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null }, last_download: usable };
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<DeviceWorkspace detail={detail} tree={tree} onApplied={vi.fn()} />));
  expect(host.querySelector(".com-last-download")?.textContent).toContain(en["comTable.lastDownload.differs.other"].replace("{count}", "5"));
  expect(host.querySelector('tr[data-object-id="21"]')?.getAttribute("data-differs-from-download")).toBe("true");
  await act(async () => { host.querySelector<HTMLElement>('tr[data-object-id="21"] summary')!.click(); });
  expect(host.querySelector('tr[data-editor-id="21"] [data-link-change="added"] .mono')?.textContent).toBe("1/515");
  await act(async () => root.unmount()); host.remove();
});
