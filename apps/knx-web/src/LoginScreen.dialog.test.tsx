/** Pins the LoginScreen dialog against the shared dialog checklist (WP-49.4c). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { expect, it, vi } from "vitest";
import * as api from "./api";
import { setNativeValue } from "./test/comboboxChecklist";
import { describeDialogChecklist, type DialogAdapter } from "./test/dialogChecklist";

vi.mock("./api", () => ({ login: vi.fn(), errorStatus: vi.fn(), errorMessage: String }));
import LoginScreen, { type LoginReason } from "./LoginScreen";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;
const panel = () => host!.querySelector<HTMLElement>('[role="dialog"]');

/**
 * Not an Overlay: the screen has no opener, no Escape, no trap and no isolation of its own. `AuthGate`
 * makes the application behind it inert (pinned in AuthGate.test.tsx), and nothing can dismiss it.
 */
const adapter = (reason: LoginReason): DialogAdapter => ({
  name: `LoginScreen (${reason})`,
  async mount(parentKeyDown) {
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<div onKeyDown={parentKeyDown}>
      <main><button>Behind</button></main>
      <LoginScreen reason={reason} onAuthenticated={vi.fn()} />
    </div>));
  },
  async unmount() {
    await act(async () => root!.unmount());
    host!.remove(); host = undefined; root = undefined;
  },
  async open() { /* shown on mount */ },
  dialog: () => panel()!,
  isOpen: () => panel() !== null,
  background: () => [host!.querySelector<HTMLElement>("main")!],
  initialFocus: () => host!.querySelector<HTMLElement>("#login-password")!,
  // The submit button is disabled while the field is empty, so the password field is the only stop.
  tabStops: () => [...panel()!.querySelectorAll<HTMLElement>("input, button")].filter(el => !(el as HTMLInputElement).disabled),
  nameSource: "aria-labelledby",
  describedBy: reason === "initial" ? "none" : "wired",
  shell: "standalone",
  escape: { closes: false, prevented: false, stopped: false },
});

for (const reason of ["initial", "expired", "signedOut"] satisfies LoginReason[]) describeDialogChecklist(adapter(reason));

it("wires the password field to the error only while an error is shown", async () => {
  vi.mocked(api.login).mockRejectedValue(new Error("nope"));
  const a = adapter("initial");
  await a.mount(() => {});
  try {
    const field = host!.querySelector<HTMLInputElement>("#login-password")!;
    expect(field.hasAttribute("aria-describedby"), "no error yet").toBe(false);
    expect(field.getAttribute("aria-invalid")).toBe("false");
    await act(async () => setNativeValue(field, "wrong"));
    await act(async () => { host!.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })); });
    const error = host!.querySelector("#login-error");
    expect(error?.getAttribute("role")).toBe("alert");
    expect(field.getAttribute("aria-describedby")).toBe("login-error");
    expect(field.getAttribute("aria-invalid")).toBe("true");
  } finally { await a.unmount(); }
});
