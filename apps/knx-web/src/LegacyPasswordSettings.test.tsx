/** LegacyPasswordSettings: shows whether a password is remembered and forgets it. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import LegacyPasswordSettings from "./LegacyPasswordSettings";

const apiMock = {
  legacyPasswordStatus: vi.fn(),
  forgetLegacyPassword: vi.fn(),
};
const access = {
  status: () => apiMock.legacyPasswordStatus(),
  forget: () => apiMock.forgetLegacyPassword(),
};

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

async function render() {
  host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<LegacyPasswordSettings access={access} />);
  });
  return root;
}

const forgetButton = () =>
  Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === "Forget the remembered password");

describe("LegacyPasswordSettings", () => {
  it("forgets a remembered password on request", async () => {
    apiMock.legacyPasswordStatus.mockResolvedValueOnce({ available: true, remembered: true, problem: null });
    apiMock.forgetLegacyPassword.mockResolvedValueOnce({
      available: true, remembered: false, problem: null, forgot: true,
    });
    const root = await render();
    expect(host!.textContent).toContain("A password is remembered.");
    await act(async () => forgetButton()!.click());
    expect(apiMock.forgetLegacyPassword).toHaveBeenCalledTimes(1);
    expect(host!.textContent).toContain("The remembered password was forgotten.");
    expect(forgetButton()).toBeUndefined();
    root.unmount();
  });

  it("offers nothing to forget when none is remembered or there is nowhere to keep one", async () => {
    apiMock.legacyPasswordStatus.mockResolvedValueOnce({ available: true, remembered: false, problem: null });
    let root = await render();
    expect(host!.textContent).toContain("No password is remembered.");
    expect(forgetButton()).toBeUndefined();
    root.unmount();
    host!.remove();
    apiMock.legacyPasswordStatus.mockResolvedValueOnce({ available: false, remembered: false, problem: null });
    root = await render();
    expect(host!.textContent).toContain("no configuration directory");
    root.unmount();
  });

  it("shows why a remembered file is unusable and still lets it be forgotten", async () => {
    apiMock.legacyPasswordStatus.mockResolvedValueOnce({
      available: true, remembered: true, problem: "mode 0644; it must be 0600",
    });
    const root = await render();
    expect(host!.textContent).toContain("mode 0644; it must be 0600");
    expect(forgetButton()).toBeDefined();
    root.unmount();
  });

  it("reports a failed status request", async () => {
    apiMock.legacyPasswordStatus.mockRejectedValueOnce(new Error("offline"));
    const root = await render();
    expect(host!.querySelector('[role="alert"]')!.textContent).toBe(
      "The password setting could not be read or changed.",
    );
    root.unmount();
  });
});
