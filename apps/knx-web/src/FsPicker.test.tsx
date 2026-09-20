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
