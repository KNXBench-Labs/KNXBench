/** A bounded, loss-aware JSON v1 snapshot of one bus-monitor session. */
import { invoke } from "@tauri-apps/api/core";
import type { BusTelegramRow } from "./api";
import { isTauri } from "./filePicker";
import { downloadLocalJson } from "./localJsonDownload";

export const CAPTURE_CAPACITY = 1000;
const MAX_EXPORT_BYTES = 16 * 1024 * 1024;

export interface BusCaptureProvenance {
  sessionId: number;
  serverIncarnation: string;
  status: "active" | "closed";
  serverDroppedBefore: number;
  clientPrunedCount: number;
  exportedAt: string;
}

export function appendCapturedRows(previous: BusTelegramRow[], incoming: BusTelegramRow[]): { rows: BusTelegramRow[]; pruned: number } {
  const combined = [...previous, ...incoming];
  const pruned = Math.max(0, combined.length - CAPTURE_CAPACITY);
  return { rows: pruned ? combined.slice(pruned) : combined, pruned };
}

export function serializeBusCapture(rows: BusTelegramRow[], provenance: BusCaptureProvenance): string {
  if (rows.length > CAPTURE_CAPACITY) throw new Error("Bus capture exceeds retained capacity");
  return `${JSON.stringify({
    format: "knxbench-bus-monitor",
    version: 1,
    capacity: CAPTURE_CAPACITY,
    ...provenance,
    notice: "Only client-retained rows from one server session are included; server or client losses are reported separately. This is not a complete bus history.",
    rows,
  }, null, 2)}\n`;
}

/** Browser Blob or a native dialog; neither path writes to the server's filesystem. */
export async function saveBusCapture(rows: BusTelegramRow[], provenance: BusCaptureProvenance): Promise<boolean> {
  const contents = serializeBusCapture(rows, provenance);
  if (new TextEncoder().encode(contents).length > MAX_EXPORT_BYTES) {
    throw new Error("Bus monitor capture exceeds the 16 MiB export limit");
  }
  if (isTauri()) {
    return invoke<boolean>("save_bus_monitor_capture", { contents });
  }
  downloadLocalJson(contents, "bus-monitor-capture.json");
  return true;
}
