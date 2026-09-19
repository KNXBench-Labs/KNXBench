/** Tests the load banner: a measurement only where one exists, and where a failure happened. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import LoadProgressBanner from "./LoadProgressBanner";
import type { LoadProgressSnapshot } from "./api";
import { messages as enMessages } from "./messages/en";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
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
    ...overrides,
  };
}

async function render(source: string, snap: LoadProgressSnapshot | null) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<LoadProgressBanner source={source} snapshot={snap} />);
  });
  return root;
}

const bar = () => host!.querySelector('[role="progressbar"]')!;

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
