/** Same-instance offline support preview and separately consented bounded identity acquisition. */
import { noteRefusal } from "./api";
import type { ProjectContext } from "./diagnosticBundleShape";

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

async function post(path: string, body: unknown, signal?: AbortSignal): Promise<SupportPreview> {
  const response = await fetch(path, {
    method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body), signal, cache: "no-store",
  });
  if (!response.ok) {
    noteRefusal(path, response.status);
    throw Object.assign(new Error("support-request-refused"), { status: response.status });
  }
  const payload: unknown = await response.json();
  if (typeof payload !== "object" || payload === null) throw new Error("support-response-invalid");
  const value = payload as Partial<SupportPreview>;
  if (typeof value.token !== "string" || !/^[a-f0-9]{64}$/.test(value.token)
    || typeof value.digest !== "string" || !/^[a-f0-9]{64}$/.test(value.digest)
    || typeof value.summary !== "string" || value.report === null || typeof value.report !== "object" || Array.isArray(value.report)
    || !Array.isArray(value.reads) || !value.reads.every((read) => typeof read === "string")
    || typeof value.profile !== "string" || !(value.localAddress === null || typeof value.localAddress === "string")
    || !(value.mask === null || (typeof value.mask === "number" && Number.isInteger(value.mask) && value.mask >= 0 && value.mask <= 65535))
    || typeof value.deadlineSeconds !== "number" || !Number.isInteger(value.deadlineSeconds) || value.deadlineSeconds < 0) {
    throw new Error("support-response-invalid");
  }
  return value as SupportPreview;
}

export function previewSupport(
  input: SupportInput, deviceId: number | null, context: ProjectContext | undefined,
  downloadId: number | undefined, address: string | undefined, signal?: AbortSignal,
): Promise<SupportPreview> {
  return post("/api/support/preview", { input, deviceId, context, downloadId, address }, signal);
}

export function diagnoseSupport(token: string, gateway: string, signal?: AbortSignal): Promise<SupportPreview> {
  return post("/api/support/diagnose", { token, gateway, consent: true }, signal);
}
