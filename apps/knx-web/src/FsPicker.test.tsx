/** Tests for the filesystem picker's keyboard file choice and focus return on Escape. */
// @vitest-environment happy-dom
import { act } from "react";
import { afterEach, expect, it, vi } from "vitest";
import { openMountPicker } from "./FsPicker";
afterEach(() => { vi.restoreAllMocks(); document.body.innerHTML = ""; });
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
