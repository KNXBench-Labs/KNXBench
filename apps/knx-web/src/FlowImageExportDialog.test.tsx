/** Explicit image scope and consent gate precede every local download. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import FlowImageExportDialog from "./FlowImageExportDialog";
import { selectSvgViewport } from "./flowImageExport";
import type { RasterFrame } from "./flowImageCapture";
const raster = vi.hoisted(() => vi.fn());
vi.mock("./flowImageExport", async original => ({ ...await original<typeof import("./flowImageExport")>(), rasterizeSelectedSvg: raster }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let root: Root | undefined; let host: HTMLDivElement;
afterEach(async () => { if (root) await act(async () => root!.unmount()); root = undefined; document.body.innerHTML = ""; vi.restoreAllMocks(); vi.unstubAllGlobals(); raster.mockReset(); });
it("requires selected-view consent and refuses a duration over thirty seconds", async () => {
  const OriginalURL = URL; class UrlControl extends OriginalURL { static createObjectURL = (_blob: Blob | MediaSource) => "blob:preview-control"; static revokeObjectURL = vi.fn(); }
  vi.stubGlobal("URL", UrlControl);
  host = document.createElement("div"); document.body.append(host);
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg"); svg.setAttribute("viewBox", "0 0 100 50"); document.body.append(svg);
  root = createRoot(host);
  await act(async () => root!.render(<FlowImageExportDialog svgRef={{ current: svg }} selection={selectSvgViewport(svg)} sourceKey="fixture:1:scope" active initialFormat="gif" onClose={() => {}} />));
  const start = () => host.querySelector<HTMLButtonElement>("[data-export-start]")!;
  expect(start().disabled).toBe(true);
  await act(async () => host.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click());
  expect(start().disabled).toBe(false);
  const seconds = host.querySelector<HTMLInputElement>('input[type="number"]')!;
  expect(seconds.max).toBe("30"); expect(seconds.value).toBe("3");
  await act(async () => { const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!; setter.call(seconds, "31"); seconds.dispatchEvent(new Event("input", { bubbles: true })); });
  expect(start().disabled).toBe(true);
  expect(seconds.getAttribute("aria-invalid")).toBe("true");
});

async function dialog(sourceKey = "fixture:1:scope") {
  const downloads: string[] = [];
  const OriginalURL = URL; class UrlControl extends OriginalURL { static createObjectURL = (_blob: Blob | MediaSource) => "blob:local-control"; static revokeObjectURL = vi.fn(); }
  vi.stubGlobal("URL", UrlControl);
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) { downloads.push(this.download); });
  host = document.createElement("div"); document.body.append(host);
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg"); svg.setAttribute("viewBox", "0 0 100 50"); document.body.append(svg);
  root = createRoot(host);
  const props = { svgRef: { current: svg }, selection: selectSvgViewport(svg), sourceKey, active: true, initialFormat: "png" as const, onClose: () => {} };
  await act(async () => root!.render(<FlowImageExportDialog {...props} />));
  await act(async () => host.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click());
  return { downloads, props, start: () => host.querySelector<HTMLButtonElement>("[data-export-start]")! };
}
it("starts one local PNG job for two rapid clicks, retaining the explicit selection", async () => {
  raster.mockImplementation(async () => ({ rgba: new Uint8ClampedArray(4), png: async () => new Blob(["adapter-only"], { type: "image/png" }), dispose: vi.fn() }));
  const { downloads, props, start } = await dialog();
  await act(async () => { start().click(); start().click(); });
  expect(raster).toHaveBeenCalledOnce(); expect(raster.mock.calls[0][1]).toBe(props.selection);
  expect(downloads).toEqual(["knxbench-flow.png"]);
});
it("refuses a late old-source image after the view/session source is replaced", async () => {
  let finish!: (frame: RasterFrame) => void;
  raster.mockImplementation(() => new Promise<RasterFrame>(resolve => { finish = resolve; }));
  const { downloads, props, start } = await dialog();
  await act(async () => start().click());
  await act(async () => root!.render(<FlowImageExportDialog {...props} sourceKey="fixture:2:different-project" />));
  const png = vi.fn(async () => new Blob(["stale"], { type: "image/png" })); const dispose = vi.fn();
  await act(async () => finish({ rgba: new Uint8ClampedArray(4), png, dispose }));
  expect(downloads).toEqual([]); expect(png).not.toHaveBeenCalled(); expect(dispose).toHaveBeenCalledOnce();
  expect(start().disabled).toBe(true); expect(host.querySelector('[role="alert"]')?.textContent).toContain("source");
  await act(async () => root!.render(<FlowImageExportDialog {...props} />));
  expect(start().disabled).toBe(true);
});
