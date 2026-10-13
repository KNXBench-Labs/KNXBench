/** Selected-view pixels preserve visible labels, never external SVG content. */
// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { rasterizeSelectedSvg, selectSvgViewport, serializeSelectedSvg } from "./flowImageExport";
function graph(content: string) {
  const host = document.createElement("div"); host.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50"><g data-zoom="1">${content}</g></svg>`;
  document.body.append(host); return host.firstElementChild as unknown as SVGSVGElement;
}
afterEach(() => { document.body.innerHTML = ""; vi.restoreAllMocks(); vi.unstubAllGlobals(); });
it("serializes only the chosen graph, retaining its original visible text", () => {
  const svg = graph('<text x="1" y="20">Actual &amp; visible</text>');
  const unrelated = document.createElement("p"); unrelated.textContent = "SECRET OUTSIDE GRAPH"; document.body.append(unrelated);
  const xml = serializeSelectedSvg(svg);
  expect(xml).toContain("Actual &amp; visible");
  expect(xml).not.toContain("SECRET OUTSIDE GRAPH");
  expect(xml).toContain('viewBox="0 0 100 50"');
});

it.each([
  '<image href="https://example.invalid/private.png" />',
  '<foreignObject><div>not SVG</div></foreignObject>',
  '<script>unsafe()</script>',
  '<style>text { fill: url(https://example.invalid/color.svg#x); }</style>',
  '<text onclick="unsafe()">Do not execute</text>',
])("refuses non-graph or active/external SVG content: %s", content => {
  expect(() => serializeSelectedSvg(graph(content))).toThrow("unsafeSvg");
});
it("refuses a graph above the element budget before cloning/painting", () => {
  expect(() => serializeSelectedSvg(graph("<g/>".repeat(4001)))).toThrow("tooLarge");
});

it("cancels image loading, revokes the transient SVG URL and releases its canvas", async () => {
  const create = vi.fn((_blob: Blob | MediaSource) => "blob:export-control"); const revoke = vi.fn();
  const OriginalURL = URL; class UrlControl extends OriginalURL { static createObjectURL = create; static revokeObjectURL = revoke; }
  vi.stubGlobal("URL", UrlControl);
  vi.stubGlobal("Image", vi.fn(function ImageControl() { return document.createElement("img"); }));
  let canvas: HTMLCanvasElement | null = null;
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(function (this: HTMLCanvasElement) { canvas = this; return {} as CanvasRenderingContext2D; });
  const svg = graph('<text>selected</text>'); const abort = new AbortController();
  const job = rasterizeSelectedSvg(svg, selectSvgViewport(svg), { width: 200, height: 100 }, abort.signal);
  const refused = expect(job).rejects.toThrow("cancelled"); abort.abort(); await refused;
  expect(create).toHaveBeenCalledOnce(); expect(revoke).toHaveBeenCalledWith("blob:export-control");
  expect(canvas!.width).toBe(0); expect(canvas!.height).toBe(0);
});

it("preserves a visible literal URL-like label while removing its unpainted accessibility copy", () => {
  const xml = serializeSelectedSvg(graph('<text aria-label="url(https://example.invalid/literal)">A literal url(https://example.invalid/literal)</text>'));
  expect(xml).toContain("A literal url(https://example.invalid/literal)");
  expect(xml).not.toContain("aria-label");
});
it("rewrites a valid same-document paint reference into a self-contained marker", () => {
  const svg = graph('<defs><marker id="m"><path d="M0,0 L1,1"/></marker></defs><path d="M0,0 L20,20"/>');
  svg.querySelector('g > path')!.setAttribute('marker-end', `url(${document.URL}#m)`);
  const xml = serializeSelectedSvg(svg);
  expect(xml).toContain("url(#m)"); expect(xml).not.toContain(document.URL);
});
it("rejects excessive source text before allocating a cloned SVG", () => {
  const svg = graph('<text>' + 'A'.repeat(1024 * 1024 + 1) + '</text>');
  const clone = vi.spyOn(svg, "cloneNode");
  expect(() => serializeSelectedSvg(svg)).toThrow("tooLarge");
  expect(clone).not.toHaveBeenCalled();
});

it("releases the canvas when the transient URL allocator refuses", async () => {
  const svg = graph('<text>Selected control</text>');
  const { rasterizeSelectedSvg, selectSvgViewport } = await import("./flowImageExport");
  const selection = selectSvgViewport(svg); const canvas = document.createElement("canvas");
  const create = document.createElement.bind(document);
  try {
    vi.spyOn(document, "createElement").mockImplementation((name: string) => name === "canvas" ? canvas : create(name));
    vi.spyOn(canvas, "getContext").mockReturnValue({} as CanvasRenderingContext2D);
    vi.stubGlobal("URL", { createObjectURL: () => { throw new Error("allocator refused"); }, revokeObjectURL: vi.fn() });
    await expect(rasterizeSelectedSvg(svg, selection, { width: 16, height: 8 }, new AbortController().signal)).rejects.toThrow("allocator refused");
    expect(canvas.width).toBe(0); expect(canvas.height).toBe(0);
  } finally { vi.restoreAllMocks(); vi.unstubAllGlobals(); }
});
