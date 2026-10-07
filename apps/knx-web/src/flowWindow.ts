/** Opens one dedicated flow view per source lifetime, reporting blocked and failed creation. */
import { isTauri } from "./filePicker";
import { validFlowOwner } from "./flowChannel";
import type { CompanionOpenResult } from "./diagnosticsWindow";

let browserFlow: { owner: string; window: Window } | null = null;
export const isFlowWindow = (search: string) => new URLSearchParams(search).get("view") === "flow";
export function flowWindowUrl(href: string, owner: string): string {
  if (!validFlowOwner(owner)) throw new Error("invalid flow source");
  const url = new URL(href); url.search = ""; url.hash = "";
  url.searchParams.set("view", "flow"); url.searchParams.set("source", owner);
  return url.toString();
}
export async function openFlowWindow(href: string, owner: string): Promise<CompanionOpenResult> {
  try {
    const url = flowWindowUrl(href, owner);
    if (isTauri()) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const label = `flow-${owner}`;
      const existing = await WebviewWindow.getByLabel(label);
      if (existing) { await existing.setFocus(); return "focused"; }
      const created = new WebviewWindow(label, { url, title: "KNXBench — Telegram flow", width: 1440, height: 900 });
      return await new Promise<CompanionOpenResult>(resolve => {
        let settled = false;
        const unlisten: (() => void)[] = [];
        const finish = (result: CompanionOpenResult) => {
          if (settled) return; settled = true; clearTimeout(timer);
          for (const stop of unlisten) stop(); resolve(result);
        };
        const timer = setTimeout(() => finish("failed"), 10000);
        for (const [event, result] of [["tauri://created", "opened"], ["tauri://error", "failed"]] as const) {
          void created.once(event, () => finish(result)).then(stop => { if (settled) stop(); else unlisten.push(stop); }, () => finish("failed"));
        }
      });
    }
    if (browserFlow?.owner === owner && !browserFlow.window.closed) { browserFlow.window.focus(); return "focused"; }
    const opened = window.open(url, "knxbench-flow", "popup,width=1440,height=900");
    if (!opened) return "blocked";
    browserFlow = { owner, window: opened }; opened.focus(); return "opened";
  } catch { return "failed"; }
}
export function resetFlowWindowForTests() { browserFlow = null; }
