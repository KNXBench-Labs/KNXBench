/** Same-instance support reporting; no external submission or hardware call on preview. */
import { noteRefusal } from "./api";

export interface SupportInput {
  programId?: string;
  firmware?: string;
  mask?: number;
  functionalCheck?: "not-checked" | "works-as-configured" | "does-not-work-as-configured";
  includeContext: boolean;
  contextRights: boolean;
}
export interface SupportPreview {
  token: string;
  digest: string;
  report: Record<string, unknown>;
  summary: string;
  localAddress: string | null;
  mask: number | null;
  profile: string;
  reads: string[];
  deadlineSeconds: number;
}
export interface SupportBinding {
  serverIncarnation: string;
  projectIncarnation: number;
  revision: number;
}

async function post(path: string, body: unknown, signal?: AbortSignal): Promise<Response> {
  const response = await fetch(path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
    signal,
  });
  if (!response.ok) {
    noteRefusal(path, response.status);
    throw new Error("support-request-refused");
  }
  return response;
}

function admit(payload: unknown): SupportPreview {
  if (typeof payload !== "object" || payload === null) throw new Error("support-response-invalid");
  const value = payload as Partial<SupportPreview>;
  if (
    typeof value.token !== "string" || !/^[a-f0-9]{64}$/.test(value.token) ||
    typeof value.digest !== "string" || !/^[a-f0-9]{64}$/.test(value.digest) ||
    typeof value.summary !== "string" || value.report === null ||
    typeof value.report !== "object" || Array.isArray(value.report) ||
    !Array.isArray(value.reads) || !value.reads.every(read => typeof read === "string") ||
    typeof value.profile !== "string" ||
    !(value.localAddress === null || typeof value.localAddress === "string") ||
    !(value.mask === null || (typeof value.mask === "number" && Number.isInteger(value.mask) && value.mask >= 0 && value.mask <= 65535)) ||
    typeof value.deadlineSeconds !== "number" || !Number.isInteger(value.deadlineSeconds) || value.deadlineSeconds < 0
  ) throw new Error("support-response-invalid");
  return value as SupportPreview;
}

export async function previewSupport(
  input: SupportInput, deviceId: number | null, context: SupportBinding | undefined,
  downloadId: number | undefined, address: string | undefined, signal?: AbortSignal,
): Promise<SupportPreview> {
  return admit(await (await post("/api/support/preview", { input, deviceId, context, downloadId, address }, signal)).json());
}

export async function diagnoseSupport(token: string, gateway: string, signal?: AbortSignal): Promise<SupportPreview> {
  return admit(await (await post("/api/support/diagnose", { token, gateway, consent: true }, signal)).json());
}

export async function saveSupport(preview: SupportPreview, signal?: AbortSignal): Promise<void> {
  const response = await post("/api/support/export", { token: preview.token, digest: preview.digest, consent: true }, signal);
  if (!response.headers.get("content-type")?.startsWith("application/zip")) throw new Error("support-export-invalid");
  const blob = await response.blob();
  if (signal?.aborted) return;
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "knxbench-support.zip";
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}
