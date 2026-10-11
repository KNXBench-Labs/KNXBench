/** Branding on every authentication entry point, without an authenticated API read. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { version } from "../package.json";
import brandMark from "./assets/knxbench-mark.png";
vi.mock("./api", () => ({ login: vi.fn(), errorStatus: vi.fn(), errorMessage: String }));
import LoginScreen, { type LoginReason } from "./LoginScreen";

it.each(["initial", "expired", "signedOut"] satisfies LoginReason[])("shows the existing logo on %s login", async (reason) => {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  try {
    await act(() => root.render(<LoginScreen reason={reason} onAuthenticated={vi.fn()} />));
    const image = host.querySelector<HTMLImageElement>(".login-logo");
    expect(image).not.toBeNull();
    expect(image!.getAttribute("src")).toBe(brandMark);
    expect(image!.alt).toBe("");
    expect(host.querySelector(".brand-mark")).toBeNull();
    expect(host.querySelector("h1")!.textContent).toBe("KNXBench");
    expect(document.activeElement).toBe(host.querySelector("input"));
  } finally { await act(() => root.unmount()); host.remove(); }
});
it("shows the frontend build version before authentication", async () => {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  try {
    await act(() => root.render(<LoginScreen reason="initial" onAuthenticated={vi.fn()} />));
    expect(host.querySelector(".login-version")?.textContent).toBe(`v${version}`);
  } finally { await act(() => root.unmount()); host.remove(); }
});
