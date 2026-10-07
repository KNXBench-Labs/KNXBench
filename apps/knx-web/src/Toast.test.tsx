/** Tests for ToastStack's dismiss handler and the error-only English-text disclosure. */
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

async function renderStack(toasts: ToastEntry[], onDismiss: (id: number) => void = vi.fn()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<ToastStack toasts={toasts} onDismiss={onDismiss} />);
  });
  return root;
}

describe("ToastStack", () => {
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
});
