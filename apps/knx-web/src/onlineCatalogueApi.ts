/** Own-backend catalogue acquisition; no provider requests or browser-held saved credentials. */
import { request, type LegacyInstallReport } from "./api";

export type OnlineProgram = {
  displayKey: string;
  name: string;
  applicationNumber: string;
  version: string;
  maskVersion: string;
  productNames: string[];
};

export type OnlineEntry = {
  id: string;
  manufacturer: string;
  manufacturerId: string;
  name: string;
  orderNumbers: string[];
  fileName: string;
  fileType: string;
  size: number;
  sha256: string;
  updated: string;
  programs: OnlineProgram[];
  importable: boolean;
  downloaded: boolean;
};

export type OnlineErrorInfo = {
  kind: string;
  message?: string;
  resetAt?: string | null;
  retryAfterSecs?: number | null;
};

export type OnlineDownload = {
  state: "idle" | "running" | "done" | "failed";
  entryId: string | null;
  receivedBytes: number;
  totalBytes: number | null;
  error: OnlineErrorInfo | null;
};

export type OnlineStatus = {
  source: string;
  label: string;
  defaultBaseUrl: string;
  baseUrl: string;
  baseUrlIsDefault: boolean;
  index: {
    entryCount: number;
    skippedEntries: number;
    version: string;
    generatedAt: string;
    lastCheckedAt: string;
    fetchedAt: string;
  } | null;
  indexProblem: string | null;
  credentials: {
    state: string;
    mode: string | null;
    user: string | null;
    problem: string | null;
  };
  limit: {
    kind: string;
    retryAfterSecs: number;
    resetAt: string | null;
  } | null;
  syncRunning: boolean;
  download: OnlineDownload;
};

export type OnlinePage = {
  total: number;
  offset: number;
  limit: number;
  entries: OnlineEntry[];
};

export type OnlineCredentials =
  | { mode: "password"; user: string; password: string }
  | { mode: "token"; token: string };

export const ONLINE_BASE = "/api/online-catalogue/reko";

export const onlineEntryPath = (
  entry: Pick<OnlineEntry, "id" | "sha256">,
  action: string,
) =>
  `${ONLINE_BASE}/entries/${encodeURIComponent(entry.id)}/${action}?${new URLSearchParams({ sha256: entry.sha256 })}`;

const json = (method: string, body?: unknown): RequestInit => ({
  method,
  cache: "no-store",
  ...(body === undefined ? {} : { body: JSON.stringify(body) }),
});

export const onlineCatalogueApi = {
  sources: () =>
    request<{
      available: boolean;
      sources: { id: string; label: string; defaultBaseUrl: string }[];
    }>("/api/online-catalogue", { cache: "no-store" }),
  status: () => request<OnlineStatus>(ONLINE_BASE, { cache: "no-store" }),
  configure: (baseUrl: string | null) =>
    request<OnlineStatus>(`${ONLINE_BASE}/config`, json("PUT", { baseUrl })),
  sync: () =>
    request<{
      outcome: "updated" | "notModified";
      entryCount?: number;
      status: OnlineStatus;
    }>(`${ONLINE_BASE}/sync`, json("POST")),
  search: (q: string, type: string, offset = 0) =>
    request<OnlinePage>(
      `${ONLINE_BASE}/search?${new URLSearchParams({
        q,
        ...(type ? { type } : {}),
        offset: String(offset),
        limit: "25",
      })}`,
      { cache: "no-store" },
    ),
  credentials: (value: OnlineCredentials) =>
    request<OnlineStatus>(`${ONLINE_BASE}/credentials`, json("PUT", value)),
  forget: () => request<OnlineStatus>(`${ONLINE_BASE}/credentials`, json("DELETE")),
  download: (entry: OnlineEntry) =>
    request<OnlineDownload>(onlineEntryPath(entry, "download"), json("POST")),
  cancel: () =>
    request<OnlineDownload>(`${ONLINE_BASE}/download`, json("DELETE")),
  legacy: (entry: OnlineEntry, password?: string, remember = false) =>
    request<LegacyInstallReport>(
      onlineEntryPath(entry, "install-legacy"),
      json("POST", { password: password ?? null, remember }),
    ),
};

/** No error text from a provider is rendered or treated as an application-session refusal. */
export function onlineError(error: unknown): OnlineErrorInfo {
  const body = (error as { body?: OnlineErrorInfo } | null)?.body;
  return body && typeof body.kind === "string" ? body : { kind: "transport" };
}
