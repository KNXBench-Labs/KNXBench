/** Read-only, own-instance evidence boundary; never a GitHub uploader. */
export interface ContributionFinding {
  id: string; stage: string; category: string; sourcePath: string | null;
  xpath: string | null; name: string | null; occurrences: number; detail: string; sample: string | null;
}
export interface ContributionAnalysis {
  formatVersion: number; analyzerVersion: string; kind: string; scheme: number | null;
  status: string; sourceSize: number; originalAllowed: boolean;
  findings: ContributionFinding[];
  metrics: { stage: string; entity: string; disposition: string; count: number }[];
  checks: { name: string; status: string; detail: string }[];
  members: { id: string; path: string; size: number; sampleAllowed: boolean; reason: string }[];
  structure: { memberId: string; path: string[]; namespace: string; name: string; kind: string; occurrences: number }[];
}
export interface ContributionOptions {
  audience: "public" | "private"; sampleIds: string[];
  includeOriginal: boolean; originalConsent: boolean; consent: boolean; expectedManifestSha256?: string;
}
export interface ContributionPreview {
  manifestSha256: string;
  manifest: { formatVersion: number; analyzerVersion: string; audience: "public" | "private"; disclosure: string;
    originalSha256: string | null; files: { path: string; size: number; sha256: string }[]; limitations: string[] };
  files: { path: string; text: string | null }[];
}

async function post(file: File, operation: "analyze" | "preview" | "export", options?: ContributionOptions, signal?: AbortSignal) {
  const form = new FormData();
  form.append("file", file);
  if (options) form.append("options", JSON.stringify(options));
  const response = await fetch(`/api/contributions/${operation}`, { method: "POST", body: form, signal });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.error ?? `${response.status} ${response.statusText}`);
  }
  return response;
}
export async function analyzeContribution(file: File, signal?: AbortSignal): Promise<ContributionAnalysis> {
  const response = await (await post(file, "analyze", undefined, signal)).json();
  if (response.formatVersion !== 1 || !["project", "product"].includes(response.kind) || !["complete", "partial", "refused"].includes(response.status) ||
    !["findings", "checks", "members", "structure", "metrics"].every(key => Array.isArray(response[key]))) {
    throw new Error("unsupported analysis response");
  }
  return response;
}
export async function previewContribution(file: File, options: ContributionOptions, signal?: AbortSignal): Promise<ContributionPreview> {
  const response = await (await post(file, "preview", options, signal)).json();
  if (response.manifest?.formatVersion !== 1 || !Array.isArray(response.files) || !Array.isArray(response.manifest.files) ||
      typeof response.manifestSha256 !== "string" || !/^[a-f0-9]{64}$/.test(response.manifestSha256)) {
    throw new Error("unsupported evidence preview");
  }
  return response;
}
export async function exportContribution(file: File, options: ContributionOptions, signal?: AbortSignal): Promise<Blob> {
  const response = await post(file, "export", options, signal);
  if (response.headers.get("content-type") !== "application/zip") throw new Error("expected evidence ZIP");
  return response.blob();
}
