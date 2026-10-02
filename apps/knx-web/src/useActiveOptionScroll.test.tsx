/** Tests active-option scrolling for inactive and empty lists. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { useActiveOptionScroll } from "./useActiveOptionScroll";

it("does not scroll an inactive list and safely releases an empty list", async () => {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  const previous = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollIntoView");
  const scroll = vi.fn();
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: scroll });
  const items = [0, 1];
  function List({ active, empty = false }: { active: boolean; empty?: boolean }) {
    const ref = useActiveOptionScroll(0, empty ? [] : items, active);
    return <ul>{!empty && <li ref={ref}>Active</li>}</ul>;
  }
  try {
    await act(async () => root.render(<List active={false} />));
    expect(scroll).not.toHaveBeenCalled();
    await act(async () => root.render(<List active />));
    expect(scroll).toHaveBeenCalledOnce();
    await act(async () => root.render(<List active={false} />));
    expect(scroll).toHaveBeenCalledOnce();
    await act(async () => root.render(<List active empty />));
    expect(scroll).toHaveBeenCalledOnce();
  } finally {
    await act(async () => root.unmount());
    host.remove();
    if (previous) Object.defineProperty(HTMLElement.prototype, "scrollIntoView", previous);
    else delete (HTMLElement.prototype as { scrollIntoView?: unknown }).scrollIntoView;
  }
});
