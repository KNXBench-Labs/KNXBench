/** Bounded instruction HTTP client with explicit mutation confirmation. */
import { noteRefusal } from "../../api";
import { validateDocument, type Document } from "./document";
export type Context = Readonly<{ serverIncarnation: string; projectIncarnation: number; snapshotRevision: number }>;
/** `native`: made in KNXBench, so there is no imported original and no hash (Goal B4). */
export type InstructionView = Context & Readonly<{ device: number; origin: "imported" | "native"; originalXmlSha256: string | null; originalPresent: boolean; originalBytes: number; originalFormat: "plain" | "rtf" | null; unsupported: readonly string[]; previewError: string | null; effectiveDocument: Document | null; overrideDocument: Document | null; persistence: "session-only" | "native-working-history" }>;
export type InstructionPreview = Readonly<{ token: string; device: number; replacement: Document | null; conversionRequired: boolean; noOp: boolean; expiresInSeconds: number }>;
export type InstructionApplied = Readonly<{ changed: boolean; nativeGeneration: number | null; snapshotRevision: number; persistence: "session-only" | "native-working-history" }>;
export type InstructionClient = Readonly<{
  read(device: number, signal?: AbortSignal): Promise<InstructionView>;
  preview(device: number, context: Context, document: Document | null, signal?: AbortSignal): Promise<InstructionPreview>;
  apply(device: number, token: string, confirmConversion: boolean, signal?: AbortSignal): Promise<InstructionApplied>;
  cancel(device: number, token: string): Promise<void>;
}>;
function record(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
function integer(value: unknown): value is number { return typeof value === "number" && Number.isSafeInteger(value) && value >= 0; }
function persistence(value: unknown): value is InstructionView["persistence"] { return value === "session-only" || value === "native-working-history"; }
function nullableDocument(value: unknown): value is Document | null { return value === null || validateDocument(value); }
function context(value: unknown): value is Context & Record<string, unknown> { return record(value) && typeof value.serverIncarnation === "string" && value.serverIncarnation.length > 0 && value.serverIncarnation.length <= 128 && integer(value.projectIncarnation) && integer(value.snapshotRevision); }
function documentsEqual(a: Document, b: Document): boolean { return a.runs.length === b.runs.length && a.runs.every((run, i) => { const other = b.runs[i]; return run.text === other.text && run.style.bold === other.style.bold && run.style.italic === other.style.italic && run.style.underline === other.style.underline; }); }
export function sameContext(a: Context, b: Context): boolean { return a.serverIncarnation === b.serverIncarnation && a.projectIncarnation === b.projectIncarnation && a.snapshotRevision === b.snapshotRevision; }
export class InstructionContractError extends Error { constructor() { super("Unsupported or malformed instruction response"); this.name = "InstructionContractError"; } }
/** Imported: the device XML's hash. Native (Goal B4): no hash and no original, never an invented one. */
function origin(value: Record<string, unknown>): boolean {
  if (value.origin === "imported") return typeof value.originalXmlSha256 === "string" && /^[a-f0-9]{64}$/.test(value.originalXmlSha256);
  return value.origin === "native" && value.originalXmlSha256 === null && value.originalPresent === false;
}
function admitView(value: unknown, device: number): InstructionView {
  if (!record(value) || !context(value) || value.device !== device || !origin(value) || typeof value.originalPresent !== "boolean" || !integer(value.originalBytes) || value.originalBytes > 8 * 1024 * 1024 || ![null, "plain", "rtf"].includes(value.originalFormat as null | string) || !Array.isArray(value.unsupported) || value.unsupported.length > 256 || !value.unsupported.every(word => typeof word === "string") || !(value.previewError === null || typeof value.previewError === "string") || !nullableDocument(value.effectiveDocument) || !nullableDocument(value.overrideDocument) || !persistence(value.persistence)) throw new InstructionContractError();
  if (value.overrideDocument !== null && (value.effectiveDocument === null || !documentsEqual(value.overrideDocument, value.effectiveDocument))) throw new InstructionContractError();
  return value as InstructionView;
}
function admitPreview(value: unknown, device: number): InstructionPreview {
  if (!record(value) || value.device !== device || typeof value.token !== "string" || !value.token || value.token.length > 128 || !nullableDocument(value.replacement) || typeof value.conversionRequired !== "boolean" || typeof value.noOp !== "boolean" || !integer(value.expiresInSeconds) || value.expiresInSeconds > 300) throw new InstructionContractError();
  return value as InstructionPreview;
}
function admitApplied(value: unknown): InstructionApplied {
  if (!record(value) || typeof value.changed !== "boolean" || !(value.nativeGeneration === null || integer(value.nativeGeneration)) || !integer(value.snapshotRevision) || !persistence(value.persistence)) throw new InstructionContractError();
  return value as InstructionApplied;
}
function path(device: number): string { if (!integer(device) || device > 0xffff_ffff) throw new InstructionContractError(); return `/api/devices/${device}/instructions`; }
function identity(token: string): void { if (typeof token !== "string" || !token || token.length > 128) throw new InstructionContractError(); }
export function createInstructionClient(transport: typeof fetch = fetch): InstructionClient {
  async function request(path: string, body?: unknown, signal?: AbortSignal): Promise<unknown> {
    if (signal?.aborted) throw new DOMException("Aborted", "AbortError");
    const response = await transport(path, { method: body === undefined ? "GET" : "POST", credentials: "same-origin", signal, ...(body === undefined ? {} : { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }) });
    const value: unknown = await response.json().catch(() => null);
    if (signal?.aborted) throw new DOMException("Aborted", "AbortError");
    if (!response.ok) {
      noteRefusal(path, response.status);
      const message = record(value) && typeof value.error === "string" ? value.error : `Instruction request failed (${response.status})`;
      throw Object.assign(new Error(message), { status: response.status, kind: record(value) && typeof value.kind === "string" ? value.kind : null });
    }
    return value;
  }
  return {
    read: async (device, signal) => admitView(await request(path(device), undefined, signal), device),
    preview: async (device, expected, document, signal) => {
      const base = path(device);
      if (!context(expected) || !nullableDocument(document)) throw new InstructionContractError();
      return admitPreview(await request(`${base}/preview`, { serverIncarnation: expected.serverIncarnation, projectIncarnation: expected.projectIncarnation, snapshotRevision: expected.snapshotRevision, document }, signal), device);
    },
    apply: async (device, token, confirmConversion, signal) => {
      const base = path(device); identity(token);
      if (typeof confirmConversion !== "boolean") throw new InstructionContractError();
      return admitApplied(await request(`${base}/apply`, { token, confirmed: true, confirmConversion }, signal));
    },
    cancel: async (device, token) => {
      const base = path(device); identity(token); const result = await request(`${base}/cancel`, { token });
      if (!record(result) || result.cancelled !== true) throw new InstructionContractError();
    },
  };
}
