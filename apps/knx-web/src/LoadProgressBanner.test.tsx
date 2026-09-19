/** Tests the load banner: a measurement only where one exists, and where a failure happened. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import LoadProgressBanner from "./LoadProgressBanner";
import type { LoadProgressSnapshot } from "./api";
import { FLAVOUR_INTERVAL_MS } from "./loadFlavour";
import { messages as enMessages } from "./messages/en";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  document.documentElement.removeAttribute("data-motion-level");
  vi.useRealTimers();
});

function snapshot(overrides: Partial<LoadProgressSnapshot> = {}): LoadProgressSnapshot {
  return {
    operationId: 1,
    kind: "import",
    source: "villa.knxproj",
    phase: "parseTopology",
    completed: null,
    total: null,
    status: "running",
    error: null,
    clientToken: "own-token",
    ...overrides,
  };
}

async function render(source: string, snap: LoadProgressSnapshot | null, random?: () => number) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<LoadProgressBanner source={source} snapshot={snap} random={random} />);
  });
  return root;
}

const bar = () => host!.querySelector('[role="progressbar"]')!;
const flavour = () => host!.querySelector(".load-progress-flavour");

/** A source that makes the shuffle a left rotation by one, so the banner
 * opens on entry 02 and goes on to 03, 04, … — pinned in
 * `loadFlavour.test.ts`, relied on here. */
const rotateByOne = () => 0;

describe("LoadProgressBanner", () => {
  it("announces itself politely, so a phase change reaches a screen reader", async () => {
    const root = await render("villa.knxproj", snapshot());
    const region = host!.querySelector('[role="status"]')!;
    expect(region.getAttribute("aria-live")).toBe("polite");
    expect(region.textContent).toContain(enMessages["loadProgress.phase.parseTopology"]);
    root.unmount();
  });

  it("says something true before the first poll has answered", async () => {
    const root = await render("villa.knxproj", null);
    expect(host!.textContent).toContain("villa.knxproj");
    expect(host!.textContent).toContain(enMessages["loadProgress.phase.starting"]);
    expect(bar().hasAttribute("aria-valuenow")).toBe(false);
    root.unmount();
  });

  it("is indeterminate — no aria-valuenow at all — while no count exists", async () => {
    const root = await render("villa.knxproj", snapshot());
    expect(bar().hasAttribute("aria-valuenow")).toBe(false);
    expect(host!.querySelector(".load-progress-bar")!.getAttribute("data-indeterminate")).toBe("true");
    expect(host!.querySelector(".load-progress-count")).toBeNull();
    root.unmount();
  });

  it("shows the real count, and only the real count, when the server sent one", async () => {
    const root = await render(
      "villa.knxproj",
      snapshot({ phase: "collectContainerEntries", completed: 9, total: 36 }),
    );
    expect(bar().getAttribute("aria-valuenow")).toBe("25");
    expect(host!.querySelector(".load-progress-bar")!.getAttribute("style")).toContain("width: 25%");
    expect(host!.textContent).toContain("9 of 36");
    root.unmount();
  });

  // Round 1 (F10) put `aria-live="off"` on the progressbar and on the
  // count paragraph, so only the phase in the `role="status"` region gets
  // announced — a screen reader given the bar or the count directly would
  // read every one of a 38-entry count as its own event. The re-review
  // removed both attributes and every other test here stayed green, which
  // is exactly the failure mode an assertion exists to catch.
  it("silences the progressbar and the count from their own aria-live (F10)", async () => {
    const root = await render(
      "villa.knxproj",
      snapshot({ phase: "collectContainerEntries", completed: 9, total: 36 }),
    );
    expect(bar().getAttribute("aria-live")).toBe("off");
    expect(host!.querySelector(".load-progress-count")!.getAttribute("aria-live")).toBe("off");
    root.unmount();
  });

  it("names the phase a failed load died in, and keeps the server's message", async () => {
    const root = await render(
      "villa.knxproj",
      snapshot({ phase: "parseTopology", status: "failed", error: "invalid Zip archive" }),
    );
    expect(host!.textContent).toContain("Could not load villa.knxproj");
    expect(host!.textContent).toContain(enMessages["loadProgress.phase.parseTopology"]);
    expect(host!.textContent).toContain("invalid Zip archive");
    // A bar for something that is not running would be a bar that never
    // finishes; the failure keeps the phase, not the progress indicator.
    expect(host!.querySelector('[role="progressbar"]')).toBeNull();
    root.unmount();
  });

  it("shows an unknown phase as itself rather than hiding a version mismatch", async () => {
    const root = await render("villa.knxproj", snapshot({ phase: "parseSomethingNewer" }));
    expect(host!.textContent).toContain("parseSomethingNewer");
    root.unmount();
  });

  // Task 26. The flavour line is decoration and must stay decoration: it
  // sits beside the truthful phase, never in place of it, it is silent to
  // assistive technology, and its timer advances a joke and nothing else.
  it("shows a flavour line beside the real phase, never instead of it", async () => {
    const root = await render("villa.knxproj", snapshot(), rotateByOne);
    expect(flavour()!.textContent).toBe(enMessages["loadProgress.flavour.02"]);
    // The thing the banner exists to say is still on screen, still first.
    const heading = host!.querySelector(".load-progress-heading")!;
    expect(heading.textContent).toContain(enMessages["loadProgress.phase.parseTopology"]);
    expect(heading.contains(flavour())).toBe(false);
    root.unmount();
  });

  it("keeps the flavour line out of the accessibility tree and the live region", async () => {
    const root = await render("villa.knxproj", snapshot(), rotateByOne);
    expect(flavour()!.getAttribute("aria-hidden")).toBe("true");
    expect(flavour()!.getAttribute("aria-live")).toBe("off");
    root.unmount();
  });

  it("takes its sequence from the injected source, in order", async () => {
    vi.useFakeTimers();
    const root = await render("villa.knxproj", snapshot(), rotateByOne);
    expect(flavour()!.textContent).toBe(enMessages["loadProgress.flavour.02"]);
    await act(async () => void vi.advanceTimersByTime(FLAVOUR_INTERVAL_MS));
    expect(flavour()!.textContent).toBe(enMessages["loadProgress.flavour.03"]);
    await act(async () => void vi.advanceTimersByTime(FLAVOUR_INTERVAL_MS));
    expect(flavour()!.textContent).toBe(enMessages["loadProgress.flavour.04"]);
    root.unmount();
  });

  it("rotates without touching the bar, the count or data-indeterminate", async () => {
    vi.useFakeTimers();
    const root = await render(
      "villa.knxproj",
      snapshot({ phase: "collectContainerEntries", completed: 9, total: 36 }),
      rotateByOne,
    );
    const before = {
      valuenow: bar().getAttribute("aria-valuenow"),
      indeterminate: host!.querySelector(".load-progress-bar")!.getAttribute("data-indeterminate"),
      style: host!.querySelector(".load-progress-bar")!.getAttribute("style"),
      count: host!.querySelector(".load-progress-count")!.textContent,
    };
    await act(async () => void vi.advanceTimersByTime(FLAVOUR_INTERVAL_MS * 4));
    expect(flavour()!.textContent).toBe(enMessages["loadProgress.flavour.06"]);
    expect(bar().getAttribute("aria-valuenow")).toBe(before.valuenow);
    expect(host!.querySelector(".load-progress-bar")!.getAttribute("data-indeterminate")).toBe(before.indeterminate);
    expect(host!.querySelector(".load-progress-bar")!.getAttribute("style")).toBe(before.style);
    expect(host!.querySelector(".load-progress-count")!.textContent).toBe(before.count);
    root.unmount();
  });

  it("makes no jokes on a failed load, and stops the timer that was making them", async () => {
    vi.useFakeTimers();
    const root = await render("villa.knxproj", snapshot(), rotateByOne);
    expect(flavour()).not.toBeNull();
    expect(vi.getTimerCount()).toBe(1);
    await act(async () => {
      root.render(
        <LoadProgressBanner
          source="villa.knxproj"
          snapshot={snapshot({ status: "failed", error: "invalid Zip archive" })}
          random={rotateByOne}
        />,
      );
    });
    expect(flavour()).toBeNull();
    // Not merely un-rendered: the interval is gone. A `clearInterval` that
    // stopped being called would leave this at 1.
    expect(vi.getTimerCount()).toBe(0);
    await act(async () => void vi.advanceTimersByTime(FLAVOUR_INTERVAL_MS * 10));
    expect(flavour()).toBeNull();
    root.unmount();
  });

  it("stops the rotation timer on unmount", async () => {
    vi.useFakeTimers();
    const root = await render("villa.knxproj", snapshot(), rotateByOne);
    expect(vi.getTimerCount()).toBe(1);
    await act(async () => void root.unmount());
    expect(vi.getTimerCount()).toBe(0);
  });

  // A line that swaps every 1 800 ms is motion, even though nothing
  // slides. "Motion: off" freezes it on its first entry — still a joke,
  // just a patient one — exactly as it stops the bar's shuttle dead.
  it("freezes the flavour line, without hiding it, under Motion: off", async () => {
    vi.useFakeTimers();
    document.documentElement.setAttribute("data-motion-level", "off");
    const root = await render("villa.knxproj", snapshot(), rotateByOne);
    expect(flavour()!.textContent).toBe(enMessages["loadProgress.flavour.02"]);
    expect(vi.getTimerCount()).toBe(0);
    await act(async () => void vi.advanceTimersByTime(FLAVOUR_INTERVAL_MS * 5));
    expect(flavour()!.textContent).toBe(enMessages["loadProgress.flavour.02"]);
    root.unmount();
  });

  it("says 'opening' for a native project and 'importing' for an ETS one", async () => {
    let root = await render("villa.knxdb", snapshot({ kind: "open", phase: "openStore" }));
    expect(host!.textContent).toContain("Opening villa.knxdb");
    root.unmount();
    host!.remove();

    root = await render("villa.knxproj", snapshot());
    expect(host!.textContent).toContain("Importing villa.knxproj");
    root.unmount();
  });
});
