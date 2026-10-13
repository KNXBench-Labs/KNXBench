/** Selected SVG viewport rasterization; never publishes or records the bus. */
import { ImageExportError, MAX_EXPORT_ELEMENTS, MAX_EXPORT_SVG_BYTES } from "./flowImageExportLimits";

export interface ViewSelection {
  readonly viewBox: string; readonly width: number; readonly height: number;
  readonly cameraAttribute: string | null; readonly cameraTransform: string;
  readonly background: string;
}
const SAFE_TAGS = new Set(["svg", "g", "defs", "marker", "path", "text", "tspan", "rect", "circle", "ellipse", "line", "polyline", "polygon", "title", "desc"]);
const PRESENTATION = ["fill", "fill-opacity", "stroke", "stroke-opacity", "stroke-width", "stroke-linecap", "stroke-linejoin", "stroke-dasharray", "paint-order", "vector-effect", "font-family", "font-size", "font-weight", "letter-spacing", "text-anchor", "dominant-baseline", "opacity", "visibility", "display", "marker-end", "transform", "transform-origin"];

export function selectSvgViewport(svg: SVGSVGElement): ViewSelection {
  const viewBox = svg.getAttribute("viewBox") ?? "";
  const values = viewBox.trim().split(/[ ,]+/).map(Number);
  if (values.length !== 4 || !values.every(Number.isFinite) || values[2] <= 0 || values[3] <= 0) throw new ImageExportError("invalidInput");
  const camera = svg.querySelector("g[data-zoom]");
  let background = "rgb(255, 255, 255)";
  for (let element: Element | null = svg; element; element = element.parentElement) {
    const color = getComputedStyle(element).backgroundColor;
    if (color && color !== "transparent" && color !== "rgba(0, 0, 0, 0)") { background = color; break; }
  }
  return { viewBox, width: values[2], height: values[3], background,
    cameraAttribute: camera?.getAttribute("transform") ?? null,
    cameraTransform: camera ? getComputedStyle(camera).getPropertyValue("transform") : "" };
}

function localReferences(value: string, ids: ReadonlySet<string>, documentUrl: string): string {
  return value.replace(/url\(\s*([^)]*?)\s*\)/gi, (_match, raw: string) => {
    const target = raw.replace(/^["']|["']$/g, "");
    let id: string;
    if (target.startsWith("#")) id = target.slice(1);
    else {
      let url: URL, base: URL;
      try { url = new URL(target, documentUrl); base = new URL(documentUrl); } catch { throw new ImageExportError("unsafeSvg"); }
      if (url.origin !== base.origin || url.pathname !== base.pathname || url.search !== base.search || !url.hash) throw new ImageExportError("unsafeSvg");
      id = url.hash.slice(1);
    }
    if (!ids.has(id) || /[\s"'()]/.test(id)) throw new ImageExportError("unsafeSvg");
    return `url(#${id})`;
  });
}

/** Freeze the selected region/camera, inline painted styles; only graph markup is admitted. */
export function serializeSelectedSvg(svg: SVGSVGElement, selection = selectSvgViewport(svg), dimensions?: { width: number; height: number }): string {
  const originals = [svg, ...svg.querySelectorAll("*")];
  if (originals.length > MAX_EXPORT_ELEMENTS) throw new ImageExportError("tooLarge");
  const ids = new Set(originals.map(element => element.id).filter(Boolean));
  let sourceCharacters = 0;
  for (const element of originals) {
    sourceCharacters += element.localName.length + 5;
    for (const child of element.childNodes) if (child.nodeType === Node.TEXT_NODE) sourceCharacters += child.textContent?.length ?? 0;
    for (const attribute of element.attributes) sourceCharacters += attribute.name.length + attribute.value.length + 4;
    if (sourceCharacters > MAX_EXPORT_SVG_BYTES) throw new ImageExportError("tooLarge");
    if (element.namespaceURI !== "http://www.w3.org/2000/svg" || !SAFE_TAGS.has(element.localName)) throw new ImageExportError("unsafeSvg");
    for (const attribute of element.attributes) {
      if (/^on/i.test(attribute.name) || /(?:^|:)href$/i.test(attribute.name) || attribute.name === "xml:base") throw new ImageExportError("unsafeSvg");
      if (!/^(data-|aria-)/.test(attribute.name)) localReferences(attribute.value, ids, svg.ownerDocument.URL);
    }
  }
  const clone = svg.cloneNode(true) as SVGSVGElement;
  const copies = [clone, ...clone.querySelectorAll("*")];
  originals.forEach((original, index) => {
    const copy = copies[index] as SVGElement; const style = getComputedStyle(original);
    copy.removeAttribute("style"); copy.removeAttribute("class");
    for (const attribute of [...copy.attributes]) {
      if (/^(data-|aria-)/.test(attribute.name) || ["role", "tabindex"].includes(attribute.name)) copy.removeAttribute(attribute.name);
      else copy.setAttribute(attribute.name, localReferences(attribute.value, ids, svg.ownerDocument.URL));
    }
    for (const property of PRESENTATION) {
      const value = style.getPropertyValue(property);
      if (value && !(property === "transform" && value === "none" && original.hasAttribute("transform"))) copy.style.setProperty(property, localReferences(value, ids, svg.ownerDocument.URL));
    }
    if (original.hasAttribute("data-zoom")) {
      if (selection.cameraAttribute) copy.setAttribute("transform", selection.cameraAttribute);
      else copy.removeAttribute("transform");
      if (selection.cameraTransform && selection.cameraTransform !== "none") copy.style.transform = selection.cameraTransform;
      else copy.style.removeProperty("transform");
    }
    for (const attribute of [...copy.attributes]) if (/^(data-|aria-)/.test(attribute.name) || ["role", "tabindex"].includes(attribute.name)) copy.removeAttribute(attribute.name);
  });
  clone.setAttribute("viewBox", selection.viewBox);
  if (dimensions) { clone.setAttribute("width", String(dimensions.width)); clone.setAttribute("height", String(dimensions.height)); }
  const text = new XMLSerializer().serializeToString(clone);
  if (text.length > MAX_EXPORT_SVG_BYTES || new TextEncoder().encode(text).length > MAX_EXPORT_SVG_BYTES) throw new ImageExportError("tooLarge");
  return text;
}

export async function rasterizeSelectedSvg(svg: SVGSVGElement, selection: ViewSelection, dimensions: { width: number; height: number }, signal: AbortSignal): Promise<import("./flowImageCapture").RasterFrame> {
  const cancelled = () => new ImageExportError(signal.reason === "timeout" ? "timeout" : "cancelled");
  if (signal.aborted) throw cancelled();
  const xml = serializeSelectedSvg(svg, selection, dimensions);
  const canvas = document.createElement("canvas");
  canvas.width = dimensions.width; canvas.height = dimensions.height;
  const dispose = () => { canvas.width = 0; canvas.height = 0; };
  let image: HTMLImageElement | undefined; let url: string | undefined; let retained = false;
  let removeAbortListener = () => {};
  try {
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) throw new ImageExportError("rasterFailed");
    const objectUrl = URL.createObjectURL(new Blob([xml], { type: "image/svg+xml;charset=utf-8" })); url = objectUrl;
    const loadedImage = new Image(); image = loadedImage;
    await new Promise<void>((resolve, reject) => {
      const cleanup = () => signal.removeEventListener("abort", abort);
      const abort = () => { cleanup(); reject(cancelled()); };
      removeAbortListener = cleanup;
      loadedImage.onload = () => { cleanup(); resolve(); };
      loadedImage.onerror = () => { cleanup(); reject(new ImageExportError("rasterFailed")); };
      signal.addEventListener("abort", abort, { once: true });
      loadedImage.src = objectUrl;
      if (signal.aborted) abort();
    });
    if (signal.aborted) throw cancelled();
    context.fillStyle = selection.background; context.fillRect(0, 0, canvas.width, canvas.height);
    context.drawImage(loadedImage, 0, 0, canvas.width, canvas.height);
    const rgba = context.getImageData(0, 0, canvas.width, canvas.height).data;
    const png = () => new Promise<Blob>((resolve, reject) => {
      if (signal.aborted) { reject(cancelled()); return; }
      canvas.toBlob(blob => blob ? resolve(blob) : reject(new ImageExportError("rasterFailed")), "image/png");
    });
    retained = true; return { rgba, png, dispose };
  } finally {
    removeAbortListener();
    if (image) { image.onload = null; image.onerror = null; image.removeAttribute("src"); }
    if (url) URL.revokeObjectURL(url); if (!retained) dispose();
  }
}
