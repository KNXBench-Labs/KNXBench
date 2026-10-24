/** Association rows: strict selection admission and the link changes in the review. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import ComparisonAdoptionPanel from "./ComparisonAdoptionPanel";
import { offersAdoption, parseAdoptionPreview, type ComparisonAdoptionPreview } from "./comparisonAdoptionApi";
import type { BoundDeviceComparison, DecodedComparisonRow } from "./deviceComparisons";
import { setSetting, resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

let dispose = () => {};
afterEach(async () => {
  await act(async () => dispose());
  vi.unstubAllGlobals(); resetSettingsForTests(); resetUiLanguageForTests(); window.localStorage.clear();
});

const decoded: DecodedComparisonRow[] = [
  { index: 0, adoptable: false, kind: "associationCount", label: "", address: 0x4100, project: "1", device: "3", reason: null },
  { index: 1, adoptable: true, kind: "association", label: "0", address: 0x4101, project: "TSAP 1 → 1/0/1 · object 0", device: "TSAP 2 → 1/0/2 · object 0", reason: null },
  // A server hint on a kind the client does not admit is ignored.
  { index: 2, adoptable: true, kind: "groupAddress", label: "2", address: 0x4005, project: null, device: "1/0/2", reason: "Missing" },
  { index: 3, adoptable: false, kind: "association", label: "1", address: 0x4103, project: null, device: null, reason: "Missing" },
];
const preview: ComparisonAdoptionPreview = {
  previewToken: "p".repeat(64), selected: [1], changed: true, rows: decoded, expiresInSeconds: 120,
  linkChanges: [
    { add: false, object: 0, comObjectId: 1, groupAddress: "1/0/1", groupAddressId: 1, sending: true },
    { add: true, object: 0, comObjectId: 1, groupAddress: "1/0/2", groupAddressId: 2, sending: true },
  ],
};
const result = { observationToken: "o".repeat(64), decoded, changes: [] } as unknown as BoundDeviceComparison;

it("admits only server-hinted parameter and association rows at their original index", () => {
  expect(decoded.map((row, index) => offersAdoption(row, index))).toEqual([false, true, false, false]);
  expect(offersAdoption({ ...decoded[1], index: 7 }, 1)).toBe(false);
  expect(offersAdoption({ ...decoded[1], adoptable: undefined }, 1)).toBe(false);
});

it.each([undefined, null, {}, "hidden", [{ add: "yes" }]])("refuses a preview whose linkChanges value is missing or malformed: %s", (linkChanges) => {
  expect(() => parseAdoptionPreview({ ...preview, linkChanges } as unknown as ComparisonAdoptionPreview)).toThrow();
});

it("keeps parameter-only preview compatibility through an explicit empty linkChanges array", () => {
  const parameterPreview: ComparisonAdoptionPreview = {
    ...preview,
    selected: [0],
    rows: [{ index: 0, adoptable: true, kind: "parameter", label: "P", address: 0x440f, project: "0", device: "1", reason: null }],
    linkChanges: [],
  };
  expect(parseAdoptionPreview(parameterPreview).linkChanges).toEqual([]);
});

it("does not offer Apply when an association preview omits linkChanges", async () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.stubGlobal("fetch", vi.fn(async () => {
    const { linkChanges: _, ...malformed } = preview;
    return new Response(JSON.stringify(malformed), { status: 200, headers: { "Content-Type": "application/json" } });
  }));
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  dispose = () => { root.unmount(); host.remove(); };
  await act(async () => root.render(<ComparisonAdoptionPanel result={result} disabled={false} onBusy={() => {}} onApplied={() => {}} />));
  const choice = host.querySelector<HTMLInputElement>("tbody input[type=checkbox]")!;
  await act(async () => choice.click());
  const review = [...host.querySelectorAll("button")].find((button) => button.textContent === en["compare.adoption.preview"])!;
  await act(async () => review.click());
  expect(host.querySelector("[role=alert]")).not.toBeNull();
  expect([...host.querySelectorAll("button")].some((button) => button.textContent === en["compare.adoption.apply"])).toBe(false);
});

it.each(["en", "de"] as const)("%s previews association rows with their link changes before confirmation", async (language) => {
  await act(async () => setSetting("uiLanguage", language));
  const m = language === "de" ? de : en;
  const bodies: unknown[] = [];
  vi.stubGlobal("fetch", vi.fn(async (_path: string, init?: RequestInit) => {
    bodies.push(JSON.parse(String(init?.body)));
    return new Response(JSON.stringify(preview), { status: 200, headers: { "Content-Type": "application/json" } });
  }));
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  dispose = () => { root.unmount(); host.remove(); };
  await act(async () => root.render(<ComparisonAdoptionPanel result={result} disabled={false} onBusy={() => {}} onApplied={() => {}} />));
  expect(host.textContent).toContain(m["compare.adoption.title"]);
  const choices = host.querySelectorAll<HTMLInputElement>("tbody input[type=checkbox]");
  expect(choices).toHaveLength(1);
  expect(choices[0].getAttribute("aria-label")).toContain(m["compare.kind.association"]);
  await act(async () => choices[0].click());
  const review = [...host.querySelectorAll("button")].find((b) => b.textContent === m["compare.adoption.preview"])!;
  await act(async () => review.click());
  expect(bodies).toEqual([{ observationToken: "o".repeat(64), selected: [1] }]);
  const links = [...host.querySelectorAll(".comparison-adoption-links li")];
  expect(links.map((li) => li.getAttribute("data-change"))).toEqual(["remove", "add"]);
  expect(links[0].textContent).toContain(m["compare.adoption.linkRemove"].replace("{object}", "0").replace("{address}", "1/0/1"));
  expect(links[1].textContent).toContain(m["compare.adoption.linkAdd"].replace("{object}", "0").replace("{address}", "1/0/2"));
  expect(links[0].textContent).toContain(m["compare.adoption.linkSending"]);
  const apply = [...host.querySelectorAll("button")].find((b) => b.textContent === m["compare.adoption.apply"])!;
  expect(apply.disabled).toBe(true); // a separate confirmation is still required
});
