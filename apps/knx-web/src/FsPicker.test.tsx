/** Tests for the filesystem picker's keyboard choice, focus return on Escape, and size bounds. */
// @vitest-environment happy-dom
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { act } from "react";
import { afterEach, expect, it, vi } from "vitest";
import { openMountPicker } from "./FsPicker";
import { resetSessionListenersForTests, subscribeSessionExpired } from "./session";
afterEach(() => { vi.restoreAllMocks(); document.body.innerHTML = ""; resetSessionListenersForTests(); });
it("provides a keyboard file choice and Escape returns focus to its opener", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ok:true,json:async()=>[{name:"Example.knxproj",is_dir:false}]}));
  const opener=document.createElement("button"); document.body.append(opener); opener.focus();
  let result!: Promise<string|null>;
  await act(async()=>{result=openMountPicker([{name:"Project",extensions:["knxproj"]}]);});
  const dialog=document.querySelector<HTMLElement>('[role="dialog"]');
  expect(dialog).not.toBeNull();
  expect(dialog!.querySelector('button')?.textContent).toContain("Example.knxproj");
  await act(async()=>dialog!.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true})));
  expect(await result).toBeNull(); expect(document.activeElement).toBe(opener);
  vi.unstubAllGlobals();
});

// T28/F2. The defect was a size, and happy-dom applies no author
// stylesheet, so a rendered box would measure 0 whatever the CSS says.
// The honest place to assert a rule is the rule. Same trick as
// `motionGuard.test.ts` and `themeTokens.test.ts`, which read this file
// for the same reason.
function ruleFor(selector: string): string {
  const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "styles.css"), "utf8");
  const start = css.indexOf(`\n${selector} {`);
  expect(start, `no \`${selector} {\` block in styles.css`).toBeGreaterThan(-1);
  const end = css.indexOf("}", start);
  expect(end, `unterminated ${selector} block`).toBeGreaterThan(start);
  return css.slice(start, end);
}

it("bounds the picker on both axes and leaves the scrolling to its list", () => {
  const picker = ruleFor(".fs-picker");
  // The finding itself: a `min-width` floor that a phone-width viewport
  // cannot honour, and no ceiling of any kind.
  expect(picker).not.toMatch(/min-width/);
  expect(picker).toMatch(/\n\s*width:\s*min\(/);
  expect(picker).toMatch(/\n\s*max-width:/);
  // `align-self` keeps the dialog as tall as its content instead of
  // stretching to the cap; the cap is only the ceiling.
  expect(picker).toMatch(/\n\s*align-self:/);
  expect(picker).toMatch(/\n\s*max-height:\s*min\(/);

  const list = ruleFor(".fs-picker-list");
  expect(list).toMatch(/\n\s*flex:\s*1 1 auto/);
  // Without this a flex item refuses to shrink below its content, and the
  // dialog grows past its own `max-height` instead of the list scrolling.
  expect(list).toMatch(/\n\s*min-height:\s*0/);
  expect(list).toMatch(/\n\s*overflow-y:\s*auto/);
  // A long file name wraps. Truncating it would hide the one part of a
  // path that tells two backups apart.
  expect(list).toMatch(/\n\s*overflow-wrap:\s*anywhere/);
  expect(list).not.toMatch(/text-overflow/);
});

// T01b. `/api/fs/*` sits behind the same session guard as every other
// `/api/` route (ADR-0026), but these two helpers hold their own `fetch`
// rather than going through `api.ts`'s `request()`. Before this, an expired
// session printed "authentication required" inside the modal and left the
// user in a file browser with no way forward — the first thing they touch
// after coming back to an idle tab.
it("reports a refused listing as an ended session and closes itself", async () => {
  const expiries = vi.fn();
  resetSessionListenersForTests();
  subscribeSessionExpired(expiries);
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
    ok: false,
    status: 401,
    statusText: "Unauthorized",
    json: async () => ({ error: "authentication required" }),
  }));

  let result!: Promise<string | null>;
  await act(async () => { result = openMountPicker([]); });

  expect(expiries).toHaveBeenCalledTimes(1);
  // Closed, not left holding a focus trap the login screen cannot reach:
  // the picker is mounted on its own root outside `AuthGate`'s `inert`
  // subtree, so it has to get out of the way by itself.
  expect(await result).toBeNull();
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  vi.unstubAllGlobals();
});

it("treats an ordinary failure as an ordinary failure", async () => {
  const expiries = vi.fn();
  resetSessionListenersForTests();
  subscribeSessionExpired(expiries);
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
    ok: false,
    status: 403,
    statusText: "Forbidden",
    json: async () => ({ error: "outside the allowed roots" }),
  }));

  let result!: Promise<string | null>;
  await act(async () => { result = openMountPicker([]); });

  // A path the server refuses to list is not a session that ended, and
  // bouncing the user to a login screen over one would be a worse lie than
  // the error line.
  expect(expiries).not.toHaveBeenCalled();
  const dialog = document.querySelector<HTMLElement>('[role="dialog"]');
  expect(dialog).not.toBeNull();
  expect(dialog!.querySelector(".field-error")?.textContent).toContain("outside the allowed roots");
  await act(async () => dialog!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(await result).toBeNull();
  vi.unstubAllGlobals();
});

function fileDragTransfer(files: File[], protectedMode = false): DataTransfer {
  return {
    types: ["Files"],
    get files() {
      if (protectedMode) throw new Error("dragover must not read dropped files");
      return files;
    },
    dropEffect: "none",
  } as unknown as DataTransfer;
}

it("uploads each selected local file sequentially, then shows the uploaded directory without selecting one", async () => {
  let finishFirstUpload!: () => void;
  const firstUpload = new Promise<{ ok: boolean; json: () => Promise<{ path: string }> }>((resolve) => {
    finishFirstUpload = () => resolve({ ok: true, json: async () => ({ path: "uploads/first.knxproj" }) });
  });
  const fetchMock = vi.fn()
    .mockResolvedValueOnce({ ok: true, json: async () => [] })
    .mockImplementationOnce(() => firstUpload)
    .mockResolvedValueOnce({ ok: true, json: async () => ({ path: "uploads/second.knxproj" }) })
    .mockResolvedValue({ ok: true, json: async () => [
      { name: "first.knxproj", is_dir: false },
      { name: "second.knxproj", is_dir: false },
    ] });
  vi.stubGlobal("fetch", fetchMock);

  let result!: Promise<string | null>;
  await act(async () => { result = openMountPicker([]); });
  const input = document.querySelector<HTMLInputElement>(".fs-picker-upload-input")!;
  expect(input.multiple).toBe(true);
  Object.defineProperty(input, "files", {
    configurable: true,
    value: [
      new File(["first"], "first.knxproj"),
      new File(["second"], "second.knxproj"),
    ],
  });

  await act(async () => {
    input.dispatchEvent(new Event("change", { bubbles: true }));
    await Promise.resolve();
  });

  expect(fetchMock.mock.calls.filter(([url]) => url === "/api/fs/upload")).toHaveLength(1);
  await act(async () => {
    finishFirstUpload();
    await Promise.resolve();
    await Promise.resolve();
  });

  const uploads = fetchMock.mock.calls.filter(([url]) => url === "/api/fs/upload");
  expect(uploads).toHaveLength(2);
  expect(uploads.map(([, init]) => (init as RequestInit).method)).toEqual(["POST", "POST"]);
  expect(fetchMock.mock.calls.some(([url]) => url === "/api/fs/list?path=uploads")).toBe(true);
  expect(document.querySelector('[role="status"]')?.textContent).toBe("Uploaded 2 files. Choose one to open.");
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  await act(async () => document.querySelector<HTMLElement>('[role="dialog"]')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(await result).toBeNull();
  vi.unstubAllGlobals();
});

it("accepts only file drags in protected mode and consumes their files only at drop", async () => {
  const fetchMock = vi.fn()
    .mockResolvedValueOnce({ ok: true, json: async () => [] })
    .mockResolvedValueOnce({ ok: true, json: async () => ({ path: "uploads/dropped.knxproj" }) })
    .mockResolvedValue({ ok: true, json: async () => [{ name: "dropped.knxproj", is_dir: false }] });
  vi.stubGlobal("fetch", fetchMock);

  await act(async () => { openMountPicker([]); });
  const label = document.querySelector<HTMLElement>(".fs-picker-upload")!;
  const protectedTransfer = fileDragTransfer([], true);
  const dragOver = Object.assign(new Event("dragover", { bubbles: true, cancelable: true }), { dataTransfer: protectedTransfer });
  await act(async () => label.dispatchEvent(dragOver));
  expect(dragOver.defaultPrevented).toBe(true);
  expect(protectedTransfer.dropEffect).toBe("copy");
  expect(label.dataset.dropReady).toBe("true");

  const rejection = Object.assign(new Event("dragover", { bubbles: true, cancelable: true }), {
    dataTransfer: { types: ["text/plain"], dropEffect: "none" } as unknown as DataTransfer,
  });
  await act(async () => label.dispatchEvent(rejection));
  expect(rejection.defaultPrevented).toBe(false);
  expect(label.dataset.dropReady).toBe("true");
  await act(async () => label.dispatchEvent(new Event("dragleave", { bubbles: true })));
  expect(label.dataset.dropReady).toBeUndefined();

  const dropped = new File(["dropped"], "dropped.knxproj");
  const drop = Object.assign(new Event("drop", { bubbles: true, cancelable: true }), {
    dataTransfer: fileDragTransfer([dropped]),
  });
  await act(async () => {
    label.dispatchEvent(drop);
    await Promise.resolve();
    await Promise.resolve();
  });
  expect(drop.defaultPrevented).toBe(true);
  expect(fetchMock.mock.calls.filter(([url]) => url === "/api/fs/upload")).toHaveLength(1);
  await act(async () => document.querySelector<HTMLElement>('[role="dialog"]')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  vi.unstubAllGlobals();
});

it("reports a failed filename without claiming a partially uploaded batch succeeded", async () => {
  const fetchMock = vi.fn()
    .mockResolvedValueOnce({ ok: true, json: async () => [] })
    .mockResolvedValueOnce({ ok: true, json: async () => ({ path: "uploads/kept.knxproj" }) })
    .mockResolvedValueOnce({ ok: false, status: 422, statusText: "Unprocessable", json: async () => ({ error: "not a project" }) })
    .mockResolvedValue({ ok: true, json: async () => [{ name: "kept.knxproj", is_dir: false }] });
  vi.stubGlobal("fetch", fetchMock);

  await act(async () => { openMountPicker([]); });
  const input = document.querySelector<HTMLInputElement>(".fs-picker-upload-input")!;
  Object.defineProperty(input, "files", {
    configurable: true,
    value: [new File(["kept"], "kept.knxproj"), new File(["bad"], "bad.txt")],
  });
  await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));

  expect(fetchMock.mock.calls.filter(([url]) => url === "/api/fs/upload")).toHaveLength(2);
  expect(document.querySelector('[role="status"]')).toBeNull();
  expect(document.querySelector(".field-error")?.textContent).toContain("bad.txt");
  expect(document.querySelector(".field-error")?.textContent).toContain("1 of 2");
  await act(async () => document.querySelector<HTMLElement>('[role="dialog"]')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  vi.unstubAllGlobals();
});
