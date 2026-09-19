/** Tests for ToastStack's dismiss handler and the error-only English-text disclosure (fix round 2, B4). */
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
  it("shows the messageIsEnglish disclosure on an error toast", async () => {
    const root = await renderStack([{ id: 1, kind: "error", message: "The bus objects: boom" }]);
    expect(host!.textContent).toContain(enMessages["toast.error.messageIsEnglish"]);
    root.unmount();
  });

  it("does not show the disclosure on a fun toast, which quotes no server text", async () => {
    const root = await renderStack([{ id: 1, kind: "fun", message: "Happy New Year!" }]);
    expect(host!.textContent).not.toContain(enMessages["toast.error.messageIsEnglish"]);
    root.unmount();
  });
});
