/** Tests for independently collapsible navigation sections retaining child state. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it } from "vitest";
import NavigationSection from "./NavigationSection";

it("hides the entire content without unmounting or changing sibling sections", async () => {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<><NavigationSection title="Project"><input defaultValue="draft" /></NavigationSection>
    <NavigationSection title="Diagnostics"><span>Log</span></NavigationSection></>));
  const buttons = host.querySelectorAll<HTMLButtonElement>("button");
  const input = host.querySelector("input")!;
  expect([...buttons].map(button => button.getAttribute("aria-expanded"))).toEqual(["false", "false"]);
  await act(async () => { buttons[0].click(); buttons[1].click(); });
  input.value = "retained";
  await act(async () => buttons[0].click());
  expect(buttons[0].getAttribute("aria-expanded")).toBe("false");
  expect(input.closest("[hidden]")).not.toBeNull();
  expect(buttons[1].getAttribute("aria-expanded")).toBe("true");
  await act(async () => buttons[0].click());
  expect(host.querySelector("input")).toBe(input);
  expect(input.value).toBe("retained");
  expect(input.closest("[hidden]")).toBeNull();
  await act(async () => root.unmount()); host.remove();
});

it("only opens for a new explicit reveal, not when a reveal is cleared", async () => {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  const render = (revealKey: number | undefined) => root.render(<NavigationSection title="Project" revealKey={revealKey}><input /></NavigationSection>);
  await act(async () => render(undefined));
  await act(async () => host.querySelector<HTMLButtonElement>("button")!.click());
  await act(async () => render(1));
  expect(host.querySelector("button")!.getAttribute("aria-expanded")).toBe("true");
  await act(async () => host.querySelector<HTMLButtonElement>("button")!.click());
  await act(async () => render(undefined));
  expect(host.querySelector("button")!.getAttribute("aria-expanded")).toBe("false");
  await act(async () => render(2));
  expect(host.querySelector("button")!.getAttribute("aria-expanded")).toBe("true");
  await act(async () => root.unmount()); host.remove();
});
