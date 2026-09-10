//! fetch()-based replacement for @tauri-apps/api/core's invoke() — same
//! function names and argument shapes the components already called, so
//! swapping the import at each call site is the only change there.
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    ...init,
  });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.error ?? `${response.status} ${response.statusText}`);
  }
  if (response.headers.get("content-length") === "0") {
    return undefined as T;
  }
  return response.json() as Promise<T>;
}

export function importProject(path: string): Promise<ProjectTree> {
  return request("/api/project/import", { method: "POST", body: JSON.stringify({ path }) });
}

export function openProject(path: string): Promise<ProjectTree> {
  return request("/api/project/open", { method: "POST", body: JSON.stringify({ path }) });
}

export function saveProject(): Promise<void> {
  return request("/api/project/save", { method: "POST" });
}

export function saveProjectAs(path: string): Promise<void> {
  return request("/api/project/save-as", { method: "POST", body: JSON.stringify({ path }) });
}

export function deviceDetail(deviceId: number): Promise<DeviceDetail> {
  return request(`/api/device/${deviceId}`);
}

export function setIndividualAddress(deviceId: number, address: string | null): Promise<ProjectTree> {
  return request("/api/individual-address", {
    method: "POST",
    body: JSON.stringify({ deviceId, address }),
  });
}

export function setComObjectDpt(comObjectId: number, dpt: string | null): Promise<ProjectTree> {
  return request("/api/com-object-dpt", {
    method: "POST",
    body: JSON.stringify({ comObjectId, dpt }),
  });
}

export function setDeviceDescription(
  deviceId: number,
  description: string | null,
): Promise<ProjectTree> {
  return request("/api/device-description", {
    method: "POST",
    body: JSON.stringify({ deviceId, description }),
  });
}

export function setComObjectDescription(
  comObjectId: number,
  description: string | null,
): Promise<ProjectTree> {
  return request("/api/com-object-description", {
    method: "POST",
    body: JSON.stringify({ comObjectId, description }),
  });
}

export type ComFlagName = "Read" | "Write" | "Transmit" | "Update" | "Communication";

export function setComObjectFlag(
  comObjectId: number,
  flag: ComFlagName,
  value: boolean,
): Promise<ProjectTree> {
  return request("/api/com-object-flag", {
    method: "POST",
    body: JSON.stringify({ comObjectId, flag, value }),
  });
}

export function createGroupAddress(
  name: string,
  address: string,
  rangeId?: number,
): Promise<ProjectTree> {
  return request("/api/group-addresses", {
    method: "POST",
    body: JSON.stringify({ name, address, rangeId }),
  });
}

export function deleteGroupAddress(id: number): Promise<ProjectTree> {
  return request(`/api/group-addresses/${id}`, { method: "DELETE" });
}

export function createGroupRange(
  name: string,
  start: string,
  end: string,
  parentId?: number,
): Promise<ProjectTree> {
  return request("/api/group-ranges", {
    method: "POST",
    body: JSON.stringify({ name, start, end, parentId }),
  });
}

export function deleteGroupRange(id: number): Promise<ProjectTree> {
  return request(`/api/group-ranges/${id}`, { method: "DELETE" });
}

export function renameGroupRange(id: number, name: string): Promise<ProjectTree> {
  return request(`/api/group-ranges/${id}`, { method: "PATCH", body: JSON.stringify({ name }) });
}

// `direction` is `"Send"` or `"Receive"` — the same strings
// `ComObjectNode.links[].direction` already carries (both sides format
// `knx_core::Direction`'s `Debug` form), so a link read from the Inspector
// can be passed straight back to `unlinkComObject` unchanged.
export function linkComObject(
  comObjectId: number,
  gaId: number,
  direction: string,
): Promise<ProjectTree> {
  return request("/api/group-links", {
    method: "POST",
    body: JSON.stringify({ comObjectId, gaId, direction }),
  });
}

export function unlinkComObject(
  comObjectId: number,
  gaId: number,
  direction: string,
): Promise<ProjectTree> {
  return request("/api/group-links", {
    method: "DELETE",
    body: JSON.stringify({ comObjectId, gaId, direction }),
  });
}

export function createArea(name: string, address: number): Promise<ProjectTree> {
  return request("/api/areas", { method: "POST", body: JSON.stringify({ name, address }) });
}

export function deleteArea(id: number): Promise<ProjectTree> {
  return request(`/api/areas/${id}`, { method: "DELETE" });
}

export function createLine(
  areaId: number,
  name: string,
  address: number,
  mediumRef: string,
): Promise<ProjectTree> {
  return request("/api/lines", {
    method: "POST",
    body: JSON.stringify({ areaId, name, address, mediumRef }),
  });
}

export function deleteLine(id: number): Promise<ProjectTree> {
  return request(`/api/lines/${id}`, { method: "DELETE" });
}

export function moveDeviceToLine(deviceId: number, lineId: number | null): Promise<ProjectTree> {
  return request("/api/move-device", {
    method: "POST",
    body: JSON.stringify({ deviceId, lineId }),
  });
}

// Server-only DTOs (`apps/knx-server/src/routes.rs`'s `CatalogManufacturerDto`/
// `CatalogItemDto`) — not `knx-projection` types, so no `ts-rs` binding exists
// for them; hand-written to match the JSON shape, same as every request
// body's plain object literal above.
export interface CatalogManufacturer {
  id: string;
  name: string | null;
}

export interface CatalogItem {
  id: string;
  manufacturerId: string;
  name: string | null;
  number: string | null;
  visibleDescription: string | null;
  productRefId: string | null;
  hardware2programRefId: string | null;
}

export interface CatalogInstallMember {
  path: string;
  role: string;
  sha256: string;
  size: number;
}

export interface CatalogInstallReport {
  sha256: string;
  scheme: number;
  skipped: boolean;
  members: CatalogInstallMember[];
  unknown: number;
  conflicts: number;
}

export interface CreationDiagnostic {
  kind:
    | "programlessProduct"
    | "ambiguousDpt"
    | "comObjectRefMissing"
    | "programRefMissing"
    | "dynamicOrModuleNotEvaluated";
  catalogItemId?: string;
  refId?: string;
  alternatives?: string[];
  programRef?: string;
  programId?: string;
  /** Ready-to-display sentence built server-side; prefer this over
   * re-deriving wording from the structured fields above. */
  detail: string;
}

export interface CreateDeviceResponse {
  tree: ProjectTree;
  diagnostics: CreationDiagnostic[];
}

export function catalogManufacturers(): Promise<CatalogManufacturer[]> {
  return request("/api/catalog/manufacturers");
}

export function catalogItems(manufacturer?: string, search?: string): Promise<CatalogItem[]> {
  const params = new URLSearchParams();
  if (manufacturer) params.set("manufacturer", manufacturer);
  if (search) params.set("search", search);
  const qs = params.toString();
  return request(`/api/catalog/items${qs ? `?${qs}` : ""}`);
}

/// Uses multipart directly rather than `request()`: setting JSON's
/// `Content-Type` on a FormData request would remove the required boundary.
export async function installProductPackage(file: File): Promise<CatalogInstallReport> {
  const form = new FormData();
  form.append("file", file);
  const response = await fetch("/api/catalog/install", { method: "POST", body: form });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.error ?? `${response.status} ${response.statusText}`);
  }
  return response.json() as Promise<CatalogInstallReport>;
}

export function createDevice(
  lineId: number | null,
  catalogItemId: string,
  name: string,
): Promise<CreateDeviceResponse> {
  return request("/api/devices", {
    method: "POST",
    body: JSON.stringify(lineId === null ? { catalogItemId, name } : { lineId, catalogItemId, name }),
  });
}

export function deleteDevice(id: number): Promise<ProjectTree> {
  return request(`/api/devices/${id}`, { method: "DELETE" });
}

export function undo(): Promise<ProjectTree> {
  return request("/api/undo", { method: "POST" });
}

export function redo(): Promise<ProjectTree> {
  return request("/api/redo", { method: "POST" });
}

/// Unwraps the message from an error thrown by `request()` (or anything
/// else `Error`-shaped); falls back to `String(e)` for non-`Error` throws.
/// Callers used to do `String(e)` directly, back when Tauri's `invoke()`
/// rejected with a bare string — now that every API error is a real
/// `Error`, that produced a doubled `Error: <message>` in the UI.
export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
