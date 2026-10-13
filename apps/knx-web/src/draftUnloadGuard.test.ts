/** Browser reload/tab-close guard while an unapplied editor draft exists. */
// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { installDraftUnloadGuard } from "./draftUnloadGuard";

function unload(target: EventTarget): Event {
  const event = new Event("beforeunload", { cancelable: true });
  target.dispatchEvent(event);
  return event;
}

describe("installDraftUnloadGuard", () => {
  it("asks the browser to confirm only while a draft exists", () => {
    const target = new EventTarget();
    let draft = false;
    const remove = installDraftUnloadGuard(target, () => draft);
    expect(unload(target).defaultPrevented).toBe(false);
    draft = true;
    const guarded = unload(target);
    expect(guarded.defaultPrevented).toBe(true);
    // Older engines only honour a truthy legacy returnValue.
    expect((guarded as Event & { returnValue?: unknown }).returnValue).toBeTruthy();
    remove();
    expect(unload(target).defaultPrevented).toBe(false);
  });

  it("reads the draft state at unload time, not at install time", () => {
    const target = new EventTarget();
    const drafts = { notes: false, instructions: false };
    installDraftUnloadGuard(target, () => drafts.notes || drafts.instructions);
    drafts.instructions = true;
    expect(unload(target).defaultPrevented).toBe(true);
  });
});
