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
