/** Opening, addressing and focusing the second window that hosts the diagnostic companion. */
// apps/knx-web/src/diagnosticsWindow.ts
//
// The companion is not a separate build, a separate bundle or a separate
// origin: it is the same document, loaded a second time with `?view=
// diagnostics`, which `main.tsx` branches on. That choice is what makes the
// rest of this stage possible — same origin means one `localStorage` (the
// only cross-window channel available, see `busContext.ts`), one `/api`
// base, and one already-shipped stylesheet, on both platforms.
//
// Two platforms, one URL:
// - Plain browser: `window.open`, with a fixed window name so pressing the
//   button twice focuses the existing window instead of stacking a second
//   one. A `null` return is the browser's popup blocker, which is a normal
//   outcome to report, not an error to swallow (codex-goal.md's "blockierte
//   Popups").
// - Tauri desktop: `@tauri-apps/api/webviewWindow`. The desktop shell loads
//   an ordinary `http://` URL (`apps/knx-desktop/src-tauri/src/lib.rs`), so
//   the same URL works unchanged; the window needs its own capability entry
//   (`apps/knx-desktop/src-tauri/capabilities/diagnostics.json`), because
//   `capabilities/default.json:4` scopes the default one to `["main"]`.
//
// If neither path yields a window, nothing is lost: the monitor and the log
// remain exactly where they were in the main window. The companion is an
// addition, never a relocation.

import { isTauri } from "./filePicker";

/// The `?view=` value that selects the companion. One value, one place.
export const COMPANION_VIEW = "diagnostics";

/// Query parameter name, and the browser window name / Tauri window label.
/// The label must match `a-zA-Z-/:_` (Tauri's `WebviewLabel`), which this
/// does.
const VIEW_PARAM = "view";
const COMPANION_LABEL = "knxbench-diagnostics";
const TAURI_LABEL = "diagnostics";

/// Whether this document is the companion. Takes the search string rather
/// than reading `window.location` itself so `main.tsx`'s branch and its
/// test look at the same function.
export function isCompanionView(search: string): boolean {
  return new URLSearchParams(search).get(VIEW_PARAM) === COMPANION_VIEW;
}

/// The companion's URL for a given document location. Deliberately rebuilds
/// the query from scratch rather than appending: the companion takes no
/// other parameters, and carrying over whatever the main window happened to
/// have would make the companion's address depend on the main window's
/// history. The hash is dropped for the same reason.
export function companionUrl(href: string): string {
  const url = new URL(href);
  url.search = `?${VIEW_PARAM}=${COMPANION_VIEW}`;
  url.hash = "";
  return url.toString();
}

/// What happened when the button was pressed. `"focused"` is distinguished
/// from `"opened"` so the UI can say "it is already open, over there"
/// instead of implying a second window just appeared; `"blocked"` is
/// distinguished from `"failed"` because only one of the two has an obvious
/// user fix (allow popups for this origin).
export type CompanionOpenResult = "opened" | "focused" | "blocked" | "failed";

/// The browser window this module opened, if any. Module-level because it
/// is a property of the document, not of any component: a remounted button
/// must still know the companion it opened a minute ago is still up.
/// `window.open` with the same name returns that same window rather than a
/// new one, so this reference is what distinguishes `"opened"` from
/// `"focused"` — the return value alone cannot.
let browserCompanion: Window | null = null;

async function openTauriCompanion(url: string): Promise<CompanionOpenResult> {
  const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
  const existing = await WebviewWindow.getByLabel(TAURI_LABEL);
  if (existing) {
    await existing.setFocus();
    return "focused";
  }
  const created = new WebviewWindow(TAURI_LABEL, {
    url,
    title: "KNXBench — Diagnostics",
    width: 1100,
    height: 760,
  });
  // `new WebviewWindow` is fire-and-forget: creation happens on the Rust
  // side and reports back as an event. Waiting for one of the two settles
  // this promise on the real outcome instead of an optimistic guess — a
  // missing capability, for instance, surfaces here as `tauri://error`
  // rather than as a window that silently never appears.
  return await new Promise<CompanionOpenResult>((resolve) => {
    void created.once("tauri://created", () => resolve("opened"));
    void created.once("tauri://error", () => resolve("failed"));
  });
}

function openBrowserCompanion(url: string): CompanionOpenResult {
  const alreadyOpen = browserCompanion !== null && !browserCompanion.closed;
  const opened = window.open(url, COMPANION_LABEL);
  if (opened === null) {
    // Popup blocked. The previously opened window (if any) is unaffected.
    return "blocked";
  }
  browserCompanion = opened;
  opened.focus();
  return alreadyOpen ? "focused" : "opened";
}

/// Opens (or focuses) the companion window for the current document.
export async function openCompanionWindow(href: string): Promise<CompanionOpenResult> {
  const url = companionUrl(href);
  try {
    if (isTauri()) return await openTauriCompanion(url);
    return openBrowserCompanion(url);
  } catch {
    // A thrown error here means the platform refused outright (no webview
    // permission, a sandboxed frame). Reported, never rethrown into a
    // click handler.
    return "failed";
  }
}

/// Whether a main window exists to return to at all. Asked separately from
/// `focusMainWindow` so the companion can disable and explain its return
/// button up front, instead of offering a control that quietly does
/// nothing when the companion was reopened from a bookmark.
export async function canFocusMainWindow(): Promise<boolean> {
  try {
    if (isTauri()) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      return (await WebviewWindow.getByLabel("main")) !== null;
    }
    return window.opener !== null && !window.opener.closed;
  } catch {
    return false;
  }
}

/// Brings the editing window back to the front, from the companion.
/// codex-goal.md asks for "Rückweg zum Hauptfenster" explicitly, and the
/// companion has no navigation of its own to offer instead.
export async function focusMainWindow(): Promise<boolean> {
  try {
    if (isTauri()) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const main = await WebviewWindow.getByLabel("main");
      if (!main) return false;
      await main.setFocus();
      return true;
    }
    // `window.opener` is `null` if the companion was reloaded from a
    // bookmark or opened by hand — there is simply no main window to
    // return to, which the caller renders as a disabled control rather
    // than a button that does nothing.
    if (window.opener && !window.opener.closed) {
      window.opener.focus();
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

/// Test seam only: resets the module-level reference described above.
export function resetCompanionWindowRef(): void {
  browserCompanion = null;
}
