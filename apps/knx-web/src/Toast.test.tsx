/** Tests for ToastStack: dismiss/open, disclosure, exit animation, pause and queue notes. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import ToastStack from "./Toast";
import type { ToastEntry } from "./toast";
import { messages as enMessages } from "./messages/en";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
});

async function renderStack(
  toasts: ToastEntry[],
  onDismiss: (id: number) => void = vi.fn(),
  onExited: (id: number) => void = vi.fn(),
) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<ToastStack toasts={toasts} onDismiss={onDismiss} onExited={onExited} />);
  });
  return root;
}

describe("ToastStack", () => {
  it("opens the exact toast from its message button without triggering dismissal", async () => {
    const onOpen = vi.fn(), onDismiss = vi.fn();
    host = document.createElement("div"); document.body.append(host);
    const root = createRoot(host);
    await act(() => root.render(<ToastStack toasts={[{ id: 8, kind: "error", message: "Message", serverText: false }]} onDismiss={onDismiss} onOpen={onOpen} />));
    const open = host.querySelector<HTMLButtonElement>(".toast-open");
    expect(open).not.toBeNull();
    await act(() => open!.click());
    expect(onOpen).toHaveBeenCalledExactlyOnceWith(8);
    expect(onDismiss).not.toHaveBeenCalled();
    await act(() => host!.querySelector<HTMLButtonElement>(`[aria-label="${enMessages["toast.dismiss"]}"]`)!.click());
    expect(onDismiss).toHaveBeenCalledExactlyOnceWith(8);
    expect(onOpen).toHaveBeenCalledTimes(1);
    await act(() => root.unmount());
  });

  it("shows the messageIsEnglish disclosure on an error toast quoting server text", async () => {
    const root = await renderStack([
      { id: 1, kind: "error", message: "The bus objects: boom", serverText: true },
    ]);
    expect(host!.textContent).toContain(enMessages["toast.error.messageIsEnglish"]);
    root.unmount();
  });

  it("does not show the disclosure on an error toast with serverText: false", async () => {
    const root = await renderStack([
      { id: 1, kind: "error", message: "The popup blocker got in the way", serverText: false },
    ]);
    expect(host!.textContent).toContain("The popup blocker got in the way");
    expect(host!.textContent).not.toContain(enMessages["toast.error.messageIsEnglish"]);
    root.unmount();
  });

  it("does not show the disclosure on a fun toast, which quotes no server text", async () => {
    const root = await renderStack([{ id: 1, kind: "fun", message: "Happy New Year!", serverText: false }]);
    expect(host!.textContent).not.toContain(enMessages["toast.error.messageIsEnglish"]);
    root.unmount();
  });

  it("renders an achievement toast with its label, title, description and tier", async () => {
    const root = await renderStack([
      {
        id: 3,
        kind: "achievement",
        message: "Night Shift",
        serverText: false,
        achievement: { title: "Night Shift", description: "Saved at night.", tier: "gold", glyph: "moon" },
      },
    ]);
    const toast = host!.querySelector(".toast--achievement")!;
    expect(toast.getAttribute("role")).toBe("status");
    expect(toast.textContent).toContain(enMessages["achievements.unlockedLabel"]);
    expect(toast.textContent).toContain("Night Shift");
    expect(toast.textContent).toContain("Saved at night.");
    expect(toast.querySelector(".achievement-badge--gold")).not.toBeNull();
    expect(toast.textContent).not.toContain(enMessages["toast.error.messageIsEnglish"]);
    root.unmount();
  });

  it("renders an achievement summary toast without a badge", async () => {
    const root = await renderStack([
      { id: 4, kind: "achievement", message: "+2 more achievements unlocked", serverText: false },
    ]);
    const toast = host!.querySelector(".toast--achievement")!;
    expect(toast.textContent).toContain("+2 more achievements unlocked");
    expect(toast.querySelector(".achievement-badge")).toBeNull();
    root.unmount();
  });

  it("uses the same message typography for standard and achievement toasts", async () => {
    const root = await renderStack([
      { id: 1, kind: "fun", message: "Saved.", serverText: false },
      { id: 2, kind: "error", message: "Save refused.", serverText: true },
    ]);
    expect([...host!.querySelectorAll(".toast-title")].map((el) => el.textContent))
      .toEqual(["Saved.", "Save refused."]);
    root.unmount();
  });

  it("calls onDismiss with the toast's id when its close button is clicked", async () => {
    const onDismiss = vi.fn();
    const root = await renderStack(
      [{ id: 7, kind: "error", message: "The bus objects: boom", serverText: true }],
      onDismiss,
    );
    const button = host!.querySelector("button")!;
    await act(async () => {
      button.click();
    });
    expect(onDismiss).toHaveBeenCalledWith(7);
    root.unmount();
  });

  it("marks a leaving toast and reports the end of its exit animation, not of any other", async () => {
    const onExited = vi.fn();
    const leaving: ToastEntry = { id: 7, kind: "achievement", message: "Bus Master", serverText: false, leaving: true };
    const root = await renderStack([leaving], vi.fn(), onExited);
    const toast = host!.querySelector(".toast--achievement")!;
    expect(toast.classList.contains("toast--leaving")).toBe(true);
    await act(async () => {
      toast.dispatchEvent(Object.assign(new Event("animationend", { bubbles: true }), { animationName: "knx-toast-in" }));
    });
    expect(onExited).not.toHaveBeenCalled();
    await act(async () => {
      toast.dispatchEvent(Object.assign(new Event("animationend", { bubbles: true }), { animationName: "knx-achievement-out" }));
    });
    expect(onExited).toHaveBeenCalledExactlyOnceWith(7);
    root.unmount();
  });

  it("ignores an exit animation that bubbles up from a child element", async () => {
    const onExited = vi.fn();
    const leaving: ToastEntry = { id: 9, kind: "achievement", message: "Bus Master", serverText: false, leaving: true,
      achievement: { title: "Bus Master", description: "All of them.", tier: "legendary", glyph: "trophy" } };
    const root = await renderStack([leaving], vi.fn(), onExited);
    const badge = host!.querySelector(".toast--achievement .achievement-badge")!;
    expect(badge).not.toBeNull();
    await act(async () => {
      badge.dispatchEvent(Object.assign(new Event("animationend", { bubbles: true }), { animationName: "knx-achievement-out" }));
    });
    expect(onExited).not.toHaveBeenCalled();
    root.unmount();
  });

  it("pauses on hover and focus, and resumes focus only when it leaves the card", async () => {
    const onPause = vi.fn(), onResume = vi.fn();
    host = document.createElement("div"); document.body.append(host);
    const root = createRoot(host);
    await act(() => root.render(<ToastStack toasts={[{ id: 4, kind: "error", message: "Refused", serverText: false }]}
      onDismiss={vi.fn()} onOpen={vi.fn()} onPause={onPause} onResume={onResume} />));
    const card = host.querySelector<HTMLElement>(".toast")!;
    const open = host.querySelector<HTMLButtonElement>(".toast-open")!;
    const dismiss = host.querySelector<HTMLButtonElement>(".toast-dismiss")!;
    await act(async () => { card.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: document.body })); });
    expect(onPause).toHaveBeenLastCalledWith(4, "hover");
    await act(async () => { card.dispatchEvent(new MouseEvent("mouseout", { bubbles: true, relatedTarget: document.body })); });
    expect(onResume).toHaveBeenLastCalledWith(4, "hover");
    onResume.mockClear();
    await act(async () => open.focus());
    expect(onPause).toHaveBeenLastCalledWith(4, "focus");
    // Moving focus to the card's own close button keeps the pause.
    await act(async () => dismiss.focus());
    expect(onResume).not.toHaveBeenCalled();
    await act(async () => dismiss.blur());
    expect(onResume).toHaveBeenCalledExactlyOnceWith(4, "focus");
    await act(() => root.unmount());
  });

  it("states how many cards wait and how many the queue retired", async () => {
    host = document.createElement("div"); document.body.append(host);
    const root = createRoot(host);
    await act(() => root.render(<ToastStack toasts={[{ id: 1, kind: "fun", message: "Saved", serverText: false }]}
      onDismiss={vi.fn()} pendingCount={2} queueDropped={1} />));
    const notes = [...host.querySelectorAll(".toast-queue-note")].map((note) => note.textContent);
    expect(notes).toEqual([
      enMessages["toast.pending.other"].replace("{count}", "2"),
      enMessages["toast.queueDropped.one"].replace("{count}", "1"),
    ]);
    await act(() => root.unmount());
  });
});
