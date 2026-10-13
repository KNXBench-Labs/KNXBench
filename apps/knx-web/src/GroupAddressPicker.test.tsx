/** Issue #8: the searchable group-address picker's keyboard, filter and DPT marking. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it } from "vitest";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import GroupAddressPicker from "./GroupAddressPicker";
import { PICKER_WINDOW } from "./groupAddressPicker";

const ga = (id: number, address: string, name: string, dpts: string[] = []): GroupAddressNode =>
  ({ id, address, name, range: null, dpts, links: [] });
const LIST = [ga(1, "1/0/1", "Kitchen ceiling", ["DPST-1-1"]), ga(2, "1/0/2", "Kitchen dimming", ["DPST-5-1"]),
  ga(3, "2/1/7", "Hallway", []), ga(4, "3/3/3", "Blind", ["DPST-1-8"])];

let host: HTMLDivElement; let root: Root; let chosen: (number | null)[];
beforeEach(() => { host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host); chosen = []; });
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

function Harness({ addresses = LIST, objectDpt = "DPST-1-1" }: { addresses?: GroupAddressNode[]; objectDpt?: string | null }) {
  const [value, setValue] = useState<number | null>(null);
  return <>
    <GroupAddressPicker addresses={addresses} objectDpt={objectDpt} value={value}
      onChange={id => { chosen.push(id); setValue(id); }} />
    <button type="button" data-reset onClick={() => setValue(null)}>reset</button>
  </>;
}
const input = () => host.querySelector<HTMLInputElement>("input[role=combobox]")!;
const options = () => [...host.querySelectorAll<HTMLElement>("[role=option]")];
const optionIds = () => options().map(o => Number(o.dataset.gaOption));
async function type(text: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input(), text);
    input().dispatchEvent(new Event("input", { bubbles: true }));
  });
}
async function key(name: string) {
  let event!: KeyboardEvent;
  await act(async () => { event = new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true }); input().dispatchEvent(event); });
  return event;
}
async function focus() { await act(async () => { input().focus(); input().dispatchEvent(new FocusEvent("focus")); }); }

it("is a labelled combobox that lists every linkable address, in order, when opened", async () => {
  await act(async () => root.render(<Harness />));
  expect(input().getAttribute("aria-label")).toBeTruthy();
  expect(input().getAttribute("aria-expanded")).toBe("false");
  await focus();
  expect(input().getAttribute("aria-expanded")).toBe("true");
  expect(optionIds()).toEqual([1, 2, 3, 4]);
  const listbox = host.querySelector("[role=listbox]")!;
  expect(input().getAttribute("aria-controls")).toBe(listbox.id);
});

it("filters while typing over address, name and DPT, keeping the order", async () => {
  await act(async () => root.render(<Harness />));
  await focus(); await type("kitchen");
  expect(optionIds()).toEqual([1, 2]);
  await type("5.001");
  expect(optionIds()).toEqual([2]);
  await type("nothing like this");
  expect(optionIds()).toEqual([]);
  expect(host.textContent).toContain("No group address matches");
});

it("marks a DPT mismatch and a subtype difference on the option, never hiding or moving it", async () => {
  await act(async () => root.render(<Harness />));
  await focus();
  const agreement = Object.fromEntries(options().map(o => [o.dataset.gaOption, o.dataset.dptAgreement]));
  expect(agreement).toEqual({ 1: "match", 2: "mismatch", 3: "unknown", 4: "subtype" });
  const [kitchen, dimming, hallway, blind] = options();
  expect(kitchen.textContent).not.toMatch(/different/i);
  expect(dimming.textContent).toContain("Different type than the object");
  expect(dimming.textContent).toContain("Scaling");
  expect(blind.textContent).toContain("different subtype");
  expect(hallway.textContent).toContain("none stated");
});

it("does not mark anything when the object has no DPT", async () => {
  await act(async () => root.render(<Harness objectDpt={null} />));
  await focus();
  expect(options().map(o => o.dataset.dptAgreement)).toEqual(["unknown", "unknown", "unknown", "unknown"]);
});

it("moves with the arrow keys, chooses with Enter and closes with Escape", async () => {
  await act(async () => root.render(<Harness />));
  await focus();
  expect(options()[0].getAttribute("aria-selected")).toBe("true");
  await key("ArrowDown"); await key("ArrowDown");
  expect(input().getAttribute("aria-activedescendant")).toBe(options()[2].id);
  await key("ArrowUp");
  const enter = await key("Enter");
  expect(enter.defaultPrevented).toBe(true);
  expect(chosen).toEqual([2]);
  expect(input().getAttribute("aria-expanded")).toBe("false");
  expect(input().value).toContain("1/0/2");
  expect(input().value).toContain("Kitchen dimming");
  await key("ArrowDown");
  expect(input().getAttribute("aria-expanded")).toBe("true");
  expect(optionIds()).toEqual([1, 2, 3, 4]);
  const escape = await key("Escape");
  expect(escape.defaultPrevented).toBe(true);
  expect(input().getAttribute("aria-expanded")).toBe("false");
  const second = await key("Escape");
  expect(second.defaultPrevented).toBe(false);
});

it("stops at the ends of the list instead of wrapping", async () => {
  await act(async () => root.render(<Harness />));
  await focus();
  await key("ArrowUp");
  expect(input().getAttribute("aria-activedescendant")).toBe(options()[0].id);
  for (let i = 0; i < 10; i++) await key("ArrowDown");
  expect(input().getAttribute("aria-activedescendant")).toBe(options()[3].id);
});

it("chooses with a pointer and clears the choice as soon as the text changes", async () => {
  await act(async () => root.render(<Harness />));
  await focus();
  await act(async () => {
    options()[3].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
    options()[3].dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
  expect(chosen).toEqual([4]);
  await type("Blind x");
  expect(chosen).toEqual([4, null]);
});

it("empties itself when the owner resets the choice after linking", async () => {
  await act(async () => root.render(<Harness />));
  await focus(); await key("Enter");
  expect(input().value).toContain("1/0/1");
  await act(async () => host.querySelector<HTMLButtonElement>("[data-reset]")!.click());
  expect(input().value).toBe("");
});

it("renders a bounded window and says how many it shows", async () => {
  const many = Array.from({ length: PICKER_WINDOW + 25 }, (_, i) => ga(i + 1, `1/1/${i}`, `Address ${i}`));
  await act(async () => root.render(<Harness addresses={many} />));
  await focus();
  expect(options()).toHaveLength(PICKER_WINDOW);
  expect(host.textContent).toContain(`Showing ${PICKER_WINDOW} of ${PICKER_WINDOW + 25}`);
  await type("Address 124");
  expect(optionIds()).toEqual([125]);
});
