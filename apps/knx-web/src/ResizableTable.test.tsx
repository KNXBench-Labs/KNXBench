/** Accessible header resizing isolates gestures from sorting, row selection and drag/drop. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import ResizableTable, { TableColumnReset } from "./ResizableTable";
import { TableWordFloors } from "./tableWordFloors";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
const columns = [{ id: "name", width: 180, min: 100, max: 400 }, { id: "address", width: 140, min: 80 }];
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
let host: HTMLDivElement; let root: ReturnType<typeof createRoot>;
const sort = vi.fn(); const select = vi.fn(); const drag = vi.fn();
beforeEach(async () => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  window.localStorage.clear(); sort.mockClear(); select.mockClear(); drag.mockClear();
  await act(async () => root.render(<StrictMode><ResizableTable tableId="component" columns={columns}>
    <thead><tr>{columns.map(column => <th key={column.id} onClick={sort} draggable onDragStart={drag}><button onClick={sort}>{column.id}</button></th>)}</tr></thead>
    <tbody><tr onClick={select}><td>Alpha</td><td>1.1.1</td></tr></tbody>
  </ResizableTable></StrictMode>));
});
afterEach(async () => { vi.restoreAllMocks(); await act(async () => root.unmount()); host.remove(); resetSettingsForTests(); resetUiLanguageForTests(); window.localStorage.clear(); });
const handle = () => host.querySelector<HTMLElement>('[role="separator"]')!;
const width = () => Number(handle().getAttribute("aria-valuenow"));
async function key(key: string, shiftKey = false) { await act(async () => handle().dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true }))); }
async function pointer(type: string, x: number, pointerId = 1) { await act(async () => handle().dispatchEvent(new PointerEvent(type, { clientX: x, pointerId, button: 0, bubbles: true, cancelable: true }))); if (type === "pointermove") await act(async () => { await new Promise<void>(resolve => requestAnimationFrame(() => resolve())); }); }

it("exposes named vertical separators with current, minimum and maximum values", () => {
  expect(host.querySelectorAll('[role="separator"]')).toHaveLength(2);
  expect(handle().tabIndex).toBe(0); expect(handle().getAttribute("aria-orientation")).toBe("vertical");
  expect(document.getElementById(handle().getAttribute("aria-labelledby")!)?.textContent).toBe("name");
  expect(handle().getAttribute("aria-valuemin")).toBe("100"); expect(handle().getAttribute("aria-valuemax")).toBe("400");
  expect(width()).toBe(180);
});
it("supports arrows, shift steps, bounds and column reset without sorting", async () => {
  handle().focus(); await key("ArrowRight"); expect(width()).toBe(188);
  await key("ArrowRight", true); expect(width()).toBe(228);
  await key("ArrowLeft"); expect(width()).toBe(220);
  await key("Home"); await key("ArrowLeft"); expect(width()).toBe(100);
  await key("End"); await key("ArrowRight"); expect(width()).toBe(400);
  await key("Enter"); expect(width()).toBe(180);
  expect(document.activeElement).toBe(handle()); expect(sort).not.toHaveBeenCalled(); expect(select).not.toHaveBeenCalled();
});
it("captures a pointer, clamps width, and never sorts, drags or selects a row", async () => {
  const capture = vi.fn(); const release = vi.fn(); handle().setPointerCapture = capture; handle().releasePointerCapture = release;
  await pointer("pointerdown", 200); expect(capture).toHaveBeenCalledWith(1);
  await pointer("pointermove", 250, 2); expect(width()).toBe(180);
  await pointer("pointermove", 250); expect(width()).toBe(230);
  expect(host.querySelector("col")?.style.width).toBe("230px");
  await pointer("pointermove", -1000); expect(width()).toBe(100);
  await pointer("pointerup", -1000); expect(release).toHaveBeenCalledWith(1);
  await pointer("pointermove", 500); expect(width()).toBe(100);
  await act(async () => { handle().click(); handle().dispatchEvent(new Event("dragstart", { bubbles: true, cancelable: true })); });
  expect(sort).not.toHaveBeenCalled(); expect(drag).not.toHaveBeenCalled(); expect(select).not.toHaveBeenCalled();
});
it("cancellation or capture loss ends the drag", async () => {
  handle().setPointerCapture = vi.fn();
  await pointer("pointerdown", 100); await pointer("pointermove", 120); expect(width()).toBe(200);
  await pointer("pointercancel", 120); await pointer("pointermove", 160); expect(width()).toBe(200);
  await pointer("pointerdown", 100); await pointer("lostpointercapture", 100); await pointer("pointermove", 160); expect(width()).toBe(200);
});
it("double-click resets a column and the table action resets all columns", async () => {
  await key("ArrowRight", true); expect(width()).toBe(220);
  await act(async () => handle().dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true })));
  expect(width()).toBe(180); await key("ArrowRight");
  await act(async () => host.querySelector<HTMLButtonElement>(".table-column-reset")!.click());
  expect(width()).toBe(180); expect(sort).not.toHaveBeenCalled();
  await act(async () => host.querySelector<HTMLButtonElement>("th button")!.click()); expect(sort).toHaveBeenCalled();
});

it("omits unnamed non-overflowing landmarks and names reset from its table", async () => {
  expect(host.querySelector(".resizable-table-scroll")!.hasAttribute("role")).toBe(false);
  expect(host.querySelector(".resizable-table-scroll")!.hasAttribute("tabindex")).toBe(false);
  await act(async () => root.render(<ResizableTable tableId="named" columns={columns} scrollLabel="Device list"><thead><tr><th>Name</th><th>Address</th></tr></thead></ResizableTable>));
  expect(host.querySelector(".resizable-table-scroll")!.getAttribute("role")).toBe("region");
  expect(host.querySelector(".table-column-reset")!.getAttribute("aria-label")).toBe("Reset column widths: Device list");
});
it("announces pixel units and references keyboard help", () => {
  expect(handle().getAttribute("aria-valuetext")).toBe("180 px");
  expect(document.getElementById(handle().getAttribute("aria-describedby")!)?.textContent).toContain("Left/Right");
});
it("coalesces drag previews and persists once on pointerup or lost capture", async () => {
  const write = vi.spyOn(window.localStorage, "setItem");
  handle().setPointerCapture = vi.fn(); handle().releasePointerCapture = vi.fn();
  await pointer("pointerdown", 100); await pointer("pointermove", 120); await pointer("pointermove", 140);
  expect(write).not.toHaveBeenCalled();
  await pointer("pointerup", 140); expect(width()).toBe(220); expect(write).toHaveBeenCalledTimes(1);
  await pointer("lostpointercapture", 140); expect(write).toHaveBeenCalledTimes(1);
  await pointer("pointerdown", 100); await pointer("pointermove", 120); await pointer("lostpointercapture", 120);
  expect(width()).toBe(240); expect(write).toHaveBeenCalledTimes(2);
  write.mockRestore();
});
it("Escape restores the drag start without persisting or sorting", async () => {
  const write = vi.spyOn(window.localStorage, "setItem"); handle().setPointerCapture = vi.fn(); handle().releasePointerCapture = vi.fn();
  await pointer("pointerdown", 100); await pointer("pointermove", 140); await key("Escape");
  await pointer("lostpointercapture", 140); expect(width()).toBe(180); expect(write).not.toHaveBeenCalled();
  expect(sort).not.toHaveBeenCalled(); write.mockRestore();
});
it("warns in development when decorated heading and column counts differ", async () => {
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
  await act(async () => root.render(<ResizableTable tableId="mismatch" columns={columns}><thead><tr><th>Only one</th></tr></thead></ResizableTable>));
  expect(warn).toHaveBeenCalledWith(expect.stringContaining("mismatch"), 1, 2); warn.mockRestore();
});

it.each(["en", "de"] as const)("%s reset identifies an unlabeled table by its localized name", async language => {
  await act(async () => setSetting("uiLanguage", language));
  await act(async () => root.render(<ResizableTable tableId="flow-values" columns={columns}><thead><tr><th>Name</th><th>Value</th></tr></thead></ResizableTable>));
  expect(host.querySelector(".table-column-reset")!.getAttribute("aria-label")).toBe(language === "de" ? "Spaltenbreiten zurücksetzen: Aktuelle Werte" : "Reset column widths: Current values");
  expect(host.querySelector(".table-column-reset")!.textContent).toBe(language === "de" ? "Spaltenbreiten zurücksetzen" : "Reset column widths");
});
it("publishes one completed drag to a peer and flushes the last unpainted move", async () => {
  await act(async () => root.render(<>{[0, 1].map(i => <ResizableTable key={i} tableId="peers" columns={columns}><thead><tr><th>Name</th><th>Address</th></tr></thead></ResizableTable>)}</>));
  const peers = () => host.querySelectorAll<HTMLElement>('[role="separator"]');
  peers()[0].setPointerCapture = vi.fn(); peers()[0].releasePointerCapture = vi.fn();
  await pointer("pointerdown", 100);
  await act(async () => { for (const x of [120, 140, 160]) peers()[0].dispatchEvent(new PointerEvent("pointermove", { pointerId: 1, clientX: x, bubbles: true })); });
  expect(peers()[2].getAttribute("aria-valuenow")).toBe("180");
  await pointer("pointerup", 160);
  expect(peers()[0].getAttribute("aria-valuenow")).toBe("240"); expect(peers()[2].getAttribute("aria-valuenow")).toBe("240");
});

it("announces measured heading pixels instead of the stored preference", async () => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width: this.tagName === "TH" ? 211 : 0 } as DOMRect;
  });
  await act(async () => root.render(<ResizableTable key="measured" tableId="measured" columns={columns}><thead><tr><th>Name</th><th>Address</th></tr></thead></ResizableTable>));
  expect(width()).toBe(211);
  expect(handle().getAttribute("aria-valuetext")).toBe("211 px");
  await key("ArrowRight");
  expect(JSON.parse(window.localStorage.getItem("knxbench:table-column-widths:v1:measured")!).name).toBe(188);
  expect(width()).toBe(211);
});
it("commits the last unpainted drag preview when its table unmounts", async () => {
  const write = vi.spyOn(window.localStorage, "setItem"); handle().setPointerCapture = vi.fn();
  await pointer("pointerdown", 100);
  await act(async () => handle().dispatchEvent(new PointerEvent("pointermove", { pointerId: 1, clientX: 157, bubbles: true })));
  await act(async () => root.render(null));
  expect(write).toHaveBeenCalledTimes(1);
  expect(JSON.parse(window.localStorage.getItem("knxbench:table-column-widths:v1:component")!).name).toBe(237);
});
it.each(["ArrowLeft", "ArrowRight", "Home", "End", "Enter"])("ignores %s during a pointer drag", async pressed => {
  const write = vi.spyOn(window.localStorage, "setItem"); handle().setPointerCapture = vi.fn(); handle().releasePointerCapture = vi.fn();
  await pointer("pointerdown", 100); await pointer("pointermove", 140);
  await key(pressed);
  expect(write).not.toHaveBeenCalled(); expect(width()).toBe(220);
  await pointer("pointerup", 140);
  expect(write).toHaveBeenCalledTimes(1); expect(width()).toBe(220);
});
it("uses the caller's trailing width without fitting its fixed columns", async () => {
  const trailing = { fixedTrailingColumns: 1, fixedTrailingWidth: 96 };
  await act(async () => root.render(<ResizableTable tableId="trailing" columns={columns} {...trailing}><thead><tr><th>Name</th><th>Address</th><th>Fixed</th></tr></thead></ResizableTable>));
  expect(host.querySelector("col:last-child")?.getAttribute("style")).toContain("width: 96px");
  expect(host.querySelector("table")?.style.width).toBe("416px");
});

it("fits to the widest header or cell token, including its padding and computed font", async () => {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width: this.hasAttribute("data-table-word-measure") ? this.textContent!.length * parseFloat(this.style.fontSize) : 0 } as DOMRect;
  });
  await act(async () => root.render(<ResizableTable tableId="words" columns={columns}>
    <thead><tr><th style={{ padding: "0 12px", fontSize: 10 }}>LongestHeader short</th><th style={{ padding: "0 12px", fontSize: 10 }}>Small</th></tr></thead>
    <tbody><tr><td style={{ padding: "0 12px", fontSize: 10 }}>small words</td><td style={{ padding: "0 12px", fontSize: 10 }}><strong style={{ fontSize: 12 }}>Beleuchtungsstärke</strong> (9.004)</td></tr></tbody>
  </ResizableTable>));
  expect(Array.from(host.querySelectorAll("col"), col => parseFloat(col.style.width))).toEqual([154, 240]);
  expect(parseFloat(host.querySelector("table")!.style.width)).toBe(394);
});

it("caps pathological tokens and caches measurement through resizing until content changes", async () => {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
  const measured: string[] = [];
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    if (!this.hasAttribute("data-table-word-measure")) return { width: 0 } as DOMRect;
    measured.push(this.textContent!);
    return { width: this.textContent!.length * 10 } as DOMRect;
  });
  const renderWords = (text: string) => <ResizableTable tableId="cached-words" columns={columns}>
    <thead><tr><th>Name</th><th>Address</th></tr></thead><tbody><tr><td>{text}</td><td>1.1.1</td></tr></tbody>
  </ResizableTable>;
  await act(async () => root.render(renderWords("x".repeat(100))));
  expect(host.querySelector("col")!.style.width).toBe("320px");
  const count = measured.length;
  const addressCount = measured.filter(text => text === "Address").length;
  const computedStyle = vi.spyOn(globalThis, "getComputedStyle");
  await key("ArrowRight"); await key("Enter");
  expect(measured).toHaveLength(count);
  await act(async () => root.render(renderWords("x".repeat(100))));
  expect(computedStyle).not.toHaveBeenCalled();
  const unchangedCell = host.querySelector("tbody td:last-child");
  await act(async () => root.render(renderWords("Freshword")));
  expect(host.querySelector("col")!.style.width).toBe("320px");
  expect(computedStyle.mock.calls.some(([element]) => element === unchangedCell)).toBe(false);
  expect(measured).toContain("Freshword");
  expect(measured.filter(text => text === "Address")).toHaveLength(addressCount);
});

it("defers hidden-pane word measurement until the viewport becomes visible", async () => {
  let paneWidth = 0;
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(() => paneWidth);
  const words: string[] = [];
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    if (this.hasAttribute("data-table-word-measure")) { words.push(this.textContent!); return { width: 250 } as DOMRect; }
    return { width: 0 } as DOMRect;
  });
  const table = () => <ResizableTable tableId="hidden-pane" columns={columns}>
    <thead><tr><th>Name</th><th>Address</th></tr></thead><tbody><tr><td>Windgeschwindigkeit</td><td>1.1.1</td></tr></tbody>
  </ResizableTable>;
  await act(async () => root.render(<div hidden>{table()}</div>));
  expect(words).toEqual([]);
  paneWidth = 300;
  await act(async () => root.render(<div>{table()}</div>));
  expect(words).toContain("Windgeschwindigkeit");
  expect(host.querySelector("col")!.style.width).toBe("250px");
});

it("invalidates only a changed cell in a large table without serializing unchanged cells", async () => {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width: this.hasAttribute("data-table-word-measure") ? this.textContent!.length * 10 : 0 } as DOMRect;
  });
  await act(async () => root.render(<ResizableTable tableId="large-mutations" columns={columns}>
    <thead><tr><th>Name</th><th>Address</th></tr></thead>
    <tbody>{Array.from({ length: 1000 }, (_, i) => <tr key={i}><td>Device {i}</td><td>1.1.{i}</td></tr>)}</tbody>
  </ResizableTable>));
  const serialized = vi.spyOn(Element.prototype, "outerHTML", "get");
  const walked = vi.spyOn(document, "createTreeWalker");
  const styled = vi.spyOn(globalThis, "getComputedStyle");
  const changed = host.querySelector("tbody td")!;
  await act(async () => { changed.firstChild!.textContent = "Freshmeasurement"; });
  expect(walked.mock.calls.map(([cell]) => cell)).toEqual([changed]);
  expect(serialized).not.toHaveBeenCalled();
  expect(styled.mock.calls.every(([element]) => changed.contains(element))).toBe(true);
  walked.mockClear(); styled.mockClear(); serialized.mockClear();
  await act(async () => {
    handle().setAttribute("aria-valuenow", "123");
    handle().classList.add("focused");
    host.querySelector("tbody tr")!.classList.add("bus-monitor-row-new");
    host.querySelector("tbody tr")!.setAttribute("hidden", "");
  });
  expect(walked).not.toHaveBeenCalled();
  expect(styled).not.toHaveBeenCalled();
  expect(serialized).not.toHaveBeenCalled();
});

it("keeps session floors after filtering, grows for longer content and starts fresh on remount", async () => {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width: this.hasAttribute("data-table-word-measure") ? this.textContent!.length * 10 : 0 } as DOMRect;
  });
  const table = (text: string, show: boolean, key = "session") => <ResizableTable key={key} tableId="stable-floors" columns={columns}>
    <thead><tr><th>Name</th><th>Address</th></tr></thead>
    <tbody><tr><td>Short</td><td>1.1.1</td></tr>{show && <tr><td>{text}</td><td>1.1.2</td></tr>}</tbody>
  </ResizableTable>;
  await act(async () => root.render(table("Longercontent", true)));
  expect(host.querySelector("col")!.style.width).toBe("130px");
  await act(async () => root.render(table("Longercontent", false)));
  expect(host.querySelector("col")!.style.width).toBe("130px");
  await act(async () => root.render(table("Evenlongercontent", true)));
  expect(host.querySelector("col")!.style.width).toBe("170px");
  await act(async () => root.render(table("", false)));
  expect(host.querySelector("col")!.style.width).toBe("170px");
  await act(async () => host.querySelector<HTMLButtonElement>(".table-column-reset")!.click());
  expect(host.querySelector("col")!.style.width).toBe("50px");
  await act(async () => root.render(table("Evenlongercontent", true)));
  expect(host.querySelector("col")!.style.width).toBe("170px");
  await act(async () => root.render(table("", false, "new-session")));
  expect(host.querySelector("col")!.style.width).toBe("50px");
});


it("owns bounded ancestor attribute subscriptions and disconnects them on unmount", async () => {
  const observe = vi.spyOn(MutationObserver.prototype, "observe");
  const disconnect = vi.spyOn(MutationObserver.prototype, "disconnect");
  await act(async () => root.render(<section className="pane"><ResizableTable key="bounded" tableId="bounded" columns={columns}>
    <thead><tr><th>Name</th><th>Address</th></tr></thead><tbody><tr><td>Device</td><td>1.1.1</td></tr></tbody>
  </ResizableTable></section>));
  const table = host.querySelector("table")!;
  const ancestry: Element[] = [];
  for (let e: Element | null = table; e; e = e.parentElement) ancestry.push(e);
  const attributes = observe.mock.calls.filter(([, options]) => options?.attributes && !options.subtree);
  expect(attributes.map(([target]) => target)).toEqual(ancestry);
  const subtrees = observe.mock.calls.filter(([, options]) => options?.subtree);
  expect(subtrees.map(([target]) => (target as Element).tagName)).toEqual(["THEAD", "TBODY"]);
  expect(subtrees.every(([, options]) => options?.attributeFilter?.includes("colspan"))).toBe(true);
  disconnect.mockClear();
  await act(async () => root.render(null));
  expect(disconnect).toHaveBeenCalledTimes(2);
});


it("does not remeasure words for ancestor pane width and height edits", async () => {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
  await act(async () => root.render(<section className="pane"><ResizableTable key="geometry" tableId="geometry" columns={columns}>
    <thead><tr><th>Name</th><th>Address</th></tr></thead><tbody><tr><td>Device</td><td>1.1.1</td></tr></tbody>
  </ResizableTable></section>));
  const walked = vi.spyOn(document, "createTreeWalker");
  await act(async () => { host.querySelector("section")!.style.width = "80px"; host.querySelector("section")!.style.height = "200px"; });
  expect(walked).not.toHaveBeenCalled();
});

it.each(["padding-top", "padding-bottom", "padding-left", "margin-right", "min-width", "max-height", "inset", "transform", "translate", "scroll-margin-top"])(
  "ancestor %s geometry changes neither clear caches nor walk text", async property => {
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
    await act(async () => root.render(<section style={{ fontSize: "14px", wordSpacing: "2px" }}><ResizableTable key="noise" tableId="noise" columns={columns}>
      <thead><tr><th>Name</th><th>Address</th></tr></thead><tbody><tr><td>Device</td><td>1.1.1</td></tr></tbody>
    </ResizableTable></section>));
    const cleared = vi.spyOn(TableWordFloors.prototype, "clear");
    const walked = vi.spyOn(document, "createTreeWalker");
    const styled = vi.spyOn(globalThis, "getComputedStyle");
    const section = host.querySelector("section")!;
    const before = section.getAttribute("style");
    const records: MutationRecord[] = [];
    const witness = new MutationObserver(list => records.push(...list));
    witness.observe(section, { attributes: true, attributeFilter: ["style"] });
    await act(async () => { section.style.setProperty(property,
      property === "transform" ? "translateY(100px)" : "100px"); });
    witness.disconnect();
    // Positive control: the DOM must really have changed and reported it, or
    // this case would pass without exercising the observer at all.
    expect(section.getAttribute("style")).not.toBe(before);
    expect(records.length).toBeGreaterThan(0);
    expect(cleared).not.toHaveBeenCalled();
    expect(walked).not.toHaveBeenCalled();
    expect(styled).not.toHaveBeenCalled();
  });

it.each([
  ["font shorthand", (pane: HTMLElement) => { pane.style.setProperty("font", "20px serif"); }],
  ["!important priority flip", (pane: HTMLElement) => { pane.style.setProperty("font-size", "14px", "important"); }],
  ["root data-density flip", () => { document.documentElement.setAttribute("data-density", "comfortable"); }],
] as const)("ancestor %s remeasures once", async (_name, change) => {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
  await act(async () => root.render(<section style={{ fontSize: "14px" }}><ResizableTable key="shorthand" tableId="shorthand" columns={columns}>
    <thead><tr><th>Name</th><th>Address</th></tr></thead><tbody><tr><td>Device</td><td>1.1.1</td></tr></tbody>
  </ResizableTable></section>));
  const cleared = vi.spyOn(TableWordFloors.prototype, "clear");
  const pane = host.querySelector("section")!;
  const density = document.documentElement.getAttribute("data-density");
  try {
    await act(async () => change(pane));
    expect(cleared).toHaveBeenCalledTimes(1);
  } finally {
    if (density === null) document.documentElement.removeAttribute("data-density");
    else document.documentElement.setAttribute("data-density", density);
  }
});

it.each(["font-size", "--knx-cell-padding", "letter-spacing", "word-spacing", "text-transform", "line-height"])(
  "ancestor %s presentation changes remeasure, including removal", async property => {
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
    await act(async () => root.render(<section><ResizableTable key="appearance" tableId="appearance" columns={columns}>
      <thead><tr><th>Name</th><th>Address</th></tr></thead><tbody><tr><td>Device</td><td>1.1.1</td></tr></tbody>
    </ResizableTable></section>));
    const cleared = vi.spyOn(TableWordFloors.prototype, "clear");
    const walked = vi.spyOn(document, "createTreeWalker");
    const pane = host.querySelector("section")!;
    await act(async () => { pane.style.setProperty(property, property === "text-transform" ? "uppercase" : "20px"); });
    expect(cleared).toHaveBeenCalledTimes(1);
    expect(walked).toHaveBeenCalledTimes(4);
    cleared.mockClear(); walked.mockClear();
    await act(async () => { pane.style.removeProperty(property); });
    expect(cleared).toHaveBeenCalledTimes(1);
    expect(walked).toHaveBeenCalledTimes(4);
  });

it.each(["en", "de"] as const)("%s owner reset visibly names its repeated table kind", async language => {
  await act(async () => setSetting("uiLanguage", language));
  await act(async () => root.render(<TableColumnReset tableId="project-diff-fields" columns={columns} />));
  const button = host.querySelector("button")!;
  const label = language === "de" ? "Spaltenbreiten zurücksetzen: Feldänderungen" : "Reset column widths: Field changes";
  expect(button.textContent).toBe(label);
  expect(button.getAttribute("aria-label") ?? button.textContent).toBe(label);
});
