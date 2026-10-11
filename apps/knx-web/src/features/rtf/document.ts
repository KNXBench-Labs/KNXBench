/** Pure normalized instruction documents and selection edits. */
export type Style = Readonly<{ bold: boolean; italic: boolean; underline: boolean }>;
export type Run = Readonly<{ text: string; style: Style }>;
export type Document = Readonly<{ runs: readonly Run[] }>;
export const MAX_DOCUMENT_BYTES = 64 * 1024;
export const MAX_DOCUMENT_RUNS = 4096;
export const normalStyle: Style = Object.freeze({ bold: false, italic: false, underline: false });
export function plainText(document: Document): string { return document.runs.map(run => run.text).join(""); }
function styleEqual(a: Style, b: Style): boolean { return a.bold === b.bold && a.italic === b.italic && a.underline === b.underline; }
function object(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
function keys(value: Record<string, unknown>, expected: readonly string[]): boolean { const actual = Object.keys(value); return actual.length === expected.length && expected.every(key => Object.hasOwn(value, key)); }
function wellFormed(text: string): boolean {
  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = text.charCodeAt(++i);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false;
    } else if (code >= 0xdc00 && code <= 0xdfff) return false;
  }
  return true;
}
export function validateDocument(value: unknown): value is Document {
  if (!object(value) || !keys(value, ["runs"]) || !Array.isArray(value.runs) || value.runs.length > MAX_DOCUMENT_RUNS) return false;
  let bytes = 0; let previous: Style | undefined;
  for (const run of value.runs) {
    if (!object(run) || !keys(run, ["text", "style"]) || typeof run.text !== "string" || !run.text || run.text.length > MAX_DOCUMENT_BYTES || !wellFormed(run.text)) return false;
    if (!object(run.style) || !keys(run.style, ["bold", "italic", "underline"]) || ![run.style.bold, run.style.italic, run.style.underline].every(flag => typeof flag === "boolean")) return false;
    const style = run.style as Style;
    bytes += new TextEncoder().encode(run.text).byteLength;
    if (bytes > MAX_DOCUMENT_BYTES || (previous && styleEqual(previous, style))) return false;
    previous = style;
  }
  return true;
}
function checked(document: Document): Document { if (!validateDocument(document)) throw new RangeError("Invalid normalized instruction document"); return document; }
export function literal(text: string): Document { return checked({ runs: text ? [{ text, style: normalStyle }] : [] }); }
function append(runs: Run[], text: string, style: Style): void {
  if (!text) return;
  const previous = runs.at(-1);
  if (previous && styleEqual(previous.style, style)) runs[runs.length - 1] = { text: previous.text + text, style: previous.style };
  else runs.push({ text, style: { ...style } });
}
function slice(document: Document, start: number, end: number): Run[] {
  const runs: Run[] = []; let offset = 0;
  for (const run of document.runs) {
    const begin = Math.max(start - offset, 0); const finish = Math.min(end - offset, run.text.length);
    if (finish > begin) append(runs, run.text.slice(begin, finish), run.style);
    offset += run.text.length;
  }
  return runs;
}
/** Textarea normalizes CR/CRLF visually; map its UTF-16 offsets back to exact literal data. */
function displayBuffer(document: Document): { text: string; offsets: number[] } {
  const source = plainText(checked(document)); const chars: string[] = []; const offsets = [0];
  for (let i = 0; i < source.length; i++) {
    if (source[i] === "\r") { chars.push("\n"); if (source[i + 1] === "\n") i++; }
    else chars.push(source[i]);
    offsets.push(i + 1);
  }
  return { text: chars.join(""), offsets };
}
export function displayText(document: Document): string { return displayBuffer(document).text; }
function boundary(text: string, index: number): boolean {
  if (index <= 0 || index >= text.length) return true;
  const a = text.charCodeAt(index - 1); const b = text.charCodeAt(index);
  return !(a >= 0xd800 && a <= 0xdbff && b >= 0xdc00 && b <= 0xdfff);
}
function styleAt(document: Document, offset: number): Style {
  let position = 0;
  for (const run of document.runs) { position += run.text.length; if (offset < position) return run.style; }
  return document.runs.at(-1)?.style ?? normalStyle;
}
export function replaceText(document: Document, next: string): Document {
  const buffer = displayBuffer(document);
  if (!wellFormed(next)) throw new RangeError("Invalid Unicode input");
  if (next === buffer.text) return checked(document);
  let prefix = 0;
  while (prefix < buffer.text.length && prefix < next.length && buffer.text[prefix] === next[prefix]) prefix++;
  if (!boundary(buffer.text, prefix) || !boundary(next, prefix)) prefix--;
  let suffix = 0;
  while (suffix < buffer.text.length - prefix && suffix < next.length - prefix && buffer.text[buffer.text.length - 1 - suffix] === next[next.length - 1 - suffix]) suffix++;
  if (!boundary(buffer.text, buffer.text.length - suffix) || !boundary(next, next.length - suffix)) suffix--;
  const start = buffer.offsets[prefix]; const end = buffer.offsets[buffer.text.length - suffix];
  const runs: Run[] = [];
  for (const run of slice(document, 0, start)) append(runs, run.text, run.style);
  append(runs, next.slice(prefix, next.length - suffix), styleAt(document, start));
  for (const run of slice(document, end, plainText(document).length)) append(runs, run.text, run.style);
  return checked({ runs });
}
export function formatSelection(document: Document, start: number, end: number, flag: keyof Style, enabled: boolean): Document {
  const buffer = displayBuffer(document);
  if (!["bold", "italic", "underline"].includes(flag) || typeof enabled !== "boolean" || !Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 0 || end < start || end > buffer.text.length || !boundary(buffer.text, start) || !boundary(buffer.text, end)) throw new RangeError("Invalid instruction selection");
  if (start === end) return document;
  const a = buffer.offsets[start]; const b = buffer.offsets[end]; const runs: Run[] = [];
  for (const run of slice(document, 0, a)) append(runs, run.text, run.style);
  for (const run of slice(document, a, b)) append(runs, run.text, { ...run.style, [flag]: enabled });
  for (const run of slice(document, b, plainText(document).length)) append(runs, run.text, run.style);
  return checked({ runs });
}
