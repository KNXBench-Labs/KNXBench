/** Named values, translated decoder refusals and always-visible raw bytes. */
// @vitest-environment happy-dom
import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import ComparisonTable from "./ComparisonTable";
import { setSetting, resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
let dispose = () => {};
afterEach(async () => { await act(async () => dispose()); resetSettingsForTests(); resetUiLanguageForTests(); window.localStorage.clear(); });
it.each(["en", "de"] as const)("%s shows exact named values and explains nondecodable bits without hiding raw", async (language) => {
  await act(async () => setSetting("uiLanguage", language));
  const host = document.createElement("div");document.body.append(host);const root = createRoot(host);
  dispose = () => {root.unmount();host.remove();};
  await act(async () => root.render(<ComparisonTable rows={[
    { kind: "parameter", label: "Delay · R-1 · bit 5 / 9", address: 0x4400, project: "235", device: "345", reason: null },
    { kind: "parameter", label: "Text · R-2 · bit 0 / 48", address: 0x4410, project: "5 € ok", device: null, reason: "parameterNotDecoded" },
    { kind: "raw", label: "", address: 0x4400, project: "AB AC", device: "FD 64", reason: null },
  ]} />));
  const rows=host.querySelectorAll("tbody tr");expect(rows).toHaveLength(3);
  expect(rows[0].querySelectorAll("td")[0].textContent).toBe("235");
  expect(rows[0].querySelectorAll("td")[1].textContent).toBe("345");
  expect(rows[0].getAttribute("data-different")).toBe("true");
  expect(rows[1].querySelector(".comparison-reason")?.textContent).not.toBe("parameterNotDecoded");
  expect(rows[1].querySelector(".comparison-reason")?.textContent).toContain(language === "de" ? "Rohbytes" : "raw bytes");
  expect(rows[1].textContent).toContain((language === "de" ? de : en)["compare.unknown"]);
  expect(rows[2].textContent).toContain("FD 64");expect(rows[2].closest("details")).toBeNull();
});

async function mount(rows: Parameters<typeof ComparisonTable>[0]["rows"], changes?: Parameters<typeof ComparisonTable>[0]["changes"]) {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  dispose = () => { root.unmount(); host.remove(); };
  await act(async () => root.render(<ComparisonTable rows={rows} changes={changes} />));
  return host;
}
const marks = (cell: Element) => [...cell.querySelectorAll(".comparison-byte")].map((b) => [b.textContent, b.getAttribute("data-mark")]);
const toggle = (host: HTMLElement) => host.querySelector<HTMLInputElement>(".comparison-filter input[type=checkbox]")!;
const mixed = [
  { kind: "groupAddress", label: "1", address: 0x4003, project: "2/2/3", device: "2/2/3", reason: null },
  { kind: "groupAddress", label: "2", address: 0x4005, project: "2/2/3", device: "2/2/4", reason: null },
  { kind: "parameter", label: "Text · R-2", address: 0x4410, project: "ok", device: null, reason: "parameterNotDecoded" },
  { kind: "raw", label: "", address: 0x4000, project: "21 22", device: "21 22", reason: null },
  { kind: "raw", label: "", address: 0x4100, project: "01 02 03 04 05", device: "01 FF FE 04 06", reason: null },
  { kind: "raw", label: "", address: 0x4200, project: null, device: "AA BB", reason: null },
];

it.each(["en", "de"] as const)("%s marks differing and unread bytes on both sides with a count, not colour alone", async (language) => {
  await act(async () => setSetting("uiLanguage", language)); const m = language === "de" ? de : en;
  const host = await mount(mixed);
  const rows = [...host.querySelectorAll("tbody tr")];
  const differing = rows.find((row) => row.textContent?.includes("FE"))!;
  const [project, device] = differing.querySelectorAll("td");
  const pattern = [["01", "equal"], ["02", "different"], ["03", "different"], ["04", "equal"], ["05", "different"]];
  expect(marks(project)).toEqual(pattern);
  expect(marks(device)).toEqual(pattern.map(([, mark], i) => [["01", "FF", "FE", "04", "06"][i], mark]));
  expect(project.textContent).toBe("01 02 03 04 05");
  expect(differing.getAttribute("data-different")).toBe("true");
  expect(differing.querySelector(".comparison-byte-count")?.textContent).toBe(m["compare.bytesDifferent.other"].replace("{count}", "3"));
  expect(device.querySelector("[data-mark=different]")?.getAttribute("title")).toBe(m["compare.byteDifferent"]);
  const unread = rows.find((row) => row.textContent?.includes("AA BB"))!;
  expect(unread.querySelectorAll("td")[0].textContent).toBe(m["compare.unknown"]);
  expect(marks(unread.querySelectorAll("td")[1])).toEqual([["AA", "unknown"], ["BB", "unknown"]]);
  expect(unread.querySelector(".comparison-byte-count")?.textContent).toBe(m["compare.bytesUnknown.other"].replace("{count}", "2"));
  expect(unread.querySelector("[data-mark=unknown]")?.getAttribute("title")).toBe(m["compare.byteUnknown"]);
  expect(unread.hasAttribute("data-different")).toBe(false);
});

it("marks a byte the server reports in its changes even where the shown text matches", async () => {
  const host = await mount([{ kind: "raw", label: "", address: 0x4400, project: "12 34", device: "12 34", reason: null }],
    [{ address: 0x4401, segment: null, device: [0x34], project: [0x35] }]);
  const row = host.querySelector("tbody tr")!;
  expect(marks(row.querySelectorAll("td")[1])).toEqual([["12", "equal"], ["34", "different"]]);
  expect(row.querySelector(".comparison-byte-count")?.textContent).toBe("1 differing byte");
});

it.each(["en", "de"] as const)("%s opens with differences only and the toggle reveals every original read range", async (language) => {
  await act(async () => setSetting("uiLanguage", language)); const m = language === "de" ? de : en;
  const host = await mount(mixed);
  const box = toggle(host);
  expect(box.checked).toBe(false);
  expect(host.querySelector(".comparison-filter")?.textContent).toBe(m["compare.showEqual"].replace("{count}", "2"));
  expect(host.querySelector(".comparison-hidden")?.textContent).toBe(m["compare.hiddenEqual.other"].replace("{count}", "2"));
  const location = () => [...host.querySelectorAll("tbody tr td.mono")].map((cell) => cell.textContent);
  expect(location()).toEqual(["4005h", "4410h", "4100h", "4200h"]);
  await act(async () => box.click());
  expect(toggle(host).checked).toBe(true);
  expect(location()).toEqual(["4003h", "4005h", "4410h", "4000h", "4100h", "4200h"]);
  expect([...host.querySelectorAll("tbody tr.comparison-raw td.mono")].map((cell) => cell.textContent)).toEqual(["4000h", "4100h", "4200h"]);
  expect(host.querySelector(".comparison-hidden")).toBeNull();
  await act(async () => toggle(host).click());
  expect(location()).toEqual(["4005h", "4410h", "4100h", "4200h"]);
});

it("explains an all-equal result instead of showing an empty table without a reason", async () => {
  const host = await mount([
    { kind: "association", label: "0", address: 0x4101, project: "TSAP 1", device: "TSAP 1", reason: null },
    { kind: "raw", label: "", address: 0x4000, project: "21", device: "21", reason: null },
  ]);
  expect(host.querySelectorAll("tbody tr")).toHaveLength(0);
  expect(host.querySelector(".comparison-hidden")?.textContent).toBe("2 equal rows are hidden. Only differing, unread and not decoded rows are shown.");
  expect(host.querySelector(".comparison-no-differences")?.textContent).toBe(en["compare.noDifferences"]);
  await act(async () => toggle(host).click());
  expect(host.querySelectorAll("tbody tr")).toHaveLength(2);
  expect(host.querySelector(".comparison-no-differences")).toBeNull();
});

it("keeps rows with unknown sides and results without raw ranges visible under the filter", async () => {
  const host = await mount([
    { kind: "association", label: "0", address: 0x4101, project: "TSAP 1", device: "TSAP 1", reason: null },
    { kind: "association", label: "1", address: 0x4103, project: null, device: null, reason: "Missing" },
    { kind: "parameter", label: "P", address: 0x4420, project: "12", device: null, reason: "parameterNotDecoded" },
  ]);
  expect([...host.querySelectorAll("tbody tr td.mono")].map((cell) => cell.textContent)).toEqual(["4103h", "4420h"]);
  expect(host.querySelector(".comparison-hidden")?.textContent).toBe("1 equal row is hidden. Only differing, unread and not decoded rows are shown.");
});

it("keeps a raw range with a single unread byte visible and marks only that byte", async () => {
  const host = await mount([{ kind: "raw", label: "", address: 0x4600, project: "01 02", device: "01", reason: null }]);
  const row = host.querySelector("tbody tr")!;
  expect(host.querySelector(".comparison-hidden")).toBeNull();
  expect(marks(row.querySelectorAll("td")[0])).toEqual([["01", "equal"], ["02", "unknown"]]);
  expect(row.querySelector(".comparison-byte-count")?.textContent).toBe("1 byte not read or not comparable");
});

it("offers keyboard resizing and reset on the production comparison table", async () => {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  dispose = () => { root.unmount(); host.remove(); };
  await act(async () => root.render(<ComparisonTable rows={[]} />));
  expect(host.querySelectorAll('[role="separator"]')).toHaveLength(4);
  const handle = host.querySelector<HTMLElement>('[role="separator"]')!;
  const initial = Number(handle.getAttribute("aria-valuenow"));
  await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(Number(handle.getAttribute("aria-valuenow"))).toBe(initial + 8);
  const reset = host.querySelector<HTMLButtonElement>(".table-column-reset")!;
  expect(reset.textContent).toBe("Reset column widths");
  await act(async () => reset.click());
  expect(Number(handle.getAttribute("aria-valuenow"))).toBe(initial);
});

tableColumnSmoke([["device-comparison", 4]], async () => {
  const host = document.createElement("div"); const root = createRoot(host);
  dispose = () => root.unmount(); await act(async () => root.render(<ComparisonTable rows={[]} />)); return { host };
});
